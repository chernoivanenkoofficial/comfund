mod definition;
mod implelentation;
mod model;

use implelentation::*;
use model::*;

use crate::axum_initializators;

axum_initializators!(
    "127.0.0.1:10000",
    CLIENT = definition::reqwest::ServiceClient,
    launch_server = definition::axum::route_service::<ServiceImpl>[()]
);

#[tokio::test]
async fn hello_world() {
    launch_server().await.unwrap();
    assert_eq!(CLIENT.hello_world().await.unwrap(), "Hello world!");
}

#[tokio::test]
async fn add_two() {
    launch_server().await.unwrap();

    assert_eq!(CLIENT.add_two(10, 20).await.unwrap(), 30);
}

#[tokio::test]
async fn add_three() {
    launch_server().await.unwrap();
    CLIENT.add_three(0, 1, 2).await.unwrap();
}