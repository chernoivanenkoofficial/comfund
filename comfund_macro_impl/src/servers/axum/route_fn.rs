use syn::{parse_quote, parse_quote_spanned};
use quote::quote;

use crate::{contract::{endpoint::Endpoint, method::Method, Contract}, servers::{names::Names, server_endpoint}};

pub fn def(contract: &Contract) -> syn::ItemFn {
    let contract_id = &contract.id;
    let route_fn_id = get_route_fn_id(&contract.id);
    let service_trait_var = server_endpoint::service_trait_var();
    let routing_expressions = get_routing_expressions(contract);

    parse_quote! {
        pub fn #route_fn_id<#service_trait_var: #contract_id>(state: #service_trait_var::State) -> ::axum::Router<#service_trait_var::State> {
            ::axum::Router::new()
                #(#routing_expressions)*
                .with_state(state)
        }
    }
}

fn get_route_fn_id(contract_id: &syn::Ident) -> syn::Ident {
    let route_fn_str = format!("route_{}", stringcase::snake_case(&contract_id.to_string()));
    syn::Ident::new(&route_fn_str, contract_id.span())
}

fn get_routing_expressions(contract: &Contract) -> impl Iterator<Item = impl quote::ToTokens> {
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
        let method_router_exprs = eps.into_iter().map(routing_expr);

        let expr = quote! {
            .route(#path, #(#method_router_exprs).*)
        };

        exprs.push(expr);
    }

    exprs.into_iter()
}

fn routing_expr(ep: &Endpoint) -> syn::Expr {
    let names = Names::new(ep);

    let mut method: syn::Ident = match ep.meta.method() {
        Method::Get => parse_quote!(get),
        Method::Post => parse_quote!(post),
        Method::Delete => parse_quote!(delete),
        Method::Patch => parse_quote!(update),
        Method::Put => parse_quote!(put),
    };

    let handler_id = names.handler_id();
    method.set_span(handler_id.span());

    let service_trait_var = server_endpoint::service_trait_var();
    let decorator_id = names.decorator_id();

    parse_quote_spanned! {
        ep.id.span()=>
        ::axum::routing::#method(
            ::axum::handler::Handler::layer(
                ___wrappers::#handler_id::<#service_trait_var>,
                #service_trait_var::#decorator_id()
            )
        )
    }
}
