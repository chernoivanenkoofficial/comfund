use std::fmt::Display;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub struct PathTemplate<'s> {
    segments: Segments<'s>,
    idents: Idents<'s>,
    wildcard: Option<&'s str>,
}

impl PathTemplate<'static> {
    /// Generate path template from static raw data.
    pub const fn new_static(
        segments: &'static [Segment<'static>],
        idents: &'static [&'static str],
        wildcard: Option<&'static str>,
    ) -> Self {
        Self {
            segments: Segments::Static(segments),
            idents: Idents::Static(idents),
            wildcard,
        }
    }

    pub fn leak(self) -> Self {
        let PathTemplate {
            segments,
            idents,
            wildcard,
        } = self;

        Self {
            segments: segments.leak(),
            idents: idents.leak(),
            wildcard,
        }
    }
}

impl<'s> PathTemplate<'s> {
    /// Parse dynamic path expression, normalizing it in the process.
    pub fn new(expr: &'s str) -> Result<Self> {
        let expr = expr.trim_end_matches('/');
        if expr.is_empty() {
            return Ok(Self {
                segments: vec![].into(),
                idents: vec![].into(),
                wildcard: None,
            });
        }

        let (expr, wildcard) = trim_wildcard(expr)?;
        let mut segments = vec![];
        let mut idents = vec![];

        for seg in expr.split('/') {
            if seg.is_empty() {
                continue;
            }

            let capture = get_capture(seg)?;

            if let Some(ident) = capture {
                if ident.starts_with('*') {
                    return Err(Error::InvalidWildcard);
                } else {
                    let ident = assert_ident(ident)?;
                    segments.push(Segment::Capture(ident));
                    idents.push(ident);
                }
            } else {
                let seg = assert_url_segment(seg)?;
                segments.push(Segment::Static(seg));
            }
        }

        Ok(Self {
            segments: segments.into(),
            idents: idents.into(),
            wildcard,
        })
    }

    /// Get slash-separated segments of parsed URL template.
    pub fn segments(&self) -> &[Segment<'s>] {
        &self.segments
    }

    /// Get idents of capture variables.
    pub fn idents(&self) -> &[&'s str] {
        &self.idents
    }

    /// Get ident of wildcard capture (if present).
    pub fn wildcard(&self) -> Option<&'s str> {
        self.wildcard
    }

    /// Get count of captures in this template (including wildcard capture)
    pub fn param_count(&self) -> usize {
        self.idents.len() + if self.wildcard.is_some() { 1 } else { 0 }
    }

    /// Check, if template contains dynamic captures
    pub fn is_blank(&self) -> bool {
        self.segments.is_empty() && self.wildcard.is_none()
    }

    /// Generate a valid path template to use in [axum](https://docs.rs/axum/latest/axum/).
    pub fn generate_axum_template(&self) -> String {
        let mut output = String::new();

        for seg in self.segments.iter() {
            output.push('/');
            match seg {
                Segment::Static(seg) => output.push_str(seg),
                Segment::Capture(ident) => {
                    output.push('{');
                    output.push_str(ident);
                    output.push('}');
                }
            }
        }

        if let Some(ident) = self.wildcard {
            output.push('/');
            output.push_str("{*");
            output.push_str(ident);
            output.push('}');
        }

        output
    }

    /// Generate a valid path template to use in [actix-web](https://docs.rs/actix-web/latest/actix_web/).
    pub fn generate_actix_web_template(&self) -> String {
        let mut output = String::new();

        for seg in self.segments.iter() {
            output.push('/');
            match seg {
                Segment::Static(seg) => output.push_str(seg),
                Segment::Capture(ident) => {
                    output.push('{');
                    output.push_str(ident);
                    output.push('}');
                }
            }
        }

        if let Some(ident) = self.wildcard {
            output.push('/');
            output.push('{');
            output.push_str(ident);
            output.push_str(":.*}");
        }

        output
    }
}

/// A segment of dynamic path template.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment<'s> {
    /// A static segment, that shouldn't be substituted for an actual value
    /// (contains valid, percent-encoded value for segment).
    Static(&'s str),
    /// A dynamic segment, that should be substituted for a value
    /// (contains a name of capture variable, that is a valid Rust ident).
    Capture(&'s str),
}

/// An error type for parsing dynamic URL paths.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    UnclosedCapture,
    /// Wildcard captures are only accepted at the end of dynamic path
    InvalidWildcard,
    /// Capture valriable wasn't a valid Rust ident.
    InvalidIdent,
    /// Static segment contained invalid URL path character.
    InvalidPathChar,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnclosedCapture => write!(f, "unclosed capture"),
            Self::InvalidWildcard => {
                write!(f, "wildcard can only be the last capture in path template")
            }
            Self::InvalidIdent => write!(f, "capture ident should be a valid Rust ident"),
            Self::InvalidPathChar => write!(
                f,
                "static segments of template should be valid url path substrings"
            ),
        }
    }
}

fn is_valid_ident(segment: &str) -> bool {
    segment.starts_with(|ch| char::is_alphabetic(ch) || ch == '_')
        && segment.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
}

fn assert_ident(seg: &str) -> Result<&str> {
    if is_valid_ident(seg) {
        Ok(seg)
    } else {
        Err(Error::InvalidIdent)
    }
}

fn get_wildcard(seg: &str) -> Result<Option<&str>> {
    let capture = get_capture(seg)?;

    if let Some(capture) = capture {
        if let Some(ident) = capture.strip_prefix('*') {
            assert_ident(ident)?;
            Ok(Some(ident))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn trim_wildcard(expr: &str) -> Result<(&str, Option<&str>)> {
    let last_segment = expr.rsplit('/').next().unwrap();
    let wildcard = get_wildcard(last_segment)?;

    let expr = if wildcard.is_some() {
        expr.trim_end_matches(last_segment)
    } else {
        expr
    };

    Ok((expr, wildcard))
}

fn get_capture(seg: &str) -> Result<Option<&str>> {
    let capture_start = seg.starts_with('{');
    let capture_end = seg.ends_with('}');

    if capture_start ^ capture_end {
        return Err(Error::UnclosedCapture);
    }

    if capture_start & capture_end {
        let seg = seg.strip_prefix('{').unwrap().strip_suffix('}').unwrap();

        Ok(Some(seg))
    } else {
        Ok(None)
    }
}

fn is_valid_url_path_char(ch: char) -> bool {
    matches!(ch,
        'A'..='Z'
        | 'a'..='z'
        | '0'..='9'
        | '-'
        | '.'
        | '_'
        | '~'
        | '!'
        | '$'
        | '&'
        | '\''
        | '('
        | ')'
        | '*'
        | '+'
        | ','
        | ';'
        | '='
        | ':'
        | '@'
    )
}

fn assert_url_segment(seg: &str) -> Result<&str> {
    if seg.chars().all(is_valid_url_path_char) {
        Ok(seg)
    } else {
        Err(Error::InvalidPathChar)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Segments<'s> {
    Owned(Vec<Segment<'s>>),
    Static(&'static [Segment<'static>]),
}

impl Segments<'static> {
    pub fn leak(self) -> Self {
        match self {
            Self::Owned(owned) => Self::Static(owned.leak()),
            Self::Static(st) => Self::Static(st),
        }
    }
}

impl<'s> std::ops::Deref for Segments<'s> {
    type Target = [Segment<'s>];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(vec) => vec,
            Self::Static(slice) => slice,
        }
    }
}

impl<'s> From<Vec<Segment<'s>>> for Segments<'s> {
    fn from(value: Vec<Segment<'s>>) -> Self {
        Self::Owned(value)
    }
}

impl From<&'static [Segment<'static>]> for Segments<'static> {
    fn from(value: &'static [Segment<'static>]) -> Self {
        Segments::Static(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Idents<'s> {
    Owned(Vec<&'s str>),
    Static(&'static [&'static str]),
}

impl Idents<'static> {
    pub fn leak(self) -> Self {
        match self {
            Self::Owned(owned) => Self::Static(owned.leak()),
            Self::Static(st) => Self::Static(st),
        }
    }
}

impl<'s> std::ops::Deref for Idents<'s> {
    type Target = [&'s str];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(vec) => vec,
            Self::Static(slice) => slice,
        }
    }
}

impl<'s> From<Vec<&'s str>> for Idents<'s> {
    fn from(value: Vec<&'s str>) -> Self {
        Self::Owned(value)
    }
}

impl From<&'static [&'static str]> for Idents<'static> {
    fn from(value: &'static [&'static str]) -> Self {
        Self::Static(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::path_template::Error;

    use super::PathTemplate;
    use super::Segment::*;

    #[test]
    fn test_empty() {
        let parsed = PathTemplate::new("/");
        let template = PathTemplate {
            idents: vec![].into(),
            segments: vec![].into(),
            wildcard: None,
        };

        assert_eq!(Ok(template), parsed);
    }

    #[test]
    fn test_static_only() {
        let parsed = PathTemplate::new("/a/b/c");
        let template = PathTemplate {
            idents: vec![].into(),
            segments: vec![Static("a"), Static("b"), Static("c")].into(),
            wildcard: None,
        };

        assert_eq!(Ok(template), parsed);
    }

    #[test]
    fn test_captures_only() {
        let parsed = PathTemplate::new("/{a}/{b}/{c}");
        let template = PathTemplate {
            idents: vec!["a", "b", "c"].into(),
            segments: vec![Capture("a"), Capture("b"), Capture("c")].into(),
            wildcard: None,
        };

        assert_eq!(Ok(template), parsed);
    }

    #[test]
    fn test_wildcard_only() {
        let parsed = PathTemplate::new("/{*a}");
        let template = PathTemplate {
            idents: vec![].into(),
            segments: vec![].into(),
            wildcard: Some("a"),
        };

        assert_eq!(Ok(template), parsed);
    }

    #[test]
    fn test_normal() {
        let parsed = PathTemplate::new("/a/{b}/c/{d}/{*f}");
        let template = PathTemplate {
            idents: vec!["b", "d"].into(),
            segments: vec![Static("a"), Capture("b"), Static("c"), Capture("d")].into(),
            wildcard: Some("f"),
        };

        assert_eq!(Ok(template), parsed);
    }

    #[test]
    fn test_no_leading_slash() {
        let expr = "a/b/c/d";
        assert!(PathTemplate::new(expr).is_ok())
    }

    #[test]
    fn test_repeated_slashes() {
        let parsed = PathTemplate::new("//a//b////c//d");
        let template = PathTemplate {
            idents: vec![].into(),
            segments: vec![Static("a"), Static("b"), Static("c"), Static("d")].into(),
            wildcard: None,
        };
        assert_eq!(Ok(template), parsed);
    }

    #[test]
    fn test_unclosed_capture() {
        let parsed = PathTemplate::new("/{a/b/c");
        let parsed2 = PathTemplate::new("/a/b}/c/d");

        let error = Err(Error::UnclosedCapture);

        assert_eq!(parsed, error);
        assert_eq!(parsed2, error);
    }

    #[test]
    fn test_invalid_ident() {
        let parsed = PathTemplate::new("/a/{b-s}/c/d");
        let parsed2 = PathTemplate::new("/a/{b?s}/c/d");
        let parsed3 = PathTemplate::new("/a/{b.s}/c/d");
        let parsed4 = PathTemplate::new("/a/{11b}/c/d");

        let error: Result<PathTemplate<'_>, Error> = Err(Error::InvalidIdent);

        assert_eq!(parsed, error.clone());
        assert_eq!(parsed2, error.clone());
        assert_eq!(parsed3, error.clone());
        assert_eq!(parsed4, error);
    }

    #[test]
    fn test_invalid_wildcard() {
        let parsed = PathTemplate::new("/a/{*bs}/c/");
        let error = Err(Error::InvalidWildcard);

        assert_eq!(parsed, error);
    }
}
