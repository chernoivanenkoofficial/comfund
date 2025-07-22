use actix_web::web;

use super::model::*;

use super::definition;

pub struct ServiceImpl;

impl definition::actix_web::Service for ServiceImpl {
    type HelloWorldExtensions = ();
    async fn hello_world(_extensions: Self::HelloWorldExtensions) -> String {
        "Hello world!".to_owned()
    }

    type AddTwoExtensions = ();
    async fn add_two(a: u32, b: u32, extensions: Self::AddTwoExtensions) -> u32 {
        a + b
    }

    type AddThreeExtensions = ();
    async fn add_three(a: u32, b: u32, c: u32, extensions: Self::AddThreeExtensions) -> u32 {
        a + b + c
    }
}
