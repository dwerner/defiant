#![doc(html_root_url = "https://docs.rs/prost-derive/0.14.1")]
// The `quote!` macro requires deep recursion.
#![recursion_limit = "4096"]

extern crate alloc;
extern crate proc_macro;

use anyhow::{bail, Context, Error};
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    punctuated::Punctuated, Data, DataEnum, DataStruct, DeriveInput, Expr, ExprLit, Fields,
    FieldsNamed, FieldsUnnamed, Ident, Index, Variant,
};
use syn::{Attribute, Lit, Meta, MetaNameValue, Path, Token};

mod field;
use crate::field::Field;

use self::field::set_option;

/// Checks if a type uses arena allocation (has references with lifetimes, slices, etc.)
fn type_uses_arena(ty: &syn::Type) -> bool {
    match ty {
        // &'a T or &'a [T] - uses arena
        syn::Type::Reference(_) => true,
        // Path types (including Option<T>, custom types like Value<'arena>, etc.)
        syn::Type::Path(type_path) => {
            // Check if this is Option<T> - recurse into T
            if let Some(last_seg) = type_path.path.segments.last() {
                if last_seg.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            return type_uses_arena(inner_type);
                        }
                    }
                    return false;
                }

                // For any path type, check if it has lifetime arguments
                // e.g., value::Kind<'arena>, Struct<'arena>
                if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                    for arg in &args.args {
                        if matches!(arg, syn::GenericArgument::Lifetime(_)) {
                            return true;
                        }
                        // Also recurse into type arguments (for nested generics)
                        if let syn::GenericArgument::Type(ty) = arg {
                            if type_uses_arena(ty) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
        // Primitive types
        _ => false,
    }
}

/// Converts a slice type `&'arena [T]` to `BumpVec<'arena, T>`
fn slice_to_bumpvec(field_type: &syn::Type, prost_path: &Path) -> TokenStream {
    // Try to parse as a reference to a slice
    if let syn::Type::Reference(type_ref) = field_type {
        if let syn::Type::Slice(type_slice) = &*type_ref.elem {
            let elem_type = &type_slice.elem;
            let lifetime = &type_ref.lifetime;
            return quote!(#prost_path::arena::BumpVec<#lifetime, #elem_type>);
        }
    }
    // If not a slice, return original type
    quote!(#field_type)
}

/// Extracts the base type path from a message field type, stripping lifetimes
/// Examples:
/// - `Option<&'arena Address<'arena>>` → Address
/// - `code_generator_response::File<'arena>` → code_generator_response::File
/// - `&'arena [descriptor_proto::ExtensionRange<'arena>]` → descriptor_proto::ExtensionRange
fn extract_type_path(field_type: &syn::Type) -> syn::Path {
    match field_type {
        // ::core::option::Option<T> or Option<T> → extract T
        // Handles both qualified (::core::option::Option) and unqualified (Option) forms
        syn::Type::Path(type_path) if type_path.path.segments.last().unwrap().ident == "Option"
            && (type_path.path.segments.len() == 1  // Unqualified: Option<T>
                || type_path.path.segments.iter().any(|s| s.ident == "option" || s.ident == "core")) => {  // Qualified: ::core::option::Option<T>
            if let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments.last().unwrap().arguments {
                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                    return extract_type_path(inner_type);
                }
            }
            panic!("Failed to extract type from Option");
        }
        // &'arena T or &'arena [T] → extract T
        syn::Type::Reference(type_ref) => {
            // Check if it's a slice &[T]
            if let syn::Type::Slice(type_slice) = &*type_ref.elem {
                return extract_type_path(&type_slice.elem);
            }
            // Otherwise it's a reference &T, recurse to extract T
            extract_type_path(&type_ref.elem)
        }
        // T<'arena> → extract T (preserving module path, stripping lifetimes)
        syn::Type::Path(type_path) => {
            // Clone the path and strip lifetimes from segments
            let mut path = type_path.path.clone();
            for segment in &mut path.segments {
                segment.arguments = syn::PathArguments::None;
            }
            path
        }
        _ => panic!("Unsupported message field type"),
    }
}

/// Checks if a nested message type in a field type has a lifetime parameter
/// Examples:
/// - `&'arena [Address<'arena>]` → true (Address has <'arena>)
/// - `&'arena [ReservedRange]` → false (ReservedRange has no lifetime)
/// - `Option<&'arena FileOptions<'arena>>` → true
fn nested_message_uses_arena(field_type: &syn::Type) -> bool {
    match field_type {
        // Option<T> → check T
        syn::Type::Path(type_path) => {
            if let Some(last_seg) = type_path.path.segments.last() {
                if last_seg.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            return nested_message_uses_arena(inner_type);
                        }
                    }
                }
            }
            false
        }
        // &'arena T or &'arena [T] → check T
        syn::Type::Reference(type_ref) => {
            match &*type_ref.elem {
                // &'arena [T] → check if T has lifetime args
                syn::Type::Slice(type_slice) => {
                    // Check if the element type is a Path with lifetime arguments
                    if let syn::Type::Path(elem_path) = &*type_slice.elem {
                        // Check if any segment has lifetime arguments
                        elem_path.path.segments.iter().any(|seg| {
                            matches!(&seg.arguments, syn::PathArguments::AngleBracketed(args)
                                if args.args.iter().any(|arg| matches!(arg, syn::GenericArgument::Lifetime(_))))
                        })
                    } else {
                        false
                    }
                }
                // &'arena T → check if T has lifetime args
                syn::Type::Path(elem_path) => {
                    elem_path.path.segments.iter().any(|seg| {
                        matches!(&seg.arguments, syn::PathArguments::AngleBracketed(args)
                            if args.args.iter().any(|arg| matches!(arg, syn::GenericArgument::Lifetime(_))))
                    })
                }
                _ => false
            }
        }
        _ => false
    }
}

fn try_message(input: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse2(input)?;
    let ident = input.ident;

    let Attributes {
        skip_debug,
        prost_path,
    } = Attributes::new(input.attrs)?;

    let variant_data = match input.data {
        Data::Struct(variant_data) => variant_data,
        Data::Enum(..) => bail!("Message can not be derived for an enum"),
        Data::Union(..) => bail!("Message can not be derived for a union"),
    };

    // Check if the struct has a _phantom field
    let has_phantom_field = match &variant_data.fields {
        Fields::Named(fields) => fields.named.iter().any(|f| {
            f.ident.as_ref().map(|id| id == "_phantom").unwrap_or(false)
        }),
        _ => false,
    };

    // Check if the struct actually uses arena allocation by examining field types
    let needs_arena = match &variant_data.fields {
        Fields::Named(fields) => fields.named.iter().any(|f| type_uses_arena(&f.ty)),
        Fields::Unnamed(fields) => fields.unnamed.iter().any(|f| type_uses_arena(&f.ty)),
        Fields::Unit => false,
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (is_struct, fields) = match variant_data {
        DataStruct {
            fields: Fields::Named(FieldsNamed { named: fields, .. }),
            ..
        } => (true, fields.into_iter().collect()),
        DataStruct {
            fields:
                Fields::Unnamed(FieldsUnnamed {
                    unnamed: fields, ..
                }),
            ..
        } => (false, fields.into_iter().collect()),
        DataStruct {
            fields: Fields::Unit,
            ..
        } => (false, Vec::new()),
    };

    let mut next_tag: u32 = 1;
    let mut fields_with_types: Vec<(TokenStream, syn::Type, field::Field)> = Vec::new();

    for (i, syn_field) in fields.into_iter().enumerate() {
        let field_ident = syn_field.ident.clone().map(|x| quote!(#x)).unwrap_or_else(|| {
            let index = Index {
                index: i as u32,
                span: Span::call_site(),
            };
            quote!(#index)
        });
        let field_type = syn_field.ty.clone();

        match Field::new(syn_field.attrs, Some(next_tag)) {
            Ok(Some(field)) => {
                next_tag = field.tags().iter().max().map(|t| t + 1).unwrap_or(next_tag);
                fields_with_types.push((field_ident, field_type, field));
            }
            Ok(None) => {},
            Err(err) => {
                bail!(err.context(format!("invalid message field {ident}.{field_ident}")))
            }
        }
    }

    // Extract just (ident, field) for existing code
    let mut fields: Vec<(TokenStream, field::Field)> = fields_with_types
        .iter()
        .map(|(ident, _ty, field)| (ident.clone(), field.clone()))
        .collect();

    // We want Debug to be in declaration order
    let unsorted_fields = fields.clone();

    // Sort the fields by tag number so that fields will be encoded in tag order.
    // TODO: This encodes oneof fields in the position of their lowest tag,
    // regardless of the currently occupied variant, is that consequential?
    // See: https://protobuf.dev/programming-guides/encoding/#order
    fields.sort_by_key(|(_, field)| field.tags().into_iter().min().unwrap());
    let fields = fields;

    if let Some(duplicate_tag) = fields
        .iter()
        .flat_map(|(_, field)| field.tags())
        .duplicates()
        .next()
    {
        bail!(
            "message {} has multiple fields with tag {}",
            ident,
            duplicate_tag
        )
    };

    let encoded_len: Vec<_> = fields
        .iter()
        .map(|(field_ident, field)| field.encoded_len(&prost_path, quote!(self.#field_ident)))
        .collect();

    let encode: Vec<_> = fields
        .iter()
        .map(|(field_ident, field)| field.encode(&prost_path, quote!(self.#field_ident)))
        .collect();

    // Generate View-specific encode/encoded_len that uses _ref variants for repeated strings/bytes
    let view_encoded_len_stmts: Vec<_> = fields
        .iter()
        .map(|(field_ident, field)| {
            use crate::field::Field;
            // For repeated string/bytes fields in View, use encoded_len_repeated_ref
            match field {
                Field::Scalar(ref scalar_field) => {
                    use crate::field::scalar::{Ty, Kind};
                    if matches!(scalar_field.kind, Kind::Repeated) {
                        let tag = scalar_field.tag;
                        match scalar_field.ty {
                            Ty::String => quote! {
                                #prost_path::encoding::string::encoded_len_repeated_ref(#tag, self.#field_ident)
                            },
                            Ty::Bytes(_) => quote! {
                                #prost_path::encoding::bytes::encoded_len_repeated_ref(#tag, self.#field_ident)
                            },
                            _ => field.encoded_len(&prost_path, quote!(self.#field_ident)),
                        }
                    } else {
                        field.encoded_len(&prost_path, quote!(self.#field_ident))
                    }
                },
                _ => field.encoded_len(&prost_path, quote!(self.#field_ident)),
            }
        })
        .collect();

    let view_encode_stmts: Vec<_> = fields
        .iter()
        .map(|(field_ident, field)| {
            use crate::field::Field;
            // For repeated string/bytes fields in View, use encode_repeated_ref
            match field {
                Field::Scalar(ref scalar_field) => {
                    use crate::field::scalar::{Ty, Kind};
                    if matches!(scalar_field.kind, Kind::Repeated) {
                        let tag = scalar_field.tag;
                        match scalar_field.ty {
                            Ty::String => quote! {
                                #prost_path::encoding::string::encode_repeated_ref(#tag, self.#field_ident, buf);
                            },
                            Ty::Bytes(_) => quote! {
                                #prost_path::encoding::bytes::encode_repeated_ref(#tag, self.#field_ident, buf);
                            },
                            _ => field.encode(&prost_path, quote!(self.#field_ident)),
                        }
                    } else {
                        field.encode(&prost_path, quote!(self.#field_ident))
                    }
                },
                _ => field.encode(&prost_path, quote!(self.#field_ident)),
            }
        })
        .collect();

    let merge = fields_with_types.iter().map(|(field_ident, field_type, field)| {
        use crate::field::Field;

        let tags = field.tags().into_iter().map(|tag| quote!(#tag));
        let tags = Itertools::intersperse(tags, quote!(|));

        // Special handling for message fields
        if let Field::Message(ref msg_field) = field {
            use crate::field::Label;

            // Extract the type path and build the Message companion path
            let mut base_path = extract_type_path(field_type);
            // Append "Message" to the last segment
            if let Some(last_seg) = base_path.segments.last_mut() {
                let type_name = last_seg.ident.to_string();
                last_seg.ident = Ident::new(&format!("{}Message", type_name), Span::call_site());
            }
            let builder_type_name = base_path;

            let merge_code = match msg_field.label {
                Label::Optional => quote! {
                    #prost_path::encoding::check_wire_type(#prost_path::encoding::WireType::LengthDelimited, wire_type)
                        .map_err(|mut error| {
                            error.push(STRUCT_NAME, stringify!(#field_ident));
                            error
                        })?;
                    ctx.limit_reached()
                        .map_err(|mut error| {
                            error.push(STRUCT_NAME, stringify!(#field_ident));
                            error
                        })?;
                    let mut builder = #builder_type_name::new_in(arena);
                    #prost_path::encoding::merge_loop(
                        &mut builder,
                        buf,
                        ctx.enter_recursion(),
                        |builder, buf, ctx| {
                            let (tag, wire_type) = #prost_path::encoding::decode_key(buf)?;
                            builder.merge_field(tag, wire_type, buf, ctx)
                        }
                    ).map_err(|mut error| {
                        error.push(STRUCT_NAME, stringify!(#field_ident));
                        error
                    })?;
                    let view = builder.into_view();
                    self.#field_ident = Some(&*arena.alloc(view));
                    Ok(())
                },
                Label::Required => quote! {
                    #prost_path::encoding::check_wire_type(#prost_path::encoding::WireType::LengthDelimited, wire_type)
                        .map_err(|mut error| {
                            error.push(STRUCT_NAME, stringify!(#field_ident));
                            error
                        })?;
                    ctx.limit_reached()
                        .map_err(|mut error| {
                            error.push(STRUCT_NAME, stringify!(#field_ident));
                            error
                        })?;
                    let mut builder = #builder_type_name::new_in(arena);
                    #prost_path::encoding::merge_loop(
                        &mut builder,
                        buf,
                        ctx.enter_recursion(),
                        |builder, buf, ctx| {
                            let (tag, wire_type) = #prost_path::encoding::decode_key(buf)?;
                            builder.merge_field(tag, wire_type, buf, ctx)
                        }
                    ).map_err(|mut error| {
                        error.push(STRUCT_NAME, stringify!(#field_ident));
                        error
                    })?;
                    let view = builder.into_view();
                    self.#field_ident = &*arena.alloc(view);
                    Ok(())
                },
                Label::Repeated => quote! {
                    #prost_path::encoding::check_wire_type(#prost_path::encoding::WireType::LengthDelimited, wire_type)
                        .map_err(|mut error| {
                            error.push(STRUCT_NAME, stringify!(#field_ident));
                            error
                        })?;
                    ctx.limit_reached()
                        .map_err(|mut error| {
                            error.push(STRUCT_NAME, stringify!(#field_ident));
                            error
                        })?;
                    let mut builder = #builder_type_name::new_in(arena);
                    #prost_path::encoding::merge_loop(
                        &mut builder,
                        buf,
                        ctx.enter_recursion(),
                        |builder, buf, ctx| {
                            let (tag, wire_type) = #prost_path::encoding::decode_key(buf)?;
                            builder.merge_field(tag, wire_type, buf, ctx)
                        }
                    ).map_err(|mut error| {
                        error.push(STRUCT_NAME, stringify!(#field_ident));
                        error
                    })?;
                    self.#field_ident.push(builder.into_view());
                    Ok(())
                },
            };

            quote! {
                #(#tags)* => {
                    #merge_code
                },
            }
        } else {
            // Regular field - use existing merge logic
            let merge = field.merge(&prost_path, quote!(value));

            quote! {
                #(#tags)* => {
                    let mut value = &mut self.#field_ident;
                    #merge.map_err(|mut error| {
                        error.push(STRUCT_NAME, stringify!(#field_ident));
                        error
                    })
                },
            }
        }
    });

    let struct_name = if fields.is_empty() {
        quote!()
    } else {
        quote!(
            const STRUCT_NAME: &'static str = stringify!(#ident);
        )
    };

    let clear = fields
        .iter()
        .map(|(field_ident, field)| field.clear(quote!(self.#field_ident)));

    let _default = if is_struct {
        let default = fields.iter().map(|(field_ident, field)| {
            let value = field.default(&prost_path);
            quote!(#field_ident: #value,)
        });
        quote! {#ident {
            #(#default)*
        }}
    } else {
        let default = fields.iter().map(|(_, field)| {
            let value = field.default(&prost_path);
            quote!(#value,)
        });
        quote! {#ident (
            #(#default)*
        )}
    };

    let methods = fields
        .iter()
        .flat_map(|(field_ident, field)| field.methods(&prost_path, field_ident))
        .collect::<Vec<_>>();
    let methods = if methods.is_empty() {
        quote!()
    } else {
        quote! {
            #[allow(dead_code)]
            impl #impl_generics #ident #ty_generics #where_clause {
                #(#methods)*
            }
        }
    };

    // Generate *Message struct name
    let message_ident = Ident::new(&format!("{}Message", ident), ident.span());

    // Generate *Message struct fields (arena + all fields)
    let message_fields = if is_struct {
        let field_defs = fields_with_types.iter().map(|(field_ident, field_type, field)| {
            use crate::field::Field;

            // For repeated fields, convert &[T] → BumpVec<T>
            // For map fields (ArenaMap<K,V>), convert to BumpVec<(K,V)>
            let message_field_type = if matches!(field, Field::Map(_)) {
                // Extract K and V from ArenaMap<'arena, K, V>
                let extracted_type = if let syn::Type::Path(type_path) = field_type {
                    if let Some(last_seg) = type_path.path.segments.last() {
                        if last_seg.ident == "ArenaMap" {
                            if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                                // Skip first argument (lifetime), take K and V
                                let type_args: Vec<_> = args.args.iter().skip(1).collect();
                                if type_args.len() == 2 {
                                    let key_ty = &type_args[0];
                                    let val_ty = &type_args[1];
                                    Some(quote!(#prost_path::arena::BumpVec<'arena, (#key_ty, #val_ty)>))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                extracted_type.unwrap_or_else(|| quote!(#field_type))
            } else if field.is_repeated() {
                slice_to_bumpvec(field_type, &prost_path)
            } else {
                quote!(#field_type)
            };
            quote!(#field_ident: #message_field_type)
        });

        if needs_arena {
            quote! {
                arena: &'arena #prost_path::Arena,
                #(#field_defs,)*
            }
        } else {
            quote! {
                #(#field_defs,)*
            }
        }
    } else {
        // Tuple structs not yet implemented
        quote!()
    };

    // Generate *Message struct definition
    let message_struct = quote! {
        #[allow(dead_code)]
        pub struct #message_ident #ty_generics {
            #message_fields
        }
    };

    // Generate new_in() constructor and setter methods for *Message
    let message_impl = if is_struct {
        let field_inits = fields_with_types.iter().map(|(field_ident, _field_type, field)| {
            if field.is_repeated() {
                // Repeated fields initialize with arena.new_vec()
                quote!(#field_ident: arena.new_vec())
            } else {
                // Non-repeated fields use default values
                let default_value = field.default(&prost_path);
                quote!(#field_ident: #default_value)
            }
        });

        // Generate setter methods (set_* for singular, push_* for repeated)
        let setter_methods = fields_with_types.iter().map(|(field_ident, field_type, field)| {
            use crate::field::{Field, Ty};

            let ident_string = field_ident.to_string();
            let method_name_str = ident_string.strip_prefix("r#").unwrap_or(&ident_string);

            if field.is_repeated() {
                // push_* method for repeated fields
                let push_method = Ident::new(&format!("push_{}", method_name_str), Span::call_site());

                match field {
                    Field::Scalar(ref scalar_field) => {
                        match scalar_field.ty {
                            Ty::String => {
                                quote! {
                                    pub fn #push_method(&mut self, value: &str) {
                                        let allocated = self.arena.alloc_str(value);
                                        self.#field_ident.push(allocated);
                                    }
                                }
                            }
                            Ty::Int32 | Ty::Int64 | Ty::Uint32 | Ty::Uint64 |
                            Ty::Sint32 | Ty::Sint64 | Ty::Fixed32 | Ty::Fixed64 |
                            Ty::Sfixed32 | Ty::Sfixed64 | Ty::Float | Ty::Double | Ty::Bool => {
                                let rust_type = scalar_field.ty.rust_type(&prost_path);
                                quote! {
                                    pub fn #push_method(&mut self, value: #rust_type) {
                                        self.#field_ident.push(value);
                                    }
                                }
                            }
                            Ty::Bytes(_) => {
                                quote! {
                                    pub fn #push_method(&mut self, value: &[u8]) {
                                        let allocated = self.arena.alloc_slice_copy(value);
                                        self.#field_ident.push(allocated);
                                    }
                                }
                            }
                            _ => quote!()
                        }
                    }
                    Field::Message(_) => {
                        // For repeated message fields, extract view type from &[T<'arena>]
                        let view_type_path = extract_type_path(field_type);
                        // Check if the nested message type actually uses arena
                        let type_with_lifetime = if nested_message_uses_arena(field_type) {
                            quote!(#view_type_path<'arena>)
                        } else {
                            quote!(#view_type_path)
                        };
                        quote! {
                            pub fn #push_method(&mut self, value: #type_with_lifetime) {
                                self.#field_ident.push(value);
                            }
                        }
                    }
                    _ => quote!()
                }
            } else {
                // set_* method for singular fields
                let set_method = Ident::new(&format!("set_{}", method_name_str), Span::call_site());

                match field {
                    Field::Scalar(ref scalar_field) => {
                        use crate::field::scalar::Kind;
                        let is_optional = matches!(scalar_field.kind, Kind::Optional(_));

                        match scalar_field.ty {
                            Ty::String => {
                                if is_optional {
                                    quote! {
                                        pub fn #set_method(&mut self, value: &str) {
                                            self.#field_ident = ::core::option::Option::Some(self.arena.alloc_str(value));
                                        }
                                    }
                                } else {
                                    quote! {
                                        pub fn #set_method(&mut self, value: &str) {
                                            self.#field_ident = self.arena.alloc_str(value);
                                        }
                                    }
                                }
                            }
                            Ty::Int32 | Ty::Int64 | Ty::Uint32 | Ty::Uint64 |
                            Ty::Sint32 | Ty::Sint64 | Ty::Fixed32 | Ty::Fixed64 |
                            Ty::Sfixed32 | Ty::Sfixed64 | Ty::Float | Ty::Double | Ty::Bool => {
                                let rust_type = scalar_field.ty.rust_type(&prost_path);
                                if is_optional {
                                    quote! {
                                        pub fn #set_method(&mut self, value: #rust_type) {
                                            self.#field_ident = ::core::option::Option::Some(value);
                                        }
                                    }
                                } else {
                                    quote! {
                                        pub fn #set_method(&mut self, value: #rust_type) {
                                            self.#field_ident = value;
                                        }
                                    }
                                }
                            }
                            Ty::Bytes(_) => {
                                if is_optional {
                                    quote! {
                                        pub fn #set_method(&mut self, value: &[u8]) {
                                            self.#field_ident = ::core::option::Option::Some(self.arena.alloc_slice_copy(value));
                                        }
                                    }
                                } else {
                                    quote! {
                                        pub fn #set_method(&mut self, value: &[u8]) {
                                            self.#field_ident = self.arena.alloc_slice_copy(value);
                                        }
                                    }
                                }
                            }
                            _ => quote!()
                        }
                    }
                    Field::Message(ref msg_field) => {
                        use crate::field::Label;
                        // For message fields, setter takes the View by value and handles arena allocation
                        let view_type_path = extract_type_path(field_type);
                        // Check if the nested message type actually uses arena
                        let type_with_lifetime = if nested_message_uses_arena(field_type) {
                            quote!(#view_type_path<'arena>)
                        } else {
                            quote!(#view_type_path)
                        };

                        match msg_field.label {
                            Label::Optional => quote! {
                                pub fn #set_method(&mut self, value: Option<#type_with_lifetime>) {
                                    self.#field_ident = value.map(|v| &*self.arena.alloc(v));
                                }
                            },
                            Label::Required => quote! {
                                pub fn #set_method(&mut self, value: #type_with_lifetime) {
                                    self.#field_ident = &*self.arena.alloc(value);
                                }
                            },
                            _ => quote!()  // Repeated uses push, not set
                        }
                    }
                    _ => quote!()
                }
            }
        });

        // Generate getter methods
        let getter_methods = fields_with_types.iter().map(|(field_ident, field_type, field)| {
            use crate::field::Field;

            // For getters, use the field identifier directly (preserving r# for keywords)
            let method_name = field_ident.clone();

            if matches!(field, Field::Map(_)) {
                // For map fields (ArenaMap<K,V>), return &[(K,V)]
                if let syn::Type::Path(type_path) = field_type {
                    if let Some(last_seg) = type_path.path.segments.last() {
                        if last_seg.ident == "ArenaMap" {
                            if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                                let type_args: Vec<_> = args.args.iter().skip(1).collect();
                                if type_args.len() == 2 {
                                    let key_ty = &type_args[0];
                                    let val_ty = &type_args[1];
                                    return quote! {
                                        pub fn #method_name(&self) -> &[(#key_ty, #val_ty)] {
                                            &self.#field_ident
                                        }
                                    };
                                }
                            }
                        }
                    }
                }
                // Fallback for map
                quote! {
                    pub fn #method_name(&self) -> &[_] {
                        &self.#field_ident
                    }
                }
            } else if field.is_repeated() {
                // For BumpVec, return slice reference
                // Extract the element type from &[T] to get T
                if let syn::Type::Reference(type_ref) = field_type {
                    if let syn::Type::Slice(type_slice) = &*type_ref.elem {
                        let elem_type = &type_slice.elem;
                        return quote! {
                            pub fn #method_name(&self) -> &[#elem_type] {
                                &self.#field_ident
                            }
                        };
                    }
                }
                // Fallback
                quote! {
                    pub fn #method_name(&self) -> &[_] {
                        &self.#field_ident
                    }
                }
            } else {
                // For singular fields
                use crate::field::Field;
                if matches!(field, Field::Oneof(_)) && needs_arena {
                    // For oneofs with arena types, return by reference to avoid move errors
                    quote! {
                        pub fn #method_name(&self) -> &#field_type {
                            &self.#field_ident
                        }
                    }
                } else {
                    // For Copy types and owned data, return by value
                    quote! {
                        pub fn #method_name(&self) -> #field_type {
                            self.#field_ident
                        }
                    }
                }
            }
        });

        // Generate into_view() method (converts BumpVec to arena slice)
        let into_view_field_inits: Vec<_> = fields_with_types.iter().map(|(field_ident, _field_type, field)| {
            use crate::field::Field;

            if matches!(field, Field::Map(_)) {
                // For map fields, sort by key and wrap in ArenaMap
                quote! {
                    #field_ident: {
                        let mut entries = self.#field_ident;
                        entries.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                        #prost_path::ArenaMap::new(entries.into_bump_slice())
                    }
                }
            } else if field.is_repeated() {
                // For repeated fields, convert BumpVec to arena slice
                quote!(#field_ident: self.#field_ident.into_bump_slice())
            } else {
                // For singular fields, move value
                quote!(#field_ident: self.#field_ident)
            }
        }).collect();

        // Add _phantom field initialization if the original struct has a _phantom field
        // (This happens for structs with lifetimes but only owned data)
        let phantom_init = if has_phantom_field {
            quote!(_phantom: ::core::marker::PhantomData,)
        } else {
            quote!()
        };

        if needs_arena {
            quote! {
                impl #impl_generics #message_ident #ty_generics #where_clause {
                    pub fn new_in(arena: &'arena #prost_path::Arena) -> Self {
                        Self {
                            arena,
                            #(#field_inits,)*
                        }
                    }

                    #(#setter_methods)*

                    #(#getter_methods)*

                    pub fn into_view(self) -> #ident #ty_generics {
                        #ident {
                            #(#into_view_field_inits,)*
                            #phantom_init
                        }
                    }

                    pub fn decode(mut buf: impl #prost_path::bytes::Buf, arena: &'arena #prost_path::Arena)
                        -> ::core::result::Result<#ident #ty_generics, #prost_path::DecodeError>
                    {
                        let mut message = Self::new_in(arena);
                        message.merge(buf)?;
                        // Convert BumpVecs to arena slices using into_bump_slice()
                        Ok(message.into_view())
                    }
                }
            }
        } else {
            // For primitive-only types, provide both new() and new_in(arena)
            // where new_in just ignores the arena parameter
            quote! {
                impl #impl_generics #message_ident #ty_generics #where_clause {
                    pub fn new() -> Self {
                        Self {
                            #(#field_inits,)*
                        }
                    }

                    // Provide new_in for compatibility with merge code
                    #[allow(unused_variables)]
                    pub fn new_in(arena: &#prost_path::Arena) -> Self {
                        Self::new()
                    }

                    #(#setter_methods)*

                    #(#getter_methods)*

                    pub fn into_view(self) -> #ident #ty_generics {
                        #ident {
                            #(#into_view_field_inits,)*
                            #phantom_init
                        }
                    }

                    pub fn decode(mut buf: impl #prost_path::bytes::Buf)
                        -> ::core::result::Result<#ident #ty_generics, #prost_path::DecodeError>
                    {
                        let mut message = Self::new();
                        message.merge(buf)?;
                        Ok(message.into_view())
                    }
                }
            }
        }
    } else {
        quote!()
    };

    // Generate internal methods for *Message (decode/encode infrastructure)
    // Note: *Message doesn't implement Message trait (not Send+Sync due to arena)
    let message_internal_impl = if is_struct {
        let arena_binding = if needs_arena {
            quote!(let arena = self.arena;)
        } else {
            quote!()
        };

        quote! {
            impl #impl_generics #message_ident #ty_generics #where_clause {
                #[allow(unused_variables)]
                pub fn merge_field(
                    &mut self,
                    tag: u32,
                    wire_type: #prost_path::encoding::wire_type::WireType,
                    buf: &mut impl #prost_path::bytes::Buf,
                    ctx: #prost_path::encoding::DecodeContext,
                ) -> ::core::result::Result<(), #prost_path::DecodeError>
                {
                    #arena_binding
                    #struct_name
                    match tag {
                        #(#merge)*
                        _ => #prost_path::encoding::skip_field(wire_type, tag, buf, ctx),
                    }
                }

                pub fn merge(&mut self, mut buf: impl #prost_path::bytes::Buf) -> ::core::result::Result<(), #prost_path::DecodeError> {
                    let ctx = #prost_path::encoding::DecodeContext::default();
                    while buf.has_remaining() {
                        let (tag, wire_type) = #prost_path::encoding::decode_key(&mut buf)?;
                        self.merge_field(tag, wire_type, &mut buf, ctx.clone())?;
                    }
                    Ok(())
                }
            }
        }
    } else {
        quote!()
    };

    // Generate standalone encode method for View
    // Generate Message impl for view types (arena-allocated messages)
    let view_message_impl = if needs_arena {
        quote! {
            impl #impl_generics #prost_path::Message<'arena> for #ident #ty_generics #where_clause {
                fn new_in(_arena: &'arena #prost_path::Arena) -> Self {
                    // Views should not be constructed via new_in()
                    // Use the builder's into_view() instead
                    unreachable!("Cannot create view directly - use builder")
                }

                #[allow(unused_variables)]
                fn encode_raw(&self, buf: &mut impl #prost_path::bytes::BufMut) {
                    use #prost_path::Message as _;
                    #(#view_encode_stmts)*
                }

                fn merge_field(
                    &mut self,
                    tag: u32,
                    wire_type: #prost_path::encoding::wire_type::WireType,
                    buf: &mut impl #prost_path::bytes::Buf,
                    arena: &'arena #prost_path::Arena,
                    ctx: #prost_path::encoding::DecodeContext,
                ) -> ::core::result::Result<(), #prost_path::DecodeError> {
                    Err(#prost_path::DecodeError::new("Cannot merge into immutable view - use builder"))
                }

                fn encoded_len(&self) -> usize {
                    use #prost_path::Message as _;
                    0 #(+ #view_encoded_len_stmts)*
                }

                fn clear(&mut self) {
                    // Views are immutable, cannot clear
                }
            }

            impl #impl_generics #ident #ty_generics #where_clause {
                pub fn encode(&self, buf: &mut impl #prost_path::bytes::BufMut) -> ::core::result::Result<(), #prost_path::EncodeError> {
                    let required = self.encoded_len();
                    let remaining = buf.remaining_mut();
                    if required > remaining {
                        return Err(#prost_path::EncodeError::new(required, remaining));
                    }
                    self.encode_raw(buf);
                    Ok(())
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #ident #ty_generics #where_clause {
                pub fn encode(&self, buf: &mut impl #prost_path::bytes::BufMut) -> ::core::result::Result<(), #prost_path::EncodeError> {
                    let required = self.encoded_len();
                    let remaining = buf.remaining_mut();
                    if required > remaining {
                        return Err(#prost_path::EncodeError::new(required, remaining));
                    }
                    self.encode_raw(buf);
                    Ok(())
                }

                #[allow(unused_variables)]
                pub fn encode_raw(&self, buf: &mut impl #prost_path::bytes::BufMut) {
                    #(#view_encode_stmts)*
                }

                pub fn encoded_len(&self) -> usize {
                    0 #(+ #view_encoded_len_stmts)*
                }
            }
        }
    };

    // Link View to Builder via MessageView trait (only for arena-allocated types)
    let message_view_impl = if needs_arena {
        quote! {
            impl #impl_generics #prost_path::MessageView<'arena> for #ident #ty_generics #where_clause {
                type Builder = #message_ident #ty_generics;
            }
        }
    } else {
        quote!()
    };

    let expanded = quote! {
        #message_struct
        #message_impl
        #message_internal_impl
        #view_message_impl
        #message_view_impl
    };
    let expanded = if skip_debug {
        expanded
    } else {
        let debugs = unsorted_fields.iter().map(|(field_ident, field)| {
            let wrapper = field.debug(&prost_path, quote!(self.#field_ident));
            let call = if is_struct {
                quote!(builder.field(stringify!(#field_ident), &wrapper))
            } else {
                quote!(builder.field(&wrapper))
            };
            quote! {
                 let builder = {
                     let wrapper = #wrapper;
                     #call
                 };
            }
        });
        let debug_builder = if is_struct {
            quote!(f.debug_struct(stringify!(#ident)))
        } else {
            quote!(f.debug_tuple(stringify!(#ident)))
        };
        quote! {
            #expanded

            impl #impl_generics ::core::fmt::Debug for #ident #ty_generics #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let mut builder = #debug_builder;
                    #(#debugs;)*
                    builder.finish()
                }
            }
        }
    };

    let expanded = quote! {
        #expanded

        #methods
    };

    Ok(expanded)
}

#[proc_macro_derive(Message, attributes(prost))]
pub fn message(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    try_message(input.into()).unwrap().into()
}

fn try_enumeration(input: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse2(input)?;
    let ident = input.ident;

    let Attributes { prost_path, .. } = Attributes::new(input.attrs)?;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let punctuated_variants = match input.data {
        Data::Enum(DataEnum { variants, .. }) => variants,
        Data::Struct(_) => bail!("Enumeration can not be derived for a struct"),
        Data::Union(..) => bail!("Enumeration can not be derived for a union"),
    };

    // Map the variants into 'fields'.
    let mut variants: Vec<(Ident, Expr, Option<TokenStream>)> = Vec::new();
    for Variant {
        attrs,
        ident,
        fields,
        discriminant,
        ..
    } in punctuated_variants
    {
        match fields {
            Fields::Unit => (),
            Fields::Named(_) | Fields::Unnamed(_) => {
                bail!("Enumeration variants may not have fields")
            }
        }
        match discriminant {
            Some((_, expr)) => {
                let deprecated_attr = if attrs.iter().any(|v| v.path().is_ident("deprecated")) {
                    Some(quote!(#[allow(deprecated)]))
                } else {
                    None
                };
                variants.push((ident, expr, deprecated_attr))
            }
            None => bail!("Enumeration variants must have a discriminant"),
        }
    }

    if variants.is_empty() {
        panic!("Enumeration must have at least one variant");
    }

    let (default, _, default_deprecated) = variants[0].clone();

    let is_valid = variants.iter().map(|(_, value, _)| quote!(#value => true));
    let from = variants
        .iter()
        .map(|(variant, value, deprecated)| quote!(#value => ::core::option::Option::Some(#deprecated #ident::#variant)));

    let try_from = variants
        .iter()
        .map(|(variant, value, deprecated)| quote!(#value => ::core::result::Result::Ok(#deprecated #ident::#variant)));

    let is_valid_doc = format!("Returns `true` if `value` is a variant of `{ident}`.");
    let from_i32_doc =
        format!("Converts an `i32` to a `{ident}`, or `None` if `value` is not a valid variant.");

    let expanded = quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            #[doc=#is_valid_doc]
            pub fn is_valid(value: i32) -> bool {
                match value {
                    #(#is_valid,)*
                    _ => false,
                }
            }

            #[deprecated = "Use the TryFrom<i32> implementation instead"]
            #[doc=#from_i32_doc]
            pub fn from_i32(value: i32) -> ::core::option::Option<#ident> {
                match value {
                    #(#from,)*
                    _ => ::core::option::Option::None,
                }
            }
        }

        impl #impl_generics ::core::default::Default for #ident #ty_generics #where_clause {
            fn default() -> #ident {
                #default_deprecated #ident::#default
            }
        }

        impl #impl_generics ::core::convert::From::<#ident> for i32 #ty_generics #where_clause {
            fn from(value: #ident) -> i32 {
                value as i32
            }
        }

        impl #impl_generics ::core::convert::TryFrom::<i32> for #ident #ty_generics #where_clause {
            type Error = #prost_path::UnknownEnumValue;

            fn try_from(value: i32) -> ::core::result::Result<#ident, #prost_path::UnknownEnumValue> {
                match value {
                    #(#try_from,)*
                    _ => ::core::result::Result::Err(#prost_path::UnknownEnumValue(value)),
                }
            }
        }
    };

    Ok(expanded)
}

#[proc_macro_derive(Enumeration, attributes(prost))]
pub fn enumeration(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    try_enumeration(input.into()).unwrap().into()
}

fn try_oneof(input: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse2(input)?;

    let ident = input.ident;

    let Attributes {
        skip_debug,
        prost_path,
    } = Attributes::new(input.attrs)?;

    let variants = match input.data {
        Data::Enum(DataEnum { variants, .. }) => variants,
        Data::Struct(..) => bail!("Oneof can not be derived for a struct"),
        Data::Union(..) => bail!("Oneof can not be derived for a union"),
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Map the variants into 'fields'.
    let mut fields: Vec<(Ident, Field, Option<TokenStream>)> = Vec::new();
    for Variant {
        attrs,
        ident: variant_ident,
        fields: variant_fields,
        ..
    } in variants
    {
        let variant_fields = match variant_fields {
            Fields::Unit => Punctuated::new(),
            Fields::Named(FieldsNamed { named: fields, .. })
            | Fields::Unnamed(FieldsUnnamed {
                unnamed: fields, ..
            }) => fields,
        };
        if variant_fields.len() != 1 {
            bail!("Oneof enum variants must have a single field");
        }
        let deprecated_attr = if attrs.iter().any(|v| v.path().is_ident("deprecated")) {
            Some(quote!(#[allow(deprecated)]))
        } else {
            None
        };
        match Field::new_oneof(attrs)? {
            Some(field) => fields.push((variant_ident, field, deprecated_attr)),
            None => bail!("invalid oneof variant: oneof variants may not be ignored"),
        }
    }

    // Oneof variants cannot be oneofs themselves, so it's impossible to have a field with multiple
    // tags.
    assert!(fields.iter().all(|(_, field, _)| field.tags().len() == 1));

    if let Some(duplicate_tag) = fields
        .iter()
        .flat_map(|(_, field, _)| field.tags())
        .duplicates()
        .next()
    {
        bail!(
            "invalid oneof {}: multiple variants have tag {}",
            ident,
            duplicate_tag
        );
    }

    // Check if any variant uses arena (String, Bytes, or Message types)
    let needs_arena = fields.iter().any(|(_, field, _)| {
        use crate::field::{Field, Ty};
        match field {
            Field::Scalar(scalar_field) => matches!(scalar_field.ty, Ty::String | Ty::Bytes(_)),
            Field::Message(_) => true,  // Messages always use arena
            _ => false,
        }
    });

    let encode = fields.iter().map(|(variant_ident, field, deprecated)| {
        let encode = field.encode(&prost_path, quote!(*value));
        quote!(#deprecated #ident::#variant_ident(value) => { #encode })
    });

    let merge = fields.iter().map(|(variant_ident, field, deprecated)| {
        let tag = field.tags()[0];
        let merge = field.merge(&prost_path, quote!(value));

        // For message fields in arena oneofs, we need to allocate in the arena
        // instead of using Default, since references can't implement Default
        use crate::field::Field;
        let is_message = matches!(field, Field::Message(_));

        if is_message && needs_arena {
            quote! {
                #deprecated
                #tag => {
                    // For arena message fields, always create a new value
                    // (arena-allocated values are immutable, so we can't merge into existing)
                    use #prost_path::Message as _;
                    let value = arena.alloc(::core::default::Default::default());
                    #merge.map(|_| {
                        *field = ::core::option::Option::Some(#deprecated #ident::#variant_ident(value))
                    })
                }
            }
        } else {
            quote! {
                #deprecated
                #tag => if let ::core::option::Option::Some(#ident::#variant_ident(value)) = field {
                    #merge
                } else {
                    let mut owned_value = ::core::default::Default::default();
                    let value = &mut owned_value;
                    #merge.map(|_| *field = ::core::option::Option::Some(#deprecated #ident::#variant_ident(owned_value)))
                }
            }
        }
    });

    let encoded_len = fields.iter().map(|(variant_ident, field, deprecated)| {
        let encoded_len = field.encoded_len(&prost_path, quote!(*value));
        quote!(#deprecated #ident::#variant_ident(value) => #encoded_len)
    });

    // Generate merge function signature with optional arena parameter
    let merge_signature = if needs_arena {
        quote! {
            pub fn merge(
                field: &mut ::core::option::Option<#ident #ty_generics>,
                tag: u32,
                wire_type: #prost_path::encoding::wire_type::WireType,
                buf: &mut impl #prost_path::bytes::Buf,
                arena: &'arena #prost_path::Arena,
                ctx: #prost_path::encoding::DecodeContext,
            ) -> ::core::result::Result<(), #prost_path::DecodeError>
        }
    } else {
        quote! {
            pub fn merge(
                field: &mut ::core::option::Option<#ident #ty_generics>,
                tag: u32,
                wire_type: #prost_path::encoding::wire_type::WireType,
                buf: &mut impl #prost_path::bytes::Buf,
                ctx: #prost_path::encoding::DecodeContext,
            ) -> ::core::result::Result<(), #prost_path::DecodeError>
        }
    };

    let expanded = quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            /// Encodes the message to a buffer.
            pub fn encode(&self, buf: &mut impl #prost_path::bytes::BufMut) {
                match self {
                    #(#encode,)*
                }
            }

            /// Decodes an instance of the message from a buffer, and merges it into self.
            #merge_signature
            {
                match tag {
                    #(#merge,)*
                    _ => unreachable!(concat!("invalid ", stringify!(#ident), " tag: {}"), tag),
                }
            }

            /// Returns the encoded length of the message without a length delimiter.
            #[inline]
            pub fn encoded_len(&self) -> usize {
                match self {
                    #(#encoded_len,)*
                }
            }
        }

    };
    let expanded = if skip_debug {
        expanded
    } else {
        let debug = fields.iter().map(|(variant_ident, field, deprecated)| {
            let wrapper = field.debug(&prost_path, quote!(*value));
            quote!(#deprecated #ident::#variant_ident(value) => {
                let wrapper = #wrapper;
                f.debug_tuple(stringify!(#variant_ident))
                    .field(&wrapper)
                    .finish()
            })
        });
        quote! {
            #expanded

            impl #impl_generics ::core::fmt::Debug for #ident #ty_generics #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    match self {
                        #(#debug,)*
                    }
                }
            }
        }
    };

    Ok(expanded)
}

#[proc_macro_derive(Oneof, attributes(prost))]
pub fn oneof(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    try_oneof(input.into()).unwrap().into()
}

/// Get the items belonging to the 'prost' list attribute, e.g. `#[prost(foo, bar="baz")]`.
fn prost_attrs(attrs: Vec<Attribute>) -> Result<Vec<Meta>, Error> {
    let mut result = Vec::new();
    for attr in attrs.iter() {
        if let Meta::List(meta_list) = &attr.meta {
            if meta_list.path.is_ident("prost") {
                result.extend(
                    meta_list
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                        .into_iter(),
                )
            }
        }
    }
    Ok(result)
}

/// Extracts the path to prost specified using the `#[prost(prost_path = "...")]` attribute. When
/// missing, falls back to default, which is `::prost`.
fn get_prost_path(attrs: &[Meta]) -> Result<Path, Error> {
    let mut prost_path = None;

    for attr in attrs {
        match attr {
            Meta::NameValue(MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }),
                ..
            }) if path.is_ident("prost_path") => {
                let path: Path =
                    syn::parse_str(&lit.value()).context("invalid prost_path argument")?;

                set_option(&mut prost_path, path, "duplicate prost_path attributes")?;
            }
            _ => continue,
        }
    }

    let prost_path =
        prost_path.unwrap_or_else(|| syn::parse_str("::prost").expect("default prost_path"));

    Ok(prost_path)
}

struct Attributes {
    skip_debug: bool,
    prost_path: Path,
}

impl Attributes {
    fn new(attrs: Vec<Attribute>) -> Result<Self, Error> {
        syn::custom_keyword!(skip_debug);
        let skip_debug = attrs.iter().any(|a| a.parse_args::<skip_debug>().is_ok());

        let attrs = prost_attrs(attrs)?;
        let prost_path = get_prost_path(&attrs)?;

        Ok(Self {
            skip_debug,
            prost_path,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{try_message, try_oneof};
    use quote::quote;

    #[test]
    fn test_rejects_colliding_message_fields() {
        let output = try_message(quote!(
            struct Invalid {
                #[prost(bool, tag = "1")]
                a: bool,
                #[prost(oneof = "super::Whatever", tags = "4, 5, 1")]
                b: Option<super::Whatever>,
            }
        ));
        assert_eq!(
            output
                .expect_err("did not reject colliding message fields")
                .to_string(),
            "message Invalid has multiple fields with tag 1"
        );
    }

    #[test]
    fn test_rejects_colliding_oneof_variants() {
        let output = try_oneof(quote!(
            pub enum Invalid {
                #[prost(bool, tag = "1")]
                A(bool),
                #[prost(bool, tag = "3")]
                B(bool),
                #[prost(bool, tag = "1")]
                C(bool),
            }
        ));
        assert_eq!(
            output
                .expect_err("did not reject colliding oneof variants")
                .to_string(),
            "invalid oneof Invalid: multiple variants have tag 1"
        );
    }

    #[test]
    fn test_rejects_multiple_tags_oneof_variant() {
        let output = try_oneof(quote!(
            enum What {
                #[prost(bool, tag = "1", tag = "2")]
                A(bool),
            }
        ));
        assert_eq!(
            output
                .expect_err("did not reject multiple tags on oneof variant")
                .to_string(),
            "duplicate tag attributes: 1 and 2"
        );

        let output = try_oneof(quote!(
            enum What {
                #[prost(bool, tag = "3")]
                #[prost(tag = "4")]
                A(bool),
            }
        ));
        assert!(output.is_err());
        assert_eq!(
            output
                .expect_err("did not reject multiple tags on oneof variant")
                .to_string(),
            "duplicate tag attributes: 3 and 4"
        );

        let output = try_oneof(quote!(
            enum What {
                #[prost(bool, tags = "5,6")]
                A(bool),
            }
        ));
        assert!(output.is_err());
        assert_eq!(
            output
                .expect_err("did not reject multiple tags on oneof variant")
                .to_string(),
            "unknown attribute(s): #[prost(tags = \"5,6\")]"
        );
    }
}
