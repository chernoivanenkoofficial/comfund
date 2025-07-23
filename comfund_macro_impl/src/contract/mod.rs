pub mod content_type;
pub mod endpoint;
pub mod inputs;
pub mod method;
pub mod param;
pub mod query;
pub mod transport;

use quote::quote;

use endpoint::Endpoint;

use crate::{contract::content_type::ContentType, extensions::*};

pub fn implement(contract: &Contract) -> proc_macro2::TokenStream {
    let mut stream = quote! {};

    for ep in &contract.endpoints {
        if let Some(input) = &ep.path_inputs {
            if let Some(def) = &input.definition {
                stream.extend(def.clone());
            }
        }

        if let Some(input) = &ep.query_inputs {
            if let Some(def) = &input.definition {
                stream.extend(def.clone());
            }
        }
    }

    stream
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub id: syn::Ident,
    pub endpoints: Vec<Endpoint>,
    pub meta: ServiceMeta,
    pub attrs: Vec<syn::Attribute>,
}

#[derive(Debug, Clone, deluxe::ParseMetaItem)]
pub struct ServiceMeta(
    #[deluxe(flatten)] pub endpoint::EndpointOptions,
    #[deluxe(flatten)] pub ContractOptions
);

impl ServiceMeta {
    pub fn endpoint_defaults(&self) -> &endpoint::EndpointOptions {
        &self.0
    } 

    pub fn options(&self) -> &ContractOptions {
        &self.1
    }
}

#[derive(Debug, Clone, deluxe::ParseMetaItem)]
pub struct ContractOptions {
}

impl Contract {
    pub fn parse(args: proc_macro2::TokenStream, item_trait: syn::ItemTrait) -> syn::Result<Self> {
        let meta = deluxe::parse2::<ServiceMeta>(args)?;

        let mut errors = None;

        let id = item_trait.ident;
        let attrs = item_trait.attrs;
        let fn_items = get_fn_items(item_trait.items, &mut errors);
        let endpoints = fn_items
            .into_iter()
            .map(|item| Endpoint::parse(id.clone(), item, meta.endpoint_defaults()))
            .partition_syn_err(&mut errors);

        if let Some(err) = errors {
            Err(err)
        } else {
            Ok(Self {
                id,
                endpoints,
                meta,
                attrs,
            })
        }
    }

    pub fn validate(&self) -> syn::Result<()> {
        let u = validate_endpoints_uniqueness(&self.endpoints);
        let c = validate_endpoints_correctness(&self.endpoints);

        combine_results!(u, c)?;

        Ok(())
    }
}

fn get_fn_items(
    trait_items: impl IntoIterator<Item = syn::TraitItem>,
    errors: &mut Option<syn::Error>,
) -> Vec<syn::TraitItemFn> {
    trait_items
        .into_iter()
        .map(|item| {
            if let syn::TraitItem::Fn(fn_item) = item {
                Ok(fn_item)
            } else {
                let err = syn::Error::new_spanned(item, "Non `fn` trait items are not supported.");
                Err(err)
            }
        })
        .partition_syn_err(errors)
}

fn validate_endpoints_correctness(eps: &[Endpoint]) -> syn::Result<()> {
    let errors = eps
        .iter()
        .map(Endpoint::validate)
        .filter_map(Result::err)
        .fold(None, |mut acc, err| {
            acc.combine(err);
            acc
        });

    if let Some(err) = errors {
        Err(err)
    } else {
        Ok(())
    }
}

fn validate_endpoints_uniqueness(eps: &[Endpoint]) -> syn::Result<()> {
    let mut buf = std::collections::HashSet::with_capacity(eps.len());
    let mut errors = None;

    for id in eps.iter().map(|ep| &ep.id) {
        if !buf.insert(id) {
            combine_err!(
                errors,
                id,
                format!("Repeated endpoint ident: {}", id.to_string())
            )
        }
    }

    if let Some(err) = errors {
        Err(err)
    } else {
        Ok(())
    }
}
