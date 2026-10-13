# Outpost

Backend services you host yourself. You deploy them once and point your apps
at them, like you would with Supabase. Each service runs in its own container.

Auth is the first service.

## What auth does

- **Register** creates a user and returns an access token.
- **Login** checks the password and returns an access token.
- **Refresh** is not built yet.

Apps talk to it over gRPC. The access token is a signed JWT that lasts 5
minutes by default. The API is in
[`proto/outpost/auth/v1/auth.proto`](proto/outpost/auth/v1/auth.proto).

## How it works

```
your app ──> auth ──> key-provider
              │
              └──> Redis
```

- **auth** (Rust) stores users and checks passwords.
- **key-provider** (.NET) holds the signing keys and signs the tokens.
- **Redis** holds the public keys, so apps can check that a token is real.

Auth and the key-provider each have their own Postgres database.

## How it stays safe

- Passwords are hashed with Argon2, plus a secret (the pepper) that is not
  stored in the database.
- Passwords, tokens and keys are kept out of the logs.
- The running services use database users with limited rights. Migrations use
  a separate user.
- Signing keys can be added but never changed or deleted. The database itself
  blocks it.
- Private keys are encrypted before they are stored.
- Signing keys are replaced on a schedule (every 8 hours by default), with
  retries if it fails.

The database blocks bad writes with two error codes:

- `WLA01`: something tried to change or delete a key record.
- `WLA02`: something tried to save a key whose rotation did not succeed.

## Run it locally

You need Docker, Rust, the .NET 10 SDK, [just](https://github.com/casey/just)
and [sqlx-cli](https://crates.io/crates/sqlx-cli).

1. Copy the env files and replace the `change-me` values:

   ```sh
   cp .env.example .env
   cp apps/auth/.env.example apps/auth/.env
   cp apps/key-provider/.env.example apps/key-provider/.env
   ```

2. Start the databases and Redis:

   ```sh
   docker compose up -d
   ```

3. Set up the auth database:

   ```sh
   just auth-migrate
   ```

4. Tell the key-provider how to reach its databases, then set them up. Use
   the passwords from `apps/key-provider/.env`:

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

5. Start the key-provider (port 5210):

   ```sh
   dotnet run --project apps/key-provider/src
   ```

6. Start auth (port 50051):

   ```sh
   set -a; source apps/auth/.env; set +a
   cargo run -p auth
   ```

Good to know:

- If the build fails because the database is off, run
  `SQLX_OFFLINE=true cargo build`.
- After changing a SQL query, run `cargo sqlx prepare` in `apps/auth` with the
  database on, and commit the result.
- Redis starts with a password, but auth can't send one yet.
- Don't change `AUTH__PASSWORD__PEPPER` once you have users. Their passwords
  would stop working.

## Use it from your app

1. Run auth and the key-provider. Docker images are not published yet, so run
   them from source for now.
2. Fill in the settings listed in the `.env.example` files.
3. Generate a gRPC client from `proto/`. See
   [`examples/python-client`](examples/python-client).
4. Check tokens. The token header has a `kid`. Get the public key from Redis
   at `jwk:<kid>`, check the signature, and check the token has not expired.
   The user id is in `sub`. `iss` and `aud` are both `Outpost` by default.

## Working on the code

After changing a proto file, you also need [buf](https://buf.build) and
[uv](https://docs.astral.sh/uv/):

```sh
just lint             # check the proto files
just generate         # rebuild the generated code
just check-generated  # fail if the generated code is out of date
```

## Roadmap

- A JWKS endpoint (`/.well-known/jwks.json`), so apps can check tokens without
  Redis.
- Per-app audiences.
- Refresh tokens.
- Tests and CI.
- Published Docker images.

## License

[MIT](LICENSE)
