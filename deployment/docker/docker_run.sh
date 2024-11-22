#!/bin/sh
# Start up local docker image for tms_server_cargo service.

PrgName=$(basename "$0")

USAGE1="Usage: $PrgName"

SVC_NAME="tms_server"

# Run docker image for the service
TAG="tms_server_cargo:0.1"

# Determine absolute path to location from which we are running.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

# Run server, exposed on port 3001
docker run -d --rm -p 3001:3000 "${TAG}"
cd "$RUN_DIR"
