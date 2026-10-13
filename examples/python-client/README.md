# python-client

A small Python client for the Outpost auth service. It shows how an app
generates a gRPC client from `proto/` and logs in.

## Run it

Start the auth service first (see the root README), then from the repo root:

```sh
export OUTPOST_USERNAME=your-username
export OUTPOST_PASSWORD=your-password
uv run --package python-client python examples/python-client/main.py
```

The user has to exist already. `AuthClient.register` creates one.

The client connects to `localhost:50051` by default. Change `AuthConfiguration`
in `auth/auth_client.py` to point somewhere else.

## Layout

- `auth/` is a thin wrapper around the generated stub with a small channel pool.
- `gen/` holds the generated code. Don't edit it; run `just generate` after a
  proto change. The template is `codegen/buf.gen.python-client.yaml`.
