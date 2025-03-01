use std::hash::Hash;

use quote::quote;
use syn::parse_quote;

use crate::contract::endpoint::Endpoint;
use crate::contract::param::Param;
use crate::contract::Contract;

#[derive(Debug, Clone)]
pub struct Inputs {
    pub id: Option<syn::Ident>,
    pub ty: syn::Type,
    pub params: Vec<Param>,
    pub definition: Option<proc_macro2::TokenStream>,
}

impl Inputs {
    pub fn is_empty(inputs: Option<&Self>) -> bool {
        match inputs {
            None => true,
            Some(inputs) => inputs.params.is_empty(),
        }
    }

    pub fn is_flat(&self) -> bool {
        self.definition.is_none()
    }

    pub fn initializator(&self, args: Option<&[syn::Ident]>) -> Option<proc_macro2::TokenStream> {
        if self.is_flat() {
            return None;
        }

        let fields = self.params.iter().map(|param| &param.name);
        let name = &self.ty;

        if let Some(args) = args {
            if args.len() != self.params.len() {
                panic!()
            }

            Some(quote! {
                #name {
                    #(#fields: #args),*
                }
            })
        } else {
            Some(quote! {
                #name {
                    #(#fields),*
                }
            })
        }
    }
}

pub fn from_params(ep_name: &syn::Ident, params: Vec<Param>, suffix: &str) -> Option<Inputs> {
    if params.is_empty() {
        None
    } else if params.len() == 1 {
        let id = params[0].name.clone();
        let ty = params[0].ty.clone();

        Some(Inputs {
            id: Some(id),
            ty,
            params,
            definition: None,
        })
    } else {
        let ty = gen_type(ep_name, suffix);

        let fields = params.iter().map(|param| {
            let name = &param.name;
            let ty = &param.ty;
            let flatten = if param.meta.options().flatten.is_set() {
                Some(quote! {
                    #[cfg_attr(
                        any(feature = "reqwest", feature = "actix-web", feature = "axum"),
                        serde(flatten)
                    )]
                })
            } else {
                None
            };

            quote! {
                #flatten
                pub #name: #ty
            }
        });

        let definition = quote! {
            #[cfg_attr(
                any(feature = "reqwest"),
                derive(::comfund::serde::Serialize)
            )]
            #[cfg_attr(
                any(feature = "actix-web", feature = "axum"),
                derive(::comfund::serde::Deserialize)
            )]
            #[cfg_attr(
                any(feature = "reqwest", feature = "axum", feature = "actix-web"),
                serde(crate = "::comfund::serde")
            )]
            pub struct #ty {
                #(#fields),*
            }
        };

        Some(Inputs {
            id: None,
            ty,
            params,
            definition: Some(definition),
        })
    }
}

fn gen_type(ep_name: &syn::Ident, suffix: &str) -> syn::Type {
    let mut ep_str = ep_name.to_string();
    ep_str.push_str(suffix);
    let id = syn::Ident::new(&stringcase::pascal_case(&ep_str), ep_name.span());

    parse_quote!(#id)
}
