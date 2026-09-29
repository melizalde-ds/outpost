fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../../proto");

    tonic_prost_build::configure()
        .build_client(false)
        .compile_protos(
            &["../../proto/weatherlab/auth/v1/auth.proto"],
            &["../../proto"],
        )?;

    Ok(())
}
