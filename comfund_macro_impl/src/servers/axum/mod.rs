mod axum_endpoint;

use quote::{format_ident, quote, ToTokens};

use crate::{
    contract::{inputs, Contract},
    servers::axum::axum_endpoint::AxumEndpoint,
};

pub fn implement(contract: &Contract) -> proc_macro2::TokenStream {
    let service_trait_def = def_service_trait(contract);
    let route_fn_impl = impl_route_function(contract);

    quote! {
        #[cfg(feature = "axum")]
        pub use axum_impl::*;
        #[cfg(feature = "axum")]
        mod axum_impl {
            use super::*;
            #service_trait_def
            #route_fn_impl
        }
    }
}

fn def_service_trait(contract: &Contract) -> impl quote::ToTokens {
    let contract_id = &contract.id;

    let axum_eps = contract
        .endpoints
        .iter()
        .map(AxumEndpoint::new)
        .collect::<Vec<_>>();
    let ep_trait_items = axum_eps.iter().map(AxumEndpoint::def_in_trait);

    quote! {
        pub trait #contract_id: 'static {
            type State: 'static + ::core::marker::Send + ::core::marker::Sync + ::core::clone::Clone;

            #(#ep_trait_items)*
        }
    }
}

fn impl_route_function(contract: &Contract) -> impl quote::ToTokens {
    let route_fn_id = get_route_fn_id(&contract.id);
    let contract_trait_id = &contract.id;
    let service_trait_var = format_ident!("C");
    let routing_expressions = get_routing_expressions(contract, &service_trait_var);

    quote! {
        pub fn #route_fn_id<#service_trait_var: #contract_trait_id>(state: #service_trait_var::State) -> ::comfund::axum::Router<#service_trait_var::State> {
            ::comfund::axum::Router::new()
                #(#routing_expressions)*
                .with_state(state)
        }
    }
}

fn get_route_fn_id(contract_id: &syn::Ident) -> syn::Ident {
    let route_fn_str = format!("route_{}", stringcase::snake_case(&contract_id.to_string()));
    syn::Ident::new(&route_fn_str, contract_id.span())
}

fn get_routing_expressions(
    contract: &Contract,
    service_trait_var: &syn::Ident,
) -> impl Iterator<Item = impl ToTokens> {
    use std::collections::HashMap;

    let mut ep_map = HashMap::with_capacity(contract.endpoints.len());
    for ep in &contract.endpoints {
        ep_map
            .entry(ep.meta.path_lit())
            .or_insert_with(Vec::new)
            .push(ep);
    }

    let mut exprs = Vec::with_capacity(ep_map.len());

    for (path, eps) in ep_map {
        let method_router_exprs = eps
            .into_iter()
            .map(|ep| AxumEndpoint::new(ep).method_router(service_trait_var));

        let expr = quote! {
            .route(#path, #(#method_router_exprs).*)
        };

        exprs.push(expr);
    }

    exprs.into_iter()
}
