use crate::file_storage::interface::FileStorage;
use crate::types::{KeyValue, Result};
use async_trait::async_trait;

#[async_trait]
pub trait StateManager: Sync + Send {
  type AppStateManager: AppStateManager;

  fn init_app(&self, id: &str) -> Result<()>;

  // We can't just return &mut AppStateManager because it would reference a local-scope RAII guard
  fn with_app<Out>(
    &self,
    id: &str,
    f: impl FnOnce(&mut Self::AppStateManager) -> Out,
  ) -> Result<Out>;

  // TODO: find a way to do the same as above for async functions
  async fn store_snapshot(
    &self,
    app_id: &str,
    storage: &impl FileStorage,
    prefix: &std::path::Path,
  ) -> Result<()>;

  fn drop_app(&self, id: &str) -> Result<()>;
}

#[async_trait]
pub trait AppStateManager: Sync + Send {
  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<KeyValue>>;
  fn set(&mut self, parts: Vec<KeyValue>) -> Result<()>;
  fn get_checkpoints(&self) -> Result<Vec<Checkpoint>>;
  fn create_checkpoint(&mut self, payload: &str) -> Result<String>;
  fn revert(&mut self, id: &str) -> Result<()>;
  fn cleanup(&mut self, until_checkpoint: &str) -> Result<()>;
  fn reset(&mut self) -> Result<()>;

  async fn store_snapshot(
    &self,
    storage: &impl FileStorage,
    prefix: &std::path::Path,
  ) -> Result<()>;

  fn modifications_number(&self) -> u32;
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Checkpoint {
  pub id: String,
  pub payload: String,
}
