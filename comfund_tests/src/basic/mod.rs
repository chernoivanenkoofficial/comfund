pub mod definition;
pub mod axum_implelentation;
pub mod actix_implementation;
pub mod model;

use model::*;

use crate::{actix_initializators, axum_initializators};

axum_initializators!(
    "127.0.0.1:10000",
    AXUM_CLIENT = definition::ServiceClient,
    launch_axum_server = definition::axum::route_service::<axum_implelentation::ServiceImpl>[()]
);

actix_initializators!(
    "127.0.0.1:11000",
    ACTIX_CLIENT = definition::ServiceClient,
    launch_actix_server = definition::actix_web::configure_service::<actix_implementation::ServiceImpl>[()]
);

#[tokio::test]
async fn hello_world() {
    launch_axum_server().await;
    launch_actix_server().await;

    assert_eq!(AXUM_CLIENT.hello_world().await.unwrap(), "Hello world!");
    assert_eq!(ACTIX_CLIENT.hello_world().await.unwrap(), "Hello world!");
}

#[tokio::test]
async fn add_two() {
    launch_axum_server().await;
    launch_actix_server().await;

    assert_eq!(AXUM_CLIENT.add_two(10, 20).await.unwrap(), 30);
    assert_eq!(ACTIX_CLIENT.add_two(10, 20).await.unwrap(), 30);
}

#[tokio::test]
async fn add_three() {
    launch_axum_server().await;
    launch_actix_server().await;

    AXUM_CLIENT.add_three(0, 1, 2).await.unwrap();
    ACTIX_CLIENT.add_three(0, 1, 1).await.unwrap();
}