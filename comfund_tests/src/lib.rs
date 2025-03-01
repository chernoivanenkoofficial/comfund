pub mod service;

mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;
    use rstest::*;

    #[fixture]
    fn base_valid_trait() -> TokenStream {
        quote! {
            trait Service {
                #[endpoint(get, "/{a}", content_type = "application/json")]
                fn add_two(#[param(path)] a: u32, #[param(query)] b: u32) -> ();

                #[endpoint(post, "/{new}", content_type = "text/plain")]
                fn post_new(#[param(path, flatten)]new: String) -> String;
            }
        }
    }

    #[rstest]
    fn just_print(base_valid_trait: TokenStream) {
        let output = comfund_macro_impl::contract(quote! {}, base_valid_trait);
        println!("{}", output.to_string());
    }
}
