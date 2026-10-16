set shell := ["bash", "-euo", "pipefail", "-c"]

key_provider := "apps/key-provider/src"
key_tests := "apps/key-provider/tests/KeyProvider.UnitTests"
key_it := "apps/key-provider/tests/KeyProvider.IntegrationTests"
key_scripts := "apps/key-provider/db/scripts"
key_port := "5210"
generated := "apps/key-provider/src/Api/Generated examples/python-client/gen " + key_scripts

default:
    @just --list

[group('all')]
setup: env up tools key-secrets migrate

[group('all')]
env:
    #!/usr/bin/env bash
    set -euo pipefail
    created=0
    for example in apps/*/.env.example; do
        env=${example%.example}
        [[ -e $example && ! -e $env ]] || continue
        cp -v "$example" "$env"
        created=1
    done
    if (( created )); then echo "replace the change-me values in the new .env files before going on" >&2; exit 1; fi

[group('all')]
up:
    docker compose up -d --wait

[group('all')]
down:
    docker compose down

[group('all')]
tools:
    dotnet tool restore

[group('all')]
migrate: auth-migrate key-migrate

[group('all')]
build:
    cargo build
    dotnet build

[group('all')]
test: auth-test key-test

[group('all')]
run:
    #!/usr/bin/env bash
    set -euo pipefail
    dotnet run --project {{key_provider}} &
    key_pid=$!
    trap 'kill "$key_pid" 2>/dev/null || true' EXIT
    until (exec 3<>/dev/tcp/localhost/{{key_port}}) 2>/dev/null; do
        kill -0 "$key_pid" 2>/dev/null || exit 1
        sleep 1
    done
    {{just_executable()}} auth-run

[group('proto')]
lint:
    buf lint
    buf format -d --exit-code

[group('proto')]
breaking:
    buf breaking --against '.git#branch=main'

[group('proto')]
generate:
    for t in codegen/buf.gen.*.yaml; do uv run --project examples/python-client buf generate --template "$t"; done

check-generated: generate key-scripts
    git diff --exit-code -- {{generated}}
    test -z "$(git ls-files --others --exclude-standard -- {{generated}})"

[group('auth')]
[working-directory('apps/auth')]
auth-migrate:
    sqlx migrate run

[group('auth')]
auth-run:
    set -a; source apps/auth/.env; set +a; cargo run -p auth

[group('auth')]
auth-test:
    SQLX_OFFLINE=true cargo test -p auth

[group('auth')]
auth-it:
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'docker rm -f $(docker ps -aq --filter label=outpost.tests=auth) >/dev/null 2>&1 || true' EXIT
    SQLX_OFFLINE=true cargo test -p auth --test '*'

[group('key-provider')]
key-run:
    dotnet run --project {{key_provider}}

[group('key-provider')]
key-test:
    dotnet test --project {{key_tests}}

[group('key-provider')]
key-it:
    dotnet test --project {{key_it}}

[group('key-provider')]
key-secrets:
    #!/usr/bin/env bash
    set -euo pipefail
    source apps/key-provider/.env
    set_secret() {
        dotnet user-secrets set --project {{key_provider}} "ConnectionStrings:$1" "Host=localhost;Port=5433;Database=$2;Username=$3;Password=$4" > /dev/null
        echo "set user secret ConnectionStrings:$1"
    }
    set_secret KeyProvider "$POSTGRES_DB" key_provider_app "$APP_PASSWORD"
    set_secret KeyProviderMigration "$POSTGRES_DB" key_provider_migrator "$MIGRATION_PASSWORD"
    set_secret DataProtection "$POSTGRES_ENCRYPTION_DB" dp_app "$DP_APP_PASSWORD"
    set_secret DataProtectionMigration "$POSTGRES_ENCRYPTION_DB" dp_migrator "$DP_MIGRATION_PASSWORD"

[group('key-provider')]
key-scripts: (_ef-script "KeyProviderDbContext" "key_provider") (_ef-script "DataProtectionDbContext" "data_protection")

[group('key-provider')]
key-migrate: (_ef-update "KeyProviderDbContext" "KeyProviderMigration") (_ef-update "DataProtectionDbContext" "DataProtectionMigration")

_ef-script context name:
    dotnet ef migrations script --idempotent --project {{key_provider}} --context {{context}} --output {{key_scripts}}/{{name}}.sql
    sed -i '1s/^\xEF\xBB\xBF//' {{key_scripts}}/{{name}}.sql

_ef-update context secret:
    #!/usr/bin/env bash
    set -euo pipefail
    connection=$(dotnet user-secrets list --project {{key_provider}} | sed -n 's/^ConnectionStrings:{{secret}} = //p')
    if [[ -z $connection ]]; then echo "user secret ConnectionStrings:{{secret}} is not set" >&2; exit 1; fi
    dotnet ef database update --project {{key_provider}} --context {{context}} --connection "$connection"
