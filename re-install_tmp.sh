#!/bin/bash
# Cleanup install of ver 0.3.0
# Helpful for testing after failed local installs
#
# THIS IS SCRIPT IS UNTESTED
#
# Exit if a command returns status different from 0
set -o errexit
# An unset variable is an error (avoids silently continuing after a typo in a name)
set -o nounset
# If any of the components of a pipe fails, then the pipe fails
set -o pipefail

set -xv
# Set env vars
. $HOME/tms_env/tms_env_local_install

# Reset DB
./deployment/postgres/tms_drop_db.sh
./deployment/postgres/tms_init_db.sh

#
# NOTE: Once 0.4.0 is ready we should be able to simply remove ~/.tms
#
# Clean up installed directories, but not ~/.tms/local which contains a modified tms.toml
rm -fr ~/.tms/certs
rm -fr ~/.tms/config/
rm -fr ~/.tms/logs/
rm -fr ~/.tms/migrations/
if [ -d "/tmp/tms_server" ]; then
  rm -fr /tmp/tms_server/
fi
if [ -d "/opt/tms_server" ]; then
  rm -fr /opt/tms_server/
fi
# Clean up files created during install that cause errors when re-installing
/bin/rm -f ~/.tms/local/tms-db-env
/bin/rm -f ~/.tms/local/tms-install.out
/bin/rm -f ~/.tms/local/tms_service.env

# re-install
./deployment/native/install_040.sh --test
