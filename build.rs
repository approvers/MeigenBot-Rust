#[cfg(feature = "api_grpc")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/meigen_api.proto")?;
    Ok(())
}
