//! Catalog of stateless validator guests.

#![no_std]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, FromRepr, IntoEnumIterator, IntoStaticStr};

include!(concat!(env!("OUT_DIR"), "/version_impl.rs"));

/// Stateless validator kind.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    EnumIter,
    EnumString,
    IntoStaticStr,
    Display,
    FromRepr,
)]
#[repr(u8)]
#[serde(into = "String", try_from = "String")]
#[strum(
    ascii_case_insensitive,
    serialize_all = "lowercase",
    parse_err_fn = ParseError::from,
    parse_err_ty = ParseError
)]
pub enum StatelessValidatorKind {
    // TODO(ethrex-release): Restore `Ethrex = 0` after PR #7216 publishes compatible artifacts.
    /// Reth stateless validator.
    Reth = 1,
    // TODO(zesu-devnet-8): Restore `Zesu = 2` after a compatible release is published.
}

impl StatelessValidatorKind {
    /// Returns an iterator over all kinds.
    pub fn iter() -> impl Iterator<Item = Self> {
        <Self as IntoEnumIterator>::iter()
    }

    /// Returns the kind with the given `u8` representation, if any.
    pub fn from_u8(repr: u8) -> Option<Self> {
        Self::from_repr(repr)
    }

    /// Returns the `u8` representation.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Returns the `u32` representation.
    pub fn as_u32(&self) -> u32 {
        self.as_u8() as u32
    }

    /// Returns the lowercase name.
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    /// Returns the lowercase name.
    pub fn name(&self) -> &'static str {
        self.as_str()
    }
}

impl From<StatelessValidatorKind> for String {
    fn from(value: StatelessValidatorKind) -> Self {
        value.as_str().to_string()
    }
}

impl TryFrom<String> for StatelessValidatorKind {
    type Error = ParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// Error returned when parsing an unsupported stateless validator kind.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParseError(String);

impl From<&str> for ParseError {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let unsupported = &self.0;
        let supported =
            Vec::from_iter(StatelessValidatorKind::iter().map(|k| k.as_str())).join(", ");
        write!(
            f,
            "Unsupported stateless validator kind `{unsupported}`, expect one of [{supported}]",
        )
    }
}

impl Error for ParseError {}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use crate::{ParseError, StatelessValidatorKind};

    #[test]
    fn parse_stateless_validator_kind() {
        // Valid
        let spellings = ["reth", "Reth"];
        spellings
            .iter()
            .for_each(|s| assert_eq!(s.parse(), Ok(StatelessValidatorKind::Reth)));
        assert_eq!(StatelessValidatorKind::Reth.as_str(), spellings[0]);

        for unsupported in ["ethrex", "zesu"] {
            assert_eq!(
                unsupported.parse::<StatelessValidatorKind>(),
                Err(ParseError::from(unsupported))
            );
        }

        // Invalid
        assert_eq!(
            "xxx".parse::<StatelessValidatorKind>(),
            Err(ParseError::from("xxx"))
        );
        assert_eq!(
            ParseError::from("xxx").to_string(),
            "Unsupported stateless validator kind `xxx`, expect one of [reth]".to_string()
        );
    }

    #[test]
    fn preserve_reserved_numeric_ids() {
        assert_eq!(StatelessValidatorKind::Reth.as_u8(), 1);
        assert_eq!(
            StatelessValidatorKind::from_u8(1),
            Some(StatelessValidatorKind::Reth)
        );
        assert_eq!(StatelessValidatorKind::from_u8(0), None);
        assert_eq!(StatelessValidatorKind::from_u8(2), None);
    }
}
