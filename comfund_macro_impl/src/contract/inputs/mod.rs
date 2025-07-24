use std::hash::Hash;

use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_quote, parse_quote_spanned};

use crate::contract::endpoint::Endpoint;
use crate::contract::param::Param;
use crate::contract::Contract;

use crate::extensions::SynTypeExtensions;

/// A set of params, passed through the same URL [transport](`crate::contract::transport::Transport`)
/// method. Currently, `Inputs` will be constructed only for `path` and `query` params.
///
/// # Param grouping
///
/// If several params are passed thorugh the same transport type, back-end will use
/// [`serde`] to parse them into tuple, struct or hashmap. Thus, we need to generate a struct,
/// implementing [`Serialize`] or [`Deserialie`], to get those params and pass them to handler,
/// or to serialize them into request.
///
/// On the opposite, if only one param is passed thorugh path/query, we can use the provided
/// type directly. That means, that we can use the original ident when passing those params
/// to handler as well.   
#[derive(Debug, Clone)]
pub struct Inputs {
    /// Id of single param. Wil be `None` if there were multiple params and
    /// facade struct was generated.
    pub id: Option<syn::Ident>,
    /// Type of this inputs param. Will be either a type of single param directly
    /// or a generated type for a group of params.
    pub ty: syn::Type,
    /// Type of special owned structure, where references are substituted with owned analogs.
    pub owned_ty: Option<syn::Type>,
    /// A list of params, that this struct groups.
    pub params: Vec<Param>,
    /// A struct definition for this group of params. Will be none, if group consists
    /// of single param.
    pub definition: Option<syn::ItemStruct>,
    /// A struct definition for owned version of this struct. Will be `None` if
    /// a group doesn't have reference param.
    pub owned_definition: Option<syn::ItemStruct>,
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

    /// Returns if this inputs set actually won't be wrapped into new struct.
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

    pub fn definition(&self) -> Option<&syn::ItemStruct> {
        self.definition.as_ref()
    }

    pub fn owned_definition(&self) -> Option<&syn::ItemStruct> {
        self.owned_definition.as_ref()
    }

    /// Returns type for naive version of this struct.
    ///
    /// Naive means, that ref [`Param`]s types were not substituted for
    /// owned versions.
    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }

    /// Returns type for a fully owned version of this struct.
    /// If group doesn't contain ref parameters, will be the same
    /// as [`Inputs::ty`].
    pub fn owned_ty(&self) -> &syn::Type {
        self.owned_ty.as_ref().unwrap_or(&self.ty)
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
    pub fn initializator<T: quote::ToTokens>(
        &self,
        init_exprs: Option<&[T]>,
    ) -> Option<impl quote::ToTokens> {
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
    pub fn destructor(&self, expr: impl quote::ToTokens) -> impl quote::ToTokens {        
        if let Some(id) = self.id() {
            quote! {
                let #id = #expr;
            }
        } else {
            let name = self.ty();
            let fields = self.params.iter().map(|p| &p.id);

            quote! {
                let #name {
                    #(#fields),*
                } = #expr;
            }
        }
    }

    /// Same as [`destructor`](Self::destructor), but for for owned version of 
    /// this [`Inputs`] group.
    pub fn owned_destructor(&self, expr: impl quote::ToTokens) -> impl quote::ToTokens {
        if let Some(id) = self.id() {
            quote! {
                let #id = #expr;
            }
        } else {
            let name = self.owned_ty();
            let fields = self.params.iter().map(|p| &p.id);

            quote! {
                let #name {
                    #(#fields),*
                } = #expr;
            }
        }
    }

    /// Parse this group as a [`syn::FnArg`] for server-side
    /// wrapper function.
    pub fn as_wrapper_arg(
        &self,
        wrapper: &syn::Path,
        default_id: impl FnOnce() -> syn::Ident,
    ) -> syn::FnArg {
        let id = self.id_or(default_id());
        let ty = self.owned_ty();

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
///   endpoint name and "Inputs".
///
/// ## Returns
/// `Some(Inputs)` if `params` had any elements,
/// otherwise `None`.
pub fn from_params(ep_name: &syn::Ident, params: Vec<Param>, suffix: &str) -> Option<Inputs> {
    if params.is_empty() {
        return None;
    }

    if params.len() == 1 && params[0].meta.options().flatten() {
        let param = params.first().unwrap();

        let id = param.id.clone();
        let ty = param.ty().clone();
        let owned_ty = if param.is_ref() {
            Some(param.owned_ty())
        } else {
            None
        };

        return Some(Inputs {
            id: Some(id),
            ty,
            owned_ty,
            params,
            definition: None,
            owned_definition: None,
        });
    }

    if needs_owned_definition(&params) {
        let ty = gen_type(ep_name, suffix);
        let definition = gen_borrowed_definition(&ty, &params);

        let owned_ty = gen_owned_type(ep_name, suffix);
        let owned_definition = gen_owned_definition(&owned_ty, &params);

        Some(Inputs {
            id: None,
            ty,
            owned_ty: Some(owned_ty),
            params,
            definition: Some(definition),
            owned_definition: Some(owned_definition),
        })
    } else {
        let ty = gen_type(ep_name, suffix);
        let definition = gen_owned_definition(&ty, &params);

        Some(Inputs {
            id: None,
            ty,
            owned_ty: None,
            params,
            definition: Some(definition),
            owned_definition: None,
        })
    }
}

fn gen_owned_definition(ty: &syn::Type, params: &[Param]) -> syn::ItemStruct {
    let struct_attrs = struct_level_attrs(ty.span());

    fn get_owned_field(param: &Param) -> syn::Field {
        let id = &param.id;
        let ty = param.owned_ty();
        let flatten_attr = flatten_attr(param);

        parse_quote! {
            #flatten_attr
            pub #id: #ty
        }
    }

    let fields = params.iter().map(get_owned_field);

    parse_quote! {
        #(#struct_attrs)*
        pub struct #ty {
            #(#fields),*
        }
    }
}

fn gen_borrowed_definition(ty: &syn::Type, params: &[Param]) -> syn::ItemStruct {
    let mut lifetimes = Vec::new();
    let mut lifetime_counter = 0;

    let struct_attrs = struct_level_attrs(ty.span());

    fn get_borrowed_field(
        param: &Param,
        lifetime_counter: &mut i32,
        lifetimes: &mut Vec<syn::Lifetime>,
    ) -> syn::Field {
        let id = &param.id;
        let lt = if param.ty.is_ref() {
            *lifetime_counter += 1;
            let lt = syn::parse_str(&format!("'a{}", lifetime_counter)).unwrap();
            lifetimes.push(lt);
            lifetimes.last()
        } else {
            None
        };

        let ty = param.borrowed_ty(lt);

        let flatten = flatten_attr(param);

        parse_quote! {
            #flatten
            pub #id: #ty
        }
    }

    let fields = params
        .iter()
        .map(|param| get_borrowed_field(param, &mut lifetime_counter, &mut lifetimes))
        .collect::<Vec<_>>();

    parse_quote! {
        #(#struct_attrs)*
        pub struct #ty<#(#lifetimes),*> {
            #(#fields),*
        }
    }
}

/// Check, if param list contains any references
fn needs_owned_definition(params: &[Param]) -> bool {
    params.iter().any(|param| param.ty.is_ref())
}

/// Generate type for use in the [`Inputs`] construction.
fn gen_type(ep_name: &syn::Ident, suffix: &str) -> syn::Type {
    let mut ep_str = ep_name.to_string();
    ep_str.push_str(suffix);
    let id = syn::Ident::new(&stringcase::pascal_case(&ep_str), ep_name.span());

    parse_quote!(#id)
}

fn gen_owned_type(ep_name: &syn::Ident, suffix: &str) -> syn::Type {
    let mut ep_str = ep_name.to_string();
    ep_str.push_str(suffix);
    ep_str.push_str("_owned");
    let id = syn::Ident::new(&stringcase::pascal_case(&ep_str), ep_name.span());

    parse_quote!(#id)
}

/// Atrtibute for `#[serde(flatten)]` option of generated struct's fields. 
fn flatten_attr(param: &Param) -> Option<syn::Attribute> {
    let span = param.id.span();

    if param.meta.options().flatten() {
        Some(parse_quote_spanned! {
            span=>
            #[cfg_attr(
                any(feature = "reqwest", feature = "actix-web", feature = "axum"),
                serde(flatten)
            )]
        })
    } else {
        None
    }
}

/// Get common struct level attributes of generated [`Inputs`] facade struct.
fn struct_level_attrs(span: proc_macro2::Span) -> impl Iterator<Item = syn::Attribute> {
    vec![
        parse_quote_spanned!(
            span=>
            #[cfg_attr(
                any(feature = "reqwest"),
                derive(::serde::Serialize)
            )]
        ),
        parse_quote_spanned!(
            span=>
            #[cfg_attr(
                any(feature = "actix-web", feature = "axum"),
                derive(::serde::Deserialize)
            )]
        ),
    ]
    .into_iter()
}
