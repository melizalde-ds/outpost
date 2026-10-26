# Changelog

All notable changes to this project are listed here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/).

## [0.1.0] - Unreleased

The first release. Outpost is not ready for production yet: connections between
services aren't encrypted, and the key provider doesn't check who is calling it.

### Added

- **Auth service** (Rust), over gRPC and HTTP:
  - Sign up and log in with a username and password.
  - Short-lived access tokens signed with Ed25519.
  - Refresh tokens that change on every use. Using an old one again ends the session.
  - Log out on one device, or on all of them.
  - Sessions end after an idle time and a maximum age, 7 and 30 days by default.
  - Public keys at `GET /.well-known/jwks.json`, which keeps answering while the
    key provider is down.
  - Health checks: the gRPC health service and `GET /healthz`.
  - Request time limits, and a clean shutdown on Ctrl+C or SIGTERM.
- **Key provider service** (.NET):
  - Makes new signing keys on a schedule, with retries.
  - Signs tokens only with keys published at least 3 minutes earlier, so apps
    have time to fetch them.
  - Stores private keys encrypted, in tables that can only be added to.
- **Python example** client.
- `just` recipes to set up, run and test everything.

### Security

- Unknown usernames and wrong passwords get the same answer and take the same time.
- Usernames are trimmed and compared without case, and limited to 256 characters.
  Passwords are limited to 1024 bytes.
- Passwords and tokens are hidden when gRPC messages are logged.
- Redis allows a single auth user with only the commands auth needs.
- Each service uses separate database users for migrations and for the app.
- Docker Compose publishes Postgres and Redis on `127.0.0.1` only.
