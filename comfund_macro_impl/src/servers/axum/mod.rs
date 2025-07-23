mod service_trait;
mod wrapper_mod;
mod route_fn;

use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{parse_quote, parse_quote_spanned};

use crate::contract::content_type::ContentType;
use crate::contract::endpoint::Endpoint;
use crate::contract::method::Method;
use crate::contract::param::Param;
use crate::contract::transport::Transport;
use crate::contract::Contract;
use crate::servers::names::Names;
use crate::servers::server_endpoint;
use crate::servers::wrap_fn::WrapperFn;

pub fn implement(contract: &Contract) -> proc_macro2::TokenStream {
    let service_trait = service_trait::def(contract);
    let wrapper_mod = wrapper_mod::def(contract);
    let route_fn = route_fn::def(contract);
    let attrs = contract.attrs.iter();

    quote! {
        #[cfg(all(feature = "axum", not(any(feature = "actix-web"))))]
        pub use axum::*;

        #[cfg(feature = "axum")]
        pub mod axum {
            use super::*;

            #(#attrs)*
            #service_trait

            #wrapper_mod

            #route_fn
        }
    }
}

