mod actix_endpoint;

use quote::{format_ident, quote};

use crate::{contract::Contract, servers::actix_web::actix_endpoint::ActixEndpoint};

pub fn implement(contract: &Contract) -> proc_macro2::TokenStream {
    let service_trait_def = def_service_trait(contract);
    let configure_fn_impl = impl_configure_fn(contract);

    quote! {
        #[cfg(
            all(
                feature = "actix-web",
                not(any(
                    feature = "axum"
                ))
            )
        )]
        pub use actix_web::*;

        #[cfg(feature = "actix-web")]
        pub mod actix_web {
            use super::*;
            #service_trait_def
            #configure_fn_impl
        }
    }
}

fn def_service_trait(contract: &Contract) -> impl quote::ToTokens {
    let contract_id = &contract.id;
    let actix_eps = contract
        .endpoints
        .iter()
        .map(ActixEndpoint::new)
        .collect::<Vec<_>>();

    let ep_trait_items = actix_eps.iter().map(ActixEndpoint::def_in_trait);

    quote! {
        pub trait #contract_id: 'static {
            #(#ep_trait_items)*
        }
    }
}

fn impl_configure_fn(contract: &Contract) -> impl quote::ToTokens {
    let contract_id = &contract.id;
    let configure_fn_id = get_configure_fn_id(contract_id);
    let service_trait_var = format_ident!("C");
    let routing_expressions = get_routing_expressions(contract, &service_trait_var);

    quote! {
        pub fn #configure_fn_id<#service_trait_var: #contract_id>(cfg: &mut ::actix_web::web::ServiceConfig) {
            cfg #(#routing_expressions)*;
        }
    }
}

fn get_configure_fn_id(contract_id: &syn::Ident) -> syn::Ident {
    let configure_fn_str = format!(
        "configure_{}",
        stringcase::snake_case(&contract_id.to_string())
    );
    syn::Ident::new(&configure_fn_str, contract_id.span())
}

fn get_routing_expressions(
    contract: &Contract,
    service_trait_var: &syn::Ident,
) -> impl Iterator<Item = impl quote::ToTokens> {
    use std::collections::HashMap;

    let mut ep_map = HashMap::with_capacity(contract.endpoints.len());
    for ep in &contract.endpoints {
        ep_map
            .entry(ep.meta.path_lit())
            .or_insert_with(Vec::new)
            .push(ep)
    }

    let mut exprs = Vec::with_capacity(ep_map.len());

    for (path, eps) in ep_map {
        let route_expressions = eps
            .into_iter()
            .map(|ep| ActixEndpoint::new(ep).method_router(service_trait_var));

        let expr = quote! {
            .service(
                ::actix_web::web::resource(#path)
                    #(.route(#route_expressions))*
            )
        };

        exprs.push(expr);
    }

    exprs.into_iter()
}
