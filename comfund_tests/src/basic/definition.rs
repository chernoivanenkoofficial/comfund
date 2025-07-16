/// A simple demonstration of basic features of `comfund`.
#[comfund::contract]
pub trait Service {
    /// Hello world! version of axum contract.
    #[endpoint(get, "/")]
    fn hello_world() -> String;

    /// Slightly more complex example of axum endpoint.
    #[endpoint(get, "/{a}/{b}", content_type = "application/json")]
    fn add_two(#[param(path)] a: u32, #[param(path)] b: u32) -> u32;

    /// Slightly more complex example of axum endpoint.
    #[endpoint(get, "/{a}/{b}/{c}", content_type = "application/json")]
    fn add_three(#[param(path)] a: u32, #[param(path)] b: u32, #[param(path)] c: u32);
}