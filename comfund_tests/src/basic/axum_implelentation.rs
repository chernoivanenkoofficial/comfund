use super::model::*;

use super::definition::*;

pub struct ServiceImpl;

impl axum::Service for ServiceImpl {
    type State = ();

    type HelloWorldExtensions = ();
    async fn hello_world(_extensions: Self::HelloWorldExtensions) -> String {
        "Hello world!".to_owned()
    }

    type AddTwoExtensions = ();
    async fn add_two(
        path_inputs: ::axum::extract::Path<AddTwoPathInputs>,
        _extensions: Self::AddTwoExtensions,
    ) -> ::axum::Json<u32> {
        ::axum::Json(path_inputs.a + path_inputs.b)
    }

    type AddThreeExtensions = ();
    async fn add_three(
        _path_inputs: ::axum::extract::Path<AddThreePathInputs>,
        _extensions: Self::AddThreeExtensions,
    ) -> ::axum::Json<()> {
        ::axum::Json(())
    }
}
