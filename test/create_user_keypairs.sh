#!/bin/bash
# Create ssh key pairs for users and store results in json files
# Data for each user must already be seeded in the user_hosts table.
#-- Example data for case
#--   tms tenant = test, app client = testclient1
#--   host=testhost1, tms user=testuser0001, host login user=testuser0001
# Determine absolute path to location from which we are running
#  and change to that directory.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

# Create directory for results
mkdir -p ./data

SERVER_URL="http://129.114.35.127:3001/v1/tms"
TENANT="test"
CLIENT_ID="testclient1"
CLIENT_SECRET="secret1"
HOST="testhost1"

# Allow for range of users from testuser0001 - testuser9999 (or higher)
USR_COUNT_START=1
USR_COUNT_END=100
for i in $(seq $USR_COUNT_START $USR_COUNT_END)
do
  CLIENT_USR=$(printf "%s%04d" "testclientuser" $i)
  HOST_USR=$(printf "%s%04d" "testhostuser" $i)
  OUT_FILE="./data/${HOST_USR}.json"
  FP_FILE="./data/${HOST_USR}.fp"
  echo "Creating key pair for user $i with host username = $HOST_USR. File name: $OUT_FILE"

  # Create json request body and place it in tmp file
  TMP_FILE=$(mktemp)
  echo "{\"tenant\":\"${TENANT}\",\"client_id\":\"${CLIENT_ID}\",\"client_secret\":\"${CLIENT_SECRET}\"," > ${TMP_FILE}
  echo "\"client_user_id\":\"${CLIENT_USR}\",\"host\":\"${HOST}\",\"host_account\":\"${HOST_USR}\"," >> ${TMP_FILE}
  echo "\"num_uses\":0,\"ttl_minutes\":0,\"key_type\":\"\"}" >> ${TMP_FILE}

  # Generate keypair and place output in a file
  curl -k -X POST -H 'content-type: application/json' ${SERVER_URL}/creds/sshkeys \
       -d @${TMP_FILE} | jq > ${OUT_FILE}

  # Extract fingerprint and write it to a file
  cat ${OUT_FILE} | jq -r '.public_key_fingerprint' > ${FP_FILE}
  # Clean up
  /bin/rm -f $TMP_FILE
done

# Switch back to current working directory of invoking user
cd "$RUN_DIR"
