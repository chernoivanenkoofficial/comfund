use proc_macro2::TokenStream;

use crate::contract::Contract;

mod reqwest;

pub fn implement(contract: &Contract) -> TokenStream {
    let mut stream = TokenStream::new();

    stream.extend(reqwest::implement(contract));

    stream
}
