use super::interface::KVStorage;
use crate::types::{Bytes, Error, KeyValue, Result};
use rocksdb::{
  checkpoint::Checkpoint, BlockBasedOptions, Cache, Error as RocksdbError, Options, WriteBatch, DB,
};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{sync_channel, SyncSender, TryRecvError};

pub struct RocksdbStorage {
  db: DB,
  _sender: SyncSender<()>,
}

impl From<RocksdbError> for Error {
  fn from(err: RocksdbError) -> Self {
    Self::DbError(err.into())
  }
}

impl KVStorage for RocksdbStorage {
  fn new(path: impl AsRef<Path>) -> Result<Self> {
    let mut block_opts = BlockBasedOptions::default();
    block_opts.set_block_cache(&Cache::new_lru_cache(2usize.pow(36)).unwrap());

    let mut options = Options::default();
    options.set_paranoid_checks(true);
    options.create_if_missing(true);
    options.set_block_based_table_factory(&block_opts);
    options.set_write_buffer_size(2usize.pow(30));
    options.set_max_write_buffer_number(8);
    options.enable_statistics();

    let db = DB::open(&options, &path)?;

    let statistics_path = path.as_ref().join("statistics");
    let (sender, receiver) = sync_channel::<()>(0);
    std::thread::spawn(move || {
      while receiver.try_recv() != Err(TryRecvError::Disconnected) {
        writeln!(
          std::fs::File::create(&statistics_path).unwrap(),
          "{}",
          options.get_statistics().unwrap()
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(30));
      }
    });

    Ok(Self {
      db,
      _sender: sender,
    })
  }

  fn destroy(path: impl AsRef<Path>) -> Result<()> {
    DB::destroy(&Options::default(), &path)?;
    std::fs::remove_dir_all(path)?;
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
