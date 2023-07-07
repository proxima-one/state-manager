use super::interface::{AppStateManager, Checkpoint, StateManager};
use crate::file_storage::interface::FileStorage;
use crate::storage::interface::KVStorage;
use crate::types::{Error, KeyValue, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppManifest {
  checkpoints: Vec<Checkpoint>,
  version: Option<String>,
}

impl Default for AppManifest {
  fn default() -> Self {
    Self {
      checkpoints: Vec::new(),
      version: Some("1".to_owned()),
    }
  }
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
  storage: Option<Storage>,
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
    Self::load(root)
  }

  fn load(root: PathBuf) -> Result<Self> {
    if !root.is_dir() {
      return Err(Error::NotFound("Application does not exist".to_owned()));
    }
    let manifest = Self::load_manifest(Self::manifest_path(&root))?;

    let mut result = Self {
      root: root.clone(),
      manifest,
      storage: None,
      modifications_number: 0,
    };

    result.storage = Some(Storage::open(Self::head_path(&root))?);
    result.restore_consistency()?;
    Ok(result)
  }

  fn storage(&self) -> &Storage {
    self.storage.as_ref().unwrap()
  }

  fn storage_mut(&mut self) -> &mut Storage {
    self.storage.as_mut().unwrap()
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

  fn restore_consistency(&mut self) -> Result<()> {
    let mut existing = HashSet::new();
    for entry in std::fs::read_dir(Self::checkpoints_dir(&self.root))? {
      let entry = entry?;
      existing.insert(
        entry
          .path()
          .file_name()
          .unwrap()
          .to_str()
          .unwrap()
          .to_owned(),
      );
    }
    let recorded: HashSet<_> = self
      .manifest
      .checkpoints
      .iter()
      .map(|cp| cp.id.clone())
      .collect();

    for id in existing.difference(&recorded) {
      self.remove_checkpoint(id)?;
    }

    let prev_len = self.manifest.checkpoints.len();
    self
      .manifest
      .checkpoints
      .retain(|cp| existing.contains(&cp.id));
    if prev_len != self.manifest.checkpoints.len() {
      self.save_manifest()?;
    }

    Ok(())
  }

  fn get_checkpoint_ids(&self) -> Vec<i32> {
    self
      .manifest
      .checkpoints
      .iter()
      .map(|checkpoint| {
        checkpoint
          .id
          .parse()
          .expect("Non numerical checkpoint name in manifest")
      })
      .collect()
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

  fn remove_checkpoint(&mut self, id: &str) -> Result<()> {
    std::fs::remove_dir_all(Self::checkpoint_path(&self.root, id)).map_err(From::from)
  }

  fn remove_checkpoints(&mut self, slice: impl std::ops::RangeBounds<usize>) -> Result<()> {
    let to_remove: Vec<_> = self.manifest.checkpoints.drain(slice).collect();
    info!("Cleaning up {} checkpoints", to_remove.len());
    self.save_manifest()?;
    to_remove
      .into_iter()
      .try_for_each(|checkpoint| self.remove_checkpoint(&checkpoint.id))?;
    Ok(())
  }

  // TODO: optimize for a generic case
  fn reset_head(&mut self, checkpoint_id: impl AsRef<Path>) -> Result<()> {
    let head_path = Self::head_path(&self.root);
    let checkpoint_path = Self::checkpoint_path(&self.root, checkpoint_id);

    self.storage = None; // closes connection to current db
    Storage::destroy(&head_path)?;
    let checkpoint_db = Storage::open(checkpoint_path)?;
    checkpoint_db.save_copy(&head_path)?;
    self.storage = Some(Storage::open(head_path)?);
    Ok(())
  }

  fn clean_head(&mut self) -> Result<()> {
    let head_path = Self::head_path(&self.root);
    self.storage = None;
    Storage::destroy(&head_path)?;
    self.storage = Some(Storage::open(head_path)?);
    Ok(())
  }
}

#[async_trait]
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

  async fn store_snapshot(
    &self,
    app_id: &str,
    storage: &impl FileStorage,
    prefix: &std::path::Path,
  ) -> Result<()> {
    let app = self
      .apps
      .entry(app_id.to_owned())
      .or_try_insert_with(|| PersistentAppStateManager::load(self.app_path(app_id)))?;
    app.store_snapshot(storage, prefix).await
  }

  fn drop_app(&self, id: &str) -> Result<()> {
    std::fs::remove_dir_all(self.app_path(id))?;
    self.apps.remove(id);
    Ok(())
  }
}

#[async_trait]
impl<Storage: KVStorage> AppStateManager for PersistentAppStateManager<Storage> {
  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<KeyValue>> {
    self.storage().get(keys)
  }

  fn set(&mut self, parts: Vec<KeyValue>) -> Result<()> {
    self.storage_mut().write(parts)?;
    self.modifications_number += 1;
    Ok(())
  }

  fn get_checkpoints(&self) -> Result<Vec<Checkpoint>> {
    Ok(self.manifest.checkpoints.clone())
  }

  fn create_checkpoint(&mut self, payload: &str) -> Result<String> {
    let ids = self.get_checkpoint_ids();
    let (kept, removed) = if !ids.is_empty() {
      crate::utils::exponential_sequence::extend(&ids)
    } else {
      (vec![0], vec![])
    };
    let new_id = kept.last().unwrap().to_string();
    let kept: HashSet<_> = kept.iter().map(|x| x.to_string()).collect();

    self
      .storage()
      .save_copy(Self::checkpoint_path(&self.root, &new_id))?;

    self.manifest.checkpoints.push(Checkpoint {
      id: new_id.clone(),
      payload: payload.to_owned(),
    });
    self
      .manifest
      .checkpoints
      .retain(|checkpoint| kept.contains(&checkpoint.id));
    self.save_manifest()?;

    for id in removed {
      self.remove_checkpoint(&id.to_string())?;
    }

    self.modifications_number += 1;
    Ok(new_id)
  }

  fn revert(&mut self, id: &str) -> Result<()> {
    let index = self.find_checkpoint(id)?;
    self.reset_head(id)?;
    self.remove_checkpoints((index + 1)..)?;
    self.modifications_number += 1;
    Ok(())
  }

  fn cleanup(&mut self, until_checkpoint: &str) -> Result<()> {
    let index = self.find_checkpoint(until_checkpoint)?;
    self.remove_checkpoints(..index)?;
    self.modifications_number += 1;
    Ok(())
  }

  fn reset(&mut self) -> Result<()> {
    self.clean_head()?;
    self.modifications_number += 1;
    Ok(())
  }

  async fn store_snapshot(
    &self,
    storage: &impl FileStorage,
    prefix: &std::path::Path,
  ) -> Result<()> {
    let checkpoint = self.manifest.checkpoints.last().ok_or(Error::NotFound(
      "Can't create a snapshot because there are no checkpoints yet".to_owned(),
    ))?;
    let checkpoint_id = &checkpoint.id;
    let checkpoint_path = Self::checkpoint_path(&self.root, checkpoint_id);
    storage
      .upload_folder(
        &checkpoint_path,
        &Self::checkpoint_path(prefix, checkpoint_id),
      )
      .await?;

    let mut manifest = AppManifest::default();
    manifest.checkpoints.push(checkpoint.clone());
    let manifest_contents: Vec<u8> = serde_json::to_string(&manifest)
      .map_err(std::io::Error::from)?
      .bytes()
      .collect();
    storage
      .upload_buffer(&manifest_contents, &Self::manifest_path(prefix))
      .await?;
    Ok(())
  }

  fn modifications_number(&self) -> u32 {
    self.modifications_number
  }
}
