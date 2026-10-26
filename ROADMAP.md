# Roadmap

What's planned for each version. Plans can change.

## v0.2.0

- [ ] CI: build and test both services, and check Rust dependencies with `cargo-deny`
- [ ] Rate-limit login per client
- [ ] SDKs for Python, Rust and .NET

## Later

- [ ] Encrypted connections (mTLS) between auth and the key provider, and the
      key provider checks who is calling it
- [ ] Per-app audiences
- [ ] Published Docker images
- [ ] Metrics and OpenTelemetry
- [ ] JSON logs
- [ ] Encrypt the key provider's Data Protection keys with a certificate
