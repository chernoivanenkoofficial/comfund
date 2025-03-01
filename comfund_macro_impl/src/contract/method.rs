#[derive(Debug, Clone, Copy, deluxe::ParseMetaItem)]
pub enum Method {
    #[deluxe(rename = get)]
    Get,
    #[deluxe(rename = post)]
    Post,
    #[deluxe(rename = patch)]
    Patch,
    #[deluxe(rename = delete)]
    Delete,
    #[deluxe(rename = put)]
    Put,
}

impl std::str::FromStr for Method {
    type Err = ParseMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "get" | "GET" => Ok(Self::Get),
            "post" | "POST" => Ok(Self::Post),
            "patch" | "PATCH" => Ok(Self::Patch),
            "delete" | "DELETE" => Ok(Self::Delete),
            "put" | "PUT" => Ok(Self::Put),
            _ => Err(ParseMethodError),
        }
    }
}

pub struct ParseMethodError;

impl std::fmt::Display for ParseMethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Uknown/unsupported HTTP method")
    }
}
