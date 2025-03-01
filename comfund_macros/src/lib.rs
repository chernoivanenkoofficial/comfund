//! # `comfund`: WCF-like Service Contracts in Rust
//!
//! `proc-macro` crate for comfund `contract` attribute.
//!
//! *Contract* is a rust trait representing possible requests to a service,
//! parameters of each request and its return type.
//!
//! For client side, either
//! a stateful client or static implementation will be generated.  
//!
//! For server side, a service trait will be generated. Implementation of this trait
//! can then be passed to a generated configure function to create configuration/router with
//! corresponding handlers and middleware mounted on specified paths.
//!
//! ## Additional attributes and options
//!
//! Every `fn` of contract trait should be annotated with `#[endpoint]` attribute with two
//! required arguments:
//!
//! - Method [get, post, put, delete]
//! - Endpoint path ("/"-prefixed string literal)
//!
//! ```
//! use comfund::contract;
//!
//! #[contract]
//! pub trait Service {
//!     #[endpoint(get, "/")]
//!     fn endpoint();
//! }
//! ```
//!
//! Endpoints can accept parameters. Each parameter should be annotated with `#[param]`
//! attribute with one required arg - type of transport:
//! - through endpoint URL path (`path`),
//! - URL query param (`query`)
//! - Request body (`plain text` or `json`)
//!
//! ```
//! use comfund::contract;
//!
//! #[contract]
//! pub trait Service {
//!     #[endpoint(get, "/path/{a}")]
//!     fn path(#[param(path)] a: String);
//!     
//!     #[endpoint(get, "/query")]
//!     fn query(#[param(query)] a: String);
//!     
//!     #[endpoint(post, "/body")]
//!     fn body(#[param(body)]) a: String);
//!     
//!     #[endpoint(post, "/body/json")]
//!     fn json(#[param(json)]) a: Vec<String>);
//! }
//! ```
//!
//! Endpoints can also have return types. If you want to be able to return/read error info as well,
//! you can set [`Result`] as return type.
//!
//! ```
//! use comfund::contract;
//!
//! #[contract]
//! pub trait Service {
//!     #[endpoint(get, "/")]
//!     fn infallible() -> String;
//!
//!     #[endpoint(get, "/may_fail")]
//!     fn may_fail() -> Result<String, Error>;
//! }
//! ```
//!
//! Endpoints can also specify `content-type` for returned value. Generated server and client code will
//! handle the conversion accordingly.
//!
//! ```
//! use comfund::contract;
//! use comfund::serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! #[serde(crate = comfund::serde)]
//! struct Return {
//!     status: u16,
//!     string: String
//! }
//!
//! #[contract]
//! pub trait Service {
//!     #[endpoint(get, "/may_fail", content_type = "application/json")]
//!     fn may_fail() -> Return;
//! }
//! ```

use proc_macro::TokenStream;

/// # `contract` attribute macro
///
/// `contract` should be applied to a `trait`. Сurrently, only `fn` items are supported for parsing
/// and presense of other types of items (like consts and associated types) will trigger compile errors,
/// as well as presence of generic parameters (though it's planned to support
/// trait level type generics in the future).
#[proc_macro_attribute]
pub fn contract(args: TokenStream, input: TokenStream) -> TokenStream {
    comfund_macro_impl::contract(args.into(), input.into()).into()
}
