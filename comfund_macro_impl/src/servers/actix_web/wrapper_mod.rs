use syn::{parse_quote, parse_quote_spanned};

use crate::{contract::{endpoint::Endpoint, param::Param, Contract}, servers::{names::Names, wrap_fn::WrapperFn}};

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
        parse_quote!(::actix_web::web::Path),
        parse_quote!(::actix_web::web::Query),
        map_body_ty,
        map_ret_ty,
        map_result,
        |expr| parse_quote!(#expr.into_inner()),
    )
    .impl_for(ep, &names)
}

fn map_body_ty(_ep: &Endpoint, param: &Param) -> syn::Type {
    use crate::contract::transport::Transport;

    let ty = &param.ty;

    match param.meta.transport() {
        Transport::Json => parse_quote_spanned! {
            param.id.span()=>
            ::actix_web::web::Json<#ty>
        },
        Transport::Multipart => parse_quote_spanned! {
            param.id.span()=>
            ::actix_multipart::form::MultipartForm<#ty>
        },
        _ => unreachable!(),
    }
}

fn map_ret_ty(ep: &Endpoint) -> syn::Type {
    use crate::contract::content_type::ContentType;
    let ret_ty = ep.ret.clone();

    match ep.content_type() {
        // TODO: Response types mapping when defined common supported returned content types
        ContentType::ApplicationJson => {
            parse_quote!(::actix_web::web::Json<#ret_ty>)
        }
        _ => ret_ty,
    }
}

fn map_result(ep: &Endpoint, result: syn::Expr) -> syn::Expr {
    use crate::contract::content_type::ContentType;

    match ep.content_type() {
        ContentType::ApplicationJson => {
            parse_quote!(::actix_web::web::Json(#result))
        }
        ContentType::TextPlain => parse_quote!(#result),
    }
}
