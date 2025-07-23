use crate::{
    contract::{endpoint::Endpoint, Contract},
    servers::{names::Names, server_endpoint},
};
use quote::quote;
use syn::{parse_quote, parse_quote_spanned};

pub fn def(contract: &Contract) -> syn::ItemTrait {
    let contract_id = &contract.id;

    let ep_trait_items = contract.endpoints.iter().map(def_trait_items);

    parse_quote! {
        pub trait #contract_id: 'static {
            #(#ep_trait_items)*
        }
    }
}

fn def_trait_items(ep: &Endpoint) -> impl quote::ToTokens {
    let names = Names::new(ep);

    let ext_type = def_ext_type(&names);
    let handler = def_handler(ep, &names);
    let middleware = def_middleware(&names);

    quote! {
        #ext_type
        #handler
        #middleware
    }
}

fn def_ext_type(names: &Names) -> impl quote::ToTokens {
    let bounds = parse_quote!(::actix_web::FromRequest);
    server_endpoint::def_ext_type(names.ext_type_id(), bounds)
}

fn def_handler(ep: &Endpoint, names: &Names) -> syn::TraitItemFn {
    let args = server_endpoint::handler_sig_args(ep, names);
    let handler_id = names.handler_id();
    let ret_ty = ep.ret.clone();

    parse_quote_spanned! {
        handler_id.span()=>
        fn #handler_id(#args) -> impl ::std::future::Future<Output = #ret_ty>;
    }
}

fn def_middleware(names: &Names) -> syn::TraitItemFn {
    let id = names.decorator_id();

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
