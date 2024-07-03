#!/bin/bash
# Fetch public keys associated with key pairs previously created by
#   script create_user_keypairs.sh
# SSH fingerprints are assumed to be in files matching data/*.fp
#
# Determine absolute path to location from which we are running
#  and change to that directory.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

SERVER_URL="http://129.114.35.127:3001/v1/tms"
TENANT="test"
CLIENT_ID="testclient1"
CLIENT_SECRET="secret1"
HOST="testhost1"
USR_ID="1"
KEY_TYPE="ed25519"

# Allow for range of users from testuser0001 - testuser9999 (or higher)
USR_COUNT_START=1
USR_COUNT_END=100
for i in $(seq $USR_COUNT_START $USR_COUNT_END)
do
  HOST_USR=$(printf "%s%04d" "testhostuser" $i)
  FP_FILE="./data/${HOST_USR}.fp"
  PK_FP=$(cat $FP_FILE)

  echo "Fetching public key for user $i with host username = $HOST_USR."

  # Create json request body and place it in tmp file
  TMP_FILE=$(mktemp)
  echo "{\"user\":\"${HOST_USR}\",\"user_uid\":${USR_ID},\"host\":\"${HOST}\"," > ${TMP_FILE}
  echo "\"key_type\":\"${KEY_TYPE}\",\"public_key_fingerprint\":\"${PK_FP}\"}" >> ${TMP_FILE}

  # Generate keypair and place output in a file
  curl --silent -k -X POST -H 'content-type: application/json' ${SERVER_URL}/creds/publickey \
       -d @${TMP_FILE} | jq -r ".public_key"

  /bin/rm -f $TMP_FILE
done

# Switch back to current working directory of invoking user
cd "$RUN_DIR"
