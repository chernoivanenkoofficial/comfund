#[cfg(test)]
mod basic;

macro_rules! axum_initializators {
    ($target:literal, $client_id:ident = $client_ty:path, $server_fn:ident = $reg_fn:path[$state:expr]) => {
        static CLIENT: $client_ty = <$client_ty>::new_const(concat!("http://", $target));

        async fn $server_fn() -> ::std::io::Result<()> {
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

            Ok(())
        }
    };
}

pub(crate) use axum_initializators;