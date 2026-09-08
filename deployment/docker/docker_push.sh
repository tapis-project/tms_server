#!/bin/sh
# ----------------------------------------------------------------
# Publish docker image for tms_server
# ----------------------------------------------------------------

PrgName=$(basename "$0")
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Check arguments
if [ $# -ne 1 ]; then
    echo "Usage: $PrgName <image_tag>"
    echo "  where <image_tag> is the image version tag"
    echo "For example $PrgName dev"
    exit 1
fi

# Assign the image tag
TAG=$1

# Publish image
echo "=================================================="
echo "docker push tapis/tms_server:${TAG}"
echo "=================================================="
docker push "tapis/tms_server:${TAG}"
