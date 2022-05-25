use grpc::GrpcService;
use proto::state_manager_service_server::StateManagerServiceServer;
use service::in_memory::InMemoryStateManager;
use tonic::transport::Server;

mod grpc;
mod service;
mod storage;
mod types;
mod proto {
  tonic::include_proto!("state_manager");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr = "[::1]:50051".parse()?;
  let service = GrpcService::new(InMemoryStateManager::default());

  println!("Listening on {}", addr);

  Server::builder()
    .add_service(StateManagerServiceServer::new(service))
    .serve(addr)
    .await?;

  Ok(())
}
