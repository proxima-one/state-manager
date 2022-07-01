use clap::Parser;
use grpc::GrpcService;
use log::info;
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
mod utils;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
  #[clap(long, env)]
  port: u16,

  #[clap(long, env, default_value = "/run/state-manager")]
  db_path: String,

  #[clap(env, default_value_t = log::LevelFilter::Info)]
  log_level: log::LevelFilter,
}

fn setup_logger(args: &Args) -> Result<(), fern::InitError> {
  fern::Dispatch::new()
    .format(|out, message, record| {
      out.finish(format_args!(
        "{:5} {}",
        record.level(),
        message
      ))
    })
    .level(args.log_level)
    .chain(std::io::stderr())
    .apply()?;
  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  setup_logger(&args)?;

  let addr = format!("0.0.0.0:{}", args.port).parse()?;
  let service = GrpcService::new(PersistentStateManager::<RocksdbStorage>::new(args.db_path));

  let on_finish = Server::builder()
    .add_service(StateManagerServiceServer::new(service))
    .serve(addr);
  info!("Listening on {}", addr);

  on_finish.await?;
  Ok(())
}
