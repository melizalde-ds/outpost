fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../../proto");

    // auth: server only
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &["../../proto/outpost/auth/v1/auth.proto"],
            &["../../proto"],
        )?;

    // key: client only
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../../proto/outpost/key/v1/key.proto"],
            &["../../proto"],
        )?;

    Ok(())
}
