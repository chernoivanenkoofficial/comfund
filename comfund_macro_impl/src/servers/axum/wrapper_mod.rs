use syn::parse_quote;

use crate::{
    contract::{
        content_type::ContentType, endpoint::Endpoint, param::Param, transport::Transport, Contract,
    },
    servers::{names::Names, wrap_fn::WrapperFn},
};

pub fn def(contract: &Contract) -> syn::ItemMod {
    let fns = contract.endpoints.iter().map(impl_wrapper_function);

    parse_quote! {
        mod ___wrappers {
            use super::*;

            #(#fns)*
        }
    }
}

fn impl_wrapper_function(ep: &Endpoint) -> syn::ItemFn {
    let names = Names::new(ep);

    WrapperFn::new(
        parse_quote!(::axum::extract::Path),
        parse_quote!(::axum::extract::Query),
        map_body_ty,
        map_ret_ty,
        map_result,
        |expr| parse_quote!(#expr.0),
    )
    .impl_for(ep, &names)
}

fn map_body_ty(_ep: &Endpoint, param: &Param) -> syn::Type {
    let ty = &param.ty;

    match param.meta.transport() {
        Transport::Json => parse_quote!(::axum::extract::Json<#ty>),
        Transport::Multipart => parse_quote!(::axum::extract::Multipart<#ty>),
        _ => unreachable!(),
    }
}

fn map_ret_ty(ep: &Endpoint) -> syn::Type {
    let ret_ty = ep.ret.clone();

    match ep.content_type() {
        // TODO: Response types mapping when defined common supported returned content types
        ContentType::ApplicationJson => parse_quote!(::axum::Json<#ret_ty>),
        _ => ret_ty,
    }
}

fn map_result(ep: &Endpoint, result: syn::Expr) -> syn::Expr {
    match ep.content_type() {
        // TODO: Response types mapping when defined common supported returned content types
        ContentType::ApplicationJson => {
            parse_quote!(::comfund::axum::reexport::extract::Json(#result))
        }
        ContentType::TextPlain => parse_quote!(#result),
    }
}
