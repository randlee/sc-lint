use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RustIdent(String);

impl RustIdent {
    pub(crate) fn parse(value: &str) -> Result<Self, SemanticPathError> {
        if value.trim() != value || value.is_empty() {
            return Err(SemanticPathError::InvalidIdentifier(value.to_owned()));
        }
        syn::parse_str::<syn::Ident>(value)
            .map(|_| Self(value.to_owned()))
            .map_err(|_| SemanticPathError::InvalidIdentifier(value.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PublicRustPath(Vec<RustIdent>);

impl PublicRustPath {
    pub(crate) fn parse(value: &str) -> Result<Self, SemanticPathError> {
        let segments = value
            .split("::")
            .map(RustIdent::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if segments.is_empty() {
            return Err(SemanticPathError::InvalidPublicPath(value.to_owned()));
        }
        Ok(Self(segments))
    }

    pub(crate) fn as_str(&self) -> String {
        self.0
            .iter()
            .map(RustIdent::as_str)
            .collect::<Vec<_>>()
            .join("::")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticPathError {
    InvalidIdentifier(String),
    InvalidPublicPath(String),
}

impl fmt::Display for SemanticPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(value) => {
                write!(formatter, "invalid Rust identifier `{value}`")
            }
            Self::InvalidPublicPath(value) => {
                write!(formatter, "invalid public Rust path `{value}`")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_raw_identifiers_and_public_paths() {
        assert_eq!(RustIdent::parse("r#type").unwrap().as_str(), "r#type");
        assert_eq!(
            PublicRustPath::parse("api::r#type").unwrap().as_str(),
            "api::r#type"
        );
    }

    #[test]
    fn rejects_invalid_path_segments() {
        assert!(PublicRustPath::parse("api::").is_err());
        assert!(RustIdent::parse("not-valid").is_err());
    }
}
