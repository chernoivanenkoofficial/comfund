# `comfund`: WCF-like Service Contracts in Rust

Ever stumbled upon the routine of setting up/modyfying endpoints for your REST Api for both Rust client and Rust server code? Then `comfund` is what you need.

Define your service contracts in one place and use auto generated clients and server services accordingly.

## Motivation

The aim of this crate is to provide solution for generating synchronized client and server code for full-stack Rust applications, without the need to update/define your endpoints in separate places, with clear self-documenting endpoint definitions.

Just define one crate with all your REST api endpoints and models, and then consume either auto-generated HTTP client on the client side or implement the service trait on server side and register your implementation with your web framework of chosing with automatically generated configuration function.

This crate is inspired by an idea of WCF Service Contract and its clear definition of services and [`server_fn`](https://crates.io/crates/server_fn) with its server functions, that allow for writing functions in fullstack Rust applications, that automatically resolve to either server-side logic or client-side HTTP request. 

## Alternatives

Currently, if you want to automate synchronization of your REST Api definition and consuming code in Rust you have only two options. 

### 1. Expose OpenApi definition with [`utoipa`](https://github.com/juhaku/utoipa) and use any of available generators for client code.

This one is good, if your aim is to support a lot of different third-party consumers, but if you aim to support Rust-only consumers primarily, this is a very roundabout way of generating Rust client code.

Any present OpenApi client generators are either limited in their capabilities to fully capture rust specifics or prone to generating faulty implementations, that need to be corrected by-hand. 

Setting up automatic build routines for generated client is cumbersome as well and requires quite a lot of server-side shenanigans to enable obtaining OpenApi spec for generating in build.rs scripts.

### 2. Use server functions and [`server_fn`](https://crates.io/crates/server_fn) crate.

This is the best solution for full-Rust fullstack applications. But the main goal of `server_fn` is to allow defining server-side logic alogside client views, that would be using this logic, and thus poses several restrictions that `confund` aims to resolve:

1. **Client and server code becomes tightly coupled.** If any nesseccity to separate client and server code arises, `server_fn` solution is no longer suitable or requires a lot of workarounds to be separated from client code without client code being dependent, either directly or transitionally, on server code.

2. **Limited control over endpoint semantics.** Even though `server_fn` allows for a lot of different settings for each generated endpoints, those are still quite limited, as `server_fn` aims for simplicity in use in full-Rust applications more than for flexibility. All of endpoints are registered by `server_fn` as well, and that takes away control from the consumer.

On the other hand, if you aim to stick only to Rust client side in your app and dont care much about clear REST Api declarations to be used by consumers in another languages, `server_fn` is better for you than `comfund` is.

## How does it work

The cornerstone of `comfund` is a `#[contract]` proc macro, that generates [feature-gated](#feature-gated-implementations) client and server code, that will be depent on by consuming front- and back-end.

As both client and server code are generated from the same place, synchronization of endporint URLs, methods, parameters, etc. is guaranteed. And only one place in code should be modified manually, if needed.

The general workflow is as follows:

1. Define service contract and API data structures in a common crate.
2. Expose feature flags for supported client-side and service-side implementations.
3. Consume either in client code or in a server code, enabling the corresponding implementation.

## Usage and features

`comfund` uses *feature flags* to optionally enable either of backends or frontends. 

This means that besides specifying dependency for `comfund` in the crate (with corresponding features) using `comfund`, you need to expose all of the features, that were generated, as well. 

```toml
reqwest = ["comfund/reqwest"]
axum = ["comfund/axum"]
actix-web = ["comfund/actix-web"]
static = []
```

Also, comfund reexports any backend or frontend enabled for better version synchronization, both in the `confund` itself and in the generated code, so its advised to access any of the said through the api defining crate.

```rust
// High risk of version conflicts/errors
use reqwest::*;
// Better import like this
use api_crate::reqwest::*;
```

This means, that for some derive macros to work (like `serde` macros) you need to specify a new location of crate (e.g. `#[serde(crate = comfund::serde)]`)

To allow for multiple API's to be declared in the single crate, `comfund::reexport!()` convinience macro is provided instead of generating these reexports through `#[contract]` attribute macro.

## Contracts

Contract is basically an annotated Rust `trait` with functions defined for each endpoint (in the basic case).

```rust
#[contract]
pub trait CounterService {
    #[endpoint(get, "/current")]
    async fn get_current() -> Result<u64>;
    #[endpoint(post, "/inc")]
    async fn increment() -> Result<()>;
}
```

Each function corresponds to **the unique view of endpoint** (see [equivalence of endpoints](#equivalence-of-endpoints)).

For the client-side this trait will be either substituted with stateful `[trait_name]Client` implementation:

```rust
pub struct CounterServiceClient {
    /// Implementation
}

impl CounterServiceClient {
    pub fn new(root: impl IntoUrl) -> Self {
        // ...
    }

    pub fn get_current(&self) -> Result<u64> {
        // ...
    }

    pub fn increment(&self) -> Result<()> {
        // ...
    }
}
```

or a combination of `set_[trait_name]_root(root: impl IntoUrl)` and a set of static functions (with `static` feature enabled):

```rust
static COUNTER_SERVICE_ROOT: OnceLock<Url> = OnceLock::new();

pub fn set_counter_service_root(root: impl IntoUrl) {
    // ...
}

pub fn get_current() -> Result<u64> {
    // ...
}

pub fn increment() -> Result<()> {
    // ...
}
```

The second approach is equivalent to stateful client singleton, but will be slightly more optimized.

As for the server-side, the annotated trait will be transformed to accept back-end apropriate extractors and, potentially, any more needed extensions and hook functions for adding middleware on the level of each handler.

```rust
pub trait CounterService {
    type State: // apropriate trait bounds

    type GetCurrentExtensions: // apropriate trait bounds
    async fn get_current(extensions: Self::GetCurrentExtensions) -> u64;
    fn set_get_current_middleware(handler: ...) -> ... {
        // default noop impl 
    }

    type IncrementExtensions: // apropriate trait bounds
    async fn increment(extensions: Self::IncrementExtensions) -> ();
    fn set_increment_middleware(handler: ...) -> ... {
        // default noop impl
    }
}
```

Also, a registering function will be generated.

### Arguments

Endpoint functions can have arguments, that will be resolved to either dynamic path segments, query parameters, mutlipart form data and/or single body argument with corresponding `content-type`.

```rust
#[contract]
pub trait CounterService {
    // ...

    #[endpoint(post, "/add/{value}")]
    // E.g. for value = 4 will produce 
    // POST {service_root}/add/4 request and corresponding endpoint
    fn add_path(#[param(path)] value: u64) -> Result<()>;

    #[endpoint(post, "/add")]
    // E.g. for value = 4 will produce 
    // POST {service_root}/add?value=4 request and corresponding endpoint
    fn add_query(#[param(query)] value: u64) -> Result<()>;

    // ETC


    // ...
}
```

### Error handling

*COMING SOON* 

### Equivalence of endpoints 

Even though generally any unique URL will correspond to a unique resource, HTTP requests are parametrized with much more, than only URLs. Thus, any given URL can be **viewed** as a set of different endpoints, and, as long as each of those views is **unique**, any request to a service can be unilaterally mapped to a single handler (view).  

Most of back-end web frameworks abide by the same rules for defining when endpoints overlap with each other, but, as `comfund` aims to be framework agnostic, here is a generalized set of  rules, in order of priority, that `comfund` will check for correctness. If they all apply for two or more defined endpoints, they will be considered **conflicting** and corresponding compile error will be produced.

1. Endpoints are **mounted on the same url**.
2. Endpoints have the **same HTTP method**.
3. Endpoints have the same accepted **`content-type`**.
3. Endpoints either:
    + have **matching parameters**;
    + the same number of **path parameters** and no [relative priority](#relative-priority) rules were set;
    + the same number of **required query parameters with the same names** and no [relative priority](#relative-priority) rules were set;
    + a set of required parameters of one endpoint is a subset of all the parameters of another endpoint (e.g. one endpoint requires `age` and conditionally `[gender]` and another one requires both `age` and `gender`);
    
The rule of the thumb here would be ***if i were to write such a set of endpoints, would web framework launch successfully***. Of course, these rules can be loosened in the future either by introducing new attributes or improving the generating engine of `comfund`.  

### Relative priority

### Supported back- and front-ends

**Frontends**

- [`reqwest`](https://docs.rs/reqwest/latest/reqwest/)

**Backends**

- [`axum`](https://docs.rs/axum/latest/axum/)

### Service and Handler-Local state

COMING SOON

### Milestones

- [x] MVP
- [ ] Authentication
- [ ] `actix-web` support
- [ ] Generation/forwardin fo doc comments.
- [ ] Contract defaults
- [ ] Reserved keywords tracking
- [ ] Enable restricting client/server backends for enabling implementation-dependent features
- [ ] Compression support
- [ ] Desctructuring of path and query params for server-side
- [ ] Sync client implementations?
- [ ] Result mappings?
- [ ] Versioning support
- [ ] Generate feature guards for missing features (like `json`, `http2`, etc.)
- [ ] OpenApi spec generation support
 
