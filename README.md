# yaml-datastore

API for using a set of [YAML][00] files as a cohesive datastore.

What this crate supports is having a set of YAML files be accessible as a single, uniform datastore.
So for example, if you had a set of YAML files all containing structured data, you could use this crate
to query specific values from it. In effect, it's an ergonomic wrapper for managing a set of files
doing the file I/O, and pulling specific elements out of those files.

## Usage

Assume there is a set of YAML files under `tests/data/`, and within exists a file named `complete.yaml`
with the following content:

```
name: Complete
id: 1
rating: 1.0
complete: true
tags:
  - complete
  - done
  - finished
nested:
  value: true
```

To access the `true` contained under `nested -> value`, you can do the following:

```rust
use yaml_datastore::Datastore;

let datastore: Datastore = Datastore::open("tests/data");
let parsed: bool = datastore.get("complete.nested.value").unwrap();
assert!(parsed);
```

See the [Datastore] and [keypath] documentation for more information on how the keypaths are resolved into values.

[00]: https://yaml.org/
