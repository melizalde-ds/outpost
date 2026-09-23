default:
    @just --list

lint:
    buf lint
    buf format -d --exit-code

breaking:
    buf breaking --against '.git#branch=main'

# auth (Rust) generates its own code via build.rs
generate: generate-key generate-weather

generate-key:
    buf generate --template codegen/buf.gen.key.yaml

generate-weather:
    buf generate --template codegen/buf.gen.weather.yaml

check-generated: generate
    git diff --exit-code key/Key/Gen weather/gen