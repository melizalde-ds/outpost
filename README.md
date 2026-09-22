# weatherlab

Playground for weather tooling — gRPC services, MCP servers, whatever I'm
learning next. Things here are half-finished on purpose.

## What's here

- `proto/` — protobuf definitions (`weatherlab.*`), built with [buf](https://buf.build)
- `auth/` — Rust auth service (tonic + sqlx + Postgres, Argon2 password hashing, JWT)
- `weather/` — Python client package (`weatherlab`), generated stubs + hand-written wrappers

## Running

```sh
docker compose up -d        # Postgres + Redis
buf generate                # regenerate Python stubs after proto changes
cargo run --manifest-path auth/Cargo.toml
```
