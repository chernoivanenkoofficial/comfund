use deluxe_core::{Error, ParseMode, Result};
use std::{borrow::Borrow, str::FromStr};
use syn::parse::{ParseBuffer, ParseStream};

#[inline]
pub fn parse_meta_item<T: FromStr>(input: ParseStream, _mode: ParseMode) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let s = input.parse::<syn::Ident>()?;
    T::from_str(&s.to_string()).map_err(|e| Error::new_spanned(s, e.to_string()))
}
#[inline]
pub fn parse_meta_item_inline<'s, S: Borrow<ParseBuffer<'s>>, T: FromStr>(
    inputs: &[S],
    mode: ParseMode,
) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    deluxe_core::parse_helpers::parse_first(inputs, mode, parse_meta_item)
}
#[inline]
pub fn parse_meta_item_flag<T>(span: proc_macro2::Span) -> Result<T> {
    Err(deluxe_core::parse_helpers::flag_disallowed_error(span))
}
#[inline]
pub fn parse_meta_item_named<T: FromStr>(
    input: ParseStream,
    _name: &str,
    span: proc_macro2::Span,
) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    deluxe_core::parse_named_meta_item_with!(input, span, self)
}
#[inline]
pub fn missing_meta_item<T>(name: &str, span: proc_macro2::Span) -> Result<T> {
    Err(deluxe_core::parse_helpers::missing_field_error(name, span))
}
