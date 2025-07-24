/// A simple demonstration of basic features of `comfund`.
#[comfund::contract(content_type = "application/json")]
pub trait Service {
    /// Hello world! version of axum contract.
    #[endpoint(get, "/", content_type = "text/plain")]
    fn hello_world() -> String;

    /// Slightly more complex example of axum endpoint.
    #[endpoint(get, "/{a}/{b}")]
    fn add_two(#[param(path)] a: u32, #[param(path)] b: u32) -> u32;

    /// Slightly more complex example of axum endpoint.
    #[endpoint(get, "/{a}/{b}/{c}")]
    fn add_three(#[param(path)] a: u32, #[param(path)] b: u32, #[param(path)] c: u32) -> u32;

    #[endpoint(get, "/message", content_type = "text/plain")]
    fn message(#[param(query)] message: &str) -> String;

    #[endpoint(get, "/concat", content_type = "text/plain")]
    fn concat(#[param(query)] s1: &str, #[param(query)] s2: &str) -> String;
}
