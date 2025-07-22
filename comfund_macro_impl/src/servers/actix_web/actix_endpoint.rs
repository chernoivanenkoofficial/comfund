use quote::{format_ident, quote};
use syn::{parse_quote, parse_quote_spanned};

use crate::contract::content_type::ContentType;
use crate::contract::endpoint::Endpoint;
use crate::contract::inputs::Inputs;
use crate::contract::method::Method;
use crate::contract::param::Param;
use crate::contract::transport::Transport;

pub struct ActixEndpoint<'e> {
    ep: &'e Endpoint,
    handler_id: syn::Ident,
    decorator_id: syn::Ident,
    ext_type_name: syn::Ident,
}

impl<'e> ActixEndpoint<'e> {
    pub fn new(ep: &'e Endpoint) -> Self {
        let handler_id = ep.id.clone();

        let ext_type_name = {
            let mut handler_str = handler_id.to_string();
            handler_str.push_str("_extensions");

            let extensions_str = stringcase::pascal_case(&handler_str);

            syn::Ident::new(&extensions_str, handler_id.span())
        };

        let decorator_id = format_ident!("set_{}_middleware", &handler_id);

        Self {
            ep,
            ext_type_name,
            handler_id,
            decorator_id,
        }
    }
}

impl ActixEndpoint<'_> {
    fn path(&self) -> &syn::LitStr {
        self.ep.meta.path_lit()
    }

    fn handler_id(&self) -> &syn::Ident {
        &self.handler_id
    }

    fn ext_type_name(&self) -> &syn::Ident {
        &self.ext_type_name
    }

    fn decorator_id(&self) -> &syn::Ident {
        &self.decorator_id
    }

    pub fn def_in_trait(&self) -> impl quote::ToTokens {
        let ext_type_def = def_ext_type(self.ext_type_name());
        let handler_def = def_handler(self, self.ext_type_name());
        let middleware_def = def_middleware(self);

        quote! {
            #ext_type_def
            #handler_def
            #middleware_def
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

        let decorator_id = self.decorator_id();

        quote! {
            ::actix_web::web::#method().to(
                ___wrappers::#handler_id::<#service_trait_var>).wrap(#service_trait_var::#decorator_id())
        }
    }

    pub fn impl_wrap_function(&self, contract_id: &syn::Ident) -> syn::ItemFn {
        impl_wrap_function(self, self.ext_type_name(), contract_id)
    }
}

fn def_ext_type(ext_type_name: &syn::Ident) -> impl quote::ToTokens {
    let item_type: syn::TraitItemType = parse_quote_spanned!(
        ext_type_name.span()=>

        type #ext_type_name: ::actix_web::FromRequest;
    );

    item_type
}

fn impl_wrap_function(
    aep: &ActixEndpoint,
    ext_type_name: &syn::Ident,
    contract_id: &syn::Ident,
) -> syn::ItemFn {
    use syn::punctuated::Punctuated;

    let id = aep.handler_id();

    fn destructor(inputs: Option<&Inputs>, default_name: &str) -> Option<impl quote::ToTokens> {
        inputs.and_then(|inputs| {
            let ident = inputs.id().cloned().unwrap_or(syn::Ident::new(
                default_name,
                proc_macro2::Span::call_site(),
            ));

            inputs.destructor(quote!(#ident.into_inner()))
        })
    }

    let path_inputs = aep.ep.path_inputs();
    let query_inputs = aep.ep.query_inputs();

    let args = get_wrap_fn_args(aep, ext_type_name, id.span());

    let ret = get_ret_type(aep);

    let path_destructor = destructor(path_inputs, Inputs::DEFAULT_PATH_NAME);
    let query_destructor = destructor(query_inputs, Inputs::DEFAULT_QUERY_NAME);

    let (path_names, query_names, body_name) = aep.ep.param_names();

    let mut forwarded = Punctuated::<syn::Ident, syn::Token![,]>::new();
    forwarded.extend(path_names.cloned());
    forwarded.extend(query_names.cloned());
    forwarded.push(format_ident!("extensions"));
    forwarded.extend(body_name.cloned());
    forwarded.pop_punct();

    let result_id = format_ident!("___result", span = aep.ep.id.span());

    let result_mapping: syn::Expr = match aep.ep.content_type() {
        ContentType::ApplicationJson => {
            parse_quote!(::comfund::actix_web::reexport::web::Json(#result_id))
        }
        ContentType::TextPlain => parse_quote!(#result_id),
    };

    parse_quote! {
        pub async fn #id<C: #contract_id>(#args) -> #ret {
            #path_destructor
            #query_destructor

            let #result_id = C::#id(#forwarded).await;

            #result_mapping
        }
    }
}

fn get_wrap_fn_args(
    aep: &ActixEndpoint,
    ext_type_name: &syn::Ident,
    span: proc_macro2::Span,
) -> syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]> {
    let path_inputs = aep.ep.path_inputs();
    let query_inputs = aep.ep.query_inputs();

    let mut args = syn::punctuated::Punctuated::new();

    path_inputs.inspect(|&inputs| {
        let arg = inputs.as_handler_arg(
            &parse_quote!(::comfund::actix_web::reexport::web::Path),
            || syn::Ident::new(Inputs::DEFAULT_PATH_NAME, span),
        );
        args.push(arg);
    });

    query_inputs.inspect(|&inputs| {
        let arg = inputs.as_handler_arg(
            &parse_quote!(::comfund::actix_web::reexport::web::Query),
            || syn::Ident::new(Inputs::DEFAULT_QUERY_NAME, span),
        );
        args.push(arg);
    });

    args.push(parse_quote_spanned! {
        aep.ep.id.span()=>
        extensions: C::#ext_type_name
    });

    aep.ep.body_param().inspect(|&param| {
        let name = &param.id;
        let ty = get_body_param_ty(param);

        args.push(parse_quote!(#name: #ty));
    });

    args.pop_punct();

    args
}

fn def_handler(aep: &ActixEndpoint, ext_type_name: &syn::Ident) -> impl quote::ToTokens {
    use syn::punctuated::Punctuated;

    let (path_params, query_params, body_param) = aep.ep.param_args();
    
    let mut fn_args: Punctuated<syn::FnArg, syn::Token![,]> = Punctuated::new();
    fn_args.extend(path_params);
    fn_args.extend(query_params);
    fn_args.push(parse_quote_spanned! {
        aep.ep.id.span()=>
        extensions: Self::#ext_type_name
    });
    fn_args.extend(body_param);
    fn_args.pop_punct();
    
    let handler_id = aep.handler_id();

    let ret_ty = aep.ep.ret.clone();

    let item_fn: syn::TraitItemFn = parse_quote_spanned! {
        handler_id.span()=>
        fn #handler_id(#fn_args) -> impl ::std::future::Future<Output = #ret_ty>;
    };

    item_fn
}

fn def_middleware(aep: &ActixEndpoint) -> syn::TraitItemFn {
    let id = aep.decorator_id();

    parse_quote_spanned! {
        id.span()=>
        fn #id() -> impl ::comfund::actix_web::reexport::dev::Transform<
        ::comfund::actix_web::reexport::actix_service::boxed::BoxService<
            ::comfund::actix_web::reexport::dev::ServiceRequest,
            ::comfund::actix_web::reexport::dev::ServiceResponse,
            ::comfund::actix_web::reexport::error::Error,
        >,
        ::comfund::actix_web::reexport::dev::ServiceRequest,
        Response = ::comfund::actix_web::reexport::dev::ServiceResponse<
            impl ::comfund::actix_web::reexport::actix_http::body::MessageBody + 'static,
        >,
        Error = ::comfund::actix_web::reexport::error::Error,
        InitError = (),
    > + 'static {
        ::comfund::actix_web::reexport::middleware::Identity::default()
    }
    }
}

fn get_body_param_ty(param: &Param) -> syn::Type {
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

fn get_ret_type(aep: &ActixEndpoint) -> syn::Type {
    let ret_ty = aep.ep.ret.clone();

    match aep.ep.content_type() {
        // TODO: Response types mapping when defined common supported returned content types
        ContentType::ApplicationJson => parse_quote!(::comfund::actix_web::reexport::web::Json<#ret_ty>),
        _ => ret_ty,
    }
}
