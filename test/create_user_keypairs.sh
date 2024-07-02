#!/bin/bash
# Create ssh key pairs for users and store results in json files
# Data for each user must already be seeded in the user_hosts table.
#-- Example data for case
#--   tms tenant = test, app client = testclient1
#--   host=testhost1, tms user=testuser0001, host login user=testuser0001

TENANT="test"
CLIENT_ID="testclient1"
CLIENT_SECRET="secret1"
HOST="testhost1"

# Allow for range of users from testuser0001 - testuser9999 (or higher)
USR_COUNT_START=1
USR_COUNT_END=1
for i in $(seq $USR_COUNT_START $USR_COUNT_END)
do
  USR_NAME=$(printf "%s%04d" "testuser" $i)
  OUT_FILE=${USR_NAME}.json
  echo "Creating key pair for user $i with username = $USR_NAME. File name: $OUT_FILE"

  # Create json request body and place it in tmp file
  TMP_FILE=$(mktemp)
  echo "{\"tenant\":\"${TENANT}\",\"client_id\":\"${CLIENT_ID}\",\"client_secret\":\"${CLIENT_SECRET}\"," > ${TMP_FILE}
  echo "\"client_user_id\":\"${USR_NAME}\",\"host\":\"${HOST}\",\"host_account\":\"${USR_NAME}\"," >> ${TMP_FILE}
  echo "\"num_uses\":0,\"ttl_minutes\":0,\"key_type\":\"\"}" >> ${TMP_FILE}

  # Generate keypair and place output in a file
  set -xv
  curl -k -X POST -H 'content-type: application/json' https://129.114.35.127:3001/v1/tms/creds/sshkeys \
       -d @${TMP_FILE}
       # > ${OUT_FILE}

  # Clean up
#  /bin/rm -f $TMP_FILE
done
