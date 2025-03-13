use serde::de::DeserializeOwned;
use serde_yml::value::from_value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thiserror::Error;

// static FULL_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("(.+)/(.*)").unwrap());

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error")]
    IOError(#[from] std::io::Error),

    #[error("data parse error")]
    DataParseError(#[from] serde_yml::Error),

    #[error("key not found")]
    KeyNotFound,
}

pub struct YAMLDatastore {
    _root: PathBuf,
}

impl YAMLDatastore {
    pub fn init<P: Into<PathBuf>>(path: P) -> YAMLDatastore {
        YAMLDatastore { _root: path.into() }
    }

    pub fn get<P, T>(&self, path: P) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        let full_path = self._root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let result = serde_yml::from_str(&file_string)?;
        Ok(result)
    }

    pub fn get_with_key<P, T>(&self, path: P, key: &[&str]) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        if key.is_empty() {
            return self.get(path);
        }

        let full_path = self._root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let hash_map: HashMap<&str, serde_yml::Value> = serde_yml::from_str(&file_string)?;

        // TODO test with more than one, but for now just hard-code the first member
        let value = hash_map.get(key[0]).ok_or(Error::KeyNotFound)?.to_owned();
        Ok(from_value(value)?)
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
            nested: Some(TestNested { value: true }),
        };

        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let parsed: TestFormat = datastore.get("complete.yaml").unwrap();
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

        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let parsed: TestFormat = datastore.get("no_tags.yaml").unwrap();
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_with_single_bool_key() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let result: bool = datastore
            .get_with_key("complete.yaml", &vec!["complete"])
            .unwrap();
        assert_eq!(result, true);
    }

    #[test]
    fn single_bool_key_not_found() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let result = datastore
            .get_with_key::<_, bool>("empty.yaml", &vec!["complete"])
            .unwrap_err();
        assert!(matches!(result, Error::KeyNotFound));
    }

    #[test]
    fn test_missing_file() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let parsed = datastore.get::<_, TestFormat>("nonexistent").unwrap_err();
        assert!(matches!(parsed, Error::IOError(_)));
    }

    #[test]
    fn test_parse_error() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let parsed = datastore.get::<_, TestFormat>("empty.yaml").unwrap_err();
        assert!(matches!(parsed, Error::DataParseError(_)));
    }
}
