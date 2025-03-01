#![allow(dead_code)]
#![allow(unused_imports)]

mod clients;
mod contract;
mod extensions;
mod servers;
mod utils;

use crate::contract::Contract;

use proc_macro2::token_stream::TokenStream;

#[derive(Debug)]
struct ContractAttribute {}

pub fn contract(args: TokenStream, input: TokenStream) -> TokenStream {
    let item_trait = match syn::parse2::<syn::ItemTrait>(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let contract = match Contract::parse(args, item_trait) {
        Ok(contract) => contract,
        Err(err) => return err.into_compile_error(),
    };

    let mut stream = TokenStream::new();

    stream.extend(contract::implement(&contract));
    stream.extend(clients::implement(&contract));
    stream.extend(servers::implement(&contract));

    stream
}
