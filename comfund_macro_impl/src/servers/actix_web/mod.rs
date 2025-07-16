use quote::quote;

use crate::contract::Contract;

pub fn implement(contract: &Contract) -> impl quote::ToTokens {
    let service_trait_def = def_service_trait(contract);
    let configure_fn_impl = impl_configure_fn(contract);

    quote! {
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
        mod actix_web {
            use super::*;
            #service_trait_def
            #configure_fn_impl
        }
    }
}

fn def_service_trait(contract: &Contract) -> impl quote::ToTokens {
    quote! {}
}

fn impl_configure_fn(contract: &Contract) -> impl quote::ToTokens {
    quote! {}
}