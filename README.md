# weatherlab

Playground for weather tooling — gRPC services, MCP servers, whatever I'm
learning next. Things here are half-finished on purpose.

## What's here

- `proto/` — protobuf definitions (`weatherlab.*`), built with [buf](https://buf.build)
- `apps/` — anything deployable, named `<domain>-<kind>` when a domain has more than one
  - `auth/` — Rust auth service (tonic + sqlx + Postgres, Argon2 password hashing, JWT)
  - `key-provider/` — .NET signing service (gRPC, generated stubs in `src/Generated`)
  - `weather/` — Python REST API that uses auth (generated stubs in `gen/`)
- `libs/` — shared code (empty for now)
- `codegen/` — per-app `buf generate` templates

Workspace roots live at the top: `Cargo.toml`, `pyproject.toml` (uv), `weatherlab.slnx`.

## Running

```sh
docker compose up -d        # Postgres + Redis
just generate               # regenerate Python + C# stubs after proto changes
cargo run -p auth
dotnet run --project apps/key-provider/src
uv run --package weather python apps/weather/main.py
```
