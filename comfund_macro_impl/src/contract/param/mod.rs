//! # Endpoint parameters
//! 
//! Endpoints can have parameters, that will be serialized by
//! client implementation or deserialied by server implementation 
//! before passing to user implemented handlers. The type of these 
//! parameters can be either [syn::TypePath] of [`syn::TypeReference`], that 
//! were brought into the conract's scope.
//! 
//! As server side needs owned types for deserialization, most of the functions of
//! [`Param`] have versions for declared type and owned type (if needed), resolved through 
//! [std::borrow::ToOwned]. For user defined types this works through blank implementation of
//! [`ToOwned`] for `T: Clone`. Or a custom implementaiton for unsized types can be provided.  
use core::error;
use std::borrow::Borrow;

use crate::contract::transport::Transport;
use crate::extensions::*;

use quote::quote;
use syn::{parse_quote, parse_quote_spanned, spanned::Spanned};

/// Parsed endpoint parameter.
#[derive(Debug, Clone, Eq)]
pub struct Param {
    /// Type of expected parameter.
    pub ty: syn::Type,
    /// Name of expected parameter.
    pub id: syn::Ident,
    pub meta: ParamMeta,
    pub attributes: Vec<syn::Attribute>,
}

impl PartialEq<Param> for Param {
    fn eq(&self, other: &Param) -> bool {
        self.ty.eq(&other.ty) && self.id.eq(&other.id) && self.meta.eq(&other.meta)
    }
}

impl Param {
    pub fn parse(arg: syn::FnArg) -> syn::Result<Self> {
        let mut arg = if let syn::FnArg::Typed(arg) = arg {
            arg
        } else {
            return Err(syn::Error::new_spanned(
                arg,
                "Endpoints cannot recieve self args.",
            ));
        };

        let ty = (*arg.ty).clone();
        let type_validation = validate_type(&ty);

        let id = desctruct_arg(&arg.pat);

        let meta = deluxe::extract_attributes(&mut arg);
        let attributes = arg.attrs;

        let (id, meta, _) = combine_results!(id, meta, type_validation)?;

        Ok(Self {
            id,
            ty,
            meta,
            attributes,
        })
    }

    pub fn parse_list(inputs: impl IntoIterator<Item = syn::FnArg>) -> syn::Result<Vec<Self>> {
        inputs.into_iter().map(Self::parse).collect_syn_results()
    }

    pub fn is_ref(&self) -> bool {
        self.ty.is_ref()
    }

    /// Get a type of this param in owned form 
    /// (with references resolved to [`ToOwned::Owned`] 
    /// associated type).
    pub fn as_owned_fn_arg(&self) -> syn::FnArg {
        let id = &self.id;
        let ty = self.owned_ty();
        let attrs = self.attributes.iter();

        parse_quote!(#(#attrs)* #id: #ty)
    }

    /// Get a type of this param in borrowed form 
    /// (either by value or a reference with no lifetime specified).
    pub fn as_borrowed_fn_arg(&self) -> syn::FnArg {
        let id = &self.id;
        let ty = self.borrowed_ty(None);
        let attrs = self.attributes.iter();

        parse_quote!(#(#attrs)* #id: #ty)
    }

    /// Get declared type of this param.
    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }

    /// Get a type of this param in owned context 
    /// (check [module](crate::contract::param) docs for more info).
    pub fn owned_ty(&self) -> syn::Type {
        match &self.ty {
            syn::Type::Path(_) => self.ty.clone(),
            syn::Type::Reference(ty) => {
                let inner = &*ty.elem;

                parse_quote_spanned!(
                    ty.span()=>
                    <#inner as ::std::borrow::ToOwned>::Owned
                )
                
            },
            _ => unreachable!()
        }
    }

    /// Get a type of this param in borrowed context
    /// (typically in the client side code).
    /// 
    /// ## Panics
    /// 
    /// If `self` type is [`syn::Type::Path`] and `lifetime` was `Some` 
    /// or if `self` type is [`syn::Type::Reference`] and lifetime wasn't provided.
    pub fn borrowed_ty(&self, lt: Option<&syn::Lifetime>) -> syn::Type {
        use syn::Type;
        
        match &self.ty {
            Type::Path(_) => if lt.is_none() {
                self.ty.clone()
            } else {
                panic!("Trying to assign lifetime to a non-borrowed type")
            },
            Type::Reference(reference) => {
                let inner = &*reference.elem;
                parse_quote!(&#lt #inner)
            },
            _ => unreachable!("Unsupported type of parameter.")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, deluxe::ExtractAttributes)]
#[deluxe(attributes(param))]
pub struct ParamMeta(
    #[deluxe(with = crate::utils::parse_ident)] pub Transport,
    #[deluxe(flatten)] pub ParamOptions,
);

impl ParamMeta {
    pub fn transport(&self) -> Transport {
        self.0
    }

    pub fn options(&self) -> &ParamOptions {
        &self.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, deluxe::ParseMetaItem)]
#[deluxe(default)]
pub struct ParamOptions {
    pub flatten: deluxe::Flag,
}

impl ParamOptions {
    pub fn flatten(&self) -> bool {
        self.flatten.is_set()
    }
}

fn validate_type(ty: impl Borrow<syn::Type>) -> Result<(), syn::Error> {
    match ty.borrow() {
        syn::Type::Macro(ty) => Err(syn::Error::new_spanned(
            ty,
            "Macros are currently not supported as type parameters.",
        )),
        syn::Type::BareFn(ty) => Err(syn::Error::new_spanned(
            ty,
            "Bare functions are currently not accepted as type parameters.",
        )),
        syn::Type::ImplTrait(ty) => Err(syn::Error::new_spanned(
            ty,
            "Anonymous generics are function level and not supported.",
        )),
        syn::Type::Ptr(ty) => Err(syn::Error::new_spanned(
            ty,
            "Pointers are not allowed in contract endpoints.",
        )),
        syn::Type::TraitObject(ty) => Err(syn::Error::new_spanned(
            ty,
            "Trait objects are not allowed in contracts",
        )),
        syn::Type::Verbatim(ty) => Err(syn::Error::new_spanned(ty, "Unrecognized type")),
        syn::Type::Reference(ty) if ty.lifetime.is_some() || ty.mutability.is_some() => Err(
            syn::Error::new_spanned(ty, "Mutable references or refernces with lifetimes are not accepted as type parameters.")
        ),
        syn::Type::Tuple(ty) => Err(syn::Error::new_spanned(
            ty,
            "Tuples are not supported as an endpoint parameter.",
        )),
        _ => Ok(()),
    }
}

fn desctruct_arg(arg: &syn::Pat) -> syn::Result<syn::Ident> {
    match arg {
        syn::Pat::Ident(ident) => Ok(ident.ident.clone()),
        _ => Err(syn::Error::new_spanned(arg, "Unsupported argument binding")),
    }
}
