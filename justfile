set shell := ["bash", "-euo", "pipefail", "-c"]

key_provider := "apps/key-provider/src"
key_scripts := "apps/key-provider/db/scripts"
generated := "apps/key-provider/src/Api/Generated apps/weather/gen"

default:
    @just --list

[group('proto')]
lint:
    buf lint
    buf format -d --exit-code

[group('proto')]
breaking:
    buf breaking --against '.git#branch=main'

[group('proto')]
generate:
    for t in codegen/buf.gen.*.yaml; do buf generate --template "$t"; done

[group('proto')]
check-generated: generate
    git diff --exit-code -- {{generated}}
    test -z "$(git ls-files --others --exclude-standard -- {{generated}})"

[group('auth')]
[working-directory('apps/auth')]
auth-migrate:
    sqlx migrate run

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
