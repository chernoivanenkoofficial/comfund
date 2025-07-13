//! # `comfund`: WCF-like Service Contracts in Rust
//!
//! Ever stumbled upon the routine of setting up/modyfying endpoints for your REST Api
//! for both Rust client and Rust server code? Then `comfund` is what you need.
//!
//! Define your service contracts in one place and use auto generated clients
//! and server services accordingly.
//!
//! ## How does it work
//!
//! The cornerstone of `comfund` is a [`#[contract]`](contract) attribute proc macro,
//! that generates feature-gated client and server code, that will be depent on by consuming front- and back-end.
//!
//! As both client and server code are generated from the same place,
//! synchronization of endporint URLs, methods, parameters, etc. is guaranteed.
//! And only one place in code should be modified manually, if needed.

pub use comfund_macros::contract;

#[cfg(any(feature = "reqwest"))]
pub use paths;

#[cfg(feature = "reqwest")]
pub use reqwest_exports::*;

#[cfg(feature = "reqwest")]
mod reqwest_exports {
    #[derive(Debug)]
    pub enum ClientError {
        PathSerializerError(paths::path_serializer::Error),
        Reqwest(reqwest::Error),
    }

    impl From<reqwest::Error> for ClientError {
        fn from(value: reqwest::Error) -> Self {
            Self::Reqwest(value)
        }
    }

    impl From<::paths::path_serializer::Error> for ClientError {
        fn from(value: ::paths::path_serializer::Error) -> Self {
            Self::PathSerializerError(value)
        }
    }
}

#[cfg(feature = "reqwest")]
pub type Result<T> = std::result::Result<T, ClientError>;
