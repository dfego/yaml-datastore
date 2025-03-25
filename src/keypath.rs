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
use core::num;
use std::{
    fmt::Display,
    iter::{Zip, zip},
    path::{self, Path, PathBuf},
};
use thiserror::Error;

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
                .split(DELIMITER)
                .map(validate_and_trim)
                .collect::<Result<Vec<_>, _>>()?
                .join(DELIMITER),
        })
    }
}

impl Display for KeyPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

const EXTENSIONS: &[&str] = &["yaml", "yml"];

impl KeyPath {
    pub fn components(&self) -> Vec<&str> {
        self.raw.split(DELIMITER).collect()
    }

    // pub fn split_iter(&self) -> impl Iterator<Item = (PathBuf, Vec<&str>)> {
    //     let c = self.components();
    //     let c2 = c.clone();
    //     let range = (1..=c.len()).rev();
    //     zip(
    //         range.clone().flat_map(move |i| {
    //             let path: PathBuf = c[0..i].iter().collect();
    //             [path.with_extension("yaml"), path.with_extension("yml")]
    //         }),
    //         range
    //             .clone()
    //             .flat_map(move |i| [c2[i..].to_vec(), c2[i..].to_vec()]),
    //     )
    // }

    // pub fn split_iter2(&self) -> impl Iterator<Item = (PathBuf, Vec<&str>)> {
    //     let c = self.components();
    //     let c2 = c.clone();
    //     let range = (1..=c.len()).rev();
    //     zip(
    //         range.clone().map(move |i| c[0..i].iter().collect()),
    //         range.clone().map(move |i| c2[i..].to_vec()),
    //     )
    //     .flat_map(|pair: (PathBuf, _)| {
    //         [
    //             (pair.0.with_extension("yaml"), pair.1.clone()),
    //             (pair.0.with_extension("yml"), pair.1),
    //         ]
    //     })
    // }

    /// Return an iterator
    pub fn iter(&self) -> impl Iterator<Item = (PathBuf, Vec<&str>)> {
        let paths = self.components();
        let keys = self.components();
        let range = (1..=paths.len()).rev();
        zip(
            range.clone().map(move |i| paths[0..i].iter().collect()),
            range.clone().map(move |i| keys[i..].to_vec()),
        )
        .flat_map(|pair: (PathBuf, _)| {
            EXTENSIONS
                .iter()
                .map(move |e| (pair.0.with_extension(e), pair.1.clone()))
        })
    }

    // pub fn split_iter2(&self) -> impl Iterator<Item = (PathBuf, Vec<&str>)> {
    //     let c1 = self.components();
    //     let c2 = self.components();
    //     let range = (1..=c1.len()).rev();
    //     let path_iterator = range.clone().map(move |i| c1[0..i].iter().collect());
    //     let key_iterator = range.clone().map(move |i| c2[i..].to_vec());
    //     fn add_extensions(v: (PathBuf, Vec<&str>)) -> impl Iterator<Item = (PathBuf, Vec<&str>)> {
    //         EXTENSIONS
    //             .iter()
    //             .map(move |e| (v.0.with_extension(e), v.1.clone()))
    //     }
    //     zip(path_iterator, key_iterator).flat_map(add_extensions)
    // }

    // pub fn split_iter4(&self) -> impl Iterator<Item = (PathBuf, Vec<&str>)> {
    //     let components = self.components();
    //     let mut ret = vec![];
    //     for index in (1..=components.len()).rev() {
    //         let path: PathBuf = components[0..index].iter().collect();
    //         let key_vec = components[index..].to_vec();
    //         for extension in EXTENSIONS {
    //             ret.push((path.with_extension(extension), key_vec.clone()));
    //         }
    //     }
    //     ret.into_iter()
    // }
}

#[cfg(test)]
mod adhoc_tests {
    use super::*;

    #[test]
    fn adhoc() {
        let input = "this.is.a.keypath";
        let result = KeyPath::try_from(input).unwrap();
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

        // for (a, b) in zipped {
        //     println!("{}, {:?}", a.display(), b);
        // }
    }

    // #[test]
    // fn adhoc2() {
    //     let input = "this.is.a.keypath";
    //     let result = KeyPath::try_from(input).unwrap();
    //     let zipped: Vec<_> = result.split_iter2().collect();
    //     let expected = vec![
    //         (PathBuf::from("this/is/a/keypath.yaml"), vec![]),
    //         (PathBuf::from("this/is/a/keypath.yml"), vec![]),
    //         (PathBuf::from("this/is/a.yaml"), vec!["keypath"]),
    //         (PathBuf::from("this/is/a.yml"), vec!["keypath"]),
    //         (PathBuf::from("this/is.yaml"), vec!["a", "keypath"]),
    //         (PathBuf::from("this/is.yml"), vec!["a", "keypath"]),
    //         (PathBuf::from("this.yaml"), vec!["is", "a", "keypath"]),
    //         (PathBuf::from("this.yml"), vec!["is", "a", "keypath"]),
    //     ];
    //     assert_eq!(zipped, expected);

    //     // for (a, b) in zipped {
    //     //     println!("{}, {:?}", a.display(), b);
    //     // }
    // }

    // #[test]
    // fn adhoc3() {
    //     let input = "this.is.a.keypath";
    //     let result = KeyPath::try_from(input).unwrap();
    //     let zipped: Vec<_> = result.iter().collect();
    //     let expected = vec![
    //         (PathBuf::from("this/is/a/keypath.yaml"), vec![]),
    //         (PathBuf::from("this/is/a/keypath.yml"), vec![]),
    //         (PathBuf::from("this/is/a.yaml"), vec!["keypath"]),
    //         (PathBuf::from("this/is/a.yml"), vec!["keypath"]),
    //         (PathBuf::from("this/is.yaml"), vec!["a", "keypath"]),
    //         (PathBuf::from("this/is.yml"), vec!["a", "keypath"]),
    //         (PathBuf::from("this.yaml"), vec!["is", "a", "keypath"]),
    //         (PathBuf::from("this.yml"), vec!["is", "a", "keypath"]),
    //     ];
    //     assert_eq!(zipped, expected);

    //     // for (a, b) in zipped {
    //     //     println!("{}, {:?}", a.display(), b);
    //     // }
    // }

    // #[test]
    // fn adhoc4() {
    //     let input = "this.is.a.keypath";
    //     let result = KeyPath::try_from(input).unwrap();
    //     let zipped: Vec<_> = result.split_iter4().collect();
    //     let expected = vec![
    //         (PathBuf::from("this/is/a/keypath.yaml"), vec![]),
    //         (PathBuf::from("this/is/a/keypath.yml"), vec![]),
    //         (PathBuf::from("this/is/a.yaml"), vec!["keypath"]),
    //         (PathBuf::from("this/is/a.yml"), vec!["keypath"]),
    //         (PathBuf::from("this/is.yaml"), vec!["a", "keypath"]),
    //         (PathBuf::from("this/is.yml"), vec!["a", "keypath"]),
    //         (PathBuf::from("this.yaml"), vec!["is", "a", "keypath"]),
    //         (PathBuf::from("this.yml"), vec!["is", "a", "keypath"]),
    //     ];
    //     assert_eq!(zipped, expected);

    //     // for (a, b) in zipped {
    //     //     println!("{}, {:?}", a.display(), b);
    //     // }
    // }
}

// struct Iterator<'a> {
//     keypath: &'a KeyPath,
//     index: u32,
// }

// impl<'a> Iterator for KeyPathIterator<'a> {
//     type Item = (std::path::PathBuf, Vec<String>);

//     fn next(&mut self) -> Option<Self::Item> {
//         if index > 0
//     }
// }

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
