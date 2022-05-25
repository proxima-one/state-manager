use thiserror::Error;

pub type Bytes = Vec<u8>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct KeyValue {
  pub key: String,
  pub value: Bytes,
}

#[derive(Debug, Error)]
pub enum Error {
  #[error("{0}")]
  NotFound(String),

  #[error("DB error: {0}")]
  DbError(String),

  #[error(transparent)]
  IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
