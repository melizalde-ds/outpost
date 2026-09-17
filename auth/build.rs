fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../proto");

    tonic_prost_build::configure()
        .build_client(false)
        .compile_protos(&["../proto/auth.proto"], &["../proto"])?;

    Ok(())
}
