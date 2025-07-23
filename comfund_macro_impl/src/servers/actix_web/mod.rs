mod service_trait;
mod wrapper_mod;
mod configure_fn;

use quote::{format_ident, quote, quote_spanned};
use syn::{parse_quote, parse_quote_spanned};

use crate::contract::endpoint::Endpoint;
use crate::contract::param::Param;
use crate::contract::Contract;
use crate::servers::names::Names;
use crate::servers::server_endpoint;
use crate::servers::wrap_fn::WrapperFn;

pub fn implement(contract: &Contract) -> proc_macro2::TokenStream {
    let service_trait = service_trait::def(contract);
    let wrapper_mod = wrapper_mod::def(contract);
    let configure_fn = configure_fn::def(contract);
    let attrs = contract.attrs.iter();

    quote_spanned! {
        contract.id.span()=>
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

            #(#attrs)*
            #service_trait

            #wrapper_mod

            #configure_fn
        }
    }
}