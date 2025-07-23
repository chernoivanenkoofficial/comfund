use quote::format_ident;

use crate::contract::endpoint::Endpoint;

/// Component for defining common names of items generated for endpoint. 
pub struct Names {
    handler_id: syn::Ident,
    decorator_id: syn::Ident,
    ext_type_name: syn::Ident,
}

impl Names {
    pub fn new(ep: &Endpoint) -> Self {
        let handler_id = ep.id.clone();
        let mut handler_str = handler_id.to_string();

        let decorator_id = format_ident!("set_{}_middleware", &handler_id);

        let ext_type_name = {
            handler_str.push_str("_extensions");
            let extensions_str = stringcase::pascal_case(&handler_str);

            syn::Ident::new(&extensions_str, handler_id.span())
        };

        Self {
            handler_id,
            decorator_id,
            ext_type_name,
        }
    }

    pub fn handler_id(&self) -> &syn::Ident {
        &self.handler_id
    }

    pub fn decorator_id(&self) -> &syn::Ident {
        &self.decorator_id
    }

    pub  fn ext_type_id(&self) -> &syn::Ident {
        &self.ext_type_name
    }
}