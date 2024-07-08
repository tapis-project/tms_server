#!/bin/sh
# Build and publish docker image for tms_server_cargo service.

PrgName=$(basename "$0")

USAGE1="Usage: $PrgName"

# Run docker image for the service
TAG="tapis/tms-server-cargo:0.1"

# Determine absolute path to location from which we are running.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

# Build image
cd ..
echo docker build -t ${TAG} -f deployment/Dockerfile_cargo .
docker build -t ${TAG} -f deployment/Dockerfile_cargo .

# Publish image
# docker push ${TAG}
cd $RUN_DIR