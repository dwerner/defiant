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
        // TODO: Rename these attributes to "arena_map" to reflect that all maps use
        // arena allocation now. Current syntax like `prost(btree_map = "string, message")`
        // should become `prost(arena_map = "string, message")` for clarity.
        match s {
            "map" | "hash_map" => Some(MapTy::HashMap),
            "btree_map" => Some(MapTy::BTreeMap),
            _ => None,
        }
    }

    fn module(&self) -> Ident {
        // In arena mode, always use arena_map for map fields
        // (both hash_map and btree_map use the same arena-allocated encoding)
        Ident::new("arena_map", Span::call_site())
    }

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
            _ => quote!(Default::default()),
        }
    }

    /// Returns the arena-allocated key type for maps
    fn arena_key_type(&self) -> TokenStream {
        use scalar::Ty::*;
        match &self.key_ty {
            String => quote!(&'a str),
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
        // Use _ref variants for string keys since they're &'arena str, not String
        // Wrap in closures to match the expected signature: &K -> &&str for _ref functions
        let (ke, kl) = if matches!(self.key_ty, scalar::Ty::String) {
            (quote!(|tag, key: &&str, buf| #prost_path::encoding::#key_mod::encode_ref(tag, *key, buf)),
             quote!(|tag, key: &&str| #prost_path::encoding::#key_mod::encoded_len_ref(tag, *key)))
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
                let ve = quote!(#prost_path::encoding::#val_mod::encode);
                let vl = quote!(#prost_path::encoding::#val_mod::encoded_len);
                let val_default = value_ty.rust_ref_type();
                quote! {
                    #prost_path::encoding::#module::encode_with_defaults(
                        #ke,
                        #kl,
                        #ve,
                        #vl,
                        &#key_default,
                        &#val_default::default(),
                        #tag,
                        #map_value,
                        buf,
                    );
                }
            }
            ValueTy::Message => {
                // For messages, create a temporary default for comparison
                // (though in practice, message map values are rarely equal to default)
                quote! {
                    {
                        let val_default = ::core::default::Default::default();
                        #prost_path::encoding::#module::encode_with_defaults(
                            #ke,
                            #kl,
                            #prost_path::encoding::message::encode,
                            #prost_path::encoding::message::encoded_len,
                            &#key_default,
                            &val_default,
                            #tag,
                            #map_value,
                            buf,
                        );
                    }
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
                let default = quote!(#ty::default() as i32);
                // Wrap int32::merge to ignore arena
                let vm = quote!(|wire_type, val, buf, _arena, ctx| {
                    #prost_path::encoding::int32::merge(wire_type, val, buf, ctx)
                });
                quote! {
                    #prost_path::encoding::#module::merge_with_default(
                        #km,
                        #vm,
                        #default,
                        &mut #ident,
                        buf,
                        arena,
                        ctx,
                    )
                }
            }
            ValueTy::Scalar(value_ty) => {
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
                quote!(#prost_path::encoding::#module::merge(#km, #vm, &mut #ident, buf, arena, ctx))
            }
            ValueTy::Message => {
                // Wrap message::merge in a closure to delay trait bound checking
                // This allows circular dependencies (e.g., Struct -> Value -> Struct)
                let vm = quote!(|wire_type, val, buf, arena, ctx| {
                    #prost_path::encoding::message::merge(wire_type, val, buf, arena, ctx)
                });
                quote! {
                    #prost_path::encoding::#module::merge(
                        #km,
                        #vm,
                        &mut #ident,
                        buf,
                        arena,
                        ctx,
                    )
                }
            },
        }
    }

    /// Returns an expression which evaluates to the encoded length of the map.
    pub fn encoded_len(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        let key_mod = self.key_ty.module();
        // Use _ref variant for string keys since they're &'arena str, not String
        // Wrap in closure to match the expected signature: &K -> &&str for _ref function
        let kl = if matches!(self.key_ty, scalar::Ty::String) {
            quote!(|tag, key: &&str| #prost_path::encoding::#key_mod::encoded_len_ref(tag, *key))
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
                let vl = quote!(#prost_path::encoding::#val_mod::encoded_len);
                let val_default = value_ty.rust_ref_type();
                quote! {
                    #prost_path::encoding::#module::encoded_len_with_defaults(
                        #kl,
                        #vl,
                        &#key_default,
                        &#val_default::default(),
                        #tag,
                        #map_value,
                    )
                }
            }
            ValueTy::Message => quote! {
                {
                    let val_default = ::core::default::Default::default();
                    #prost_path::encoding::#module::encoded_len_with_defaults(
                        #kl,
                        #prost_path::encoding::message::encoded_len,
                        &#key_default,
                        &val_default,
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
        let key = self.arena_key_type();
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

                let value = ty.rust_type(prost_path);
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
