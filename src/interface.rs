use anyhow::Result;


pub trait StateManager {
  type AppStateManager: AppStateManager;

  fn init_app(&mut self, id: &str) -> Result<()>;
  fn get_app(&mut self, id: &str) -> Result<&mut Self::AppStateManager>;
}

pub trait AppStateManager {
  fn get<Key: AsRef<str>>(&self, keys: &[Key]) -> Result<Vec<Part>>;
  fn set<TPart: AsRef<Part>>(&mut self, parts: &[TPart]) -> Result<()>;
  fn get_checkpoints(&self) -> Result<Vec<Checkpoint>>;
  fn create_checkpoint(&mut self, payload: &str) -> Result<String>;
  fn revert(&mut self, id: &str) -> Result<()>;
  fn cleanup(&mut self, until_checkpoint: &str) -> Result<()>;

  fn modifications_number(&self) -> u32;
}

pub type Bytes = Vec<u8>;

pub struct Part {
  pub key: String,
  pub value: Bytes,
}

pub struct Checkpoint {
  pub id: String,
  pub payload: String,
}
