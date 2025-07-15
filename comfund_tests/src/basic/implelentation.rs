use super::definition::*;
use super::model::*;

pub struct ServiceImpl;

impl Service for ServiceImpl {
    type State = ();  

    type HelloWorldExtensions = ();
    async fn hello_world(_extensions: Self::HelloWorldExtensions) -> String {
        "Hello world!".to_owned()
    }

    type AddTwoExtensions = ();
    async fn add_two(
        _path_inputs: axum::extract::Path<AddTwoPathInputs>,
        _extensions: Self::AddTwoExtensions,
    ) -> axum::Json<()> {
        axum::Json(())
    }

    type AddThreeExtensions = ();
    async fn add_three(
        _path_inputs: axum::extract::Path<AddThreePathInputs>,
        _extensions: Self::AddThreeExtensions,
    ) -> axum::Json<()> {
        axum::Json(())
    }
}