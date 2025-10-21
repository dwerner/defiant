use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Meta, Path};

use crate::field::{set_bool, set_option, tag_attr, word_attr, Label};

#[derive(Clone)]
pub struct Field {
    pub label: Label,
    pub tag: u32,
}

impl Field {
    pub fn new(attrs: &[Meta], inferred_tag: Option<u32>) -> Result<Option<Field>, Error> {
        let mut message = false;
        let mut label = None;
        let mut tag = None;
        let mut boxed = false;

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("message", attr) {
                set_bool(&mut message, "duplicate message attribute")?;
            } else if word_attr("boxed", attr) {
                set_bool(&mut boxed, "duplicate boxed attribute")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !message {
            return Ok(None);
        }

        if !unknown_attrs.is_empty() {
            bail!(
                "unknown attribute(s) for message field: #[prost({})]",
                quote!(#(#unknown_attrs),*)
            );
        }

        let tag = match tag.or(inferred_tag) {
            Some(tag) => tag,
            None => bail!("message field is missing a tag attribute"),
        };

        Ok(Some(Field {
            label: label.unwrap_or(Label::Optional),
            tag,
        }))
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(attrs, None)? {
            if let Some(attr) = attrs.iter().find(|attr| Label::from_attr(attr).is_some()) {
                bail!(
                    "invalid attribute for oneof field: {}",
                    attr.path().into_token_stream()
                );
            }
            field.label = Label::Required;
            Ok(Some(field))
        } else {
            Ok(None)
        }
    }

    pub fn encode(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                if let Some(msg) = #ident {
                    use #prost_path::Message as _;
                    #prost_path::encoding::encode_key(#tag, #prost_path::encoding::WireType::LengthDelimited, buf);
                    #prost_path::encoding::encode_varint(msg.encoded_len() as u64, buf);
                    msg.encode_raw(buf);
                }
            },
            Label::Required => quote! {
                {
                    use #prost_path::Message as _;
                    let msg = &(#ident);
                    #prost_path::encoding::encode_key(#tag, #prost_path::encoding::WireType::LengthDelimited, buf);
                    #prost_path::encoding::encode_varint(msg.encoded_len() as u64, buf);
                    msg.encode_raw(buf);
                }
            },
            Label::Repeated => quote! {
                {
                    use #prost_path::Message as _;
                    for msg in #ident.iter() {
                        #prost_path::encoding::encode_key(#tag, #prost_path::encoding::WireType::LengthDelimited, buf);
                        #prost_path::encoding::encode_varint(msg.encoded_len() as u64, buf);
                        msg.encode_raw(buf);
                    }
                }
            },
        }
    }

    pub fn merge(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => quote! {
                #prost_path::encoding::message::merge(wire_type,
                                                 #ident.get_or_insert_with(::core::default::Default::default),
                                                 buf,
                                                 arena,
                                                 ctx)
            },
            Label::Required => quote! {
                #prost_path::encoding::message::merge(wire_type, #ident, buf, arena, ctx)
            },
            Label::Repeated => quote! {
                #prost_path::encoding::message::merge_repeated(wire_type, #ident, buf, arena, ctx)
            },
        }
    }

    pub fn encoded_len(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                {
                    use #prost_path::Message as _;
                    match &#ident {
                        Some(msg) => {
                            let len: usize = msg.encoded_len();
                            #prost_path::encoding::key_len(#tag) + #prost_path::encoding::encoded_len_varint(len as u64) + len
                        }
                        None => 0,
                    }
                }
            },
            Label::Required => quote! {
                {
                    use #prost_path::Message as _;
                    let len = (#ident).encoded_len();
                    #prost_path::encoding::key_len(#tag) + #prost_path::encoding::encoded_len_varint(len as u64) + len
                }
            },
            Label::Repeated => quote! {
                {
                    use #prost_path::Message as _;
                    #prost_path::encoding::key_len(#tag) * #ident.len()
                        + #ident
                            .iter()
                            .map(|msg| msg.encoded_len())
                            .map(|len| len + #prost_path::encoding::encoded_len_varint(len as u64))
                            .sum::<usize>()
                }
            },
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => quote!(#ident = ::core::option::Option::None),
            Label::Required => quote!(#ident.clear()),
            Label::Repeated => quote!(#ident.clear()),
        }
    }
}
