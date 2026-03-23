#!/bin/bash
# Script to initialize TMS server DB using psql
# Create database, user and schema
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

# Run psql command to create database if it does not exist
echo "SELECT 'CREATE DATABASE ${DB_NAME} ENCODING=\"UTF8\" LC_COLLATE=\"en_US.utf8\" LC_CTYPE=\"en_US.utf8\" ' \
  WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '${DB_NAME}')\gexec" \
  | $PSQL_CMD --username=${DB_USER}

# Run sql to create user and schema if they do not exist
$PSQL_CMD --username=${DB_USER} --dbname=${DB_NAME} -q << EOB
-- Create user if it does not exist
DO \$\$
BEGIN
  CREATE ROLE ${TMS_DB_USER} WITH LOGIN;
  EXCEPTION WHEN DUPLICATE_OBJECT THEN
  RAISE NOTICE 'User already exists. User name: ${TMS_DB_USER}';
END
\$\$;
ALTER USER ${TMS_DB_USER} WITH ENCRYPTED PASSWORD '${POSTGRES_PASSWORD}';
GRANT ALL PRIVILEGES ON DATABASE ${DB_NAME} TO ${TMS_DB_USER};

-- Create schema if it does not exist
CREATE SCHEMA IF NOT EXISTS ${TMS_DB_SCHEMA} AUTHORIZATION ${TMS_DB_USER};
ALTER ROLE ${TMS_DB_USER} SET search_path = '${TMS_DB_SCHEMA}';
EOB

## Test SQL
## Run sql to create a test table
#$PSQL_CMD --username=${DB_USER} --dbname=${DB_NAME} -q << EOB2
#CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL);
#INSERT INTO users (name) VALUES ('Alice'), ('Bob');
#EOB2
