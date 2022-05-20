fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/state_manager/state_manager.proto")?;
    Ok(())
}
