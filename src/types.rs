pub type Bytes = Vec<u8>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct KeyValue {
  pub key: String,
  pub value: Bytes,
}
