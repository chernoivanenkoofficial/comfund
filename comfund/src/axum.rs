pub trait Layer<H, T, S>
where
    Self: ::tower_layer::Layer<
            ::axum::handler::HandlerService<H, T, S>,
            Service: ::tower_service::Service<
                axum::extract::Request,
                Error = ::std::convert::Infallible,
                Response: ::axum::response::IntoResponse,
                Future: ::std::marker::Send,
            > + Clone
                         + Send
                         + 'static,
        > + Clone
        + Send
        + Sync
        + 'static,
    H: ::axum::handler::Handler<T, S>,
    T: 'static,
    S: 'static,
{
}

impl<L, H, T, S> Layer<H, T, S> for L
where
    L: ::tower_layer::Layer<
            ::axum::handler::HandlerService<H, T, S>,
            Service: ::tower_service::Service<
                axum::extract::Request,
                Error = ::std::convert::Infallible,
                Response: ::axum::response::IntoResponse,
                Future: ::std::marker::Send,
            > + Clone
                         + Send
                         + 'static,
        > + Clone
        + Send
        + Sync
        + 'static,
    H: ::axum::handler::Handler<T, S>,
    T: 'static,
    S: 'static,
{
}

pub mod reexport {
    pub use axum::*;

    pub use tower_layer;
    pub use tower_service;
}
