use core::error;
use std::borrow::Borrow;

use crate::contract::transport::Transport;
use crate::extensions::*;

use quote::quote;

/// Parsed endpoint arg
#[derive(Debug, Clone, Eq)]
pub struct Param {
    /// Type of expected arg
    pub ty: syn::Type,
    /// Name of expected arg
    pub name: syn::Ident,
    pub meta: ParamMeta,
    pub attributes: Vec<syn::Attribute>,
}

impl PartialEq<Param> for Param {
    fn eq(&self, other: &Param) -> bool {
        self.ty.eq(&other.ty) && self.name.eq(&other.name) && self.meta.eq(&other.meta)
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

        let type_ = (*arg.ty).clone();
        let type_validation = validate_type(&type_);

        let name = desctruct_arg(&arg.pat);

        let meta = deluxe::extract_attributes(&mut arg);
        let attributes = arg.attrs;

        let (name, meta, _) = combine_results!(name, meta, type_validation)?;

        Ok(Self {
            name,
            ty: type_,
            meta,
            attributes,
        })
    }

    pub fn parse_list(inputs: impl IntoIterator<Item = syn::FnArg>) -> syn::Result<Vec<Self>> {
        inputs.into_iter().map(Self::parse).collect_syn_results()
    }

    pub fn as_function_argument(&self) -> proc_macro2::TokenStream {
        let id = &self.name;
        let ty = &self.ty;
        let attrs = self.attributes.iter();

        quote!(#(#attrs)* #id: #ty)
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
