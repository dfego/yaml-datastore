// use serde::Deserialize;
// use serde_yml::from_str;
use std::path::PathBuf;

pub struct YAMLDatastore {
    _root: PathBuf,
}

// struct PathAndKey {
//     path: PathBuf,
//     key: String,
// }

impl YAMLDatastore {
    pub fn init<P: Into<PathBuf>>(path: P) -> YAMLDatastore {
        YAMLDatastore { _root: path.into() }
    }

    // fn parse(&self, path_and_key: &str) -> PathAndKey {
    //     PathAndKey {
    //         path: "banana".into(),
    //         key: "market.cost.wegmans".into(),
    //     }
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

    // #[test]
    // fn test_get_int() {
    //     let datastore: YAMLDatastore = YAMLDatastore::init(TEST_DATASTORE_PATH);
    //     let int = datastore.get_int();
    //     assert_eq!(int, -6);
    // }
}
