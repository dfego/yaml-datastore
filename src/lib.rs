use serde::de::DeserializeOwned;
use serde_yml::{Mapping, value::from_value};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error")]
    IOError(#[from] std::io::Error),

    #[error("data parse error")]
    DataParseError(#[from] serde_yml::Error),

    #[error("key not found in data")]
    KeyNotFound,

    #[error("empty key vector")]
    EmptyKeyVector,
}

fn map_recurse<T: DeserializeOwned>(map: &Mapping, keys: &[&str]) -> Result<T, Error> {
    if keys.is_empty() {
        Err(Error::EmptyKeyVector)
    } else if keys.len() == 1 {
        // Base case, we're at the last key so we return this one
        let value = map.get(keys[0]).ok_or(Error::KeyNotFound)?.to_owned();
        Ok(from_value(value)?)
    } else {
        // Recursion case, where we pass in the sub-mapping and remaining keys
        // Having a mismatched type in the case of [as_mapping] failing means
        // there can't be a key that matches, so we return [Error::KeyNotFound].
        let sub_map = map
            .get(keys[0])
            .ok_or(Error::KeyNotFound)?
            .as_mapping()
            .ok_or(Error::KeyNotFound)?;
        map_recurse(sub_map, &keys[1..])
    }
}

#[cfg(test)]
mod hash_map_recurse_tests {
    use super::Error;
    use super::map_recurse;

    #[test]
    fn empty_keys() {
        let yaml = "";
        let data: serde_yml::Mapping = serde_yml::from_str(&yaml).unwrap();
        let value = map_recurse::<bool>(&data, &vec![]).unwrap_err();
        assert!(matches!(value, Error::EmptyKeyVector));
    }

    #[test]
    fn missing_key_in_data() {
        let yaml = "";
        let data: serde_yml::Mapping = serde_yml::from_str(&yaml).unwrap();
        let value = map_recurse::<bool>(&data, &vec!["something"]).unwrap_err();
        assert!(matches!(value, Error::KeyNotFound));
    }

    #[test]
    fn flat_keys() {
        let yaml = "
        key1: false
        key2: true
        key3: false
        ";
        let data: serde_yml::Mapping = serde_yml::from_str(&yaml).unwrap();

        let value: bool = map_recurse(&data, &vec!["key1"]).unwrap();
        assert_eq!(value, false);
        let value: bool = map_recurse(&data, &vec!["key2"]).unwrap();
        assert_eq!(value, true);
        let value: bool = map_recurse(&data, &vec!["key3"]).unwrap();
        assert_eq!(value, false);
    }

    #[test]
    fn nested_keys() {
        let yaml = "
        outer:
            middle:
                inner: true
        ";
        let data: serde_yml::Mapping = serde_yml::from_str(&yaml).unwrap();
        let value: bool = map_recurse(&data, &vec!["outer", "middle", "inner"]).unwrap();
        assert_eq!(value, true);
    }
}

pub struct YAMLDatastore {
    root: PathBuf,
}

impl YAMLDatastore {
    pub fn init<P: Into<PathBuf>>(path: P) -> YAMLDatastore {
        YAMLDatastore { root: path.into() }
    }

    pub fn get<P, T>(&self, path: P) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        let full_path = self.root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let result = serde_yml::from_str(&file_string)?;
        Ok(result)
    }

    pub fn get_with_key<P, T>(&self, path: P, key: &str) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        if key.is_empty() {
            return self.get(path);
        }

        let full_path = self.root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let mapping: Mapping = serde_yml::from_str(&file_string)?;
        let value = mapping.get(key).ok_or(Error::KeyNotFound)?.to_owned();
        Ok(from_value(value)?)
    }

    pub fn get_with_key_vec<P, T>(&self, path: P, key_vec: &[&str]) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned,
    {
        if key_vec.is_empty() {
            return self.get(path);
        }

        let full_path = self.root.join(&path);
        let file_string = std::fs::read_to_string(&full_path)?;
        let mapping: serde_yml::Mapping = serde_yml::from_str(&file_string)?;
        map_recurse(&mapping, key_vec)
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
        let result: bool = datastore.get_with_key("complete.yaml", "complete").unwrap();
        assert_eq!(result, true);
    }

    #[test]
    fn nested_bool() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let result: bool = datastore
            .get_with_key_vec("complete.yaml", &vec!["nested", "value"])
            .unwrap();
        assert_eq!(result, true);
    }

    #[test]
    fn single_bool_key_not_found() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let result = datastore
            .get_with_key::<_, bool>("empty.yaml", "complete")
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

    #[test]
    fn mismatched_type() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let result = datastore
            .get_with_key::<_, u64>("complete.yaml", "complete")
            .unwrap_err();
        assert!(matches!(result, Error::DataParseError(_)));
    }

    #[test]
    fn duplicate_key() {
        let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
        let result = datastore
            .get_with_key::<_, bool>("duplicate.yaml", "key")
            .unwrap_err();
        assert!(matches!(result, Error::DataParseError(_)));
    }
}
