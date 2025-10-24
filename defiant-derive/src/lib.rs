#![doc(html_root_url = "https://docs.rs/defiant-derive/0.1.0")]
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

/// Converts a slice type `&'arena [T]` to `ArenaVec<'arena, T>`
fn slice_to_bumpvec(field_type: &syn::Type, prost_path: &Path) -> TokenStream {
    // Try to parse as a reference to a slice
    if let syn::Type::Reference(type_ref) = field_type {
        if let syn::Type::Slice(type_slice) = &*type_ref.elem {
            let elem_type = &type_slice.elem;
            let lifetime = &type_ref.lifetime;
            return quote!(#prost_path::arena::ArenaVec<#lifetime, #elem_type>);
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

/// Validates that arena message fields don't use disallowed owned types
/// Returns an error if Box, Vec, String, HashMap, or BTreeMap are found
fn validate_arena_field_type(field_type: &syn::Type, field_name: &str) -> Result<(), Error> {
    fn check_type_path(path: &syn::Path, field_name: &str) -> Result<(), Error> {
        if let Some(last_seg) = path.segments.last() {
            let type_name = last_seg.ident.to_string();

            // Check for disallowed types
            match type_name.as_str() {
                "Box" => bail!(
                    "Field '{}' uses Box<_> which is not allowed for arena types. \
                    Use &'arena T instead of Box<&'arena T>",
                    field_name
                ),
                "Vec" => bail!(
                    "Field '{}' uses Vec<_> which is not allowed for arena types. \
                    Use &'arena [T] instead of Vec<T>",
                    field_name
                ),
                "String" => bail!(
                    "Field '{}' uses String which is not allowed for arena types. \
                    Use &'arena str instead of String",
                    field_name
                ),
                "HashMap" => bail!(
                    "Field '{}' uses HashMap<_, _> which is not allowed for arena types. \
                    Use arena-allocated map types instead",
                    field_name
                ),
                "BTreeMap" => bail!(
                    "Field '{}' uses BTreeMap<_, _> which is not allowed for arena types. \
                    Use arena-allocated map types instead",
                    field_name
                ),
                _ => {}
            }

            // Recursively check generic arguments
            if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                for arg in &args.args {
                    if let syn::GenericArgument::Type(inner_type) = arg {
                        validate_arena_field_type(inner_type, field_name)?;
                    }
                }
            }
        }

        Ok(())
    }

    match field_type {
        syn::Type::Path(type_path) => check_type_path(&type_path.path, field_name)?,
        syn::Type::Reference(type_ref) => {
            // Check the referenced type
            validate_arena_field_type(&type_ref.elem, field_name)?;
        }
        syn::Type::Slice(type_slice) => {
            // Check the element type
            validate_arena_field_type(&type_slice.elem, field_name)?;
        }
        _ => {}
    }

    Ok(())
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

        // Validate that the field doesn't use disallowed types (Box, Vec, String, HashMap, BTreeMap)
        if let Err(err) = validate_arena_field_type(&field_type, &field_ident.to_string()) {
            bail!(err.context(format!("invalid field type for {ident}.{field_ident}")));
        }

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

    let _encoded_len: Vec<_> = fields
        .iter()
        .map(|(field_ident, field)| field.encoded_len(&prost_path, quote!(self.#field_ident)))
        .collect();

    let _encode: Vec<_> = fields
        .iter()
        .map(|(field_ident, field)| field.encode(&prost_path, quote!(self.#field_ident)))
        .collect();

    // Generate View-specific encode/encoded_len that uses _ref variants for repeated strings/bytes
    let view_encoded_len_stmts: Vec<_> = fields
        .iter()
        .map(|(field_ident, field)| {
            use crate::field::Field;
            // For repeated string/bytes fields in View, use encoded_len_repeated
            match field {
                Field::Scalar(ref scalar_field) => {
                    use crate::field::scalar::{Ty, Kind};
                    if matches!(scalar_field.kind, Kind::Repeated) {
                        let tag = scalar_field.tag;
                        match scalar_field.ty {
                            Ty::String => quote! {
                                #prost_path::encoding::string::encoded_len_repeated(#tag, self.#field_ident)
                            },
                            Ty::Bytes(_) => quote! {
                                #prost_path::encoding::bytes::encoded_len_repeated(#tag, self.#field_ident)
                            },
                            _ => field.encoded_len(&prost_path, quote!(self.#field_ident)),
                        }
                    } else {
                        field.encoded_len(&prost_path, quote!(self.#field_ident))
                    }
                },
                // For repeated groups/messages in views, use encoded_len functions that work with slices
                Field::Group(_) if field.is_repeated() => {
                    let tag = match field {
                        Field::Group(g) => g.tag,
                        _ => unreachable!(),
                    };
                    quote! {
                        #prost_path::encoding::group::encoded_len_repeated(#tag, self.#field_ident)
                    }
                },
                Field::Message(_) if field.is_repeated() => {
                    quote! {
                        {
                            use #prost_path::Message as _;
                            self.#field_ident.iter().map(|msg| {
                                let len = msg.encoded_len();
                                #prost_path::encoding::encoded_len_varint(len as u64) + len
                            }).sum::<usize>()
                        }
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
            // For repeated string/bytes fields in View, use encode_repeated
            match field {
                Field::Scalar(ref scalar_field) => {
                    use crate::field::scalar::{Ty, Kind};
                    if matches!(scalar_field.kind, Kind::Repeated) {
                        let tag = scalar_field.tag;
                        match scalar_field.ty {
                            Ty::String => quote! {
                                #prost_path::encoding::string::encode_repeated(#tag, self.#field_ident, buf);
                            },
                            Ty::Bytes(_) => quote! {
                                #prost_path::encoding::bytes::encode_repeated(#tag, self.#field_ident, buf);
                            },
                            _ => field.encode(&prost_path, quote!(self.#field_ident)),
                        }
                    } else {
                        field.encode(&prost_path, quote!(self.#field_ident))
                    }
                },
                // For repeated groups in views, iterate the slice directly
                Field::Group(_) if field.is_repeated() => {
                    let tag = match field {
                        Field::Group(g) => g.tag,
                        _ => unreachable!(),
                    };
                    quote! {
                        for msg in self.#field_ident {
                            #prost_path::encoding::group::encode(#tag, msg, buf);
                        }
                    }
                },
                // For repeated messages in views, iterate and encode each
                Field::Message(_) if field.is_repeated() => {
                    let tag = match field {
                        Field::Message(m) => m.tag,
                        _ => unreachable!(),
                    };
                    quote! {
                        {
                            use #prost_path::Message as _;
                            for msg in self.#field_ident {
                                #prost_path::encoding::encode_key(#tag, #prost_path::encoding::WireType::LengthDelimited, buf);
                                #prost_path::encoding::encode_varint(msg.encoded_len() as u64, buf);
                                msg.encode_raw(buf);
                            }
                        }
                    }
                },
                _ => field.encode(&prost_path, quote!(self.#field_ident)),
            }
        })
        .collect();

    let merge = fields_with_types.iter().map(|(field_ident, field_type, field)| {
        use crate::field::Field;
        use crate::field::Label;

        let tags = field.tags().into_iter().map(|tag| quote!(#tag));
        let tags = Itertools::intersperse(tags, quote!(|));

        // Check if this is a repeated message or group (needs special inline handling)
        let is_repeated_message_or_group = match field {
            Field::Message(ref msg_field) => msg_field.label == Label::Repeated,
            Field::Group(ref group_field) => group_field.label == Label::Repeated,
            _ => false,
        };

        if (matches!(field, Field::Message(_) | Field::Group(_))) && !is_repeated_message_or_group {
            // Non-repeated messages and groups: use builder pattern
            // Extract the type path and build the Message companion path
            let mut base_path = extract_type_path(field_type);
            // Append "Message" to the last segment
            if let Some(last_seg) = base_path.segments.last_mut() {
                let type_name = last_seg.ident.to_string();
                last_seg.ident = Ident::new(&format!("{}Message", type_name), Span::call_site());
            }
            let builder_type_name = base_path;

            let label = match field {
                Field::Message(msg_field) => msg_field.label,
                Field::Group(group_field) => group_field.label,
                _ => unreachable!(),
            };

            // Groups use StartGroup wire type, messages use LengthDelimited
            let expected_wire_type = if matches!(field, Field::Group(_)) {
                quote!(#prost_path::encoding::WireType::StartGroup)
            } else {
                quote!(#prost_path::encoding::WireType::LengthDelimited)
            };

            // For groups, use group::merge; for messages, use merge_loop
            let merge_fn = if matches!(field, Field::Group(_)) {
                quote! {
                    #prost_path::encoding::group::merge(
                        tag,
                        wire_type,
                        &mut builder,
                        buf,
                        arena,
                        ctx.enter_recursion()
                    )
                }
            } else {
                quote! {
                    #prost_path::encoding::merge_loop(
                        &mut builder,
                        buf,
                        ctx.enter_recursion(),
                        |builder, buf, ctx| {
                            let (tag, wire_type) = #prost_path::encoding::decode_key(buf)?;
                            builder.merge_field(tag, wire_type, buf, ctx)
                        }
                    )
                }
            };

            let merge_code = match label {
                Label::Optional => quote! {
                    #prost_path::encoding::check_wire_type(#expected_wire_type, wire_type)
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
                    #merge_fn.map_err(|mut error| {
                        error.push(STRUCT_NAME, stringify!(#field_ident));
                        error
                    })?;
                    let view = builder.freeze();
                    self.#field_ident = Some(&*arena.alloc(view));
                    Ok(())
                },
                Label::Required => quote! {
                    #prost_path::encoding::check_wire_type(#expected_wire_type, wire_type)
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
                    #merge_fn.map_err(|mut error| {
                        error.push(STRUCT_NAME, stringify!(#field_ident));
                        error
                    })?;
                    let view = builder.freeze();
                    self.#field_ident = &*arena.alloc(view);
                    Ok(())
                },
                Label::Repeated => {
                    // Repeated is handled separately below
                    unreachable!("Repeated messages/groups handled separately")
                },
            };

            quote! {
                #(#tags)* => {
                    #merge_code
                },
            }
        } else if is_repeated_message_or_group {
            // Special handling for repeated messages and groups (inline merge code)

            // Extract the type path and build the Message companion path
            let mut base_path = extract_type_path(field_type);
            // Append "Message" to the last segment
            if let Some(last_seg) = base_path.segments.last_mut() {
                let type_name = last_seg.ident.to_string();
                last_seg.ident = Ident::new(&format!("{}Message", type_name), Span::call_site());
            }
            let builder_type_name = base_path;

            // Groups use StartGroup wire type, messages use LengthDelimited
            let expected_wire_type = if matches!(field, Field::Group(_)) {
                quote!(#prost_path::encoding::WireType::StartGroup)
            } else {
                quote!(#prost_path::encoding::WireType::LengthDelimited)
            };

            // Generate the merge code - groups use END_GROUP loop, messages use merge_loop
            let merge_code = if matches!(field, Field::Group(_)) {
                // For groups: loop until END_GROUP with matching tag
                let group_tag = field.tags()[0];  // Groups have a single tag
                quote! {
                    loop {
                        let (field_tag, field_wire_type) = #prost_path::encoding::decode_key(buf)?;
                        if field_wire_type == #prost_path::encoding::WireType::EndGroup {
                            if field_tag != #group_tag {
                                return Err(#prost_path::DecodeError::new("unexpected end group tag"));
                            }
                            break;
                        }
                        builder.merge_field(field_tag, field_wire_type, buf, ctx.enter_recursion())?;
                    }
                }
            } else {
                // For messages: use merge_loop
                quote! {
                    #prost_path::encoding::merge_loop(
                        &mut builder,
                        buf,
                        ctx.enter_recursion(),
                        |builder, buf, ctx| {
                            let (tag, wire_type) = #prost_path::encoding::decode_key(buf)?;
                            builder.merge_field(tag, wire_type, buf, ctx)
                        }
                    )?;
                }
            };

            // For repeated messages and groups, convert builder to view before pushing
            // BumpVec stores view types, not builders (to avoid double allocation)
            let push_code = quote!(self.#field_ident.push(builder.freeze()););

            quote! {
                #(#tags)* => {
                    #prost_path::encoding::check_wire_type(#expected_wire_type, wire_type)
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
                    #merge_code
                    #push_code
                    Ok(())
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

    let _clear = fields
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
                                    Some(quote!(#prost_path::arena::ArenaVec<'arena, (#key_ty, #val_ty)>))
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
                // Other repeated fields initialize with arena.new_vec()
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
                                        let mut vec = self.arena.new_vec();
                                        vec.extend_from_slice(value);
                                        let allocated = vec.freeze();
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
                    Field::Group(_) => {
                        // Skip push methods for repeated groups - they use builder types internally
                        // and are populated via group::merge_repeated during decoding
                        quote!()
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
                                            let mut vec = self.arena.new_vec();
                                            vec.extend_from_slice(value);
                                            self.#field_ident = ::core::option::Option::Some(vec.freeze());
                                        }
                                    }
                                } else {
                                    quote! {
                                        pub fn #set_method(&mut self, value: &[u8]) {
                                            let mut vec = self.arena.new_vec();
                                            vec.extend_from_slice(value);
                                            self.#field_ident = vec.freeze();
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

            // Skip getters for repeated groups in builder - they use Vec<TMessage> internally
            // and will be properly exposed via the view type
            if matches!(field, Field::Group(_)) && field.is_repeated() {
                return quote!();
            }

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

        // Generate freeze() method (converts BumpVec to arena slice)
        let freeze_field_inits: Vec<_> = fields_with_types.iter().map(|(field_ident, _field_type, field)| {
            use crate::field::Field;

            if matches!(field, Field::Map(_)) {
                // For map fields, sort by key and wrap in ArenaMap
                quote! {
                    #field_ident: {
                        let mut entries = self.#field_ident;
                        entries.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                        #prost_path::ArenaMap::new(entries.freeze())
                    }
                }
            } else if field.is_repeated() {
                // For repeated fields (including groups), convert ArenaVec to arena slice (zero-copy!)
                quote!(#field_ident: self.#field_ident.freeze())
            } else {
                // For singular fields, move value
                quote!(#field_ident: self.#field_ident)
            }
        }).collect();

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

                    pub fn freeze(self) -> #ident #ty_generics {
                        #ident {
                            #(#freeze_field_inits,)*
                        }
                    }

                    pub fn decode(mut buf: impl #prost_path::bytes::Buf, arena: &'arena #prost_path::Arena)
                        -> ::core::result::Result<#ident #ty_generics, #prost_path::DecodeError>
                    {
                        let mut message = Self::new_in(arena);
                        message.merge(buf)?;
                        // Convert ArenaVecs to arena slices using freeze()
                        Ok(message.freeze())
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

                    pub fn freeze(self) -> #ident #ty_generics {
                        #ident {
                            #(#freeze_field_inits,)*
                        }
                    }

                    pub fn decode(mut buf: impl #prost_path::bytes::Buf)
                        -> ::core::result::Result<#ident #ty_generics, #prost_path::DecodeError>
                    {
                        let mut message = Self::new();
                        message.merge(buf)?;
                        Ok(message.freeze())
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
                    // Views should not be constructed directly via new_in()
                    // Use Self::builder() or Self::decode() instead
                    unreachable!("Use <Self as MessageView>::Builder::new_in() instead")
                }

                // Override decode to use builder transparently
                fn decode(buf: impl #prost_path::bytes::Buf, arena: &'arena #prost_path::Arena) -> ::core::result::Result<Self, #prost_path::DecodeError> {
                    let mut builder = <Self as #prost_path::MessageView<'arena>>::Builder::new_in(arena);
                    builder.merge(buf)?;
                    Ok(builder.freeze())
                }

                // Override decode_length_delimited to use builder transparently
                fn decode_length_delimited(mut buf: impl #prost_path::bytes::Buf, arena: &'arena #prost_path::Arena) -> ::core::result::Result<Self, #prost_path::DecodeError> {
                    let mut builder = <Self as #prost_path::MessageView<'arena>>::Builder::new_in(arena);
                    let length = #prost_path::encoding::decode_varint(&mut buf)?;
                    if length > buf.remaining() as u64 {
                        return Err(#prost_path::DecodeError::new("buffer underflow"));
                    }
                    let limit = (buf.remaining() - length as usize) as u64;
                    let mut limited_buf = buf.take(length as usize);
                    builder.merge(&mut limited_buf)?;
                    Ok(builder.freeze())
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
            }

            impl #impl_generics #ident #ty_generics #where_clause {
                /// Creates a new builder for constructing this message
                pub fn builder(arena: &'arena #prost_path::Arena) -> <Self as #prost_path::MessageView<'arena>>::Builder {
                    <Self as #prost_path::MessageView<'arena>>::Builder::new_in(arena)
                }

                pub fn encode(&self, buf: &mut impl #prost_path::bytes::BufMut) -> ::core::result::Result<(), #prost_path::EncodeError> {
                    use #prost_path::Message as _;
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
        // For non-arena types (scalar-only), generate simple merge logic
        let scalar_merge_stmts: Vec<_> = fields_with_types.iter().map(|(field_ident, _field_type, field)| {
            let tags = field.tags().into_iter().map(|tag| quote!(#tag));
            let tags = Itertools::intersperse(tags, quote!(|));
            let merge = field.merge(&prost_path, quote!(&mut self.#field_ident));
            quote! {
                #(#tags)|* => { #merge }
            }
        }).collect();

        // Generate default field initializers for new_in
        let default_field_inits: Vec<_> = fields_with_types.iter().map(|(field_ident, _field_type, _field)| {
            quote!(#field_ident: ::core::default::Default::default())
        }).collect();

        // For non-arena types (scalar-only), implement Message<'arena> for all lifetimes
        quote! {
            impl<'arena> #prost_path::Message<'arena> for #ident #ty_generics #where_clause {
                fn new_in(_arena: &'arena #prost_path::Arena) -> Self {
                    Self {
                        #(#default_field_inits,)*
                    }
                }

                fn decode(mut buf: impl #prost_path::bytes::Buf, arena: &'arena #prost_path::Arena) -> ::core::result::Result<Self, #prost_path::DecodeError> {
                    let mut message = Self::new_in(arena);
                    message.merge(buf, arena)?;
                    Ok(message)
                }

                #[allow(unused_variables)]
                fn encode_raw(&self, buf: &mut impl #prost_path::bytes::BufMut) {
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
                    match tag {
                        #(#scalar_merge_stmts,)*
                        _ => #prost_path::encoding::skip_field(wire_type, tag, buf, ctx),
                    }
                }

                fn encoded_len(&self) -> usize {
                    0 #(+ #view_encoded_len_stmts)*
                }
            }

            impl #impl_generics #ident #ty_generics #where_clause {
                pub fn encode(&self, buf: &mut impl #prost_path::bytes::BufMut) -> ::core::result::Result<(), #prost_path::EncodeError> {
                    use #prost_path::Message as _;
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

#[proc_macro_derive(Message, attributes(prost, defiant))]
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

#[proc_macro_derive(Enumeration, attributes(prost, defiant))]
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

#[proc_macro_derive(Oneof, attributes(prost, defiant))]
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
/// missing, falls back to default, which is `::defiant`.
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
        prost_path.unwrap_or_else(|| syn::parse_str("::defiant").expect("default prost_path"));

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
