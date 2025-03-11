use std::path::PathBuf;

pub struct YAMLDatastore {
    _root: PathBuf,
}

impl YAMLDatastore {
    pub fn init<P: Into<PathBuf>>(path: P) -> YAMLDatastore {
        YAMLDatastore { _root: path.into() }
    }

    // pub fn get_value() -> String {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_yaml_datastore() {
        let _datastore: YAMLDatastore = YAMLDatastore::init(".");
    }
}
