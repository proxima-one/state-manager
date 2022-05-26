use clap::Parser;
use grpc::GrpcService;
use proto::state_manager_service_server::StateManagerServiceServer;
use service::persistent::PersistentStateManager;
use storage::rocksdb::RocksdbStorage;
use tonic::transport::Server;

mod grpc;
mod service;
mod storage;
mod types;
mod proto {
  tonic::include_proto!("state_manager");
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
  #[clap(short, long)]
  port: u16,

  #[clap(short, long, default_value = "/run/state-manager")]
  db_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();

  let addr = format!("[::1]:{}", args.port).parse()?;
  let service = GrpcService::new(PersistentStateManager::<RocksdbStorage>::new(args.db_path));

  println!("Listening on {}", addr);

  Server::builder()
    .add_service(StateManagerServiceServer::new(service))
    .serve(addr)
    .await?;

  Ok(())
}
