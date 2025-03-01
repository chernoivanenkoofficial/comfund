pub trait PartitionSynErr<T> {
    fn partition_syn_err(self, error: &mut Option<syn::Error>) -> Vec<T>;
}

impl<T, I: Iterator<Item = Result<T, syn::Error>>> PartitionSynErr<T> for I {
    fn partition_syn_err(self, error: &mut Option<syn::Error>) -> Vec<T> {
        self.filter_map(|item| match item {
            Ok(item) => Some(item),
            Err(err) => {
                error.combine(err);
                None
            }
        })
        .collect()
    }
}

pub trait CombineSynErr {
    fn combine(&mut self, err: syn::Error);
}

impl CombineSynErr for Option<syn::Error> {
    fn combine(&mut self, err: syn::Error) {
        if let Some(val) = self {
            val.combine(err);
        } else {
            *self = Some(err)
        }
    }
}

macro_rules! combine_err {
    ($errors:ident, $token:expr, $message:expr) => {
        $crate::extensions::CombineSynErr::combine(
            &mut $errors,
            ::syn::Error::new_spanned(($token), ($message)),
        )
    };
}
pub(crate) use combine_err;

macro_rules! combine_results {
    ($($result:ident),*) => {{
        let mut ___err = None;
        $(
            let $result = match $result {
                Err(err) => {
                    $crate::extensions::CombineSynErr::combine(
                        &mut ___err,
                        err
                    );
                    None
                },
                Ok(res) => Some(res)
            };
        )*

        if let Some(err) = ___err {
            Err(err)
        } else {
            Ok((
                $($result.unwrap(),)*
            ))
        }
    }};
}
pub(crate) use combine_results;

pub trait CollectSynResults<T> {
    fn collect_syn_results<C: Default + Extend<T>>(self) -> syn::Result<C>;
}

impl<T, I: Iterator<Item = syn::Result<T>>> CollectSynResults<T> for I {
    fn collect_syn_results<C: Default + Extend<T>>(self) -> syn::Result<C> {
        let mut coll = C::default();

        let mut errors = None;

        for item in self {
            match item {
                Ok(res) => {
                    if errors.is_none() {
                        coll.extend(Some(res));
                    }
                }
                Err(err) => errors.combine(err),
            }
        }

        if let Some(errors) = errors {
            Err(errors)
        } else {
            Ok(coll)
        }
    }
}
