use quote::quote;
use syn::{parse_quote, parse_quote_spanned};

use crate::contract::content_type::ContentType;
use crate::contract::endpoint::Endpoint;
use crate::contract::method::Method;
use crate::contract::param::Param;
use crate::contract::transport::Transport;

pub struct ActixEndpoint<'e> {
    ep: &'e Endpoint,
    handler_name: syn::Ident,
    ext_type_name: syn::Ident,
}

impl<'e> ActixEndpoint<'e> {
    pub fn new(ep: &'e Endpoint) -> Self {
        let handler_name = ep.id.clone();

        let ext_type_name = {
            let mut handler_str = handler_name.to_string();
            handler_str.push_str("_extensions");

            let extensions_str = stringcase::pascal_case(&handler_str);

            syn::Ident::new(&extensions_str, handler_name.span())
        };

        Self {
            ep,
            ext_type_name,
            handler_name,
        }
    }
}

impl ActixEndpoint<'_> {
    fn path(&self) -> &syn::LitStr {
        self.ep.meta.path_lit()
    }

    fn handler_id(&self) -> &syn::Ident {
        &self.handler_name
    }

    fn ext_type_name(&self) -> &syn::Ident {
        &self.ext_type_name
    }

    pub fn def_in_trait(&self) -> impl quote::ToTokens {
        let ext_type_def = def_ext_type(self.ext_type_name());
        let handler_def = def_handler(self, self.ext_type_name());

        quote! {
            #ext_type_def
            #handler_def
        }
    }

    pub fn method_router(&self, service_trait_var: &syn::Ident) -> impl quote::ToTokens {
        let mut method: syn::Ident = match self.ep.meta.method() {
            Method::Get => parse_quote!(get),
            Method::Post => parse_quote!(post),
            Method::Delete => parse_quote!(delete),
            Method::Patch => parse_quote!(update),
            Method::Put => parse_quote!(put),
        };

        let handler_id = self.handler_id();
        method.set_span(handler_id.span());

        quote! {
            ::actix_web::web::#method().to(#service_trait_var::#handler_id)
        }
    }
}

fn def_ext_type(ext_type_name: &syn::Ident) -> impl quote::ToTokens {
    let item_type: syn::TraitItemType = parse_quote_spanned!(
        ext_type_name.span()=>

        type #ext_type_name: ::actix_web::FromRequest;
    );

    item_type
}

fn def_handler(aep: &ActixEndpoint, ext_type_name: &syn::Ident) -> impl quote::ToTokens {
    use syn::punctuated::Punctuated;

    let mut fn_args: Punctuated<syn::FnArg, syn::Token![,]> = Punctuated::new();
    let handler_id = aep.handler_id();

    aep.ep.path_inputs.as_ref().inspect(|&inputs| {
        let ty = &inputs.ty;
        let id = inputs
            .id
            .clone()
            .unwrap_or_else(|| syn::Ident::new("path_inputs", aep.handler_id().span()));

        fn_args.push(parse_quote_spanned! {
            handler_id.span()=>
            #id: ::actix_web::web::Path<#ty>
        });
    });

    aep.ep.query_inputs.as_ref().inspect(|&inputs| {
        let ty = &inputs.ty;
        let id = inputs
            .id
            .clone()
            .unwrap_or_else(|| syn::Ident::new("query_inputs", aep.handler_id().span()));

        fn_args.push(parse_quote_spanned! {
            handler_id.span()=>
            #id: ::actix_web::web::Query<#ty>
        });
    });

    fn_args.push(parse_quote_spanned! {
        handler_id.span()=>
        extensions: Self::#ext_type_name
    });

    aep.ep.body_param.as_ref().inspect(|&param| {
        let name = &param.name;
        let ty = get_body_param_ty(param);

        fn_args.push(parse_quote! {
            #name: #ty
        });
    });

    fn_args.pop_punct();

    let ret_ty = {
        let ty = aep.ep.ret.clone();

        match aep.ep.meta.options().content_type.clone().unwrap_or_default() {
            ContentType::ApplicationJson => parse_quote_spanned! {
                handler_id.span()=>
                ::actix_web::web::Json<#ty>
            },
            _ => ty,
        }
    };

    let item_fn: syn::TraitItemFn = parse_quote_spanned! {
        handler_id.span()=>
        fn #handler_id(#fn_args) -> impl ::std::future::Future<Output = #ret_ty>;
    };

    item_fn
}

fn get_body_param_ty(param: &Param) -> syn::Type {
    let ty = &param.ty;

    match param.meta.transport() {
        Transport::Json => parse_quote_spanned! {
            param.name.span()=>
            ::actix_web::web::Json<#ty>
        },
        Transport::Multipart => parse_quote_spanned! {
            param.name.span()=>
            ::actix_multipart::form::MultipartForm<#ty>
        },
        _ => unreachable!(),
    }
}
