use syn::{parse_quote, parse_quote_spanned, punctuated::Punctuated};

use crate::contract::endpoint::Endpoint;
use crate::contract::inputs::Inputs;
use crate::contract::param::Param;
use crate::servers::names::Names;
use crate::servers::server_endpoint;

/// Component for creating endpoint wrap function.
pub struct WrapperFn<B, T, R, I>
where
    B: Fn(&Endpoint, &Param) -> syn::Type,
    T: Fn(&Endpoint) -> syn::Type,
    R: Fn(&Endpoint, syn::Expr) -> syn::Expr,
    I: Fn(syn::Expr) -> syn::Expr + Clone,
{
    path_extractor: syn::Path,
    query_extractor: syn::Path,
    body_type_mapper: B,
    ret_type_mapper: T,
    result_mapper: R,
    inputs_unwrapper: I,
}

impl<B, T, R, I> WrapperFn<B, T, R, I>
where
    B: Fn(&Endpoint, &Param) -> syn::Type,
    T: Fn(&Endpoint) -> syn::Type,
    R: Fn(&Endpoint, syn::Expr) -> syn::Expr,
    I: Fn(syn::Expr) -> syn::Expr + Clone,
{
    /// Create new component.
    ///
    /// ## Arguments
    ///
    /// - `path_extractor`: path to back-end's extractor of URL path parameters.
    /// - `query_extractor`: path to back-end's extractor of URL query parameters.
    /// - `body_type_mapper`: a map from endpoint and body param ty to a type, expected by
    ///   server back-end.
    /// - `ret_type_mapper`: a map from endpoint return type to a return type,
    ///   appropriate for server back-end.
    /// - `result_mapper`: a map from handler result expression to a result, returned to back-end.
    pub fn new(
        path_extractor: syn::Path,
        query_extractor: syn::Path,
        body_type_mapper: B,
        ret_type_mapper: T,
        result_mapper: R,
        inputs_unwrapper: I,
    ) -> Self {
        Self {
            path_extractor,
            query_extractor,
            body_type_mapper,
            ret_type_mapper,
            result_mapper,
            inputs_unwrapper,
        }
    }

    /// Define wrapper function for endpoint.
    ///
    /// ## Arguments
    /// * `ep`: endpoint, for which the wrapper function would be constructed.
    /// * `names`: a reference to the [`Names`] component constructed from `ep`.
    pub fn impl_for(&self, ep: &Endpoint, names: &Names) -> syn::ItemFn {
        let id = names.handler_id();
        let contract_id = &ep.contract_id;

        let args = self.define_args(ep, names);

        let ret = (self.ret_type_mapper)(ep);

        let path_destructor = server_endpoint::destructor(
            ep.path_inputs(),
            Inputs::DEFAULT_PATH_NAME,
            self.inputs_unwrapper.clone(),
        );
        let query_destructor = server_endpoint::destructor(
            ep.query_inputs(),
            Inputs::DEFAULT_QUERY_NAME,
            self.inputs_unwrapper.clone(),
        );

        let forwarded = server_endpoint::handler_call_args(ep);

        let service_trait_var = server_endpoint::service_trait_var();

        let result_expr: syn::Expr = parse_quote!(
            #service_trait_var::#id(#forwarded).await
        );

        let result_mapping = (self.result_mapper)(ep, result_expr);

        parse_quote! {
            pub async fn #id<#service_trait_var: #contract_id>(#args) -> #ret {
                #path_destructor
                #query_destructor
                #result_mapping
            }
        }
    }

    /// Define args of wrapper function for endpoint.
    ///
    /// ## Arguments
    /// * `ep`: endpoint, for which the argument list should be constructed.
    /// * `names`: a reference to the [`Names`] component with defined names
    ///   of endpoint items in contract trait.
    fn define_args(&self, ep: &Endpoint, names: &Names) -> Punctuated<syn::FnArg, syn::Token![,]> {
        let mut args = syn::punctuated::Punctuated::new();

        ep.path_inputs().inspect(|&inputs| {
            let arg = inputs.as_handler_arg(&self.path_extractor, || {
                syn::Ident::new(Inputs::DEFAULT_PATH_NAME, ep.id.span())
            });
            args.push(arg);
        });

        ep.query_inputs().inspect(|&inputs| {
            let arg = inputs.as_handler_arg(&self.query_extractor, || {
                syn::Ident::new(Inputs::DEFAULT_QUERY_NAME, ep.id.span())
            });
            args.push(arg);
        });

        let service_trait_var = server_endpoint::service_trait_var();
        let ext_type_id = names.ext_type_id();

        args.push(parse_quote_spanned! {
            ep.id.span()=>
            extensions: #service_trait_var::#ext_type_id
        });

        ep.body_param().inspect(|&param| {
            let name = &param.id;
            let ty = (self.body_type_mapper)(ep, param);

            args.push(parse_quote!(#name: #ty));
        });

        args.pop_punct();

        args
    }
}
