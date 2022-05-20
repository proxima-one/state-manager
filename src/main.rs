use tonic::{transport::Server, Request, Response, Status};
use proto::{
  state_manager_service_server::{
    StateManagerService,
    StateManagerServiceServer
  },
  InitAppRequest,
  InitAppResponse,
  GetRequest,
  GetResponse,
  SetRequest,
  SetResponse,
  CheckpointsRequest,
  CheckpointsResponse,
  CreateCheckpointRequest,
  CreateCheckpointResponse,
  RevertRequest,
  RevertResponse,
  CleanupRequest,
  CleanupResponse,
};

pub mod proto;


#[derive(Debug, Default)]
pub struct StateManager {
}

#[tonic::async_trait]
impl StateManagerService for StateManager {
  async fn init_app(
    &self,
    request: Request<InitAppRequest>,
  ) -> Result<Response<InitAppResponse>, Status> {
    return Err(Status::unimplemented("Unimplemented"));
  }

  async fn get(
    &self,
    request: Request<GetRequest>,
  ) -> Result<Response<GetResponse>, Status> {
    return Err(Status::unimplemented("Unimplemented"));
  }

  async fn set(
    &self,
    request: Request<SetRequest>,
  ) -> Result<Response<SetResponse>, Status> {
    return Err(Status::unimplemented("Unimplemented"));
  }

  async fn checkpoints(
    &self,
    request: Request<CheckpointsRequest>,
  ) -> Result<Response<CheckpointsResponse>, Status> {
    return Err(Status::unimplemented("Unimplemented"));
  }

  async fn create_checkpoint(
    &self,
    request: Request<CreateCheckpointRequest>,
  ) -> Result<Response<CreateCheckpointResponse>, Status> {
    return Err(Status::unimplemented("Unimplemented"));
  }

  async fn revert(
    &self,
    request: Request<RevertRequest>,
  ) -> Result<Response<RevertResponse>, Status> {
    return Err(Status::unimplemented("Unimplemented"));
  }

  async fn cleanup(
    &self,
    request: Request<CleanupRequest>,
  ) -> Result<Response<CleanupResponse>, Status> {
    return Err(Status::unimplemented("Unimplemented"));
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr = "[::1]:50051".parse()?;
  let service = StateManager::default();

  println!("Listening on {}", addr);

  Server::builder()
    .add_service(StateManagerServiceServer::new(service))
    .serve(addr)
    .await?;

  Ok(())
}
