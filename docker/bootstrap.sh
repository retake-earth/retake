#!/bin/bash
# shellcheck disable=SC2154

# Executed at container start to boostrap ParadeDB extensions and Postgres settings.

# Exit on subcommand errors
set -Eeuo pipefail

# Perform all actions as $POSTGRES_USER
export PGUSER="$POSTGRES_USER"

# Create the 'template_paradedb' template db
"${psql[@]}" <<- 'EOSQL'
CREATE DATABASE template_paradedb IS_TEMPLATE true;
EOSQL

# Load ParadeDB extensions into both template_database and $POSTGRES_DB
for DB in template_paradedb "$POSTGRES_DB"; do
  echo "Loading ParadeDB extensions into $DB"
  "${psql[@]}" --dbname="$DB" <<-'EOSQL'
    CREATE EXTENSION IF NOT EXISTS pg_search;
    CREATE EXTENSION IF NOT EXISTS pg_lakehouse;
    CREATE EXTENSION IF NOT EXISTS pg_ivm;
    CREATE EXTENSION IF NOT EXISTS vector;
    CREATE EXTENSION IF NOT EXISTS vectorscale;
EOSQL
done

# Add the `paradedb` schema to both template_database and $POSTGRES_DB
for DB in template_paradedb "$POSTGRES_DB"; do
  echo "Adding 'paradedb' search_path to $DB"
  "${psql[@]}" --dbname="$DB" <<-'EOSQL'
    ALTER DATABASE \"$DB\" SET search_path TO public,paradedb;
EOSQL
done

echo "ParadeDB bootstrap completed!"
