#[comfund::contract]
trait Service {
    #[endpoint(get, "/")]
    fn hello_world() -> String;

    #[endpoint(get, "/{a}/{b}", content_type = "application/json")]
    fn add_two(#[param(path)] a: u32, #[param(path)] b: u32);

    #[endpoint(get, "/{a}/{b}/{c}", content_type = "application/json")]
    fn add_three(#[param(path)] a: u32, #[param(path)] b: u32, #[param(path)] c: u32);
}

struct ServiceImpl;

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