use quote::{format_ident, quote_spanned};
use syn::{parse_quote, parse_quote_spanned};

use crate::servers::server_endpoint;
use crate::servers::names::Names;
use crate::contract::Contract;
use crate::contract::endpoint::Endpoint;

pub fn def(contract: &Contract) -> syn::ItemFn {
    let contract_id = &contract.id;
    let configure_fn_id = get_configure_fn_id(contract_id);
    let service_trait_var = format_ident!("C");
    let routing_expressions = get_routing_expressions(contract);

    parse_quote_spanned! {
        contract.id.span()=>
        pub fn #configure_fn_id<#service_trait_var: #contract_id>(cfg: &mut ::actix_web::web::ServiceConfig) {
            cfg #(#routing_expressions)*;
        }
    }
}

fn get_configure_fn_id(contract_id: &syn::Ident) -> syn::Ident {
    format_ident!(
        "configure_{}",
        stringcase::snake_case(&contract_id.to_string()),
        span = contract_id.span()
    )
}

fn get_routing_expressions(contract: &Contract) -> impl Iterator<Item = impl quote::ToTokens> {
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
        let route_expressions = eps.into_iter().map(routing_expr);

        let expr = quote_spanned! {
            contract.id.span()=>
            .service(
                ::actix_web::web::resource(#path)
                    #(.route(#route_expressions))*
            )
        };

        exprs.push(expr);
    }

    exprs.into_iter()
}

fn routing_expr(ep: &Endpoint) -> syn::Expr {
    use crate::contract::method::Method;

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

    let decorator_id = names.decorator_id();
    let service_trait_var = server_endpoint::service_trait_var();

    parse_quote! {
        ::actix_web::web::#method().to(
            ___wrappers::#handler_id::<#service_trait_var>).wrap(#service_trait_var::#decorator_id())
    }
}
