use crate::path_template::{PathTemplate, Segment};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::ser::{
    Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple, SerializeTupleStruct,
    Serializer,
};
use std::borrow::Cow;

/// Result type for this [`PathSerializer`] functionality.
pub type Result<T> = std::result::Result<T, Error>;

const FRAGMENTS: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_');

/// Dynamic URL path serializer.
///
/// # Accepted serialized values
///
/// * Plain values (or plain [sequences](https://serde.rs/data-model.html#:~:text=N(u8)%20%7D.-,seq,-A%20variably%20sized),
/// if the only capture in template was wildcard capture).
/// * Tuple of plain values, with member count equal to a number of captures in template.
/// * Maps of values.
/// * Flat structures.  
///
/// Structure being flat means that it doesn't nest from the point of view of
/// [serde data model](https://serde.rs/data-model.html). In practice,
/// this means that any nested struct passed to this serializer
/// should have nested fields annotated with
/// [`#[serde(flatten)]`](https://serde.rs/attr-flatten.html) attribute recursively
/// (or have corresponding by-hand implementation for serializing nested fields).
pub struct PathSerializer<'s, 't> {
    template: &'t PathTemplate<'s>,
    values: Vec<Option<String>>,
    wildcard_values: Vec<String>,
    nested: bool,
    next_entry: Option<usize>,
    key_mode: bool,
}

impl<'s, 't> PathSerializer<'s, 't> {
    /// Create new serializer from [`PathTemplate`].
    pub fn new(template: &'t PathTemplate<'s>) -> Self {
        let values = vec![None; template.idents().len()];

        let next_entry = if template.is_blank() { None } else { Some(0) };

        Self {
            template,
            values,
            wildcard_values: vec![],
            nested: false,
            next_entry,
            key_mode: false,
        }
    }

    /// Create interpolated URL path string after serialization
    /// and reset this instance, alowing for reuse with another serialized structure.
    ///
    /// If you don't need to reuse struct after serialization,
    /// use [`crate::serialize`] short-hand function instead.
    pub fn finalize(&mut self) -> Result<String> {
        if self.template.is_blank() {
            return Ok("/".to_owned());
        }
        let (values, wildcard_values) = self.reset();
        self.nested = false;
        self.next_entry = if self.template.idents().is_empty() {
            None
        } else {
            Some(0)
        };

        // Set empty string as outputs first elem for `join` to insert starting '/'
        let mut output = vec![Cow::Borrowed("")];
        let mut values = values.into_iter();

        for segment in self.template.segments() {
            match segment {
                Segment::Static(segment) => output.push(Cow::Borrowed(segment)),
                // TODO: Possible unsafe block, as
                // number of ids is guaranteed to match number of capture segments
                Segment::Capture(ident) => {
                    let value = values
                        .next()
                        .unwrap()
                        .ok_or(Error::MissingCapture((*ident).to_owned()))?;
                    output.push(Cow::Owned(value))
                }
            }
        }

        if self.template.wildcard().is_some() {
            for segment in wildcard_values {
                output.push(Cow::Owned(segment));
            }
        }

        // In case template only contains wildcard capture and
        // no values for wildcard were provided
        if output.len() == 1 {
            Ok("/".to_owned())
        } else {
            Ok(output.join("/"))
        }
    }

    fn reset(&mut self) -> (Vec<Option<String>>, Vec<String>) {
        let mut values = vec![None; self.template.idents().len()];
        let mut wildcard_values = vec![];

        std::mem::swap(&mut values, &mut self.values);
        std::mem::swap(&mut wildcard_values, &mut self.wildcard_values);

        self.nested = false;
        self.next_entry = if self.template.idents().is_empty() {
            None
        } else {
            Some(0)
        };

        (values, wildcard_values)
    }

    fn set_next_value(&mut self, val: String) -> Result<()> {
        if self.key_mode {
            self.key_mode = false;
            self.set_next_named_capture(&val)?;

            return Ok(());
        }

        match self.next_entry {
            Some(idx) if idx == self.values.len() => self.wildcard_values.push(val),
            Some(idx) => self.values[idx] = Some(val),
            None => return Err(Error::InvalidLen),
        }

        Ok(())
    }

    fn assert_elem(&self) -> Result<()> {
        if self.nested || self.values.len() == 1 {
            Ok(())
        } else {
            Err(Error::InvalidLen)
        }
    }

    fn set_nested(&mut self) -> Result<()> {
        if self.nested && self.next_entry.is_some() {
            Err(Error::DeepNesting)
        } else {
            self.nested = true;
            Ok(())
        }
    }

    fn assert_len(&self, len: Option<usize>) -> Result<()> {
        if self.next_entry.is_none() {
            Ok(())
        } else if let Some(len) = len {
            if self.template.param_count() == len {
                Ok(())
            } else {
                Err(Error::InvalidLen)
            }
        } else {
            Ok(())
        }
    }

    fn set_next_named_capture(&mut self, ident: &str) -> Result<()> {
        if self
            .template
            .wildcard()
            .is_some_and(|wildcard| ident == wildcard)
        {
            self.next_entry = Some(self.values.len());
            Ok(())
        } else {
            self.next_entry = Some(self.find_capture(ident)?);
            Ok(())
        }
    }

    fn find_capture(&self, ident: &str) -> Result<usize> {
        self.template
            .idents()
            .iter()
            .position(|&id| id == ident)
            .ok_or_else(|| Error::UknownCapture(ident.to_string()))
    }

    fn set_next_tuple_capture(&mut self) -> Result<()> {
        self.next_entry = match self.next_entry {
            Some(val) if val == self.values.len() => None,
            Some(val) => Some(val + 1),
            None => return Err(Error::InvalidLen),
        };

        Ok(())
    }

    fn next_capture_is_wildcard(&self) -> bool {
        let idx = self.values.len();
        self.next_entry.is_some_and(|id| idx == id)
    }

    fn assert_wildcard(&self) -> Result<()> {
        if self.next_capture_is_wildcard() {
            Ok(())
        } else {
            Err(Error::NonWildcardCapture)
        }
    }
}

macro_rules! impl_with_to_string {
    ($(($trait_fn:ident, $prim_ty:ty)),*) => {
        $(fn $trait_fn(self, v: $prim_ty) -> Result<()> {
            self.assert_elem()?;
            let value = utf8_percent_encode(&v.to_string(), &FRAGMENTS).to_string();
            self.set_next_value(value)?;

            Ok(())
        })*
    };
}

impl<'m, 's, 't> Serializer for &'m mut PathSerializer<'s, 't> {
    type Ok = ();
    type Error = Error;

    type SerializeMap = Self;
    type SerializeSeq = Self;
    type SerializeStruct = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bytes(self, _v: &[u8]) -> std::result::Result<Self::Ok, Self::Error> {
        Err(Error::TypeNotSupported("&[u8])"))
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        Err(Error::TypeNotSupported("Newtype variant"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::TypeNotSupported("Tuple variant"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::TypeNotSupported("Struct variant"))
    }

    fn is_human_readable(&self) -> bool {
        true
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.assert_elem()?;
        let value = utf8_percent_encode(v, FRAGMENTS).to_string();
        self.set_next_value(value)?;

        Ok(())
    }

    fn serialize_bool(self, v: bool) -> std::result::Result<Self::Ok, Self::Error> {
        self.assert_elem()?;

        let value = if v { "true" } else { "false" }.to_owned();

        self.set_next_value(value)?;

        Ok(())
    }

    fn serialize_char(self, v: char) -> std::result::Result<Self::Ok, Self::Error> {
        self.assert_elem()?;

        let mut buf = [0u8; 4];
        let str_repr = char::encode_utf8(v, &mut buf);
        let value = utf8_percent_encode(str_repr, FRAGMENTS).to_string();

        self.set_next_value(value)?;
        Ok(())
    }

    impl_with_to_string!(
        (serialize_u8, u8),
        (serialize_u16, u16),
        (serialize_u32, u32),
        (serialize_u64, u64),
        (serialize_u128, u128),
        (serialize_i8, i8),
        (serialize_i16, i16),
        (serialize_i32, i32),
        (serialize_i64, i64),
        (serialize_i128, i128),
        (serialize_f32, f32),
        (serialize_f64, f64)
    );

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        self.set_nested()?;
        self.assert_len(len)?;

        Ok(self)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_some<T>(self, value: &T) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.nested {
            Err(Error::TypeNotSupported("nested option"))
        } else {
            value.serialize(self)
        }
    }

    fn serialize_none(self) -> std::result::Result<Self::Ok, Self::Error> {
        if self.nested {
            Err(Error::TypeNotSupported("nested option"))
        } else {
            Ok(())
        }
    }

    fn serialize_seq(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        // sequence can only be serialized into wildcard capture
        self.assert_wildcard()?;
        // set nested for element checks
        self.nested = true;

        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        self.set_nested()?;
        self.assert_len(Some(len))?;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        self.set_nested()?;
        self.assert_len(Some(len))?;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_unit(self) -> std::result::Result<Self::Ok, Self::Error> {
        Err(Error::TypeNotSupported("()"))
    }

    fn serialize_unit_struct(
        self,
        _name: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        Err(Error::TypeNotSupported("unit structs"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_str(&variant.to_lowercase())
    }
}

impl<'m, 's, 't> SerializeSeq for &'m mut PathSerializer<'s, 't> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'m, 's, 't> SerializeMap for &'m mut PathSerializer<'s, 't> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.key_mode = true;
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'m, 's, 't> SerializeStruct for &'m mut PathSerializer<'s, 't> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.set_next_named_capture(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'m, 's, 't> SerializeTuple for &'m mut PathSerializer<'s, 't> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)?;
        self.set_next_tuple_capture()?;
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'m, 's, 't> SerializeTupleStruct for &'m mut PathSerializer<'s, 't> {
    type Ok = <Self as SerializeTuple>::Ok;
    type Error = <Self as SerializeTuple>::Error;

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        SerializeTuple::end(self)
    }
}

/// Type of errors, returned by [`PathSerializer`]
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Some values, like byte slices and any enum variant, other than
    /// unit variants are not supported
    TypeNotSupported(&'static str),
    /// Custom error variant in accordance with serde guidelines.
    Custom(String),
    /// Serialized struct, tuple or map had an element count not matching with
    /// capture count in template.
    InvalidLen,
    /// Sequence was passed as non-wildcard capture value.
    NonWildcardCapture,
    /// Serialied value was a nested struct,
    /// tuple with struct element, vec of structs/tuples, etc.
    DeepNesting,
    /// When finalizing, serialized value didn't contain a value for a certain capture.
    MissingCapture(String),
    /// When serializing, an uknown capture ident was present in serialized value
    UknownCapture(String),
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeNotSupported(ty) => write!(f, "type `{ty}` is not supported"),
            Self::Custom(msg) => write!(f, "{msg}"),
            Self::InvalidLen => write!(
                f,
                "number of serialized elements doesn't match path template",
            ),
            Self::DeepNesting => write!(
                f,
                "only plain values or serde flattened structs can be serialized to url path"
            ),
            Self::MissingCapture(name) => write!(f, "missing required capture member: {name}"),
            Self::NonWildcardCapture => {
                write!(f, "trying to write invalid type into wildcard capture")
            }
            Self::UknownCapture(id) => write!(f, "unknown capture ident: {id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::PathSerializer;
    use crate::path_template::PathTemplate;
    use serde::Serialize;

    #[derive(Debug, Clone, Copy, Serialize)]
    struct SingleField<T> {
        val: T,
    }

    impl<T> SingleField<T> {
        fn new(val: T) -> Self {
            Self { val }
        }
    }

    #[derive(Debug, Clone, Copy, Serialize)]
    struct SingleElem<T>(T);

    #[derive(Debug, Clone, Copy, Serialize)]
    enum UnitVariant {
        A,
        B,
        C,
    }

    #[derive(Debug, Clone, Copy, Serialize)]
    struct MultiFields<A, B, C> {
        a: A,
        b: B,
        c: C,
    }

    impl<A, B, C> MultiFields<A, B, C> {
        fn new(a: A, b: B, c: C) -> Self {
            Self { a, b, c }
        }
    }

    macro_rules! serialize {
        ($template:expr, $val:expr) => {{
            let template = PathTemplate::new($template).unwrap();
            let mut serializer = PathSerializer::new(&template);

            Serialize::serialize($val, &mut serializer).and_then(|_| serializer.finalize())
        }};
    }

    #[test]
    fn single_bool() {
        let result = serialize!("/{a}", &true).unwrap();
        assert_eq!(result, "/true");
    }

    #[test]
    fn single_char() {
        let result = serialize!("/{a}", &'a').unwrap();
        assert_eq!(result, "/a")
    }

    #[test]
    fn single_uint() {
        assert_eq!(serialize!("/{a}", &0u8), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0u16), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0u32), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0u64), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0u128), Ok("/0".to_owned()));
    }

    #[test]
    fn single_int() {
        assert_eq!(serialize!("/{a}", &0i8), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0i16), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0i32), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0i64), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0i128), Ok("/0".to_owned()));
    }

    #[test]
    fn single_float() {
        assert_eq!(serialize!("/{a}", &0f32), Ok("/0".to_owned()));
        assert_eq!(serialize!("/{a}", &0f64), Ok("/0".to_owned()));

        assert_eq!(serialize!("/{a}", &1.5f32), Ok("/1%2E5".to_owned()));
        assert_eq!(serialize!("/{a}", &1.5f64), Ok("/1%2E5".to_owned()));
    }

    #[test]
    fn single_vec() {
        assert_eq!(serialize!("/{*a}", &vec![1]), Ok("/1".to_owned()));
        assert_eq!(
            serialize!("/{*a}", &vec!["a", "b", "c"]),
            Ok("/a/b/c".to_owned())
        );
        assert_eq!(
            serialize!("/{*a}", &vec![true, false]),
            Ok("/true/false".to_owned())
        );
    }

    #[test]
    fn option() {}

    #[test]
    fn map() {
        assert_eq!(
            serialize!("/{val}", &HashMap::from([("val", "true")])),
            Ok("/true".to_owned())
        );
        assert_eq!(
            serialize!(
                "/{a}/{b}/{c}",
                &HashMap::from([("a", "a"), ("b", "b"), ("c", "c")])
            ),
            Ok("/a/b/c".to_owned())
        );
    }

    #[test]
    fn single_field() {
        assert_eq!(
            serialize!("/{val}", &SingleField::new(true)),
            Ok("/true".to_owned())
        );
        assert_eq!(
            serialize!("/{val}", &SingleField::new(1)),
            Ok("/1".to_owned())
        );
        assert_eq!(
            serialize!("/{val}", &SingleField::new('a')),
            Ok("/a".to_owned())
        );
        assert_eq!(
            serialize!("/{val}", &SingleField::new(1.1)),
            Ok("/1%2E1".to_owned())
        );
    }

    #[test]
    fn single_elem() {
        assert_eq!(
            serialize!("/{val}", &SingleElem(true)),
            Ok("/true".to_owned())
        );
        assert_eq!(serialize!("/{val}", &SingleElem(1)), Ok("/1".to_owned()));
        assert_eq!(serialize!("/{val}", &SingleElem('a')), Ok("/a".to_owned()));
        assert_eq!(
            serialize!("/{val}", &SingleElem(1.1)),
            Ok("/1%2E1".to_owned())
        );
    }

    #[test]
    fn unit_variant() {
        assert_eq!(serialize!("/{val}", &UnitVariant::A), Ok("/a".to_owned()));
        assert_eq!(serialize!("/{val}", &UnitVariant::B), Ok("/b".to_owned()));
        assert_eq!(serialize!("/{val}", &UnitVariant::C), Ok("/c".to_owned()));
    }

    #[test]
    fn multiple_fieds() {
        assert_eq!(
            serialize!("/{a}/{b}/{c}", &MultiFields::new(true, false, true)),
            Ok("/true/false/true".to_owned())
        );
        assert_eq!(
            serialize!("/{a}/{b}/{c}", &MultiFields::new(true, "aaaa", 1)),
            Ok("/true/aaaa/1".to_owned())
        );
        assert_eq!(
            serialize!(
                "/{a}/{b}/{*c}",
                &MultiFields::<_, _, Vec<String>>::new('c', UnitVariant::A, vec![])
            ),
            Ok("/c/a".to_owned())
        );
        assert_eq!(
            serialize!(
                "/{a}/{b}/{*c}",
                &MultiFields::new("wild", "card", vec!["test", "successfull"])
            ),
            Ok("/wild/card/test/successfull".to_owned())
        );
    }

    #[test]
    fn tuple_single() {
        let result = serialize!("/{a}", &("aaa",)).unwrap();
        assert_eq!(result, "/aaa");
    }
}
