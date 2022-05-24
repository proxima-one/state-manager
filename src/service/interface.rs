use std::io::Result;

pub trait StateManager: Sync + Send {
  type AppStateManager: AppStateManager;

  fn init_app(&self, id: &str) -> Result<()>;

  // We can't just return &mut AppStateManager because it would reference a local-scope RAII guard
  fn with_app<Out>(
    &self,
    id: &str,
    f: impl FnOnce(&mut Self::AppStateManager) -> Out,
  ) -> Result<Out>;
}

pub trait AppStateManager: Sync + Send {
  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<Part>>;
  fn set(&mut self, parts: Vec<Part>) -> Result<()>;
  fn get_checkpoints(&self) -> Result<Vec<Checkpoint>>;
  fn create_checkpoint(&mut self, payload: &str) -> Result<String>;
  fn revert(&mut self, id: &str) -> Result<()>;
  fn cleanup(&mut self, until_checkpoint: &str) -> Result<()>;

  fn modifications_number(&self) -> u32;
}

pub type Bytes = Vec<u8>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Part {
  pub key: String,
  pub value: Bytes,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Checkpoint {
  pub id: String,
  pub payload: String,
}
