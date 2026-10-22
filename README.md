# Outpost

Outpost is a set of backend services you host yourself, so your apps don't
have to build the same pieces over and over. The first one is **auth**: it
handles sign-up, login and staying logged in.

Your app talks to auth over gRPC or HTTP. Auth gives back a short-lived access token
your app can check on its own, and a refresh token to get a new one.

## Status

Outpost is early and still changing. It isn't ready for production yet:
connections between services aren't encrypted, and the key provider doesn't
check who is calling it.

## What auth can do

- Sign up and log in with a username and password
- Keep users logged in with refresh tokens that change on every use
- End a session if an old refresh token is used again
- Log out on one device, or on all of them
- Rotate its signing keys on a schedule

## Quick start

You need Docker, Rust, the .NET 10 SDK, [just](https://github.com/casey/just)
and [sqlx-cli](https://crates.io/crates/sqlx-cli).

```sh
just env     # create the .env files, then fill in the change-me values
just setup   # start the databases and Redis, and run the migrations
just run     # start the services
```

Auth listens for gRPC on `localhost:50051` and for HTTP on `localhost:8080`.
To try it, see the [Python example](examples/python-client).

## Using it from your app

1. Generate a gRPC client from [`auth.proto`](proto/outpost/auth/v1/auth.proto),
   or use the HTTP API below.
2. Call `Register` or `Login` to get tokens, and `RefreshToken` when the
   access token runs out.
3. To check an access token, get the public key for its `kid` (in the token
   header) from `GET /.well-known/jwks.json`. Then check the signature and
   expiry. The JWKS endpoint keeps answering
   while the key provider is down.

### HTTP API

Every call takes and returns JSON. The fields match `auth.proto`.

| Call | Body | Answer |
|---|---|---|
| `POST /v1/auth/register` | `{"username", "password"}` | `{"tokens": {...}}` |
| `POST /v1/auth/login` | `{"username", "password"}` | `{"tokens": {...}}` |
| `POST /v1/auth/refresh` | `{"refresh_token"}` | `{"tokens": {...}}` |
| `POST /v1/auth/logout` | `{"refresh_token"}` | 204 |
| `POST /v1/auth/logout-all` | `{"refresh_token"}` | 204 |
| `GET /.well-known/jwks.json` | | `{"keys": [...]}` |
| `GET /healthz` | | 200 while auth is running |

`tokens` holds `access_token`, `refresh_token` and `refresh_token_expires_at`
(an RFC 3339 time). Usernames can be up to 256 characters and passwords up to
1024 bytes. Errors come back as
[problem JSON](https://www.rfc-editor.org/rfc/rfc9457) with a `detail`
message.

### Health checks

Auth serves the standard gRPC health service (`grpc.health.v1.Health`) on the
gRPC port, and `GET /healthz` on the HTTP port. Both say auth is up while it
runs. They don't check Postgres, Redis or the key provider.

## Working on Outpost

Run `just` to see every command. If you change a proto file, you'll also need
[buf](https://buf.build) and [uv](https://docs.astral.sh/uv/), then run
`just generate`.

## License

[MIT](LICENSE)
