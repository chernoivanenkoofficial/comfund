#[comfund::contract]
pub trait Service {
    #[endpoint(get, "/{a}/{b}", content_type = "application/json")]
    fn add_two(#[param(path)] a: u32, #[param(path)] b: u32) -> ();

    #[endpoint(get, "/{a}/{b}/{c}", content_type = "application/json")]
    fn add_three(#[param(path)] a: u32, #[param(path)] b: u32, #[param(path)] c: u32) -> ();
}
