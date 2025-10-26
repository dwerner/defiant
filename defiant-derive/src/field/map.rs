use anyhow::{bail, Error};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Ident, Lit, Meta, MetaNameValue, Path, Token};

use crate::field::{scalar, set_option, tag_attr};

#[derive(Clone, Debug)]
pub enum MapTy {
    HashMap,
    BTreeMap,
}

impl MapTy {
    fn from_str(s: &str) -> Option<MapTy> {
        // All maps use arena allocation, so "arena_map" is the preferred annotation.
        // We also accept legacy "map", "hash_map", and "btree_map" for now.
        match s {
            "arena_map" | "map" | "hash_map" => Some(MapTy::HashMap),
            "btree_map" => Some(MapTy::BTreeMap),
            _ => None,
        }
    }

    fn module(&self) -> Ident {
        // In arena mode, always use arena_map for map fields
        // (both hash_map and btree_map use the same arena-allocated encoding)
        Ident::new("arena_map", Span::call_site())
    }

    #[allow(dead_code)]
    fn lib(&self) -> TokenStream {
        match self {
            MapTy::HashMap => quote! { std },
            MapTy::BTreeMap => quote! { prost::alloc },
        }
    }
}

fn fake_scalar(ty: scalar::Ty) -> scalar::Field {
    let kind = scalar::Kind::Plain(scalar::DefaultValue::new(&ty));
    scalar::Field {
        ty,
        kind,
        tag: 0, // Not used here
    }
}

#[derive(Clone)]
pub struct Field {
    pub map_ty: MapTy,
    pub key_ty: scalar::Ty,
    pub value_ty: ValueTy,
    pub tag: u32,
}

impl Field {
    /// Returns the default value for a map key type
    fn key_default(&self) -> TokenStream {
        use scalar::Ty::*;
        match &self.key_ty {
            String => quote!(""),
            Bool => quote!(false),
            Int32 | Sint32 | Sfixed32 => quote!(0i32),
            Int64 | Sint64 | Sfixed64 => quote!(0i64),
            Uint32 | Fixed32 => quote!(0u32),
            Uint64 | Fixed64 => quote!(0u64),
            // Map keys can only be integral types, bool, or string per protobuf spec
            // Bytes and Enumeration are not valid map key types
            Float | Double | Bytes(_) | Enumeration(_) => {
                panic!("Invalid map key type: {:?}", self.key_ty)
            }
        }
    }

    fn value_default(&self) -> TokenStream {
        match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::String) => quote!(""),
            ValueTy::Scalar(scalar::Ty::Bytes(_)) => quote!(&b""[..]),
            ValueTy::Scalar(scalar::Ty::Bool) => quote!(false),
            ValueTy::Scalar(scalar::Ty::Int32 | scalar::Ty::Sint32 | scalar::Ty::Sfixed32) => quote!(0i32),
            ValueTy::Scalar(scalar::Ty::Int64 | scalar::Ty::Sint64 | scalar::Ty::Sfixed64) => quote!(0i64),
            ValueTy::Scalar(scalar::Ty::Uint32 | scalar::Ty::Fixed32) => quote!(0u32),
            ValueTy::Scalar(scalar::Ty::Uint64 | scalar::Ty::Fixed64) => quote!(0u64),
            ValueTy::Scalar(scalar::Ty::Float) => quote!(0f32),
            ValueTy::Scalar(scalar::Ty::Double) => quote!(0f64),
            // Enumeration defaults are handled separately where used (e.g., #ty::default() as i32)
            // Message defaults are not needed - we use encode_message/encoded_len_message instead
            ValueTy::Scalar(scalar::Ty::Enumeration(_)) | ValueTy::Message => {
                panic!("value_default() should not be called for enumerations or messages")
            }
        }
    }

    /// Returns the arena-allocated key type for maps
    /// Lifetime parameter is 'arena by convention
    pub fn arena_key_type(&self) -> TokenStream {
        use scalar::Ty::*;
        match &self.key_ty {
            String => quote!(&'arena str),
            _ => self.key_ty.rust_ref_type(),
        }
    }

    pub fn new(attrs: &[Meta], inferred_tag: Option<u32>) -> Result<Option<Field>, Error> {
        let mut types = None;
        let mut tag = None;

        for attr in attrs {
            if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(map_ty) = attr
                .path()
                .get_ident()
                .and_then(|i| MapTy::from_str(&i.to_string()))
            {
                let (k, v): (String, String) = match attr {
                    Meta::NameValue(MetaNameValue {
                        value:
                            Expr::Lit(ExprLit {
                                lit: Lit::Str(lit), ..
                            }),
                        ..
                    }) => {
                        let items = lit.value();
                        let mut items = items.split(',').map(ToString::to_string);
                        let k = items.next().unwrap();
                        let v = match items.next() {
                            Some(k) => k,
                            None => bail!("invalid map attribute: must have key and value types"),
                        };
                        if items.next().is_some() {
                            bail!("invalid map attribute: {:?}", attr);
                        }
                        (k, v)
                    }
                    Meta::List(meta_list) => {
                        let nested = meta_list
                            .parse_args_with(Punctuated::<Ident, Token![,]>::parse_terminated)?
                            .into_iter()
                            .collect::<Vec<_>>();
                        if nested.len() != 2 {
                            bail!("invalid map attribute: must contain key and value types");
                        }
                        (nested[0].to_string(), nested[1].to_string())
                    }
                    _ => return Ok(None),
                };
                set_option(
                    &mut types,
                    (map_ty, key_ty_from_str(&k)?, ValueTy::from_str(&v)?),
                    "duplicate map type attribute",
                )?;
            } else {
                return Ok(None);
            }
        }

        Ok(match (types, tag.or(inferred_tag)) {
            (Some((map_ty, key_ty, value_ty)), Some(tag)) => Some(Field {
                map_ty,
                key_ty,
                value_ty,
                tag,
            }),
            _ => None,
        })
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        Field::new(attrs, None)
    }

    /// Returns a statement which encodes the map field.
    pub fn encode(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        let key_mod = self.key_ty.module();
        // String keys are &str, need to dereference from &&str to &str
        // Wrap in closures to match the expected signature
        let (ke, kl) = if matches!(self.key_ty, scalar::Ty::String) {
            (quote!(|tag, key: &&str, buf| #prost_path::encoding::#key_mod::encode(tag, *key, buf)),
             quote!(|tag, key: &&str| #prost_path::encoding::#key_mod::encoded_len(tag, *key)))
        } else {
            (quote!(#prost_path::encoding::#key_mod::encode),
             quote!(#prost_path::encoding::#key_mod::encoded_len))
        };
        let key_default = self.key_default();
        let module = self.map_ty.module();
        // For ArenaMap, extract the slice
        let map_value = quote!(#ident.as_slice());
        match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let val_default = quote!(#ty::default() as i32);
                quote! {
                    #prost_path::encoding::#module::encode_with_defaults(
                        #ke,
                        #kl,
                        #prost_path::encoding::int32::encode,
                        #prost_path::encoding::int32::encoded_len,
                        &#key_default,
                        &(#val_default),
                        #tag,
                        #map_value,
                        buf,
                    );
                }
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                // For strings and bytes in maps: map stores &[(&str, &[u8])]
                // encode_with_defaults does .iter() which yields &(&str, &[u8])
                // Destructuring gives (key, val) where key: &&str, val: &&[u8]
                // encode functions expect encode(tag, &str, buf) and encode(tag, &[u8], buf)
                // So we need to dereference: *val to go from &&[u8] -> &[u8]
                let (ve, vl) = match value_ty {
                    scalar::Ty::String => {
                        (quote!(|tag, val: &&str, buf| #prost_path::encoding::#val_mod::encode(tag, *val, buf)),
                         quote!(|tag, val: &&str| #prost_path::encoding::#val_mod::encoded_len(tag, *val)))
                    }
                    scalar::Ty::Bytes(_) => {
                        (quote!(|tag, val: &&[u8], buf| #prost_path::encoding::#val_mod::encode(tag, *val, buf)),
                         quote!(|tag, val: &&[u8]| #prost_path::encoding::#val_mod::encoded_len(tag, *val)))
                    }
                    _ => {
                        (quote!(#prost_path::encoding::#val_mod::encode),
                         quote!(#prost_path::encoding::#val_mod::encoded_len))
                    }
                };
                let val_default = self.value_default();
                quote! {
                    #prost_path::encoding::#module::encode_with_defaults(
                        #ke,
                        #kl,
                        #ve,
                        #vl,
                        &#key_default,
                        &#val_default,
                        #tag,
                        #map_value,
                        buf,
                    );
                }
            }
            ValueTy::Message => {
                // For message types, use encode_message which doesn't need val_default
                quote! {
                    #prost_path::encoding::#module::encode_message(
                        #ke,
                        #kl,
                        #prost_path::encoding::message::encode,
                        #prost_path::encoding::message::encoded_len,
                        &#key_default,
                        #tag,
                        #map_value,
                        buf,
                    );
                }
            },
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded key value pair
    /// into the map.
    pub fn merge(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        let key_mod = self.key_ty.module();
        let module = self.map_ty.module();

        // Keys are always scalars, so we need to wrap the merge function to ignore arena
        let km = if self.key_ty.is_numeric() || matches!(self.key_ty, scalar::Ty::Bool) {
            // Numeric and bool types don't use arena
            let key_merge_fn = quote!(#prost_path::encoding::#key_mod::merge);
            quote!(|wire_type, key, buf, _arena, ctx| #key_merge_fn(wire_type, key, buf, ctx))
        } else {
            // String keys use arena variant directly
            quote!(|wire_type, key, buf, arena, ctx| {
                *key = #prost_path::encoding::#key_mod::merge_arena(wire_type, buf, arena, ctx)?;
                Ok(())
            })
        };

        match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let key_default = self.key_default();
                let val_default = quote!(#ty::default() as i32);
                // Wrap int32::merge to ignore arena
                let vm = quote!(|wire_type, val, buf, _arena, ctx| {
                    #prost_path::encoding::int32::merge(wire_type, val, buf, ctx)
                });
                quote! {
                    #prost_path::encoding::#module::merge_with_defaults(
                        #km,
                        #vm,
                        #key_default,
                        #val_default,
                        &mut #ident,
                        buf,
                        arena,
                        ctx,
                    )
                }
            }
            ValueTy::Scalar(value_ty) => {
                let key_default = self.key_default();
                let val_default = self.value_default();
                let val_mod = value_ty.module();
                // Wrap scalar merge functions to ignore arena
                let vm = if value_ty.is_numeric() || matches!(value_ty, scalar::Ty::Bool) {
                    let val_merge_fn = quote!(#prost_path::encoding::#val_mod::merge);
                    quote!(|wire_type, val, buf, _arena, ctx| #val_merge_fn(wire_type, val, buf, ctx))
                } else if matches!(value_ty, scalar::Ty::String) {
                    // String values use arena variant
                    quote!(|wire_type, val, buf, arena, ctx| {
                        *val = #prost_path::encoding::#val_mod::merge_arena(wire_type, buf, arena, ctx)?;
                        Ok(())
                    })
                } else {
                    // Bytes
                    quote!(|wire_type, val, buf, arena, ctx| {
                        *val = #prost_path::encoding::#val_mod::merge_arena(wire_type, buf, arena, ctx)?;
                        Ok(())
                    })
                };
                quote! {
                    #prost_path::encoding::#module::merge_with_defaults(
                        #km,
                        #vm,
                        #key_default,
                        #val_default,
                        &mut #ident,
                        buf,
                        arena,
                        ctx,
                    )
                }
            }
            ValueTy::Message => {
                // Map fields with message values should use the custom inline merge code
                // generated in lib.rs, not the encoding::arena_map::merge_message function.
                // This is already handled by is_map_with_message_values check in lib.rs.
                // If we get here, something is wrong with the code generation logic.
                panic!("Map fields with message values should use custom inline merge code, not field.merge()")
            },
        }
    }

    /// Returns an expression which evaluates to the encoded length of the map.
    pub fn encoded_len(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        let key_mod = self.key_ty.module();
        // String keys are &str, need to dereference from &&str to &str
        // Wrap in closure to match the expected signature
        let kl = if matches!(self.key_ty, scalar::Ty::String) {
            quote!(|tag, key: &&str| #prost_path::encoding::#key_mod::encoded_len(tag, *key))
        } else {
            quote!(#prost_path::encoding::#key_mod::encoded_len)
        };
        let key_default = self.key_default();
        let module = self.map_ty.module();
        // For ArenaMap, extract the slice
        let map_value = quote!(#ident.as_slice());
        match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let val_default = quote!(#ty::default() as i32);
                quote! {
                    #prost_path::encoding::#module::encoded_len_with_defaults(
                        #kl,
                        #prost_path::encoding::int32::encoded_len,
                        &#key_default,
                        &(#val_default),
                        #tag,
                        #map_value,
                    )
                }
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                // String and bytes values are &str/&[u8], need to dereference from &&str/&&[u8]
                let vl = match value_ty {
                    scalar::Ty::String => {
                        quote!(|tag, val: &&str| #prost_path::encoding::#val_mod::encoded_len(tag, *val))
                    }
                    scalar::Ty::Bytes(_) => {
                        quote!(|tag, val: &&[u8]| #prost_path::encoding::#val_mod::encoded_len(tag, *val))
                    }
                    _ => {
                        quote!(#prost_path::encoding::#val_mod::encoded_len)
                    }
                };
                let val_default = self.value_default();
                quote! {
                    #prost_path::encoding::#module::encoded_len_with_defaults(
                        #kl,
                        #vl,
                        &#key_default,
                        &#val_default,
                        #tag,
                        #map_value,
                    )
                }
            }
            ValueTy::Message => quote! {
                {
                    // For message types, we can't create a default value without an arena
                    // Use encoded_len_message which doesn't require V: Default
                    #prost_path::encoding::#module::encoded_len_message(
                        #kl,
                        #prost_path::encoding::message::encoded_len,
                        &#key_default,
                        #tag,
                        #map_value,
                    )
                }
            },
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.clear())
    }

    /// Returns methods to embed in the message.
    pub fn methods(&self, prost_path: &Path, ident: &TokenStream) -> Option<TokenStream> {
        if let ValueTy::Scalar(scalar::Ty::Enumeration(ty)) = &self.value_ty {
            let key_ty = self.key_ty.rust_type(prost_path);
            let key_ref_ty = self.key_ty.rust_ref_type();

            let get = Ident::new(&format!("get_{ident}"), Span::call_site());
            let insert = Ident::new(&format!("insert_{ident}"), Span::call_site());
            let take_ref = if self.key_ty.is_numeric() {
                quote!(&)
            } else {
                quote!()
            };

            let get_doc = format!(
                "Returns the enum value for the corresponding key in `{ident}`, \
                 or `None` if the entry does not exist or it is not a valid enum value."
            );
            let insert_doc = format!("Inserts a key value pair into `{ident}`.");
            Some(quote! {
                #[doc=#get_doc]
                pub fn #get(&self, key: #key_ref_ty) -> ::core::option::Option<#ty> {
                    self.#ident.get(#take_ref key).cloned().and_then(|x| {
                        let result: ::core::result::Result<#ty, _> = ::core::convert::TryFrom::try_from(x);
                        result.ok()
                    })
                }
                #[doc=#insert_doc]
                pub fn #insert(&mut self, key: #key_ty, value: #ty) -> ::core::option::Option<#ty> {
                    self.#ident.insert(key, value as i32).and_then(|x| {
                        let result: ::core::result::Result<#ty, _> = ::core::convert::TryFrom::try_from(x);
                        result.ok()
                    })
                }
            })
        } else {
            None
        }
    }

    /// Returns a newtype wrapper around the map, implementing nicer Debug
    ///
    /// The Debug tries to convert any enumerations met into the variants if possible, instead of
    /// outputting the raw numbers.
    pub fn debug(&self, prost_path: &Path, wrapper_name: TokenStream) -> TokenStream {
        // A fake field for generating the debug wrapper
        let key_wrapper = fake_scalar(self.key_ty.clone()).debug(prost_path, quote!(KeyWrapper));
        // Use 'a lifetime for debug wrapper instead of 'arena
        let key = match &self.key_ty {
            scalar::Ty::String => quote!(&'a str),
            _ => self.key_ty.rust_ref_type(),
        };
        let value_wrapper = self.value_ty.debug(prost_path);

        // In arena mode, we use ArenaMap for all maps
        let fmt = quote! {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                #key_wrapper
                #value_wrapper
                let mut builder = f.debug_map();
                for (k, v) in self.0.iter() {
                    builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                }
                builder.finish()
            }
        };
        match &self.value_ty {
            ValueTy::Scalar(ty) => {
                if let scalar::Ty::Bytes(_) = *ty {
                    return quote! {
                        struct #wrapper_name<'a>(&'a dyn ::core::fmt::Debug);
                        impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                                self.0.fmt(f)
                            }
                        }
                    };
                }

                // For arena mode, use the reference type with explicit lifetime
                let value = match ty {
                    scalar::Ty::String => quote!(&'a str),
                    scalar::Ty::Bytes(_) => quote!(&'a [u8]),
                    _ => ty.rust_ref_type(),
                };
                quote! {
                    struct #wrapper_name<'a>(&'a #prost_path::ArenaMap<'a, #key, #value>);
                    impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                        #fmt
                    }
                }
            }
            ValueTy::Message => quote! {
                struct #wrapper_name<'a, V: 'a>(&'a #prost_path::ArenaMap<'a, #key, V>);
                impl<'a, V> ::core::fmt::Debug for #wrapper_name<'a, V>
                where
                    V: ::core::fmt::Debug + 'a,
                {
                    #fmt
                }
            },
        }
    }
}

fn key_ty_from_str(s: &str) -> Result<scalar::Ty, Error> {
    let ty = scalar::Ty::from_str(s)?;
    match ty {
        scalar::Ty::Int32
        | scalar::Ty::Int64
        | scalar::Ty::Uint32
        | scalar::Ty::Uint64
        | scalar::Ty::Sint32
        | scalar::Ty::Sint64
        | scalar::Ty::Fixed32
        | scalar::Ty::Fixed64
        | scalar::Ty::Sfixed32
        | scalar::Ty::Sfixed64
        | scalar::Ty::Bool
        | scalar::Ty::String => Ok(ty),
        _ => bail!("invalid map key type: {}", s),
    }
}

/// A map value type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueTy {
    Scalar(scalar::Ty),
    Message,
}

impl ValueTy {
    fn from_str(s: &str) -> Result<ValueTy, Error> {
        if let Ok(ty) = scalar::Ty::from_str(s) {
            Ok(ValueTy::Scalar(ty))
        } else if s.trim() == "message" {
            Ok(ValueTy::Message)
        } else {
            bail!("invalid map value type: {}", s);
        }
    }

    /// Returns a newtype wrapper around the ValueTy for nicer debug.
    ///
    /// If the contained value is enumeration, it tries to convert it to the variant. If not, it
    /// just forwards the implementation.
    fn debug(&self, prost_path: &Path) -> TokenStream {
        match self {
            ValueTy::Scalar(ty) => fake_scalar(ty.clone()).debug(prost_path, quote!(ValueWrapper)),
            ValueTy::Message => quote!(
                fn ValueWrapper<T>(v: T) -> T {
                    v
                }
            ),
        }
    }
}
