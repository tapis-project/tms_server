# tms_server deployment

Trust Manager System (TMS) web server deployment

This directory (deployment) contains file related to the deployment of the TMS server.
Including Dockerfiles, a docker-compose file and a temporary install script.

## Building the docker image

Example, to be run from top directory of tms_server repository.

  docker build -t tapis/tms-server-cargo:0.1 -f deployment/Dockerfile_cargo .

## Deployment instructions

TBD

## NOTES

WIP Fix issue with volume.

When started via docker the volume is created as tms-root

When started using compose the volume is created as deployment_tms-root

```
docker volume ls | grep tms
local     deployment_tms-root
local     tms-root
```


