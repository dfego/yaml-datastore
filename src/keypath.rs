//! Parsing for keypaths, which are flexible keys for navigating a datastore.
//!
//! Keypath components may represent either path components or mapping keys within a YAML file.
//!
//! # Format
//! Keypaths are of the form `a.b.c.d`, where `a`, `b`, `c`, and `d` are keys, and `.` are delimiters.
//! Forward-slash characters (i.e. `/`) must not be contained in a path, and no components may be empty.
//! This means in effect that the [delimiter](DELIMITER) (`.`) may not appear twice in a row or appear at the beginning or end of a keypath.
//!
//! ## Spaces
//! If a keypath contains spaces at the beginning or end of a component, those spaces will be stripped.
//! For example, ` a . b . c . d ` will be parsed as `a.b.c.d.`.
//!
//! ## Invalid Examples
//! The following are some examples of invalid keypaths:
//!
//! * `contains/slash`
//! * `empty.component..in.middle`
//! * `whitespace.component. .in.middle`
//! * `.empty.component.at.beginning`
//! * `empty.component.at.end.`
//!
//! # Usage
//! Keypaths provide [iterators](`KeyPath::iter`) that can be used to iterate over all possible interpretations of a keypath.
//!
//! ```rust
//! use yaml_datastore::keypath::KeyPath;
//!
//! let keypath = KeyPath::try_from("a.b.c").expect("keypath parsed");
//! for (path, keys) in keypath.iter() {
//!     println!("{:10} | {:?}", path.display(), keys);
//! }
//! ```
//!
//! The above should print out:
//!
//! ```text
//! a.yml       | ["b", "c"]
//! a.yaml      | ["b", "c"]
//! a/b.yml     | ["c"]
//! a/b.yaml    | ["c"]
//! a/b/c.yml   | []
//! a/b/c.yaml  | []
//! ```
//!
//! This iterator can then be used to search the keystore with the given precedence: directories > files > keys.
//!
//! If the inverse behavior is desired, the iterator can be reversed with [`rev()`](std::iter::Iterator::rev):
//!
//! ```rust
//! use yaml_datastore::keypath::KeyPath;
//!
//! let keypath = KeyPath::try_from("a.b.c").expect("keypath parsed");
//! for (path, keys) in keypath.iter().rev() {
//!     println!("{:10} | {:?}", path.display(), keys);
//! }
//! ```
//!
//! The above should print out:
//!
//! ```text
//! a/b/c.yaml | []
//! a/b/c.yml  | []
//! a/b.yaml   | ["c"]
//! a/b.yml    | ["c"]
//! a.yaml     | ["b", "c"]
//! a.yml      | ["b", "c"]
//! ```
use std::{ffi::OsStr, path::PathBuf};
use thiserror::Error;

/// Default file extensions for the iterator.
pub static DEFAULT_EXTENSIONS: &[&str] = &["yaml", "yml"];

/// Delimiter on which components of a keypath are split.
const DELIMITER: &str = ".";

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
/// Construct using [`try_from`](KeyPath::try_from).
#[derive(Debug)]
pub struct KeyPath {
    /// Raw string that components point to.
    raw: String,
}

/// Check a single keypath component for validity and return a String if it's valid.
fn validate_and_trim(component: &str) -> Result<&str, KeyPathParseError> {
    let component = component.trim();
    if component.is_empty() || component.contains(INVALID_CHARACTERS) {
        Err(KeyPathParseError::InvalidKeyPath)
    } else {
        Ok(component.trim())
    }
}

impl TryFrom<&str> for KeyPath {
    type Error = KeyPathParseError;

    /// Construct a [`KeyPath`] from a string.
    ///
    /// A valid `KeyPath` is a string with some components separated by `.`.
    /// See the [module-level documentation](crate::keypath) for details.
    ///
    /// # Errors
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Split up value, validate and trim it, then put it back together.
        Ok(Self {
            raw: value
                .split(DELIMITER)
                .map(validate_and_trim)
                .collect::<Result<Vec<_>, _>>()?
                .join(DELIMITER),
        })
    }
}

impl std::fmt::Display for KeyPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl KeyPath {
    /// Return an iterator over the keypath components using the [default list of extensions][DEFAULT_EXTENSIONS].
    ///
    ///
    #[must_use]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (PathBuf, Vec<&str>)> {
        self.iter_extensions(DEFAULT_EXTENSIONS)
    }

    /// Return an iterator
    pub fn iter_extensions<S: AsRef<OsStr>>(
        &self,
        extensions: &[S],
    ) -> impl DoubleEndedIterator<Item = (PathBuf, Vec<&str>)> {
        let paths = self.components();
        let keys = self.components();

        // This is intentional. We want an ExactSizeIterator so we can freely use
        // rev() on the returned iterator, but we can't if we use a RangeInclusve.
        #[allow(clippy::range_plus_one)]
        let range = (1..paths.len() + 1).rev();
        std::iter::zip(
            range.clone().map(move |i| paths[0..i].iter().collect()),
            range.clone().map(move |i| keys[i..].to_vec()),
        )
        .flat_map(|pair: (PathBuf, _)| {
            extensions
                .iter()
                .map(move |e| (pair.0.with_extension(e), pair.1.clone()))
        })
    }

    /// Return the parsed components as a list of strings.
    ///
    /// While not intended for use externally at this point, it could be useful for introspection.
    #[must_use]
    pub fn components(&self) -> Vec<&str> {
        self.raw.split(DELIMITER).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid() {
        let input = "this.is.a.valid.keypath";
        let result = KeyPath::try_from(input).expect("key parsed");
        let expected = vec!["this", "is", "a", "valid", "keypath"];
        assert_eq!(result.components(), expected);
        assert_eq!(result.to_string(), input);
    }

    #[test]
    fn iterator() {
        let input = "this.is.a.keypath";
        let result = KeyPath::try_from(input).expect("key parsed");
        let zipped: Vec<_> = result.iter().collect();
        let expected = vec![
            (PathBuf::from("this/is/a/keypath.yaml"), vec![]),
            (PathBuf::from("this/is/a/keypath.yml"), vec![]),
            (PathBuf::from("this/is/a.yaml"), vec!["keypath"]),
            (PathBuf::from("this/is/a.yml"), vec!["keypath"]),
            (PathBuf::from("this/is.yaml"), vec!["a", "keypath"]),
            (PathBuf::from("this/is.yml"), vec!["a", "keypath"]),
            (PathBuf::from("this.yaml"), vec!["is", "a", "keypath"]),
            (PathBuf::from("this.yml"), vec!["is", "a", "keypath"]),
        ];
        assert_eq!(zipped, expected);
    }

    #[test]
    fn iterator_reversed() {
        let input = "this.is.a.keypath";
        let result = KeyPath::try_from(input).expect("key parsed");
        let zipped: Vec<_> = result.iter().rev().collect();
        let mut expected = vec![
            (PathBuf::from("this/is/a/keypath.yaml"), vec![]),
            (PathBuf::from("this/is/a/keypath.yml"), vec![]),
            (PathBuf::from("this/is/a.yaml"), vec!["keypath"]),
            (PathBuf::from("this/is/a.yml"), vec!["keypath"]),
            (PathBuf::from("this/is.yaml"), vec!["a", "keypath"]),
            (PathBuf::from("this/is.yml"), vec!["a", "keypath"]),
            (PathBuf::from("this.yaml"), vec!["is", "a", "keypath"]),
            (PathBuf::from("this.yml"), vec!["is", "a", "keypath"]),
        ];
        expected.reverse();
        assert_eq!(zipped, expected);
    }

    #[test]
    fn iterator_with_explicit_extensions() {
        let input = "this.is.a.keypath";
        let result = KeyPath::try_from(input).expect("key parsed");
        let extensions = vec!["json", "xml"];
        let zipped: Vec<_> = result.iter_extensions(&extensions).collect();
        let expected = vec![
            (PathBuf::from("this/is/a/keypath.json"), vec![]),
            (PathBuf::from("this/is/a/keypath.xml"), vec![]),
            (PathBuf::from("this/is/a.json"), vec!["keypath"]),
            (PathBuf::from("this/is/a.xml"), vec!["keypath"]),
            (PathBuf::from("this/is.json"), vec!["a", "keypath"]),
            (PathBuf::from("this/is.xml"), vec!["a", "keypath"]),
            (PathBuf::from("this.json"), vec!["is", "a", "keypath"]),
            (PathBuf::from("this.xml"), vec!["is", "a", "keypath"]),
        ];
        assert_eq!(zipped, expected);
    }

    #[test]
    fn valid_with_spaces() {
        let input = " this . is . a . valid . keypath ";
        let result = KeyPath::try_from(input).expect("key parsed");
        let expected = vec!["this", "is", "a", "valid", "keypath"];
        assert_eq!(result.components(), expected);
        assert_eq!(result.to_string(), "this.is.a.valid.keypath");
    }

    #[test]
    fn err_contains_slash() {
        let input = "contains/slash";
        let result = KeyPath::try_from(input).expect_err("invalid keypath");
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }

    #[test]
    fn err_empty_component_middle() {
        let input = "empty.component..in.middle";
        let result = KeyPath::try_from(input).expect_err("invalid keypath");
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }

    #[test]
    fn err_whitespace_component_middle() {
        let input = "whitespace.component. .in.middle";
        let result = KeyPath::try_from(input).expect_err("invalid keypath");
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }

    #[test]
    fn err_empty_component_first() {
        let input = ".empty.component.at.beginning";
        let result = KeyPath::try_from(input).expect_err("invalid keypath");
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }

    #[test]
    fn err_empty_component_last() {
        let input = "empty.component.at.end.";
        let result = KeyPath::try_from(input).expect_err("invalid keypath");
        assert!(matches!(result, KeyPathParseError::InvalidKeyPath));
    }
}
