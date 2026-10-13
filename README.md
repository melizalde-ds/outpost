# Outpost

Outpost is a set of self-hosted backend services. You deploy them once and
point your apps at them, the way apps point at Supabase. Each service runs in
its own container.

Auth is the first service.

## Auth

Auth registers users and logs them in over gRPC. A successful `Register` or
`Login` returns a short-lived access token: a JWT signed with Ed25519.

| RPC            | Status                                                      |
| -------------- | ----------------------------------------------------------- |
| `Register`     | Creates the user and returns an access token.               |
| `Login`        | Checks the password and returns an access token.            |
| `RefreshToken` | Defined in the API, not implemented yet. See the roadmap.   |

The API is defined in [`proto/outpost/auth/v1/auth.proto`](proto/outpost/auth/v1/auth.proto).

Errors come back as gRPC status codes:

| Code              | Meaning                                         |
| ----------------- | ----------------------------------------------- |
| `UNAUTHENTICATED` | Wrong password, or the account is inactive.     |
| `NOT_FOUND`       | No user with that username.                     |
| `ALREADY_EXISTS`  | The username is taken.                          |
| `UNAVAILABLE`     | The key-provider or Redis is down. Retry later. |

## Architecture

```
your app ──gRPC──> auth (Rust) ──gRPC──> key-provider (.NET)
                     │    │                    │
                     │    └──> Redis           └──> Postgres (keys)
                     └──> Postgres (users)
```

- **auth** (`apps/auth`) is written in Rust with tonic and sqlx. It stores
  users in Postgres and never holds a private key.
- **key-provider** (`apps/key-provider`) is written in .NET. It owns the
  signing keys, signs tokens when auth asks, and rotates keys on a schedule.
- **Redis** holds the public verification keys. Auth publishes them on startup
  and whenever the key-provider hands back a new one. Each key is stored as a
  JWK under `jwk:<key id>` and expires when the key does.

## Security choices

- **Argon2 with a pepper.** Passwords are hashed with Argon2id plus a
  server-side secret that is not stored in the database.
- **Secrets stay out of logs.** Passwords, the pepper and the database
  password are wrapped in secret types that don't print. Token and password
  fields are marked `debug_redact` in the proto files. The key-provider zeroes
  the private key bytes it held when it shuts down.
- **Separate database users.** Each database has one role for migrations and
  one for the running service. The auth service role can only read and insert
  users.
- **Append-only key tables.** Signing keys and rotation jobs can be inserted
  but not updated or deleted. Postgres triggers enforce this, so it holds even
  for someone connected directly to the database.
- **Encrypted private keys.** Private keys are encrypted with ASP.NET Core
  Data Protection before they are stored. The Data Protection keys live in a
  separate database with its own users.
- **Scheduled key rotation.** Keys rotate on a cron schedule (every 8 hours by
  default, with a one-day key lifespan). Failed rotations are retried with
  backoff and a timeout per attempt.

### Database error codes

The key-provider triggers raise two custom Postgres error codes:

| Code    | Raised when                                                                 |
| ------- | --------------------------------------------------------------------------- |
| `WLA01` | Something tries to update or delete a row in an append-only table.          |
| `WLA02` | A key pair is inserted for a rotation job that is missing or did not succeed. |

The `WLA` prefix comes from the project's earlier name. The codes are part of
migrations that have already been applied, so they stay as they are.

## Running it locally

You need Docker, Rust, the .NET 10 SDK, [just](https://github.com/casey/just)
and [sqlx-cli](https://crates.io/crates/sqlx-cli). Changing the proto files
also needs [buf](https://buf.build) and [uv](https://docs.astral.sh/uv/).

1. Create the env files and replace the `change-me` values:

   ```sh
   cp .env.example .env
   cp apps/auth/.env.example apps/auth/.env
   cp apps/key-provider/.env.example apps/key-provider/.env
   ```

2. Start Postgres and Redis:

   ```sh
   docker compose up -d
   ```

   This starts the auth database on port 5432, the key-provider database on
   port 5433 and Redis on port 6379. The first start creates the database
   roles from the passwords in the env files.

3. Run the auth migrations. This uses `DATABASE_URL` from `apps/auth/.env`:

   ```sh
   just auth-migrate
   ```

4. Give the key-provider its connection strings, then run its migrations:

   ```sh
   cd apps/key-provider/src
   dotnet user-secrets set "ConnectionStrings:KeyProvider" "Host=localhost;Port=5433;Database=key_provider;Username=key_provider_app;Password=..."
   dotnet user-secrets set "ConnectionStrings:KeyProviderMigration" "Host=localhost;Port=5433;Database=key_provider;Username=key_provider_migrator;Password=..."
   dotnet user-secrets set "ConnectionStrings:DataProtection" "Host=localhost;Port=5433;Database=dp_key_provider;Username=dp_app;Password=..."
   dotnet user-secrets set "ConnectionStrings:DataProtectionMigration" "Host=localhost;Port=5433;Database=dp_key_provider;Username=dp_migrator;Password=..."
   cd ../../..
   dotnet tool restore
   just key-migrate
   ```

5. Start the key-provider. It listens on `localhost:5210`:

   ```sh
   dotnet run --project apps/key-provider/src
   ```

6. Start auth. It reads its settings from `AUTH__*` environment variables and
   listens on port 50051:

   ```sh
   set -a; source apps/auth/.env; set +a
   cargo run -p auth
   ```

Two things to know:

- sqlx checks queries at compile time. When `DATABASE_URL` is set it connects
  to that database, so the build fails if Postgres is down. Set
  `SQLX_OFFLINE=true` to build from the query data committed in
  `apps/auth/.sqlx` instead:

  ```sh
  SQLX_OFFLINE=true cargo build
  ```

- After changing a query, start the database and refresh that data, then
  commit the result:

  ```sh
  cd apps/auth
  cargo sqlx prepare
  ```

- The compose file starts Redis with a password, but auth has no setting for a
  Redis password yet (`AUTH__REDIS__*` is host and port only).

### Environment variables

| File                          | Variables                                           | Read by                          |
| ----------------------------- | --------------------------------------------------- | -------------------------------- |
| `.env`                        | `REDIS_PASSWORD`                                    | Root compose file                |
| `apps/auth/.env`              | `POSTGRES_*`, `MIGRATION_PASSWORD`, `APP_PASSWORD`  | Auth database container          |
| `apps/auth/.env`              | `DATABASE_URL`                                      | sqlx migrations and query checks |
| `apps/auth/.env`              | `AUTH__*`                                           | The auth service                 |
| `apps/key-provider/.env`      | `POSTGRES_*`, `MIGRATION_PASSWORD`, `APP_PASSWORD`, `DP_*` | Key-provider database container |
| `apps/key-provider/.env`      | `ConnectionStrings__*`                              | The key-provider, as an alternative to user secrets |

`AUTH__PASSWORD__PEPPER` must stay the same for the life of the deployment.
Changing it invalidates every stored password.

## Using it from your app

1. **Run the services.** Container images are not published yet, so for now
   run auth and the key-provider from source as described above.
2. **Set the env vars.** Use the `.env.example` files as the list. In
   production, pass `ConnectionStrings__KeyProvider` and
   `ConnectionStrings__DataProtection` to the key-provider as environment
   variables instead of user secrets.
3. **Generate a client.** Point your gRPC tooling at `proto/` and generate a
   client for `outpost.auth.v1.AuthService`.
   [`examples/python-client`](examples/python-client) shows this with buf and
   Python.
4. **Verify tokens.** Read the `kid` from the token header, fetch the JWK at
   `jwk:<kid>` from Redis, and verify the EdDSA signature. Then check `exp`,
   `iss` and `aud`. Both `iss` and `aud` are `Outpost` by default and are set
   in the key-provider's `Signing` settings. The user id is in `sub`.

## Working on the code

```sh
just lint             # buf lint and format check
just breaking         # buf breaking against main
just generate         # regenerate the C# and Python stubs from proto/
just check-generated  # fail if the generated code is out of date
just key-scripts      # regenerate the idempotent SQL scripts for the key-provider
```

`just generate` runs buf through uv, so the Python generator plugins are
installed for you.

Layout:

- `proto/` holds the protobuf definitions (`outpost.*`).
- `apps/` holds the services, one directory each.
- `libs/` holds shared code. `Outpost.EntityFrameworkCore` provides the
  append-only and created-at conventions.
- `codegen/` holds one `buf generate` template per target.
- `examples/` holds sample clients.

Workspace roots live at the top: `Cargo.toml`, `pyproject.toml` (uv) and
`outpost.slnx`.

## Roadmap

- A JWKS endpoint (`/.well-known/jwks.json`), so apps can verify tokens
  without access to Redis.
- Per-app audiences.
- Refresh tokens.
- Tests and CI.
- Published Docker images.

## License

[MIT](LICENSE)
