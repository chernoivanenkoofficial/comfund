use quote::{format_ident, quote, quote_spanned};
use syn::{parse_quote, parse_quote_spanned, Token, TypeParamBound};
use syn::punctuated::Punctuated;

use crate::contract::{endpoint::Endpoint, inputs::Inputs};
use crate::servers::names::Names;


pub fn service_trait_var() -> syn::Ident {
    syn::Ident::new("C", proc_macro2::Span::call_site())
}

pub fn def_ext_type(
    ext_type_id: &syn::Ident,
    bounds: Punctuated<TypeParamBound, syn::Token![+]>
) -> syn::TraitItemType {
    parse_quote_spanned!(
        ext_type_id.span()=>

        type #ext_type_id: #bounds;
    )
}

pub fn def_input_arg(
    inputs: &Inputs,
    span: proc_macro2::Span,
    default_name: &str,
    wrapper: &syn::Path,
) -> syn::FnArg {
    let ty = &inputs.ty;
    let id = inputs.id_or(syn::Ident::new(default_name, span));

    parse_quote!(#id: #wrapper::<#ty>)
}

/// Get punctuated args for handler signature, including extensions arg. 
/// 
/// # Example 
/// ``` 
/// let ep = Endpoint::parse(parse_quote! {
///     #[endpoint(get, "/some_path")]
///     fn endpoint(
///         #[param(...)] arg1: u32, 
///         #[param(...)] arg2: u32, 
///         #[param(...)] arg3: u32
///     );
/// }).unwrap();
/// 
/// let args = handler_sig_args(&ep, format_ident!("EndpointExtensions"));
/// assert_eq!(
///     args.into_token_stream(),
///     quote! {
///         arg1: u32, arg2: u32, arg3: u32, extensions: Self::EnspointExtensions
///     }
/// ) 
/// ```
pub fn handler_sig_args(
    ep: &Endpoint,
    names: &Names
) -> Punctuated<syn::FnArg, Token![,]> {
    let ext_type_id = names.ext_type_id();

    let (path_params, query_params, body_param) = ep.param_args();
    
    let mut fn_args= Punctuated::new();

    fn_args.extend(path_params);
    fn_args.extend(query_params);
    fn_args.push(parse_quote_spanned! {
        ep.id.span()=>
        extensions: Self::#ext_type_id
    });
    fn_args.extend(body_param);
    fn_args.pop_punct();

    fn_args
}

/// Get punctuated args for handler call, including extensions arg. 
/// 
/// # Example 
/// ``` 
/// let ep = Endpoint::parse(parse_quote! {
///     #[endpoint(get, "/some_path")]
///     fn endpoint(
///         #[param(...)] arg1: u32, 
///         #[param(...)] arg2: u32, 
///         #[param(...)] arg3: u32
///     );
/// }).unwrap();
/// 
/// let args = handler_sig_args(&ep);
/// assert_eq!(
///     args.into_token_stream(),
///     quote! {
///         arg1, arg2, arg3, extensions
///     }
/// ) 
/// ```
pub fn handler_call_args(
    ep: &Endpoint
) -> Punctuated<syn::Ident, Token![,]> {
    let (path_names, query_names, body_name) = ep.param_names();

    let mut forwarded = Punctuated::new();
    forwarded.extend(path_names.cloned());
    forwarded.extend(query_names.cloned());
    forwarded.push(format_ident!("extensions"));
    forwarded.extend(body_name.cloned());
    forwarded.pop_punct();

    forwarded
}


/// Get destructor statement for given inputs.
/// 
/// ## Arguments
/// 
/// - `inputs`: inputs of endpoint.
/// - `default_name`: default_name of function arg for `inputs`.
/// 
/// ## Returns
/// 
/// Returns statement, if `inputs` were not `None` and not [flat](`Inputs::is_flat`).
/// Otherwise, returns `None`.
pub fn destructor(
    inputs: Option<&Inputs>, 
    default_name: &str,
    extract: impl FnOnce(syn::Expr) -> syn::Expr
) -> Option<impl quote::ToTokens> {
    inputs.and_then(|inputs| {
        let ident = inputs.id().cloned().unwrap_or(syn::Ident::new(
            default_name,
            proc_macro2::Span::call_site(),
        ));

        inputs.destructor(extract(parse_quote!(#ident)))
    })
}