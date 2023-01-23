use clap::Parser;
use file_storage::s3::S3FileStorage;
use grpc::GrpcService;
use log::info;
use proto::state_manager_service_server::StateManagerServiceServer;
use s3::{creds::Credentials, Bucket, Region};
use service::persistent::PersistentStateManager;
use storage::filesystem::FilesystemStorage;
use tonic::transport::Server;

mod file_storage;
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

  #[clap(env)]
  s3_endpoint: Option<String>,

  #[clap(env)]
  aws_access_key_id: Option<String>,

  #[clap(env)]
  aws_secret_access_key: Option<String>,
}

fn setup_logger(args: &Args) -> Result<(), fern::InitError> {
  fern::Dispatch::new()
    .format(|out, message, record| out.finish(format_args!("{:5} {}", record.level(), message)))
    .level(args.log_level)
    .chain(std::io::stderr())
    .apply()?;
  Ok(())
}

fn build_s3_storage(args: &Args) -> Option<S3FileStorage> {
  if let Some(s3_endpoint) = &args.s3_endpoint {
    let region = Region::Custom {
      endpoint: s3_endpoint.clone(),
      region: "us-east-1".to_string(),
    };
    let creds = Credentials::from_env().unwrap();
    let bucket = Bucket::new("state-manager-snapshots", region, creds)
      .unwrap()
      .with_path_style();
    Some(S3FileStorage::new(bucket))
  } else {
    None
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  setup_logger(&args)?;

  let addr = format!("0.0.0.0:{}", &args.port).parse()?;
  let mut service = GrpcService::new(PersistentStateManager::<FilesystemStorage>::new(
    &args.db_path,
  ));
  if let Some(storage) = build_s3_storage(&args) {
    service = service.with_snapshot_storage(storage);
  }

  let on_finish = Server::builder()
    .add_service(StateManagerServiceServer::new(service))
    .serve(addr);
  info!("Listening on {}", addr);

  on_finish.await?;
  Ok(())
}
