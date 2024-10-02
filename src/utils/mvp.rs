#![forbid(unsafe_code)]

use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_types::{DelegationInput, UserMfaInput, UserHostInput};
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_to_str, MAX_TMS_UTC};
use crate::v1::tms::delegations_create::insert_delegation;
use crate::v1::tms::user_mfa_create::insert_user_mfa;
use crate::v1::tms::user_hosts_create::insert_user_host;
use log::info;

// Insert fails on conflict.        
const NOT_STRICT:bool = false;

pub struct MVPDependencyParms
{
    pub tenant: String,
    pub client_id: String,
    pub client_user_id: String,
    pub host: String,
    pub host_account: String,
}

/** The Minimal Viable Product (MVP) version of TMS simplifies migration to TMS in 
 * existing environments that meet certain requirements.  Specifically, MVP 
 * supports the following:
 * 
 *  - Keys don't expire.
 *  - Key dependency records are automatically created in these tables:
 *      - delegations - delegation established between user and client 
 *      - user_mfa - non-expiring MFA set up for user
 *      - user_host - user binding created to host_account
 *  
 * When the enable_mvp flag is turned on in the configuration file, clients can
 * create keys without prior configuration in the above 3 tables.  TMS will 
 * automatically create those records based on the input to the key create call,
 * eliminating the possibility that missing dependency records will cause key
 * creation to fail.  If a record already exists, TMS accepts accepts that 
 * record as is. 
 */
pub fn create_pubkey_dependencies(parms: MVPDependencyParms) -> Result<u64> {

    // --------------------- Variables used throughout ---------------------
    let expires_at = MAX_TMS_UTC;
    let mut insert_count: u64 = 0;

     // Use the same current UTC timestamp in all related time caculations..
     let now = timestamp_utc();
     let current_ts = timestamp_utc_to_str(now);
     
    // --------------------- Insert delegations record ---------------------
    // Required inputs: tenant, client_id, client_user_id
    //
    // Create the input record.  Note that we save the hash of
    // the hex secret, but never the secret itself.  
    let input_record = DelegationInput::new(
        parms.tenant.clone(),
        parms.client_id.clone(),
        parms.client_user_id.clone(),
        expires_at.to_string(),
        current_ts.clone(), 
        current_ts.clone(),
    );
    
    // Insert the new record if it doesn't already exist.
    let count = block_on(insert_delegation(input_record, NOT_STRICT))?;
    if count > 0 {
        insert_count += count;
        info!("MVP: Delegation for user '{}' to client '{}' created in tenant '{}' with expiration at {}.", 
                parms.client_user_id, parms.client_id, parms.tenant, expires_at);
    }

    // --------------------- Insert user_mfa record ------------------------
    // Required inputs: tenant, client_user_id
    //
    // Create the input record.  Note that we save the hash of
    // the hex secret, but never the secret itself.  
    let input_record = UserMfaInput::new(
        parms.tenant.clone(),
        parms.client_user_id.clone(),
        expires_at.to_string(),
        1,
        current_ts.clone(), 
        current_ts.clone(),
    );

    // Insert the new record if it doesn't already exist.
    let count = block_on(insert_user_mfa(input_record, NOT_STRICT))?;
    if count > 0 {
        insert_count += count;
        info!("MVP: MFA for user '{}' created in tenant '{}' with experation at {}.", 
            parms.client_user_id, parms.tenant, expires_at);
    }

    // --------------------- Insert user_hosts record ---------------------
    // Required inputs: tenant, client_user_id, host, host_account
    //
    // Create the input record.  Note that we save the hash of
    // the hex secret, but never the secret itself.  
    let input_record = UserHostInput::new(
        parms.tenant.clone(),
        parms.client_user_id.clone(),
        parms.host.clone(),
        parms.host_account.clone(),
        expires_at.to_string(),
        current_ts.clone(), 
        current_ts,
    );

    // Insert the new record if it doesn't already exist.
    let count = block_on(insert_user_host(input_record, NOT_STRICT))?;
    if count > 0 {
        insert_count += count;
        info!("MVP: Host mapping for user '{}' created in tenant '{}' with experation at {}.", 
                parms.client_user_id, parms.tenant, expires_at);
    }

    Ok(insert_count)
}
