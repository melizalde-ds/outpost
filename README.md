# Outpost

Outpost is a set of backend services you host yourself, so your apps don't
have to build the same pieces over and over. The first one is **auth**: it
handles sign-up, login and staying logged in.

Your app talks to auth over gRPC. Auth gives back a short-lived access token
your app can check on its own, and a refresh token to get a new one.

## Status

Outpost is early and still changing. It isn't ready for production yet: there
are no tests, and connections aren't encrypted.

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

Auth listens on `localhost:50051`. To try it, see the
[Python example](examples/python-client).

## Using it from your app

1. Generate a gRPC client from [`auth.proto`](proto/outpost/auth/v1/auth.proto).
2. Call `Register` or `Login` to get tokens, and `RefreshToken` when the
   access token runs out.
3. To check an access token, read the public key from Redis at `jwk:<kid>`
   (the `kid` is in the token header) and check the signature and expiry.

## Working on Outpost

Run `just` to see every command. If you change a proto file, you'll also need
[buf](https://buf.build) and [uv](https://docs.astral.sh/uv/), then run
`just generate`.

## License

[MIT](LICENSE)
