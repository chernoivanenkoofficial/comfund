use std::iter;

use crate::contract::endpoint::Endpoint;
use crate::contract::inputs::Inputs;
use crate::contract::method::Method;
use crate::contract::transport::Transport;
use crate::contract::{content_type::ContentType, param::Param};
use crate::Contract;
use comfund_paths::path_template::{PathTemplate, Segment};
use quote::{format_ident, quote, ToTokens};
use syn::parse_quote;

pub fn implement(contract: &Contract) -> proc_macro2::TokenStream {
    let client_impl = client_impl::implement(contract);
    let static_impl = static_impl::implement(contract);

    // TODO: Add "not any other feature" clause to conditional reexport,
    // when other client backends become supported
    quote! {
        #[cfg(all(feature = "reqwest"))]
        pub use reqwest::*;

        pub mod reqwest {
            use super::*;

            #[cfg(all(feature = "reqwest", not(feature = "static")))]
            pub use client_impl::*;

            #[cfg(all(feature = "reqwest", not(feature = "static")))]
            mod client_impl {
                use super::*;
                #client_impl
            }

            #[cfg(all(feature = "reqwest", feature = "static"))]
            pub use static_impl::*;

            #[cfg(all(feature = "reqwest", feature = "static"))]
            mod static_impl {
                use super::*;
                #static_impl
            }
        }
    }
}

mod client_impl {
    use super::*;

    pub fn implement(contract: &Contract) -> impl ToTokens {
        let client_ident = format_ident!("{}Client", &contract.id);
        let attrs = contract.attrs.iter();

        let client_struct = quote! {
            #(#attrs)*
            pub struct #client_ident {
                root: ::std::borrow::Cow<'static, str>
            }
        };

        let endpoints = contract.endpoints.iter().map(impl_endpoint);

        quote! {
            #client_struct

            impl #client_ident {
                pub fn new(root: &impl ::std::string::ToString) -> Self {
                    Self {
                        root: ::std::borrow::Cow::Owned(root.to_string())
                    }
                }

                pub const fn new_const(root: &'static str) -> Self {
                    Self {
                        root: ::std::borrow::Cow::Borrowed(root)
                    }
                }

                #(#endpoints)*
            }
        }
    }

    fn impl_endpoint(ep: &Endpoint) -> impl ToTokens {
        let sig = sig(ep, true);
        let body = impl_body(parse_quote! { self.root.clone() }, ep);
        let attrs = ep.attrs.iter();

        quote! {
            #(#attrs)*
            #sig {
                #body
            }
        }
    }
}

mod static_impl {
    use super::*;

    pub fn implement(contract: &Contract) -> impl ToTokens {
        let root_cell_id = format_ident!("____{}_ROOT", contract.id.to_string());

        let singleton = impl_root_singleton(&root_cell_id, contract);
        let endpoints = contract
            .endpoints
            .iter()
            .map(|ep| impl_endpoint(&root_cell_id, ep));

        quote! {
            #singleton

            #(#endpoints)*
        }
    }

    fn impl_endpoint(root_cell_id: &syn::Ident, ep: &Endpoint) -> impl ToTokens {
        let sig = sig(ep, false);
        // TODO: Default root resolver
        let body = impl_body(parse_quote!(#root_cell_id.get().unwrap()), ep);
        let attrs = ep.attrs.iter();

        quote! {
            #(#attrs)*
            #sig {
                #body
            }
        }
    }

    fn impl_root_singleton(root_cell_id: &syn::Ident, contract: &Contract) -> impl ToTokens {
        // TODO: Add snake case conversion
        let set_fn_name = format_ident!("set_{}_root", contract.id.to_string().to_lowercase());
        let get_fn_name = format_ident!("get_{}_root", contract.id.to_string().to_lowercase());

        quote! {
            #[allow(non_upper_case_globals)]
            static #root_cell_id: ::std::sync::OnceLock<&'static str> = ::std::syn::OnceLock::new();

            pub fn #set_fn_name(root: &'static str) {
                #root_cell_id.set(root).unwrap();
            }

            pub fn #get_fn_name() -> &'static str {
                #root_cell_id.get().copied().unwrap_or("")
            }
        }
    }
}

fn sig(ep: &Endpoint, with_reciever: bool) -> impl ToTokens {
    let path_params = ep.path_inputs.as_ref().map_or(quote! {}, |inputs| {
        let params = inputs.params.iter().map(Param::as_function_argument);
        quote!(#(#params,)*)
    });

    let query_params = ep.query_inputs.as_ref().map_or(quote! {}, |inputs| {
        let params = inputs.params.iter().map(Param::as_function_argument);
        quote!(#(#params,)*)
    });

    let body_param = ep.body_param.as_ref().map(Param::as_function_argument);

    let ep_name = &ep.id;

    let ret_ty = &ep.ret;

    let reciever = if with_reciever {
        Some(quote! { &self, })
    } else {
        None
    };

    quote! {
        pub async fn #ep_name(#reciever #path_params #query_params #body_param) -> ::comfund::Result<#ret_ty>
    }
}

fn impl_body(root: syn::Expr, ep: &Endpoint) -> impl ToTokens {
    let method: syn::Path = match ep.meta.method() {
        Method::Get => parse_quote!(::reqwest::Method::GET),
        Method::Post => parse_quote!(::reqwest::Method::POST),
        Method::Delete => parse_quote!(::reqwest::Method::DELETE),
        Method::Put => parse_quote!(::reqwest::Method::PUT),
        Method::Patch => parse_quote!(::request::Method::PATCH),
    };

    let path_params = path_expr(root, ep);
    let query_params = query_expr(ep).map(|expr| quote! { .query(&#expr)});
    let body_params = body_expr(ep);

    let content_mapping = match ep.meta.options().content_type.clone().unwrap_or_default() {
        ContentType::ApplicationJson => quote! { .json() },
        ContentType::TextPlain => quote! { .text() },
    };

    quote! {
        ::reqwest::Client::builder()
            .build()
            .map_err(::comfund::ClientError::Reqwest)?
            .request(#method, #path_params)
            #query_params
            #body_params
            .send()
            .await
            .map_err(::comfund::ClientError::Reqwest)?
            #content_mapping
            .await
            .map_err(::comfund::ClientError::Reqwest)
    }
}

fn path_expr(root: syn::Expr, ep: &Endpoint) -> impl ToTokens {
    let inputs = if let Some(inputs) = ep.path_inputs.as_ref() {
        inputs
    } else {
        let path_lit = ep.meta.path_lit();

        return quote! {
            format!("{}{}", #root, #path_lit)
        };
    };

    let path_span = ep.meta.path_lit().span();
    let path = ep.meta.path();

    // Template correctness validated in endpoint
    let template = PathTemplate::new(&path).unwrap();

    let segments = template.segments().iter().map(|seg| match seg {
        Segment::Capture(cap) => {
            let lit = syn::LitStr::new(cap, path_span);
            quote! {
                ::comfund::paths::Segment::Capture(#lit)
            }
        }
        Segment::Static(lit) => {
            let lit = syn::LitStr::new(lit, path_span);
            quote! {
                ::comfund::paths::Segment::Static(#lit)
            }
        }
    });

    let idents = template
        .idents()
        .iter()
        .map(|ident| syn::LitStr::new(ident, path_span));

    let wildcard = if let Some(ident) = template.wildcard() {
        let lit = syn::LitStr::new(ident, path_span);

        quote! {Some(#lit)}
    } else {
        quote! {None}
    };

    let template_id = format_ident!("______TEMPLATE");

    let template_const = quote! {
        const #template_id: ::comfund::paths::PathTemplate::<'static> = ::comfund::paths::PathTemplate::new_static(
            &[
                #(#segments),*
            ],
            &[
                #(#idents),*
            ],
            #wildcard
        );
    };

    let inputs_init = if inputs.is_flat() {
        let name = &inputs.params.first().unwrap().name;

        quote! {
            #name
        }
    } else {
        let query_struct_init = inputs.initializator(None).unwrap();

        quote! {
            #query_struct_init
        }
    };

    quote! {
        {
            #template_const
            format!("{}{}", #root, ::comfund::paths::serialize(&#template_id, &#inputs_init)?)
        }
    }
}

fn query_expr(ep: &Endpoint) -> Option<impl ToTokens> {
    let inputs = ep.query_inputs.as_ref()?;

    if inputs.is_flat() {
        let name = &inputs.params.first().unwrap().name;

        Some(quote! {
            #name
        })
    } else {
        let query_struct_init = inputs.initializator(None);

        Some(quote! {
            #query_struct_init
        })
    }
}

fn body_expr(ep: &Endpoint) -> Option<impl ToTokens> {
    let param = ep.body_param.as_ref()?;
    let param_id = &param.name;

    let ret = match param.meta.transport() {
        Transport::Body => quote! { .body(#param_id) },
        Transport::Json => quote! { .json(#param_id) },
        _ => unreachable!("Unexpected transport kind of body argument"),
    };

    Some(ret)
}
