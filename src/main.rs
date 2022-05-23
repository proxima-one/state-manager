use grpc::GrpcService;
use impls::in_memory::InMemoryStateManager;
use proto::state_manager_service_server::StateManagerServiceServer;
use tonic::transport::Server;

mod grpc;
mod impls;
mod interface;
mod proto;

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
