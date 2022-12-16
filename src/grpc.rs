use crate::proto::{self, state_manager_service_server::StateManagerService};
use crate::service::interface::{self, AppStateManager, StateManager};
use crate::types::{Error, KeyValue};
use log::{error, info};
use rand::{distributions::Alphanumeric, Rng};
use std::fmt::Display;
use tonic::{Request, Response, Status};

const ADMIN_TOKEN: &str = "iknowwhatimdoing";

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
    let start = std::time::Instant::now();
    let result = self.manager.with_app(id, |app| {
      let result = f(app)?;
      Ok(Response::new(Resp::with_etag(result, self.get_etag(app))))
    })?;
    info!("App request handled in {:?}", start.elapsed());
    result
  }

  fn remove_app(&self, id: &str, admin_token: &str) -> Result<Response<proto::RemoveAppResponse>, Status> {
    if admin_token != ADMIN_TOKEN {
      return Err(tonic::Status::permission_denied("Unauthorized"));
    }
    self.manager.drop_app(id)?;
    Ok(Response::new(proto::RemoveAppResponse {}))
  }
}

#[tonic::async_trait]
impl<TStateManager: StateManager + 'static> StateManagerService for GrpcService<TStateManager> {
  async fn init_app(
    &self,
    request: Request<proto::InitAppRequest>,
  ) -> Result<Response<proto::InitAppResponse>, Status> {
    let request = request.into_inner();
    let result = self
      .manager
      .init_app(&request.app_id)
      .map_err(From::from)
      .and_then(|()| {
        self
          .manager
          .with_app(&request.app_id, |app| {
            Ok(Response::new(proto::InitAppResponse {
              etag: self.get_etag(app),
            }))
          })
          .unwrap()
      });
    log(&request, &result);
    result
  }

  async fn get(
    &self,
    request: Request<proto::GetRequest>,
  ) -> Result<Response<proto::GetResponse>, Status> {
    let request = request.into_inner();
    let result = self.with_app(&request.app_id, |app| {
      app.get(&request.keys).map_err(From::from)
    });
    log(&request, &result);
    result
  }

  async fn set(
    &self,
    request: Request<proto::SetRequest>,
  ) -> Result<Response<proto::SetResponse>, Status> {
    let request = request.into_inner();
    let result = self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      let parts = request
        .parts
        .iter()
        .map(|part| KeyValue {
          key: part.key.clone(),
          value: part.value.clone(),
        })
        .collect();
      app.set(parts).map_err(From::from)
    });
    log(&request, &result);
    result
  }

  async fn checkpoints(
    &self,
    request: Request<proto::CheckpointsRequest>,
  ) -> Result<Response<proto::CheckpointsResponse>, Status> {
    let request = request.into_inner();
    let result = self.with_app(&request.app_id, |app| {
      app.get_checkpoints().map_err(From::from)
    });
    log(&request, &result);
    result
  }

  async fn create_checkpoint(
    &self,
    request: Request<proto::CreateCheckpointRequest>,
  ) -> Result<Response<proto::CreateCheckpointResponse>, Status> {
    let request = request.into_inner();
    let result = self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      app.create_checkpoint(&request.payload).map_err(From::from)
    });
    log(&request, &result);
    result
  }

  async fn revert(
    &self,
    request: Request<proto::RevertRequest>,
  ) -> Result<Response<proto::RevertResponse>, Status> {
    let request = request.into_inner();
    let result = self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      app.revert(&request.checkpoint_id).map_err(From::from)
    });
    log(&request, &result);
    result
  }

  async fn cleanup(
    &self,
    request: Request<proto::CleanupRequest>,
  ) -> Result<Response<proto::CleanupResponse>, Status> {
    let request = request.into_inner();
    let result = self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      app.cleanup(&request.until_checkpoint).map_err(From::from)
    });
    log(&request, &result);
    result
  }

  async fn reset(
    &self,
    request: Request<proto::ResetRequest>,
  ) -> Result<Response<proto::ResetResponse>, Status> {
    let request = request.into_inner();
    let result = self.with_app(&request.app_id, |app| {
      self.check_etag(&request.etag, app)?;
      app.reset().map_err(From::from)
    });
    log(&request, &result);
    result
  }

  async fn remove_app(
    &self,
    request: Request<proto::RemoveAppRequest>,
  ) -> Result<Response<proto::RemoveAppResponse>, Status> {
    let request = request.into_inner();
    let result = self.remove_app(&request.app_id, &request.admin_token);
    log(&request, &result);
    result
  }
}

fn log<T>(request: &impl Display, result: &Result<Response<T>, Status>) {
  match result {
    Ok(_response) => {
      info!("{} => OK", request);
    }
    Err(status) => {
      error!("{} => {}: {:?}", request, status.code(), status.message());
    }
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
impl WithEtag<()> for proto::ResetResponse {
  fn with_etag(_from: (), etag: impl Into<String>) -> Self {
    Self { etag: etag.into() }
  }
}

impl From<Error> for Status {
  fn from(err: Error) -> Self {
    match err {
      Error::NotFound(message) => Self::not_found(message),
      Error::DbError(message) => Self::internal(message),
      Error::IoError(err) => err.into(),
      Error::S3Error(err) => Self::unknown(format!("{}", err))
    }
  }
}

impl Display for proto::InitAppRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}]: InitApp()", self.app_id)
  }
}

impl Display for proto::GetRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}]: Get({:?})", self.app_id, self.keys)
  }
}

impl Display for proto::SetRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "[{}]: Set({:?}))",
      self.app_id,
      self.parts.iter().map(|part| &part.key).collect::<Vec<_>>()
    )
  }
}

impl Display for proto::CheckpointsRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}]: Checkpoints()", self.app_id)
  }
}

impl Display for proto::CreateCheckpointRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "[{}]: CreateCheckpoint(payload: {:?})",
      self.app_id, self.payload
    )
  }
}

impl Display for proto::RevertRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}]: Revert({:?})", self.app_id, self.checkpoint_id)
  }
}

impl Display for proto::CleanupRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}]: Cleanup({:?})", self.app_id, self.until_checkpoint)
  }
}

impl Display for proto::ResetRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}]: Reset()", self.app_id)
  }
}

impl Display for proto::RemoveAppRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}]: RemoveApp()", self.app_id)
  }
}
