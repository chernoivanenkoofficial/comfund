use std::{borrow::Borrow, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, deluxe::ParseMetaItem)]
pub enum Transport {
    #[deluxe(rename = path)]
    Path,
    #[deluxe(rename = query)]
    Query,
    #[deluxe(rename = body)]
    Body,
    #[deluxe(rename = json)]
    Json,
    #[deluxe(rename = multipart)]
    Multipart,
}

impl FromStr for Transport {
    type Err = ParseTransportError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "path" => Ok(Self::Path),
            "query" => Ok(Self::Query),
            "body" => Ok(Self::Body),
            "json" => Ok(Self::Json),
            "multipart" => Ok(Self::Multipart),
            _ => Err(ParseTransportError),
        }
    }
}

pub struct ParseTransportError;

impl std::fmt::Display for ParseTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Unknown/unsupported transport type.")
    }
}
