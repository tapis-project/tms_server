#!/bin/sh
# Shut down local tms-postgres
# Place output at ~/tms-postgres_stop.log
PrgName=$(basename "$0")

# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

nohup docker-compose -f $PRG_PATH/tms-postgres.yml down > ~/tms-postgres_stop.log 2>&1 &
