use super::interface::KVStorage;
use crate::types::{Bytes, Error, KeyValue, Result};
use rocksdb::{checkpoint::Checkpoint, Error as RocksdbError, Options, WriteBatch, DB};
use std::path::Path;

struct RocksdbStorage {
  db: DB,
}

impl From<RocksdbError> for Error {
  fn from(err: RocksdbError) -> Self {
    Self::DbError(err.into())
  }
}

impl KVStorage for RocksdbStorage {
  fn new(path: impl AsRef<Path>) -> Result<Self> {
    let db = DB::open_default(path)?;
    Ok(Self { db })
  }

  fn destroy(path: impl AsRef<Path>) -> Result<()> {
    DB::destroy(&Options::default(), path)?;
    Ok(())
  }

  fn get_one<Key: AsRef<str>>(&self, key: Key) -> Result<Bytes> {
    self
      .db
      .get(key.as_ref().as_bytes())?
      .ok_or_else(|| Error::NotFound(format!("Key {} not found", key.as_ref())))
  }

  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<KeyValue>> {
    let resp: Vec<Option<Bytes>> = self
      .db
      .multi_get(keys.iter().map(|s| s.as_ref().as_bytes()))
      .into_iter()
      .collect::<std::result::Result<_, _>>()?;
    let mut result = Vec::new();
    for (key, value) in std::iter::zip(keys, resp) {
      if let Some(value) = value {
        result.push(KeyValue {
          key: key.as_ref().to_owned(),
          value,
        });
      }
    }
    Ok(result)
  }

  fn write(&mut self, parts: Vec<KeyValue>) -> Result<()> {
    let mut batch = WriteBatch::default();
    for part in parts.into_iter() {
      batch.put(part.key, part.value);
    }
    self.db.write(batch)?;
    Ok(())
  }

  fn save_copy(&self, path: impl AsRef<Path>) -> Result<()> {
    // TODO: consider reusing the same checkpoint manager.
    // It is problematic because storing it in the struct would cause
    // a self-reference, which makes the struct non-movable.
    let checkpoint_manager = Checkpoint::new(&self.db)?;
    checkpoint_manager.create_checkpoint(path)?;
    Ok(())
  }
}
