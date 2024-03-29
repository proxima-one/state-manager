use super::interface::{AppStateManager, Checkpoint, StateManager};
use crate::file_storage::interface::FileStorage;
use crate::types::{Bytes, Error, KeyValue, Result};
use dashmap::DashMap;
use log::info;
use std::collections::HashMap;

type KVMap = HashMap<String, Bytes>;

#[derive(Default, Debug)]
pub struct InMemoryStateManager {
  apps: DashMap<String, InMemoryAppStateManager>,
}

#[derive(Default, Debug)]
pub struct InMemoryAppStateManager {
  current: KVMap,
  checkpoints: Vec<AppCheckpoint>,
  modifications_number: u32,
}

#[derive(Default, Debug)]
struct AppCheckpoint {
  id: String,
  payload: String,
  values: KVMap,
}

impl InMemoryAppStateManager {
  pub fn new() -> Self {
    Self::default()
  }
}

#[async_trait::async_trait]
impl StateManager for InMemoryStateManager {
  type AppStateManager = InMemoryAppStateManager;

  fn init_app(&self, id: &str) -> Result<()> {
    self
      .apps
      .entry(id.to_owned())
      .or_insert_with(InMemoryAppStateManager::new);
    Ok(())
  }

  fn with_app<Out>(
    &self,
    id: &str,
    f: impl FnOnce(&mut Self::AppStateManager) -> Out,
  ) -> Result<Out> {
    if let Some(mut app) = self.apps.get_mut(id) {
      Ok(f(&mut app))
    } else {
      Err(Error::NotFound(format!("Unknown app: {}", id)))
    }
  }

  fn drop_app(&self, id: &str) -> Result<()> {
    match self.apps.remove(id) {
      Some(_) => Ok(()),
      None => Err(Error::NotFound(format!("App {} not found", id))),
    }
  }

  async fn store_snapshot(
    &self,
    _app_id: &str,
    _storage: &impl FileStorage,
    _prefix: &std::path::Path,
  ) -> Result<()> {
    unimplemented!();
  }
}

#[async_trait::async_trait]
impl AppStateManager for InMemoryAppStateManager {
  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<KeyValue>> {
    let mut result = Vec::new();
    for key in keys {
      let key = key.as_ref();
      if let Some(value) = self.current.get(key) {
        result.push(KeyValue {
          key: key.to_owned(),
          value: value.clone(),
        });
        continue;
      }
      if let Some(last_checkpoint) = self.checkpoints.last() {
        if let Some(value) = last_checkpoint.values.get(key) {
          result.push(KeyValue {
            key: key.to_owned(),
            value: value.clone(),
          });
        }
      }
    }
    Ok(result)
  }

  fn set(&mut self, parts: Vec<KeyValue>) -> Result<()> {
    self.modifications_number += 1;
    for part in parts {
      self.current.insert(part.key, part.value);
    }
    Ok(())
  }

  fn get_checkpoints(&self) -> Result<Vec<Checkpoint>> {
    let mut result = Vec::new();
    for checkpoint in &self.checkpoints {
      result.push(Checkpoint {
        id: checkpoint.id.clone(),
        payload: checkpoint.payload.clone(),
      });
    }
    Ok(result)
  }

  // Simply clone the last checkpoint entirely and apply new changes
  fn create_checkpoint(&mut self, payload: &str) -> Result<String> {
    self.modifications_number += 1;

    let mut values = match self.checkpoints.last() {
      Some(last_checkpoint) => last_checkpoint.values.clone(),
      None => KVMap::default(),
    };

    for (key, value) in self.current.drain() {
      values.insert(key, value);
    }

    let new_id = self.modifications_number.to_string();
    self.checkpoints.push(AppCheckpoint {
      id: new_id.clone(),
      payload: payload.to_owned(),
      values,
    });
    self.current.clear();
    Ok(new_id)
  }

  fn revert(&mut self, id: &str) -> Result<()> {
    let index = self
      .checkpoints
      .iter()
      .enumerate()
      .find(|(_i, checkpoint)| checkpoint.id == id)
      .map(|(i, _checkpoint)| i);
    if let Some(index) = index {
      self.modifications_number += 1;
      info!(
        "Dropping {} latest checkpoints to end up at {}",
        self.checkpoints.len() - index - 1,
        id
      );
      self.current.clear();
      self.checkpoints.truncate(index + 1);
    } else {
      return Err(Error::NotFound(format!(
        "Checkpoint with id {} does not exist",
        id
      )));
    }
    Ok(())
  }

  fn cleanup(&mut self, until_checkpoint: &str) -> Result<()> {
    let index = self
      .checkpoints
      .iter()
      .enumerate()
      .find(|(_i, checkpoint)| checkpoint.id == until_checkpoint)
      .map(|(i, _checkpoint)| i);
    if let Some(index) = index {
      self.modifications_number += 1;
      let removed = self.checkpoints.drain(..index);
      info!("Cleaned up {} checkpoints", removed.len());
    } else {
      return Err(Error::NotFound(format!(
        "Checkpoint with id {} does not exist",
        until_checkpoint
      )));
    }
    Ok(())
  }

  fn reset(&mut self) -> Result<()> {
    self.current.clear();
    Ok(())
  }

  fn modifications_number(&self) -> u32 {
    self.modifications_number
  }

  async fn store_snapshot(
    &self,
    _storage: &impl FileStorage,
    _prefix: &std::path::Path,
  ) -> Result<()> {
    unimplemented!();
  }
}
