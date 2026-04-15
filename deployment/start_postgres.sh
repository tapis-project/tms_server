#!/bin/sh
# Start local tms-postgres for use by tms_server
# Place output at ~/tms-postgres_start.log
PrgName=$(basename "$0")

# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

nohup docker-compose -f $PRG_PATH/tms-postgres.yml up > ~/tms-postgres_start.log 2>&1 &
