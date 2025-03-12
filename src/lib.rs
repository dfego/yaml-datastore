use std::{path::PathBuf, str::FromStr};
use thiserror::Error;

// static FULL_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("(.+)/(.*)").unwrap());

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("failed to parse key")]
    KeyParseError(String),
}

pub struct YAMLDatastore {
    _root: PathBuf,
}

/// A "full" key to a path and file data.
#[derive(Debug, PartialEq)]
struct FullKey {
    path: PathBuf,
    key: String,
}

impl FromStr for FullKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path_and_key = s
            .rsplit_once("/")
            .ok_or(Error::KeyParseError("no slash (/) delimiter".into()))?;

        // path is required to be non-empty, but key can be empty
        let path = path_and_key.0.trim();
        if path.is_empty() {
            return Err(Error::KeyParseError("empty path before slash".into()));
        };

        // TODO validate key
        let key = path_and_key.1.trim();

        Ok(FullKey {
            path: path.into(),
            key: key.into(),
        })
    }
}

impl YAMLDatastore {
    pub fn init<P: Into<PathBuf>>(path: P) -> YAMLDatastore {
        YAMLDatastore { _root: path.into() }
    }

    // Parse a key string into a path and key
    // Key format is file/key
    // file is a filename minus extension
    // key is a string of the form "a.b.c", where each is a YAML key
    // fn parse_key(&self, key: &str) -> Result<FullKey, Error> {
    //     todo!()
    //     // Err(Error::KeyParseError)
    // }

    // pub fn get<'a, T>() -> T
    // where
    //     T: Deserialize<'a>,
    // {
    //     todo!()
    // }

    // pub fn get_float() -> f64 {
    //     0f64
    // }

    // pub fn get_int(&self) -> i64 {
    //     0i64
    // }

    // pub fn get_uint() -> u64 {
    //     0u64
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct TestFormat {
        name: String,
        id: u64,
        rating: Option<f64>,

        #[serde(default)]
        complete: bool,

        #[serde(default)]
        tags: Vec<String>,
    }

    static TEST_DATASTORE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");

    #[test]
    fn initialize_yaml_datastore() {
        let _datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
    }

    #[test]
    fn test_complete() {
        let reference = TestFormat {
            name: "Complete".into(),
            id: 1,
            rating: Some(1.0),
            complete: true,
            tags: vec!["complete".into(), "done".into(), "finished".into()],
        };

        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let path = datastore._root.join("complete.yaml");
        let file_string = std::fs::read_to_string(path).unwrap();
        let parsed: TestFormat = serde_yml::from_str(&file_string).unwrap();
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
        };

        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let path = datastore._root.join("no_tags.yaml");
        let file_string = std::fs::read_to_string(path).unwrap();
        let parsed: TestFormat = serde_yml::from_str(&file_string).unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_full_key_one_slash() {
        let reference = FullKey {
            path: "test_path".into(),
            key: "test_key".into(),
        };
        let parsed = FullKey::from_str("test_path/test_key").unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_full_key_multiple_slashes() {
        let reference = FullKey {
            path: "test/path".into(),
            key: "test_key".into(),
        };
        let parsed = FullKey::from_str("test/path/test_key").unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_full_key_error_no_slash() {
        let parsed = FullKey::from_str("test_key");
        assert!(parsed.is_err_and(|e| e == Error::KeyParseError("no slash (/) delimiter".into())));
    }

    #[test]
    fn test_full_key_error_no_path() {
        let parsed = FullKey::from_str("/test_key");
        assert!(parsed.is_err_and(|e| e == Error::KeyParseError("empty path before slash".into())));
    }

    // #[test]
    // fn test_get_int() {
    //     let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
    //     let int = datastore.get_int();
    //     assert_eq!(int, -6);
    // }
}
