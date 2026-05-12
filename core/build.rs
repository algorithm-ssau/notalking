fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto = "../common/proto/notalking/v1/core.proto";
    println!("cargo:rerun-if-changed={proto}");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[proto], &["../common/proto"])?;
    Ok(())
}
