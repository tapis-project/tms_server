#!/bin/bash
# Build docker image
# docker build -t tapis/tms-server-cargo:0.1 -f Dockerfile_cargo .
mkdir -p ~/tms
docker run --rm --mount type=bind,source="${HOME}/tms",target=/home/tapistms -it tapis/tms-server-cargo:0.1 /bin/bash -c "/opt/tms/tms_server --create-dirs-only"
docker run --rm --mount type=bind,source="${HOME}/tms",target=/home/tapistms -it tapis/tms-server-cargo:0.1 /bin/bash -c "cd /opt/tms/resources; ./install-resources.sh ~/.tms"
ls -la ~/tms/.tms/config
docker compose up
