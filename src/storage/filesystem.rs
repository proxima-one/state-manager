use super::interface;
use crate::types::{Bytes, Error, KeyValue, Result};
use std::collections::hash_map::HashMap;
use std::path::Path;

/// Simple storage implementation which caches all the values in memory and
/// dumps them to files as a checkpoint.
/// Effective only for cases with a small amount of keys.
pub struct FilesystemStorage {
  values: HashMap<String, Bytes>,
}

impl interface::KVStorage for FilesystemStorage {
  fn open(path: impl AsRef<Path>) -> Result<Self> {
    let path = path.as_ref();
    if !path.exists() {
      return Ok(Self {
        values: HashMap::new(),
      });
    }
    let mut values: HashMap<String, Bytes> = HashMap::new();
    for file in path
      .read_dir()
      .unwrap_or_else(|_| panic!("Couldn't open dir {}", path.display()))
      .flatten()
    {
      let filepath = file.path();
      let key = filepath.file_name().unwrap().to_str().unwrap();
      let value: Bytes =
        std::fs::read(&filepath).unwrap_or_else(|_| panic!("Couldn't read file {}", filepath.display()));
      values.insert(key.to_owned(), value);
    }
    Ok(Self { values })
  }

  fn destroy(path: impl AsRef<Path>) -> Result<()> {
    if path.as_ref().exists() {
      std::fs::remove_dir_all(path).map_err(From::from)
    } else {
      Ok(())
    }
  }

  fn get_one<Key: AsRef<str>>(&self, key: Key) -> Result<Bytes> {
    match self.values.get(key.as_ref()) {
      Some(value) => Ok(value.clone()),
      None => Err(Error::NotFound(format!("Key {} not found", key.as_ref()))),
    }
  }

  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<KeyValue>> {
    let mut result = vec![];
    for key in keys {
      if let Some(value) = self.values.get(key.as_ref()) {
        result.push(KeyValue {
          key: key.as_ref().to_owned(),
          value: value.clone(),
        });
      }
    }
    Ok(result)
  }

  fn write(&mut self, parts: Vec<KeyValue>) -> Result<()> {
    for part in parts.into_iter() {
      self.values.insert(part.key, part.value);
    }
    Ok(())
  }

  fn save_copy(&self, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    std::fs::create_dir(path)?;
    for (key, value) in self.values.iter() {
      let filepath = path.join(key);
      std::fs::write(filepath, value)?;
    }
    Ok(())
  }
}
