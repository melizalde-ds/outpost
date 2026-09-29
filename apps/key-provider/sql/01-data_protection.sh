#!/bin/bash
set -euo pipefail

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" \
  -v dp_db="$POSTGRES_ENCRYPTION_DB" \
  -v dp_migrator_password="$DP_MIGRATION_PASSWORD" \
  -v dp_app_password="$DP_APP_PASSWORD" <<'EOSQL'
CREATE DATABASE :"dp_db";

CREATE ROLE dp_migrator LOGIN PASSWORD :'dp_migrator_password'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
CREATE ROLE dp_app LOGIN PASSWORD :'dp_app_password'
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;

REVOKE ALL ON DATABASE :"dp_db" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"dp_db" TO dp_migrator, dp_app;

\connect :"dp_db"

GRANT USAGE, CREATE ON SCHEMA public TO dp_migrator;
GRANT USAGE ON SCHEMA public TO dp_app;
EOSQL