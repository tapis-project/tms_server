#!/bin/bash
# Script to drop the TMS server DB schema using psql
# Postgres password must be set in env var POSTGRES_PASSWORD

if [ -z "$TMS_DB_HOST" ]; then
  TMS_DB_HOST=localhost
fi

if [ -z "$TMS_DB_PORT" ]; then
  TMS_DB_PORT=5432
fi

PSQL_CMD="psql --host=${TMS_DB_HOST} --port=${TMS_DB_PORT}"

DB_USER=postgres
DB_NAME=tmsdb
export TMS_DB_USER=tms
export TMS_DB_SCHEMA=tms

if [ -z "${POSTGRES_PASSWORD}" ]; then
  echo "Please set env var POSTGRES_PASSWORD before running this script"
  exit 1
fi

# Put PGPASSWORD in environment for psql to pick up
export PGPASSWORD=${POSTGRES_PASSWORD}

# Run sql to create user and schema if they do not exist
$PSQL_CMD --username=${DB_USER} --dbname=${DB_NAME} -q << EOB
-- Drop schema if it exists
DROP SCHEMA IF EXISTS ${TMS_DB_SCHEMA} cascade;
EOB