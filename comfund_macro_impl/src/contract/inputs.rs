use std::hash::Hash;

use quote::{quote, quote_spanned};
use syn::parse_quote;

use crate::contract::endpoint::Endpoint;
use crate::contract::param::Param;
use crate::contract::Contract;

/// A set of params, passed through the same URL [transport](`crate::contract::transport::Transport`) 
/// method. Currently, `Inputs` will be constructed only for `path` and `query` params.  
#[derive(Debug, Clone)]
pub struct Inputs {
    pub id: Option<syn::Ident>,
    pub ty: syn::Type,
    pub params: Vec<Param>,
    pub definition: Option<proc_macro2::TokenStream>,
}

impl Inputs {
    pub const DEFAULT_PATH_NAME: &'static str = "path_inputs"; 
    pub const DEFAULT_QUERY_NAME: &'static str = "query_inputs"; 

    pub fn is_empty(inputs: Option<&Self>) -> bool {
        match inputs {
            None => true,
            Some(inputs) => inputs.params.is_empty(),
        }
    }

    /// Returns, if this inputs set actually won't be wrapped into new struct.
    /// 
    /// `Inputs` are considered flat, if only one arg is passed through 
    /// the given [transport](`crate::contract::transport::Transport`), thus no 
    /// wrapping for serializing/deserializing is required.
    pub fn is_flat(&self) -> bool {
        self.definition.is_none()
    }

    pub fn id(&self) -> Option<&syn::Ident> {
        self.id.as_ref()
    }

    pub fn id_or(&self, default: syn::Ident) -> syn::Ident {
        self.id.clone().unwrap_or(default)
    }

    /// Get initializator statement for this [`Inputs`] struct.
    /// 
    /// # Example
    /// 
    /// ```
    /// let inputs = from_params(
    ///     &parse_quote!(hello_world), 
    ///     /* params like `hello: String`, world: `bool` */,
    ///     ""
    /// );    
    /// 
    /// // Definition will look like:
    /// //
    /// // #[attributes]
    /// // pub struct HelloWorldInputs {
    /// //     hello: String,
    /// //     world: bool
    /// // }
    /// 
    /// let initialiator = inputs.initializator(&parse_quote!(hello_world_inputs), None).unwrap();
    /// // initialiator will be an expression of next content:
    /// //
    /// // HelloWorldInputs {
    /// //   hello, 
    /// //   world
    /// // } 
    /// // 
    /// // or, if a list of init expressions was specified:
    /// //
    /// // HelloWorldInputs {
    /// //   hello: { exprs[1] }, 
    /// //   world: { exprs[2] }
    /// // } 
    /// ```
    pub fn initializator<T: quote::ToTokens>(&self, init_exprs: Option<&[T]>) -> Option<impl quote::ToTokens> {
        if self.is_flat() {
            return None;
        }

        let fields = self.params.iter().map(|param| &param.id);
        let name = &self.ty;

        if let Some(args) = init_exprs {
            if args.len() != self.params.len() {
                panic!()
            }

            Some(quote! {
                #name {
                    #(#fields: #args),*
                }
            })
        } else {
            Some(quote! {
                #name {
                    #(#fields),*
                }
            })
        }
    }

    /// Get destructor statement for this [`Inputs`] struct.
    /// 
    /// # Example
    /// 
    /// ```
    /// let inputs = from_params(
    ///     &parse_quote!(hello_world), 
    ///     /* params like `hello: String`, world: `bool` */,
    ///     ""
    /// );    
    /// 
    /// // Definition will look like 
    /// // #[attributes]
    /// // pub struct HelloWorldInputs {
    /// //     hello: String,
    /// //     world: bool
    /// // }
    /// 
    /// let destructor = inputs.destructor(&parse_quote!(hello_world_inputs)).unwrap();
    /// // destructor will be stream of next content:
    /// // let HelloWorldInputs {
    /// //   hello, 
    /// //   world
    /// // } = hello_world_inputs;
    /// ```
    pub fn destructor(&self, var: impl quote::ToTokens) -> Option<impl quote::ToTokens> {
        if self.is_flat() {
            return None;
        }

        let fields = self.params.iter().map(|p| &p.id);
        let name = &self.ty;

        Some(
            quote! {
                let #name {
                    #(#fields),*
                } = #var;
            }
        )
    }

    pub fn as_handler_arg(&self, wrapper: &syn::Path, default_id: impl FnOnce() -> syn::Ident) -> syn::FnArg {
        let id = self.id_or(default_id());
        let ty = &self.ty;

        parse_quote!(
            #id: #wrapper::<#ty>
        )
    }
}

/// Generate `Inputs` struct for endpoint params. 
/// 
/// It's up to caller to ensure, that all [params](`crate::contract::param::Param`) 
/// have the same transport type.
/// 
/// ## Arguments
/// - `ep_name`: name of endpoint, which will be used for generating wrapper type name.
/// - `params`: a vec of params to be included in result [`Inputs`] set.
/// - `suffix`: a suffix for generated type, that will be included between 
/// endpoint name and "Inputs".
/// 
/// ## Returns
/// `Some(Inputs)` if `params` had any elements,
/// otherwise `None`.
pub fn from_params(ep_name: &syn::Ident, params: Vec<Param>, suffix: &str) -> Option<Inputs> {
    if params.is_empty() {
        None
    } else if params.len() == 1 {
        let id = params[0].id.clone();
        let ty = params[0].ty.clone();

        Some(Inputs {
            id: Some(id),
            ty,
            params,
            definition: None,
        })
    } else {
        let ty = gen_type(ep_name, suffix);

        let fields = params.iter().map(|param| {
            let name = &param.id;
            let ty = &param.ty;
            let flatten = if param.meta.options().flatten.is_set() {
                Some(quote_spanned! {
                    ep_name.span()=>
                    #[cfg_attr(
                        any(feature = "reqwest", feature = "actix-web", feature = "axum"),
                        serde(flatten)
                    )]
                })
            } else {
                None
            };

            quote! {
                #flatten
                pub #name: #ty
            }
        });

        let definition = quote_spanned! {
            ep_name.span()=>
            #[cfg_attr(
                any(feature = "reqwest"),
                derive(::serde::Serialize)
            )]
            #[cfg_attr(
                any(feature = "actix-web", feature = "axum"),
                derive(::serde::Deserialize)
            )]
            pub struct #ty {
                #(#fields),*
            }
        };

        Some(Inputs {
            id: None,
            ty,
            params,
            definition: Some(definition),
        })
    }
}

/// Generate type for use in the [`Inputs`] construction.
fn gen_type(ep_name: &syn::Ident, suffix: &str) -> syn::Type {
    let mut ep_str = ep_name.to_string();
    ep_str.push_str(suffix);
    let id = syn::Ident::new(&stringcase::pascal_case(&ep_str), ep_name.span());

    parse_quote!(#id)
}
