//! Dynamic path serializer and parser, used by `comfund` crate

pub mod path_serializer;
pub mod path_template;

pub use path_serializer::{PathSerializer, Result};
#[cfg(feature = "serde")]
pub use path_template::{PathTemplate, Segment};

/// Serialize structure into dynamic path template.
///
/// ## Returns
///
/// Valid, percent encoded url string with dynamic segments of template substitued
/// for struct fields
#[cfg(feature = "serde")]
pub fn serialize<'s, T: serde::Serialize>(template: &PathTemplate<'s>, args: &T) -> Result<String> {
    let mut serializer = PathSerializer::new(template);
    serde::Serialize::serialize(args, &mut serializer)?;
    serializer.finalize()
}
