use super::in_memory::InMemoryStateManager;
use super::interface::{AppStateManager, Checkpoint, StateManager};
use crate::types::KeyValue;

fn part(key: impl AsRef<str>, value: impl AsRef<[u8]>) -> KeyValue {
  KeyValue {
    key: key.as_ref().to_owned(),
    value: value.as_ref().to_owned(),
  }
}

fn test_service(manager: &impl StateManager) {
  const APP_ID: &str = "test";
  manager.init_app(APP_ID).unwrap();
  assert!(manager.init_app(APP_ID).is_err());
  manager
    .with_app(APP_ID, |app| {
      assert_eq!(app.modifications_number(), 0);
      assert!(app.get_checkpoints().unwrap().is_empty());

      let checkpoint0 = app.create_checkpoint("0").unwrap();
      app.revert(&checkpoint0).unwrap();
      assert!(app.get(&["a", "b", "c"]).unwrap().is_empty());
      app.set(vec![part("a", "0"), part("b", "0")]).unwrap();
      app.set(vec![part("a", "1")]).unwrap();
      assert_eq!(
        app.get(&["a", "b", "c"]).unwrap(),
        vec![part("a", "1"), part("b", "0")]
      );

      let checkpoint1 = app.create_checkpoint("1").unwrap();
      assert_eq!(
        app.get(&["a", "b", "c"]).unwrap(),
        vec![part("a", "1"), part("b", "0")]
      );
      app.set(vec![part("a", "2"), part("c", "2")]).unwrap();
      app.cleanup(&checkpoint1).unwrap();
      assert_eq!(
        app.get_checkpoints().unwrap(),
        vec![Checkpoint {
          id: checkpoint1.clone(),
          payload: "1".to_owned()
        }]
      );
      assert!(app.revert(&checkpoint0).is_err());
      assert_eq!(
        app.get(&["a", "b", "c"]).unwrap(),
        vec![part("a", "2"), part("b", "0"), part("c", "2")]
      );

      app.revert(&checkpoint1).unwrap();
      assert_eq!(
        app.get(&["a", "b", "c"]).unwrap(),
        vec![part("a", "1"), part("b", "0")]
      );

      assert_eq!(app.modifications_number(), 8);
    })
    .unwrap();
}

#[test]
fn test_basic() {
  let manager = InMemoryStateManager::default();
  test_service(&manager);
}
