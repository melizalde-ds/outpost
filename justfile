default:
    @just --list

lint:
    buf lint
    buf format -d --exit-code

breaking:
    buf breaking --against '.git#branch=main'

generate: generate-key generate-weather

generate-key:
    buf generate --template codegen/buf.gen.key.yaml

generate-weather:
    buf generate --template codegen/buf.gen.weather.yaml

check-generated: generate
    git diff --exit-code apps/key-provider/src/Api/Generated apps/weather/gen

key_provider := "apps/key-provider/src"

migrations-script:
    dotnet ef migrations script --idempotent --project {{key_provider}} --context KeyProviderDbContext --output apps/key-provider/db/scripts/key_provider.sql
    dotnet ef migrations script --idempotent --project {{key_provider}} --context DataProtectionDbContext --output apps/key-provider/db/scripts/data_protection.sql
    sed -i '1s/^\xEF\xBB\xBF//' apps/key-provider/db/scripts/*.sql

migrations-update:
    dotnet ef database update --project {{key_provider}} --context KeyProviderDbContext --connection "$(dotnet user-secrets list --project {{key_provider}} | sed -n 's/^ConnectionStrings:KeyProviderMigration = //p')"
    dotnet ef database update --project {{key_provider}} --context DataProtectionDbContext --connection "$(dotnet user-secrets list --project {{key_provider}} | sed -n 's/^ConnectionStrings:DataProtectionMigration = //p')"
