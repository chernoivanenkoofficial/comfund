use comfund_paths::path_template::PathTemplate;

use crate::contract::content_type::ContentType;
use crate::contract::method::Method;
use crate::contract::param::Param;
use crate::contract::transport::Transport;
use crate::contract::ContractOptions;

use crate::extensions::*;

use super::inputs::{self, Inputs};

/// Parsed service endpoint
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Name of function to be rendered for client/server
    pub id: syn::Ident,
    /// Endpoint metadata
    pub meta: EndpointMeta,
    /// Params passed in path part of endpoint request
    pub path_inputs: Option<Inputs>,
    /// Params passed in query part of endpoint request
    pub query_inputs: Option<Inputs>,
    /// Body param of endpoint request
    pub body_param: Option<Param>,
    /// Expected result of endpoint
    pub ret: syn::Type,
    /// Forwarded fn attributes
    pub attrs: Vec<syn::Attribute>,
}

impl Endpoint {
    pub fn parse(
        fn_item: syn::TraitItemFn,
        endpoint_defaults: &EndpointOptions,
    ) -> Result<Self, syn::Error> {
        let id = fn_item.sig.ident.clone();

        let mut attrs = fn_item.attrs;
        let meta = deluxe::extract_attributes::<_, EndpointMeta>(&mut attrs);

        let sig_validation = validate_signature(&fn_item.sig);

        let params = Param::parse_list(fn_item.sig.inputs);
        let ret = get_returned_type(&fn_item.sig.output);

        let (_, mut meta, params, ret) = combine_results!(sig_validation, meta, params, ret)?;

        let (path_inputs, query_inputs, body_param) = gen_inputs(&id, params)?;

        meta.2 = meta.2.merge(endpoint_defaults);

        Ok(Self {
            id,
            meta,
            path_inputs,
            query_inputs,
            body_param,
            ret,
            attrs,
        })
    }

    pub fn validate(&self) -> Result<(), syn::Error> {
        validate_path(self.meta.path_lit())
    }
}

#[derive(Debug, Clone, deluxe::ExtractAttributes)]
#[deluxe(attributes(endpoint))]
pub struct EndpointMeta(
    /// An HTTP request method for endpoint
    #[deluxe(with = crate::utils::parse_ident)]
    pub Method,
    /// Path to an endpoint from service root
    pub syn::LitStr,
    /// Options
    #[deluxe(flatten)]
    pub EndpointOptions,
);

impl EndpointMeta {
    pub fn method(&self) -> Method {
        self.0
    }

    pub fn path(&self) -> String {
        self.1.value()
    }

    pub fn path_lit(&self) -> &syn::LitStr {
        &self.1
    }

    pub fn options(&self) -> &EndpointOptions {
        &self.2
    }
}

deluxe::define_with_optional!(
    mod content_type_optional,
    deluxe::with::from_str,
    crate::contract::content_type::ContentType
);

#[derive(Debug, Clone, Default, deluxe::ParseMetaItem)]
#[deluxe(default)]
pub struct EndpointOptions {
    /// Content type for endpoint
    #[deluxe(with = content_type_optional)]
    pub content_type: Option<ContentType>,
}

impl EndpointOptions {
    pub fn merge(mut self, defaults: &Self) -> Self {
        self.content_type = self.content_type.or(defaults.content_type.clone());
        
        self
    }
}

fn get_returned_type(ty: &syn::ReturnType) -> syn::Result<syn::Type> {
    match ty {
        syn::ReturnType::Default => Ok(syn::Type::Tuple(syn::TypeTuple {
            elems: Default::default(),
            paren_token: Default::default(),
        })),
        syn::ReturnType::Type(_, ty) => match ty.as_ref() {
            syn::Type::Array(_)
            | syn::Type::Group(_)
            | syn::Type::Paren(_)
            | syn::Type::Path(_)
            | syn::Type::Tuple(_) => Ok(ty.as_ref().clone()),
            unsupported => Err(syn::Error::new_spanned(
                unsupported,
                "Unsupported return type.",
            )),
        },
    }
}

fn gen_inputs(
    ep_name: &syn::Ident,
    params: Vec<Param>,
) -> syn::Result<(Option<Inputs>, Option<Inputs>, Option<Param>)> {
    let mut errors = None;
    let mut params = params.into_iter().peekable();

    // Path params

    let mut path_params = vec![];

    while let Some(p) = params.peek() {
        if p.meta.0 != Transport::Path {
            break;
        }

        path_params.push(params.next().unwrap());
    }

    let path_inputs = inputs::from_params(ep_name, path_params, "_path_inputs");
    // Query params

    let mut query_params = vec![];

    while let Some(p) = params.peek() {
        match p.meta.0 {
            Transport::Path => {
                combine_err!(
                    errors,
                    &p.name,
                    "Path params should be specified before query params."
                );
                params.next().unwrap();
            }
            Transport::Query => query_params.push(params.next().unwrap()),
            _ => break,
        }
    }

    let query_inputs = inputs::from_params(ep_name, query_params, "_query_inputs");

    // Body param

    let body_param = params.next().and_then(|param| match param.meta.0 {
        Transport::Path | Transport::Query => {
            combine_err!(errors, &param.name, "Unexpected transport type");
            None
        }
        _ => Some(param),
    });

    // Leftover incorrect params

    for leftover in params {
        combine_err!(
            errors,
            leftover.name,
            "Unexpected param. At most one body param is supported and no other params can be passed after body param.")
    }

    if let Some(err) = errors {
        Err(err)
    } else {
        Ok((path_inputs, query_inputs, body_param))
    }
}

fn validate_signature(sig: &syn::Signature) -> Result<(), syn::Error> {
    let mut errors = None;

    if let Some(ref constness) = sig.constness {
        combine_err!(
            errors,
            constness,
            "Const functions are not allowed in contracts."
        );
    }

    if let Some(ref asynncness) = sig.asyncness {
        errors.combine(syn::Error::new_spanned(
            asynncness,
            "Asyncness of endpoints is not controlled by contracts.",
        ));
    }

    if let Some(ref unsafety) = sig.unsafety {
        errors.combine(syn::Error::new_spanned(
            unsafety,
            "Unsafe functions are not allowed in contracts.",
        ));
    }

    if let Some(ref variadic) = sig.variadic {
        errors.combine(syn::Error::new_spanned(
            variadic,
            "Variadic functions are not allowed in contracts.",
        ));
    }

    if let Some(ref abi) = sig.abi {
        errors.combine(syn::Error::new_spanned(
            abi,
            "Abi specifications are no allowed in contracts.",
        ));
    }

    if !sig.generics.params.is_empty() {
        errors.combine(syn::Error::new_spanned(
            &sig.generics,
            "Generics are not allowed for endpoints.",
        ));
    }

    if let Some(err) = errors {
        Err(err)
    } else {
        Ok(())
    }
}

fn validate_path(path: &syn::LitStr) -> syn::Result<()> {
    let path_str = path.value();
    comfund_paths::PathTemplate::new(&path_str)
        .map_err(|err| syn::Error::new_spanned(path, format!("invalid path: {err}")))?;

    Ok(())
}
