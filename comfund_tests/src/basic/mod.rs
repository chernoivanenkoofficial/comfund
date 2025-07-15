mod definition;
mod implelentation;
mod model;

use definition::*;
use implelentation::*;
use model::*;

static HOST: &str = "http://127.0.0.1:10000";
static BIND_TARGET: &str = "127.0.0.1:10000";
static CLIENT: ServiceClient = ServiceClient::new_const(&HOST);
static SERVER_LOCK: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn launch_server() -> std::io::Result<()> {
    use tokio::net::TcpListener;

    SERVER_LOCK
        .get_or_init(move || async {
            let listener = TcpListener::bind(BIND_TARGET).await.unwrap();
            let router = route_service::<ServiceImpl>(());

            tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap();
            });
        })
        .await;

    Ok(())
}

#[tokio::test]
async fn hello_world() {
    launch_server().await.unwrap();
    assert_eq!(CLIENT.hello_world().await.unwrap(), "Hello world!");
}

#[tokio::test]
async fn add_two() {
    launch_server().await.unwrap();
    CLIENT.add_two(10, 20).await.unwrap();
}

#[tokio::test]
async fn add_three() {
    launch_server().await.unwrap();
    CLIENT.add_three(0, 1, 2).await.unwrap();
}