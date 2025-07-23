use proc_macro2::TokenStream;

use crate::contract::Contract;

mod actix_web;
mod axum;

mod server_endpoint;
mod wrap_fn;
mod names;

use names::Names;
use wrap_fn::WrapperFn;

pub fn implement(contract: &Contract) -> TokenStream {
    let mut stream = TokenStream::new();

    stream.extend(axum::implement(contract));
    stream.extend(actix_web::implement(contract));

    stream
}
