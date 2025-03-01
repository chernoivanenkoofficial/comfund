#[derive(Debug, Clone)]
pub enum PathSegment<'a> {
    Dynamic(syn::Ident),
    Static(&'a str),
}

impl<'a> PathSegment<'a> {
    pub fn in_path(path: &'a str) -> impl Iterator<Item = Self> {
        path.strip_suffix("/")
            .unwrap_or(path)
            .strip_prefix("/")
            .unwrap_or(path)
            .split_terminator("/")
            .map(|segment| {
                if segment.starts_with("{") && segment.ends_with("}") {
                    let val = segment
                        .strip_prefix("{")
                        .unwrap_or("")
                        .strip_suffix("}")
                        .unwrap_or("");

                    let ident = quote::format_ident!("{val}");

                    Self::Dynamic(ident)
                } else {
                    Self::Static(segment)
                }
            })
    }

    pub fn is_dyn(&self) -> bool {
        matches!(self, Self::Dynamic(_))
    }
}
