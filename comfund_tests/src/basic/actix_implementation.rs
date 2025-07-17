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
    async fn add_two(
        path_inputs: web::Path<definition::AddTwoPathInputs>,
        _extensions: Self::AddTwoExtensions,
    ) -> web::Json<u32> {
        web::Json(path_inputs.a + path_inputs.b)
    }

    type AddThreeExtensions = ();
    async fn add_three(
        _path_inputs: web::Path<definition::AddThreePathInputs>,
        _extensions: Self::AddThreeExtensions,
    ) -> web::Json<()> {
        web::Json(())
    }
}
