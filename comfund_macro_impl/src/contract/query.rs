use proc_macro2::TokenStream;

pub enum Query {
    Flat(syn::Type),
    Generated(Generated),
}

pub struct Generated {
    definition: TokenStream,
    fields: Vec<syn::Field>,
}
