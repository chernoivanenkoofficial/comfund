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
    async fn add_two(a: u32, b: u32, _extensions: Self::AddTwoExtensions) -> u32 {
        a + b
    }

    type AddThreeExtensions = ();
    async fn add_three(a: u32, b: u32, c: u32, _extensions: Self::AddThreeExtensions) -> u32 {
        a + b + c
    }

    type MessageExtensions = ();
    async fn message(
        message: <str as ::std::borrow::ToOwned>::Owned,
        _extensions: Self::MessageExtensions,
    ) -> String {
        message
    }

    type ConcatExtensions = ();
    async fn concat(
        mut s1: <str as ::std::borrow::ToOwned>::Owned,
        s2: <str as ::std::borrow::ToOwned>::Owned,
        _extensions: Self::ConcatExtensions,
    ) -> String {
        s1.push_str(&s2);
        s1
    }
}
