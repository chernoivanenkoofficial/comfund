use quote::{format_ident, quote};
use syn::parse_quote;

use crate::contract::{
    content_type::ContentType, endpoint::Endpoint, method::Method, param::Param,
    transport::Transport,
};

pub struct AxumEndpoint<'e> {
    ep: &'e Endpoint,
    handler_name: syn::Ident,
    decorator_id: syn::Ident,
    ext_type_name: syn::Ident,
}

impl<'e> AxumEndpoint<'e> {
    pub fn new(ep: &'e Endpoint) -> Self {
        let handler_name = ep.id.clone();
        let mut handler_str = handler_name.to_string();

        let decorator_name = format_ident!("set_{}_middleware", &handler_name);

        let extensions_type_name = {
            handler_str.push_str("_extensions");
            let extensions_str = stringcase::pascal_case(&handler_str);

            syn::Ident::new(&extensions_str, handler_name.span())
        };

        Self {
            ep,
            handler_name,
            decorator_id: decorator_name,
            ext_type_name: extensions_type_name,
        }
    }

    fn path(&self) -> &syn::LitStr {
        self.ep.meta.path_lit()
    }

    fn handler_id(&self) -> &syn::Ident {
        &self.ep.id
    }

    fn decorator_id(&self) -> &syn::Ident {
        &self.decorator_id
    }

    fn ext_type_name(&self) -> &syn::Ident {
        &self.ext_type_name
    }

    pub fn def_in_trait(&self) -> impl quote::ToTokens {
        let ext_type_def = def_ext_type(self.ext_type_name());
        let handler_def = def_handler(self, self.ext_type_name());
        let decorator_def = def_decorator(self);

        quote! {
            #ext_type_def
            #handler_def
            #decorator_def
        }
    }

    pub fn method_router(&self, service_trait_var: &syn::Ident) -> impl quote::ToTokens {
        let method: syn::Ident = match self.ep.meta.method() {
            Method::Get => parse_quote!(get),
            Method::Post => parse_quote!(post),
            Method::Delete => parse_quote!(delete),
            Method::Patch => parse_quote!(update),
            Method::Put => parse_quote!(put),
        };

        let handler_id = self.handler_id();
        let decorator_id = self.decorator_id();

        quote! {
            ::comfund::axum::routing::#method(
                #service_trait_var::#decorator_id(
                    #service_trait_var::#handler_id
                )
            )
        }
    }
}

fn def_ext_type(ext_type_name: &syn::Ident) -> impl quote::ToTokens {
    quote! {
        type #ext_type_name: ::comfund::axum::extract::FromRequestParts<Self::State> + Send;
    }
}

fn def_handler(aep: &AxumEndpoint, ext_type_name: &syn::Ident) -> impl quote::ToTokens {
    let path_inputs_param = aep.ep.path_inputs.as_ref().map(|inputs| {
        let ty = &inputs.ty;
        let id = inputs
            .id
            .clone()
            .unwrap_or_else(|| syn::Ident::new("path_inputs", aep.handler_id().span()));

        quote!(#id: ::comfund::axum::extract::Path<#ty>,)
    });

    let query_inputs_param = aep.ep.query_inputs.as_ref().map(|inputs| {
        let ty = &inputs.ty;
        let id = inputs
            .id
            .clone()
            .unwrap_or_else(|| syn::Ident::new("query_inputs", aep.handler_id().span()));

        quote!(#id: ::comfund::axum::extract::Query<#ty>,)
    });

    let ext_param = quote! { extensions: Self::#ext_type_name ,};
    let body_param = aep.ep.body_param.as_ref().map_or(quote! {}, |param| {
        let name = &param.name;
        let ty = get_body_param_ty(param);

        quote! { #name: #ty }
    });

    let handler_id = aep.handler_id();

    let ret_ty = {
        let ret_ty = aep.ep.ret.clone();

        match aep.ep.meta.options().content_type {
            // TODO: Response types mapping when defined common supported returned content types
            ContentType::ApplicationJson => parse_quote!(::comfund::axum::Json<#ret_ty>),
            _ => ret_ty,
        }
    };

    quote! {
        fn #handler_id(
            #path_inputs_param
            #query_inputs_param
            #ext_param
            #body_param
        ) -> impl ::std::future::Future<Output = #ret_ty> + ::core::marker::Send;
    }
}

fn def_decorator(aep: &AxumEndpoint) -> impl quote::ToTokens {
    let path_ty = aep.ep.path_inputs.as_ref().map(|inputs| {
        let ty = &inputs.ty;
        quote!(,::comfund::axum::extract::Path<#ty>)
    });
    let query_ty = aep.ep.query_inputs.as_ref().map(|inputs| {
        let ty = &inputs.ty;
        quote!(,::comfund::axum::extract::Query<#ty>)
    });
    let ext_ty = aep.ext_type_name();
    let body_ty = aep.ep.body_param.as_ref().map(|param| {
        let ty = get_body_param_ty(param);
        quote!(,#ty)
    });
    let decorator_id = aep.decorator_id();

    let handler_constraint = quote! {
        impl ::comfund::axum::handler::Handler<(
            M
            #path_ty
            #query_ty
            ,Self::#ext_ty
            #body_ty
        ), Self::State>
    };

    quote! {
        fn #decorator_id<M>(
            handler: #handler_constraint
        ) -> #handler_constraint {
            handler
        }
    }
}

fn get_body_param_ty(param: &Param) -> syn::Type {
    let ty = &param.ty;

    match param.meta.transport() {
        Transport::Json => parse_quote!(::comfund::axum::extract::Json<#ty>),
        Transport::Multipart => parse_quote!(::comfund::axum::extract::Multipart<#ty>),
        _ => unreachable!(),
    }
}
