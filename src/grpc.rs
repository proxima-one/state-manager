use crate::proto::{self, state_manager_service_server::StateManagerService};
use crate::service::interface::{self, AppStateManager, StateManager};
use crate::types::{Error, KeyValue};
use rand::{distributions::Alphanumeric, Rng};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct GrpcService<StateManager> {
  manager: StateManager,
  // Some string which is different across process restarts
  run_id: String,
}

impl<TStateManager: StateManager> GrpcService<TStateManager> {
  pub fn new(manager: TStateManager) -> GrpcService<TStateManager> {
    let run_id = rand::thread_rng()
      .sample_iter(&Alphanumeric)
      .take(6)
      .map(char::from)
      .collect();
    GrpcService { manager, run_id }
  }

  pub fn get_etag(&self, app: &TStateManager::AppStateManager) -> String {
    format!("{}-{}", self.run_id, app.modifications_number())
  }

  pub fn check_etag(&self, etag: &str, app: &TStateManager::AppStateManager) -> Result<(), Status> {
    let expected = self.get_etag(app);
    if etag == expected {
      Ok(())
    } else {
      Err(Status::failed_precondition(format!(
        "Invalid etag: {}, expected: {}",
        etag, expected
      )))
    }
  }

  pub fn with_app<Out, Resp: WithEtag<Out>>(
    &self,
    id: &str,
    f: impl FnOnce(&mut TStateManager::AppStateManager) -> Result<Out, Status>,
  ) -> Result<Response<Resp>, Status> {
    self.manager.with_app(id, |app| {
      let result = f(app)?;
      Ok(Response::new(Resp::with_etag(result, self.get_etag(app))))
    })?
  }
}

#[tonic::async_trait]
impl<TStateManager: StateManager + 'static> StateManagerService for GrpcService<TStateManager> {
  async fn init_app(
    &self,
    request: Request<proto::InitAppRequest>,
  ) -> Result<Response<proto::InitAppResponse>, Status> {
    let request = request.into_inner();
    self.manager.init_app(&request.app_id)?;
    self
      .manager
      .with_app(&request.app_id, |app| {
        Ok(Response::new(proto::InitAppResponse {
          etag: self.get_etag(app),
        }))
      })
      .unwrap()
  }

  async fn get(
    &self,
    request: Request<proto::GetRequest>,
  ) -> Result<Response<proto::GetResponse>, Status> {
    let request = request.into_inner();
    self.with_app(&request.app_id, |app| {
      app.get(&request.keys).map_err(From::from)
    })
  }

  async fn set(
    &self,
    request: Request<proto::SetRequest>,
  ) -> Result<Response<proto::SetResponse>, Status> {
    let request = request.into_inner();
    self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      let parts = request
        .parts
        .into_iter()
        .map(|part| KeyValue {
          key: part.key,
          value: part.value,
        })
        .collect();
      app.set(parts).map_err(From::from)
    })
  }

  async fn checkpoints(
    &self,
    request: Request<proto::CheckpointsRequest>,
  ) -> Result<Response<proto::CheckpointsResponse>, Status> {
    let request = request.into_inner();
    self.with_app(&request.app_id, |app| {
      app.get_checkpoints().map_err(From::from)
    })
  }

  async fn create_checkpoint(
    &self,
    request: Request<proto::CreateCheckpointRequest>,
  ) -> Result<Response<proto::CreateCheckpointResponse>, Status> {
    let request = request.into_inner();
    self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      app.create_checkpoint(&request.payload).map_err(From::from)
    })
  }

  async fn revert(
    &self,
    request: Request<proto::RevertRequest>,
  ) -> Result<Response<proto::RevertResponse>, Status> {
    let request = request.into_inner();
    self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      app.revert(&request.checkpoint_id).map_err(From::from)
    })
  }

  async fn cleanup(
    &self,
    request: Request<proto::CleanupRequest>,
  ) -> Result<Response<proto::CleanupResponse>, Status> {
    let request = request.into_inner();
    self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      app.cleanup(&request.until_checkpoint).map_err(From::from)
    })
  }
}

impl From<KeyValue> for proto::Part {
  fn from(part: KeyValue) -> Self {
    Self {
      key: part.key,
      value: part.value,
    }
  }
}
impl From<interface::Checkpoint> for proto::Checkpoint {
  fn from(checkpoint: interface::Checkpoint) -> Self {
    Self {
      id: checkpoint.id,
      payload: checkpoint.payload,
    }
  }
}

pub trait WithEtag<F> {
  fn with_etag(from: F, etag: impl Into<String>) -> Self;
}
impl WithEtag<()> for proto::InitAppResponse {
  fn with_etag(_from: (), etag: impl Into<String>) -> Self {
    Self { etag: etag.into() }
  }
}
impl WithEtag<Vec<KeyValue>> for proto::GetResponse {
  fn with_etag(from: Vec<KeyValue>, etag: impl Into<String>) -> Self {
    Self {
      etag: etag.into(),
      parts: from.into_iter().map(From::from).collect(),
    }
  }
}
impl WithEtag<()> for proto::SetResponse {
  fn with_etag(_from: (), etag: impl Into<String>) -> Self {
    Self { etag: etag.into() }
  }
}
impl WithEtag<Vec<interface::Checkpoint>> for proto::CheckpointsResponse {
  fn with_etag(from: Vec<interface::Checkpoint>, etag: impl Into<String>) -> Self {
    Self {
      etag: etag.into(),
      checkpoints: from.into_iter().map(From::from).collect(),
    }
  }
}
impl WithEtag<String> for proto::CreateCheckpointResponse {
  fn with_etag(from: String, etag: impl Into<String>) -> Self {
    Self {
      etag: etag.into(),
      id: from,
    }
  }
}
impl WithEtag<()> for proto::RevertResponse {
  fn with_etag(_from: (), etag: impl Into<String>) -> Self {
    Self { etag: etag.into() }
  }
}
impl WithEtag<()> for proto::CleanupResponse {
  fn with_etag(_from: (), etag: impl Into<String>) -> Self {
    Self { etag: etag.into() }
  }
}

impl From<Error> for Status {
  fn from(err: Error) -> Self {
    match err {
      Error::NotFound(message) => Self::not_found(message),
      Error::DbError(message) => Self::internal(message),
    }
  }
}
