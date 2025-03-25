//! API for using a set of [YAML][00] files as a cohesive datastore.
//!
//! What this crate supports is having a set of YAML files be accessible as a single, uniform datastore.
//! So for example, if you had a set of YAML files all containing structured data, you could use this crate
//! to query specific values from it. In effect, it's an ergonomic wrapper for managing a set of files
//! doing the file I/O, and pulling specific elements out of those files.
//!
//! # Usage
//!
//! Assume there is a set of YAML files under `tests/data/`, and within exists a file named `complete.yaml`
//! with the following content:
//!
//! ```text
//! name: Complete
//! id: 1
//! rating: 1.0
//! complete: true
//! tags:
//!   - complete
//!   - done
//!   - finished
//! nested:
//!   value: true
//! ```
//!
//! To access the `true` contained under `nested -> value`, you can do the following:
//!
//! ```
//! use yaml_datastore::Datastore;
//!
//! let datastore: Datastore = Datastore::open("tests/data");
//! let parsed: bool = datastore.get("complete.nested.value").unwrap();
//! assert!(parsed);
//! ```
//!
//! See the [`Datastore`] and [`keypath`] documentation for more information on how the keypaths are resolved into values.
//!
//! [00]: https://yaml.org/

use keypath::{KeyPath, KeyPathParseError};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_yaml::{Mapping, value::from_value};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod keypath;

/// Error type for this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// An I/O error occurred, most likely a requested file was not found or could not be read.
    #[error("I/O error")]
    IOError(#[from] std::io::Error),

    /// YAML data could not be parsed. Given YAML is very permissive, this is likely a formatting error.
    #[error("data parse error")]
    DataParseError(#[from] serde_yaml::Error),

    /// A key requested via [`Datastore::get_with_key`] or [`Datastore::get_with_key_vec`] was not found.
    #[error("key not found in data")]
    KeyNotFound,

    /// An empty key vector was passed to [`Datastore::get_with_key_vec`].
    #[error("empty key vector")]
    EmptyKeyVector,

    /// Error returned from the keypath parser during parsing.
    #[error(transparent)]
    KeyPathError(#[from] KeyPathParseError),
}

fn yaml_mapping_recurse<T, S>(map: &Mapping, keys: &[S]) -> Result<T, Error>
where
    T: DeserializeOwned,
    S: AsRef<str> + serde_yaml::mapping::Index,
{
    if keys.is_empty() {
        Err(Error::EmptyKeyVector)
    } else if keys.len() == 1 {
        // Base case, we're at the last key so we return this one
        let value = map.get(&keys[0]).ok_or(Error::KeyNotFound)?.to_owned();
        Ok(from_value(value)?)
    } else {
        // Recursion case, where we pass in the sub-mapping and remaining keys
        // Having a mismatched type in the case of [as_mapping] failing means
        // there can't be a key that matches, so we return [Error::KeyNotFound].
        let sub_map = map
            .get(&keys[0])
            .ok_or(Error::KeyNotFound)?
            .as_mapping()
            .ok_or(Error::KeyNotFound)?;
        yaml_mapping_recurse(sub_map, &keys[1..])
    }
}

#[cfg(test)]
mod yaml_mapping_recurse_tests {
    use super::Error;
    use super::yaml_mapping_recurse;
    use serde_yaml::{Mapping, from_str};

    #[test]
    fn empty_keys() {
        let yaml = "";
        let data: Mapping = from_str(yaml).unwrap();
        let value = yaml_mapping_recurse::<bool, &str>(&data, &[]).unwrap_err();
        assert!(matches!(value, Error::EmptyKeyVector));
    }

    #[test]
    fn missing_key_in_data() {
        let yaml = "";
        let data: Mapping = from_str(yaml).unwrap();
        let value = yaml_mapping_recurse::<bool, &str>(&data, &["something"]).unwrap_err();
        assert!(matches!(value, Error::KeyNotFound));
    }

    #[test]
    fn flat_keys() {
        let yaml = "
        key1: false
        key2: true
        key3: false
        ";
        let data: Mapping = from_str(yaml).unwrap();

        let value: bool = yaml_mapping_recurse(&data, &["key1"]).unwrap();
        assert!(!value);
        let value: bool = yaml_mapping_recurse(&data, &["key2"]).unwrap();
        assert!(value);
        let value: bool = yaml_mapping_recurse(&data, &["key3"]).unwrap();
        assert!(!value);
    }

    #[test]
    fn nested_keys() {
        let yaml = "
        outer:
            middle:
                inner: true
        ";
        let data: Mapping = from_str(yaml).unwrap();
        let value: bool = yaml_mapping_recurse(&data, &["outer", "middle", "inner"]).unwrap();
        assert!(value);
    }
}

/// Handle for a YAML datastore.
///
/// Open with [`open()`](Datastore::open).
/// Access with [`get()`](Datastore::get).
#[derive(Debug, Serialize, Deserialize)]
pub struct Datastore {
    /// The filesystem root of the datastore. All lookups are done relative to this path.
    root: PathBuf,
}

impl Datastore {
    /// Open a handle to a datastore at the given path.
    ///
    /// At present, this doesn't actually perform any operations.
    pub fn open<P: Into<PathBuf>>(path: P) -> Datastore {
        Datastore { root: path.into() }
    }

    /// Helper function to support [`Self::get`] that attempts to access the given path and YAML key.
    fn try_get<P, S, T>(path: P, keys: &[S]) -> Option<T>
    where
        P: AsRef<Path>,
        S: AsRef<str> + serde_yaml::mapping::Index,
        T: DeserializeOwned,
    {
        let file_string = std::fs::read_to_string(path).ok()?;
        if keys.is_empty() {
            Some(serde_yaml::from_str(&file_string).ok()?)
        } else {
            let mapping: Mapping = serde_yaml::from_str(&file_string).ok()?;
            yaml_mapping_recurse(&mapping, keys).ok()?
        }
    }

    /// Get a value from the datastore given a keypath.
    ///
    /// This method parses the given string into a [`KeyPath`] and then iterates over the possible path
    /// and key combinations until it finds a match or has exhausted them. It starts with the longest
    /// possible path and shortest key and works backwards.
    ///
    /// More explicitly, it will search each possible path for the given keypath, starting with the longest.
    /// If it finds a file that matches, it attempts to use the remainder of the keypath as keys into the YAML file.
    /// If the full keypath matches a file path, then the entire YAML file data is returned.
    ///
    /// See the documentation for [keypath] for more information on how the keypath is used to generate combinations.
    ///
    /// # Examples
    ///
    /// For a keypath of `a.b.c.d`, the first match of the following would be returned if found:
    ///
    /// 1. The entire contents of file `a/b/c/d.yaml`.
    /// 2. The contents of the key `d` in `a/b/c.yaml`.
    /// 3. The contents of the key `c.d` in `a/b.yaml`.
    /// 4. The contents of the key `b.c.d` in `a.yaml`.
    ///
    /// For the above, the dot-notation for YAML keys implies nesting. So for `b.c.d`:
    ///
    /// ```text
    /// b:
    ///   c:
    ///     d: 42
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::KeyPathError`] if `keypath` is invalid.
    ///
    /// Returns [`Error::KeyNotFound`] if the given key was not found.
    pub fn get<T: DeserializeOwned>(&self, keypath: &str) -> Result<T, Error> {
        let keypath = KeyPath::try_from(keypath)?;
        for (path, keys) in keypath.iter() {
            if let Some(data) = Self::try_get(self.root.join(path), &keys) {
                return Ok(data);
            }
        }
        Err(Error::KeyNotFound)
    }

    /// Get all the data from a given YAML file in the datastore.
    ///
    /// This function makes no assumptions about the underlying YAML data other than it being valid.
    ///
    /// On success, returns an object of the specified return type.
    ///
    /// # Errors
    ///
    /// Will return [`Error::IOError`] if a file at `path` cannot be read.
    ///
    /// Will return [`Error::DataParseError`] if:
    /// * A file at `path` is not able to be parsed as valid YAML
    /// * The return type specified does not match the type found in the input file.
    pub fn get_with_path<P, T>(&self, path: P) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        let full_path = self.root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let result = serde_yaml::from_str(&file_string)?;
        Ok(result)
    }

    /// Get a value from the given YAML file in the datastore based on a key.
    ///
    /// This function assumes the input YAML is a mapping.
    ///
    /// On success, returns an object of the specified return type.
    ///
    /// # Errors
    ///
    /// Will return [`Error::IOError`] if a file at `path` cannot be read.
    ///
    /// Will return [`Error::DataParseError`] if:
    /// * A file at `path` is not able to be parsed as valid YAML
    /// * The return type specified does not match the type found in the input file.
    ///
    /// Will return [`Error::KeyNotFound`] if the given key was not found in a top-level map of the YAML file.
    /// * A file at `path` is not able to be parsed as valid YAML
    /// * The return type specified does not match the type found in the input file.
    pub fn get_with_key<P, T>(&self, path: P, key: &str) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        if key.is_empty() {
            return self.get_with_path(path);
        }

        let full_path = self.root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let mapping: Mapping = serde_yaml::from_str(&file_string)?;
        let value = mapping.get(key).ok_or(Error::KeyNotFound)?.to_owned();
        Ok(from_value(value)?)
    }

    /// Get a value from the given YAML file in the datastore based on a set of keys.
    ///
    /// This function assumes the input YAML is a mapping.
    /// It traverses each element of `key_vec` and treats it as a level of nesting.
    /// For example, for the given input:
    ///
    /// ```yaml
    /// outer:
    ///   middle:
    ///     inner: 42
    /// ```
    ///
    /// In order to get the value of `inner` (42), A `key_vec` would be passed as:
    ///
    /// ```no_compile
    /// vec!["outer", "middle", "inner"]
    /// ```
    ///
    /// On success, returns an object of the specified return type.
    ///
    /// # Errors
    ///
    /// Will return [`Error::IOError`] if a file at `path` cannot be read.
    ///
    /// Will return [`Error::DataParseError`] if:
    /// * A file at `path` is not able to be parsed as valid YAML
    /// * The return type specified does not match the type found in the input file.
    ///
    /// Will return [`Error::KeyNotFound`] if the given key was not found in a top-level map of the YAML file.
    /// * A file at `path` is not able to be parsed as valid YAML
    /// * The return type specified does not match the type found in the input file.
    pub fn get_with_key_vec<P, T, S>(&self, path: P, key_vec: &[S]) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
        S: AsRef<str> + serde_yaml::mapping::Index,
    {
        if key_vec.is_empty() {
            return self.get_with_path(path);
        }

        let full_path = self.root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let mapping: Mapping = serde_yaml::from_str(&file_string)?;
        yaml_mapping_recurse(&mapping, key_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct TestNested {
        value: bool,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct TestFormat {
        name: String,
        id: u64,
        rating: Option<f64>,

        #[serde(default)]
        complete: bool,

        #[serde(default)]
        tags: Vec<String>,

        nested: Option<TestNested>,
    }

    static TEST_DATASTORE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");

    #[test]
    fn initialize_yaml_datastore() {
        let _datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
    }

    #[test]
    fn test_keypath_nested() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let parsed: bool = datastore.get("complete.nested.value").unwrap();
        assert!(parsed);
    }

    #[test]
    fn test_keypath_complete() {
        let reference = TestFormat {
            name: "Complete".into(),
            id: 1,
            rating: Some(1.0),
            complete: true,
            tags: vec!["complete".into(), "done".into(), "finished".into()],
            nested: Some(TestNested { value: true }),
        };

        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let parsed: TestFormat = datastore.get("complete").unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_complete() {
        let reference = TestFormat {
            name: "Complete".into(),
            id: 1,
            rating: Some(1.0),
            complete: true,
            tags: vec!["complete".into(), "done".into(), "finished".into()],
            nested: Some(TestNested { value: true }),
        };

        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let parsed: TestFormat = datastore.get_with_path("complete.yaml").unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_no_tags() {
        let reference = TestFormat {
            name: "No Tags".into(),
            id: 2,
            rating: Some(0.6),
            complete: false,
            tags: vec![],
            nested: None,
        };

        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let parsed: TestFormat = datastore.get_with_path("no_tags.yaml").unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_with_single_bool_key() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let result: bool = datastore.get_with_key("complete.yaml", "complete").unwrap();
        assert!(result);
    }

    #[test]
    fn nested_bool() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let result: bool = datastore
            .get_with_key_vec("complete.yaml", &["nested", "value"])
            .unwrap();
        assert!(result);
    }

    #[test]
    fn single_bool_key_not_found() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let result = datastore
            .get_with_key::<_, bool>("empty.yaml", "complete")
            .unwrap_err();
        assert!(matches!(result, Error::KeyNotFound));
    }

    #[test]
    fn test_missing_file() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let parsed = datastore
            .get_with_path::<_, TestFormat>("nonexistent")
            .unwrap_err();
        assert!(matches!(parsed, Error::IOError(_)));
    }

    #[test]
    fn test_parse_error() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let parsed = datastore
            .get_with_path::<_, TestFormat>("empty.yaml")
            .unwrap_err();
        assert!(matches!(parsed, Error::DataParseError(_)));
    }

    #[test]
    fn mismatched_type() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let result = datastore
            .get_with_key::<_, u64>("complete.yaml", "complete")
            .unwrap_err();
        assert!(matches!(result, Error::DataParseError(_)));
    }

    #[test]
    fn duplicate_key() {
        let datastore: Datastore = Datastore::open(TEST_DATASTORE_PATH);
        let result = datastore
            .get_with_key::<_, bool>("duplicate.yaml", "key")
            .unwrap_err();
        assert!(matches!(result, Error::DataParseError(_)));
    }
}
