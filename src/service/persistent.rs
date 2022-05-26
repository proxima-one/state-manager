use super::interface::{AppStateManager, Checkpoint, StateManager};
use crate::storage::interface::KVStorage;
use crate::types::{Error, KeyValue, Result};
use dashmap::DashMap;
use log::info;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct AppManifest {
  checkpoints: Vec<Checkpoint>,
}

#[derive(Default, Debug)]
pub struct PersistentStateManager<Storage: KVStorage> {
  root: PathBuf,
  apps: DashMap<String, PersistentAppStateManager<Storage>>,
}

#[derive(Default, Debug)]
pub struct PersistentAppStateManager<Storage: KVStorage> {
  root: PathBuf,
  manifest: AppManifest,
  storage: Storage,
  modifications_number: u32,
}

impl<Storage: KVStorage> PersistentStateManager<Storage> {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self {
      root: root.into(),
      apps: Default::default(),
    }
  }

  fn app_path(&self, app_id: impl AsRef<Path>) -> PathBuf {
    self.root.join(app_id)
  }
}

impl<Storage: KVStorage> PersistentAppStateManager<Storage> {
  fn checkpoints_dir(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join("checkpoints")
  }

  fn head_path(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join("HEAD")
  }

  fn checkpoint_path(root: impl AsRef<Path>, checkpoint: impl AsRef<Path>) -> PathBuf {
    Self::checkpoints_dir(root).join(checkpoint)
  }

  fn manifest_path(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join("manifest.json")
  }

  fn new(root: PathBuf) -> Result<Self> {
    std::fs::create_dir_all(Self::checkpoints_dir(&root))?;
    let manifest = Self::load_manifest(Self::manifest_path(&root))?;
    let storage = Storage::new(Self::head_path(&root))?;
    Ok(Self {
      root,
      manifest,
      storage,
      modifications_number: 0,
    })
  }

  fn load(root: PathBuf) -> Result<Self> {
    if !root.is_dir() {
      return Err(Error::NotFound("Application does not exist".to_owned()));
    }
    let manifest = Self::load_manifest(Self::manifest_path(&root))?;
    let storage = Storage::new(Self::head_path(&root))?;
    Ok(Self {
      root,
      manifest,
      storage,
      modifications_number: 0,
    })
  }

  fn load_manifest(path: impl AsRef<Path>) -> Result<AppManifest> {
    if let Ok(contents) = std::fs::read_to_string(path) {
      serde_json::from_str(&contents).map_err(|err| std::io::Error::from(err).into())
    } else {
      Ok(AppManifest::default())
    }
  }

  fn save_manifest(&self) -> Result<()> {
    let contents = serde_json::to_string(&self.manifest).map_err(std::io::Error::from)?;
    let path = Self::manifest_path(&self.root);
    std::fs::write(path, contents)?;
    Ok(())
  }

  fn generate_checkpoint_id(&self) -> String {
    // Could just be guids as well
    match self.manifest.checkpoints.last() {
      Some(last) => (last
        .id
        .parse::<usize>()
        .expect("Non numerical checkpoint name in manifest")
        + 1)
        .to_string(),
      None => "0".to_string(),
    }
  }

  fn find_checkpoint(&self, id: &str) -> Result<usize> {
    let index = self
      .manifest
      .checkpoints
      .iter()
      .enumerate()
      .find(|(_i, checkpoint)| checkpoint.id == id)
      .map(|(i, _checkpoint)| i);
    match index {
      Some(index) => Ok(index),
      None => Err(Error::NotFound(format!(
        "Checkpoint with id {} does not exist",
        id
      ))),
    }
  }

  fn remove_checkpoints(&mut self, slice: impl std::ops::RangeBounds<usize>) -> Result<()> {
    let to_remove: Vec<_> = self.manifest.checkpoints.drain(slice).collect();
    info!("Cleaning up {} checkpoints", to_remove.len());
    self.save_manifest()?;
    to_remove.into_iter().try_for_each(|checkpoint| {
      std::fs::remove_dir_all(Self::checkpoint_path(&self.root, checkpoint.id))
    })?;
    Ok(())
  }

  fn reset_head(&mut self, checkpoint_id: impl AsRef<Path>) -> Result<()> {
    let head_path = Self::head_path(&self.root);
    let checkpoint_path = Self::checkpoint_path(&self.root, checkpoint_id);

    let checkpoint_db = Storage::new(checkpoint_path)?;
    self.storage = checkpoint_db; // closes connection to current db
    Storage::destroy(&head_path)?;
    self.storage.save_copy(&head_path)?;
    self.storage = Storage::new(head_path)?;
    Ok(())
  }
}

impl<Storage: KVStorage> StateManager for PersistentStateManager<Storage> {
  type AppStateManager = PersistentAppStateManager<Storage>;

  fn init_app(&self, id: &str) -> Result<()> {
    self
      .apps
      .entry(id.to_owned())
      .or_try_insert_with(|| PersistentAppStateManager::new(self.app_path(id)))?;
    Ok(())
  }

  fn with_app<Out>(
    &self,
    id: &str,
    f: impl FnOnce(&mut Self::AppStateManager) -> Out,
  ) -> Result<Out> {
    let mut app = self
      .apps
      .entry(id.to_owned())
      .or_try_insert_with(|| PersistentAppStateManager::load(self.app_path(id)))?;
    Ok(f(&mut app))
  }
}

impl<Storage: KVStorage> AppStateManager for PersistentAppStateManager<Storage> {
  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<KeyValue>> {
    self.storage.get(keys)
  }

  fn set(&mut self, parts: Vec<KeyValue>) -> Result<()> {
    self.modifications_number += 1;
    self.storage.write(parts)
  }

  fn get_checkpoints(&self) -> Result<Vec<Checkpoint>> {
    Ok(self.manifest.checkpoints.clone())
  }

  fn create_checkpoint(&mut self, payload: &str) -> Result<String> {
    self.modifications_number += 1;
    let new_id = self.generate_checkpoint_id();

    self
      .storage
      .save_copy(Self::checkpoint_path(&self.root, &new_id))?;

    self.manifest.checkpoints.push(Checkpoint {
      id: new_id.clone(),
      payload: payload.to_owned(),
    });
    self.save_manifest()?;

    Ok(new_id)
  }

  fn revert(&mut self, id: &str) -> Result<()> {
    let index = self.find_checkpoint(id)?;
    self.modifications_number += 1;
    self.reset_head(id)?;
    self.remove_checkpoints((index + 1)..)?;
    Ok(())
  }

  fn cleanup(&mut self, until_checkpoint: &str) -> Result<()> {
    let index = self.find_checkpoint(until_checkpoint)?;
    self.modifications_number += 1;
    self.remove_checkpoints(..index)?;
    Ok(())
  }

  fn modifications_number(&self) -> u32 {
    self.modifications_number
  }
}
