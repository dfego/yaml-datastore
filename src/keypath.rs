//! Parsing for keypaths, which are of the form:
//!
//! a.b.c.d
//!
//! The only things precluded from this design are file components or
//! YAML keys with dots in them, and that the key may not have empty components,
//! i.e. two dots in a row.
//!
//! Might make sense to disallow slashes, too?
//!
//! For each component, the following are tried, in this order, until one is true:
//!
//! TODO move all this to the main file where it's relevant.
//!
//! 1. Is there a directory with this name at the current level from the datastore root?
//! 2. Is there file with this name and a .yaml extension at this level?
//! 3. Is there a file with this name and a .yml extension at this level?
//! 4. If we've matched a file (2 or 3 above), is there a key at the current level?
//!
//! If at any point these all fail, data parsing will fail.
use std::fmt::Display;
use thiserror::Error;

/// Delimiter on which components of a keypath are split.
const KEYPATH_DELIMITER: &str = ".";

/// Characters that are disallowed in a keypath and will cause failure.
const INVALID_CHARACTERS: &[char] = &['.', '/'];

/// Error type for keypaths.
///
/// Only one error at this time, and that is for parsing failure.
#[derive(Error, Debug)]
pub enum KeyPathParseError {
    /// keypath string is invalid
    #[error("keypath contains slashes or empty components")]
    InvalidKeyPath,
}

/// Internal struct for parsing and managing keypath components.
///
/// The only way to construct is [`try_from`].
#[derive(Debug)]
pub(crate) struct KeyPath {
    /// Raw string that components point to.
    raw: String,
}

/// Check a single keypath component for validity and return a String if it's valid.
fn validate_and_trim(component: &str) -> Result<&str, KeyPathParseError> {
    if component.is_empty() || component.contains(INVALID_CHARACTERS) {
        Err(KeyPathParseError::InvalidKeyPath)
    } else {
        Ok(component.trim())
    }
}

impl TryFrom<&str> for KeyPath {
    type Error = KeyPathParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Split up value, validate and trim it, then put it back together.
        Ok(Self {
            raw: value
                .split(KEYPATH_DELIMITER)
                .map(validate_and_trim)
                .collect::<Result<Vec<_>, _>>()?
                .join(KEYPATH_DELIMITER),
        })
    }
}

impl Display for KeyPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl KeyPath {
    pub fn components(&self) -> Vec<&str> {
        self.raw.split(KEYPATH_DELIMITER).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid() {
        let input = "this.is.a.valid.keypath";
        let result = KeyPath::try_from(input).unwrap();
        let expected = vec!["this", "is", "a", "valid", "keypath"];
        assert_eq!(result.components(), expected);
        assert_eq!(result.to_string(), input);
    }

    #[test]
    fn valid_with_spaces() {
        let input = " this . is . a . valid . keypath ";
        let result = KeyPath::try_from(input).unwrap();
        let expected = vec!["this", "is", "a", "valid", "keypath"];
        assert_eq!(result.components(), expected);
        assert_eq!(result.to_string(), "this.is.a.valid.keypath");
    }

    #[test]
    fn err_contains_slash() {
        let input = "contains/slash";
        let result = KeyPath::try_from(input).unwrap_err();
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }

    #[test]
    fn err_empty_component_middle() {
        let input = "has..component";
        let result = KeyPath::try_from(input).unwrap_err();
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }

    #[test]
    fn err_empty_component_first() {
        let input = ".has.component";
        let result = KeyPath::try_from(input).unwrap_err();
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }

    #[test]
    fn err_empty_component_last() {
        let input = "has.component.";
        let result = KeyPath::try_from(input).unwrap_err();
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }
}
