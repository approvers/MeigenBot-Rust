fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "api_grpc")]
    tonic_build::compile_protos("proto/meigen_api.proto")?;
    Ok(())
}
