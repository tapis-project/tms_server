# 
# Use cargo to build docker image for TMS server.
#
# REQUIREMENTS
# ------------
# The I/O requirements for TMS to initialize and run involve two subdirectories
# of the root installation directory.  For containers, the root installation 
# directory is tms-root and its subdirectories are:
#
#   - .tms, which has five subdirectories used my TMS during initialization and
#       when executing normally.  The .tms/certs directory needs to be written
#       from outside the container so that the host-specific certificate chain
#       and private key can be installed and updated as needed.
#
#   - tms_customizations, which is a bind mounted host directory used by TMS
#       running in the container to (1) write the tms-install.out file that 
#       contains administrative secrets, and (2) optional configuration files
#       customized for the host on which TMS executes.
#
# APPROACH
# --------
# To meet the above I/O requirements, this Dockerfile builds the TMS server as
# root, grants world access on most of the container's directories and files, 
# and assumes that the container will always run under a non-root user account. 

# Start with rust image for the build
FROM rust:1.83 AS builder

# Set up directory with cargo project
RUN USER=root cargo new --bin tms_server
WORKDIR /tms_server

# Copy build setup
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build dependencies for caching
RUN cargo build --release
RUN rm -fr src/*.rs

# Copy source and resources
COPY ./.git ./.git
COPY ./build.rs ./build.rs
COPY ./src ./src
COPY ./resources ./resources

# Build release
RUN rm -fr ./target/release/deps/tms_server*
RUN cargo build --release

# Switch to final base image
FROM debian:bookworm-slim

# Install less and vim. Add user tms
RUN apt update && apt install -y libssl-dev libssl3 less vim-tiny sqlite3 coreutils

# Create the TMS server root installation directory. At 
# runtime, all I/O takes place in this directory subtree.
RUN mkdir -p /tms-root/tms_server
RUN chmod -R 777 /tms-root

# Switch to the server executable's subdirectory.
WORKDIR /tms-root/tms_server

# Copy the server and all the resource files used installation.
# Make all files accessible to any non-root user specified on
# the docker run command or in a docker compose file.
COPY --from=builder /tms_server/target/release/tms_server .
RUN chmod 755 tms_server
COPY --from=builder /tms_server/resources ./resources
RUN chmod 777 ./resources ./resources/certs ./resources/config ./resources/migrations
RUN chmod 666 ./resources/certs/* ./resources/config/*

# Start the server after having run it once with the --install option.
CMD ["./tms_server"]
#CMD ["/usr/bin/sleep", "10000"]

