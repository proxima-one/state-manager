use std::path::Path;
use crate::types::{Bytes, KeyValue, Result};

pub trait KVStorage: Sized + Sync + Send {
  fn open(path: impl AsRef<Path>) -> Result<Self>;
  fn destroy(path: impl AsRef<Path>) -> Result<()>;
  fn get_one<Key: AsRef<str>>(&self, key: Key) -> Result<Bytes>;
  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<KeyValue>>;
  fn write(&mut self, parts: Vec<KeyValue>) -> Result<()>;
  fn save_copy(&self, path: impl AsRef<Path>) -> Result<()>;
}
