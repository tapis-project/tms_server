# tms_server deployment

Trust Manager System (TMS) web server deployment

This directory (deployment) contains files related to the deployment of the TMS server,
including Dockerfiles and a docker-compose file.

## Building the docker image

Example, to be run from top directory of tms_server repository.

  docker build -t tapis/tms-server-cargo:0.1 -f deployment/Dockerfile_cargo .

## Install instructions

Copy docker-compose.yml file to host server.

### Initial configuration

To initialize the database and the configuration files, use ``docker run``
to run the server with the ``--install`` option.
Be sure to use ``deployment_tms-root`` as the volume name.
For example:

```
docker run --rm -v deployment_tms-root:/home/tms tapis/tms-server-cargo:0.1 /bin/bash -c "/opt/tms/tms_server --install"
```

Note that during this step administrator user IDs and passwords for the
standard tenants are created. The passwords are NOT saved by TMS, only hashes
of them are saved  Please store the passwords permanently in a safe place
accessible to TMS administrators.

> [!WARNING] THIS IS THE ONLY TIME THESE PASSWORDS ARE SHOWN.
> THE PASSWORDS ARE NOT RECOVERABLE IF THEY ARE LOST!

### Starting and stopping the server

To bring the service up and down, simply run ``docker compose``. For example,

```
cd /path/to/tms
docker compose up -d
docker compose down
```

The above commands will create and start the docker container in the background and then
stop and remove the container. The data volume will remain.


## Uninstall instructions

To remove TMS server from a host, stop the container and remove the volume.
Optionally, to remove all traces, also remove the docker image and the docker compose file.

For example:

```
cd /path/to/tms
docker compose down
docker volume rm deployment_tms-root
docker rmi tapis/tms-server-cargo:0.1
rm docker-compose.yml
```


## Data persistence

Database files and logs will be persisted in a docker volume named *deployment_tms-root*

To remove the data volume the following command may be used:

```
docker volume rm deployment_tms-root
```

## Modifying the default configuration

To modify the default configuration, bring the service up and use ``docker exec`` to access the
running container. The ``vi`` editor may be used to modify the file. Exit the container and
restart the service to apply the changes.

For example,

```
cd /path/to/tms
docker compose up -d
docker exec -it deployment-tms_server-1 /bin/bash
cd /home/tms/.tms/config
vi tms.toml
exit
docker compose down
docker compose up -d
```

## Accessing the database

Data is stored in an sqlite database. One convenient way to access the database
is to run sqlite from within the running container, for example:

```
docker exec -it deployment_tms_server_1 sqlite3 /home/tms/.tms/database/tms.db
```

## Note on using docker run command

When started via docker the volume is created as ``tms-root``
When started using compose the volume is created as ``deployment_tms-root``.

```
docker volume ls | grep tms
local     deployment_tms-root
local     tms-root
```

To allow for use with either ``docker run`` or ``docker compose``, please use the volume name
``deployment_tms-root`` whenever executing ``docker run`` commands.

For example,

```
docker run -d --rm -v deployment_tms-root:/home/tms tapis/tms-server-cargo:0.1
```
