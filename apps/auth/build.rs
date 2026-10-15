fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../../proto");

    // auth: server only
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .skip_debug([
            ".outpost.auth.v1.RegisterRequest",
            ".outpost.auth.v1.LoginRequest",
            ".outpost.auth.v1.TokenPair",
            ".outpost.auth.v1.RefreshTokenRequest",
            ".outpost.auth.v1.LogoutRequest",
            ".outpost.auth.v1.LogoutAllRequest",
        ])
        .compile_protos(
            &["../../proto/outpost/auth/v1/auth.proto"],
            &["../../proto"],
        )?;

    // key: client only
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .skip_debug([".outpost.key.v1.SignTokenResponse"])
        .compile_protos(&["../../proto/outpost/key/v1/key.proto"], &["../../proto"])?;

    Ok(())
}
