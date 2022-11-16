use super::filesystem::FilesystemStorage;
use super::interface::KVStorage;
use crate::types::KeyValue;

fn part(key: impl AsRef<str>, value: impl AsRef<[u8]>) -> KeyValue {
  KeyValue {
    key: key.as_ref().to_owned(),
    value: value.as_ref().to_owned(),
  }
}

#[test]
fn test_filesystem_storage() {
  const PATH0: &str = "test_db_0";
  const PATH1: &str = "test_db_1";

  FilesystemStorage::destroy(PATH0).unwrap();
  FilesystemStorage::destroy(PATH1).unwrap();

  let mut storage = FilesystemStorage::open(PATH0).unwrap();
  assert!(storage.get_one("a").is_err());
  storage.write(vec![part("a", "123\n456"), part("b", "")]).unwrap();
  assert_eq!(storage.get(&["a", "b", "c"]).unwrap(), vec![part("a", "123\n456"), part("b", "")]);
  storage.save_copy(PATH1).unwrap();

  drop(storage);

  let storage = FilesystemStorage::open(PATH1).unwrap();
  assert_eq!(storage.get_one("a").unwrap(), b"123\n456");
}
