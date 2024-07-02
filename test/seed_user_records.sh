#!/bin/bash
#-- Data needed for test seeding will eventually include delegations and user_mfa records.
#-- Example data for case
#--   tms tenant = test, app client = testclient1
#--   host=testhost1, tms user=testuser1, host login user=scblack
#-- insert into delegations (tenant,client_id,client_user_id,expires_at,created,updated) values ('test','testclient1','testuser1','+262142-12-31T23:59:59Z','2024-05-28T15:18:03Z','2024-05-28T15:18:03Z');
#-- insert into user_mfa (tenant,tms_user_id,expires_at,enabled,created,updated) values ('test','testuser1','+262142-12-31T23:59:59Z','1','2024-05-28T15:18:03Z','2024-05-28T15:18:03Z');
#-- insert into user_hosts (tenant,tms_user_id,host,host_account,expires_at,created,updated) values ('test','testuser1','testhost1','scblack','+262142-12-31T23:59:59Z','2024-05-28T15:18:03Z','2024-05-28T15:18:03Z')

# Allow for range of users from testuser0001 - testuser9999 (or higher)
USR_COUNT_START=1
USR_COUNT_END=100
for i in $(seq $USR_COUNT_START $USR_COUNT_END)
do
  USR_NAME=$(printf "%s%04d" "testuser" $i)
  echo "Inserting record for user $i with username = $USR_NAME"
  sqlite3 ~/.tms/database/tms.db << EOB
  insert into user_hosts (tenant,tms_user_id,host,host_account,expires_at,created,updated)  values ('test',"$USR_NAME",'testhost1',"$USR_NAME",'+262142-12-31T23:59:59Z','2024-05-28T15:18:03Z','2024-05-28T15:18:03Z')
EOB
done
