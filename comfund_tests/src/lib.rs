pub mod basic;

macro_rules! axum_initializators {
    ($target:literal, $client_id:ident = $client_ty:path, $server_fn:ident = $reg_fn:path[$state:expr]) => {
        #[allow(dead_code)]
        static $client_id: $client_ty = <$client_ty>::new_const(concat!("http://", $target));

        #[allow(dead_code)]
        async fn $server_fn() {
            static SERVER_LOCK: ::tokio::sync::OnceCell<()> = ::tokio::sync::OnceCell::const_new();

            SERVER_LOCK
                .get_or_init(move || async {
                    let listener = ::tokio::net::TcpListener::bind($target).await.unwrap();
                    let router = $reg_fn($state);

                    ::tokio::spawn(async move {
                        axum::serve(listener, router).await.unwrap();
                    });
                })
                .await;
        }
    };
}

pub(crate) use axum_initializators;

macro_rules! actix_initializators {
    ($target:literal, $client_id:ident = $client_ty:path, $server_fn:ident = $configure_fn:path[$state:expr]) => {
        #[allow(dead_code)]
        static $client_id: $client_ty = <$client_ty>::new_const(concat!("http://", $target));

        #[allow(dead_code)]
        async fn $server_fn() {
            static SERVER_LOCK: ::tokio::sync::OnceCell<()> = ::tokio::sync::OnceCell::const_new();

            SERVER_LOCK
                .get_or_init(move || async {
                    let factory = || ::actix_web::App::new().configure($configure_fn);

                    ::tokio::spawn(async move {
                        ::actix_web::HttpServer::new(factory)
                            .bind($target)
                            .unwrap()
                            .run()
                            .await
                    });
                })
                .await;
        }
    };
}

pub(crate) use actix_initializators;
