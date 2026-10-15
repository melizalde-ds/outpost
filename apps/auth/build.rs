use std::{env, path::PathBuf};

include!("src/redacted_messages.rs");

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../../proto");
    println!("cargo:rerun-if-changed=src/redacted_messages.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    // auth: server only
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .skip_debug(REDACTED_MESSAGES)
        .file_descriptor_set_path(out_dir.join("auth_descriptors.bin"))
        .compile_protos(
            &["../../proto/outpost/auth/v1/auth.proto"],
            &["../../proto"],
        )?;

    // key: client only
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .skip_debug(REDACTED_MESSAGES)
        .file_descriptor_set_path(out_dir.join("key_descriptors.bin"))
        .compile_protos(&["../../proto/outpost/key/v1/key.proto"], &["../../proto"])?;

    Ok(())
}
