#[derive(Debug, Clone, PartialEq, Eq, deluxe::ParseMetaItem)]
pub enum ContentType {
    #[deluxe(rename = application_json)]
    ApplicationJson,
    #[deluxe(rename = text_plain)]
    TextPlain,
}

impl std::str::FromStr for ContentType {
    type Err = ContentTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "application/json" => Ok(Self::ApplicationJson),
            "text/plain" => Ok(Self::TextPlain),
            _ => Err(ContentTypeError),
        }
    }
}

impl Default for ContentType {
    fn default() -> Self {
        Self::TextPlain
    }
}

pub struct ContentTypeError;

impl std::fmt::Display for ContentTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Unknown/unsupported content type.")
    }
}
