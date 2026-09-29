#!/bin/bash
set -euo pipefail

psql -v ON_ERROR_STOP=1 \
  --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" \
  -v migrator_password="$MIGRATION_PASSWORD" \
  -v app_password="$APP_PASSWORD" \
  -v db="$POSTGRES_DB" <<'EOSQL'
CREATE ROLE key_provider_migrator LOGIN PASSWORD :'migrator_password'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;

CREATE ROLE key_provider_app LOGIN PASSWORD :'app_password'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;

REVOKE ALL ON DATABASE :"db" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"db" TO key_provider_migrator, key_provider_app;

GRANT USAGE, CREATE ON SCHEMA public TO key_provider_migrator;
GRANT USAGE ON SCHEMA public TO key_provider_app;
EOSQL
