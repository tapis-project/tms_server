#![forbid(unsafe_code)]

use anyhow::Result;
use crate::utils::db_types::{DelegationInput, RPLoginInput};
use crate::utils::tms_utils::{timestamp_utc};
use crate::v1::tms::delegations_create::insert_delegation;
use crate::v1::tms::rp_login_create::insert_rp_login;
use log::info;
use crate::utils::config::DB_TRUE;
use crate::utils::tms_utils;

// Insert fails on conflict.
const NOT_STRICT:bool = false;

pub struct MVPDependencyParms
{
    pub client_id: String,
    pub tms_identity: String,
    pub rp_id: String,
    pub rp_account: String,
    pub host: String,
    pub host_account: String,
}

/**
 * The Danger Mode - Minimal Viable Product (MVP) version of TMS simplifies migration to TMS in
 * existing environments that meet certain requirements. Specifically, MVP supports the following:
 * 
 *  - Keys never expire.
 *  - Note that to satisfy foreign key constraints, records must be created
 *    in the following order: resource_provider_logins, delegations
 *  - Key dependency records are automatically created in these tables:
      - resource_provider_logins - non-expiring RP_LOGIN set up for user
 *    - delegations - non-expiring delegation established between tms_identity and client
 *
 * When the enable_mvp flag is turned on in the configuration file, clients can create keys without
 *   prior configuration in the above tables. TMS will automatically create those records based on
 *   the input to the key create call, eliminating the possibility that missing dependency records
 *   will cause key creation to fail. If a record already exists, it is not overwritten.
 */
pub async fn create_pubkey_dependencies(parms: MVPDependencyParms) -> Result<u64> {

    // --------------------- Variables used throughout ---------------------
    let expires_at = tms_utils::get_max_tms_utc();
    let mut insert_count: u64 = 0;

     // Use the same current UTC timestamp in all related time calculations.
     let now = timestamp_utc();

    // --------------------- Insert rp_login record ------------------------
    // Required inputs: tms_identity, rp_id, rp_account, enabled
    //
    // Create the input record.
    let input_record = RPLoginInput::new(
        parms.tms_identity.clone(),
        parms.rp_id.clone(),
        parms.rp_account.clone(),
        DB_TRUE,
        now.clone(),
        now.clone(),
        now.clone(),
    );

    // Insert rp_login if not already present.
    let count = insert_rp_login(input_record, NOT_STRICT).await?;
    if count > 0 {
        insert_count += count;
        info!("MVP: RP_LOGIN created for tms_identity: {} rp_id: {} rp_account: {} last_login: {}.",
              parms.tms_identity, parms.rp_id, parms.rp_account, now);
    }

    // --------------------- Insert delegations record ---------------------
    // Required inputs: client_id, rp_id, rp_account, tms_identity
    //
    // Create the input record.
    let input_record = DelegationInput::new(
        parms.client_id.clone(),
        parms.tms_identity.clone(),
        parms.rp_id.clone(),
        parms.rp_account.clone(),
        expires_at,
        now.clone(),
        now.clone()
    );

    // Insert if not already present.
    let count = insert_delegation(input_record, NOT_STRICT).await?;
    if count > 0 {
        insert_count += count;
        info!("MVP/DangerMode: Delegation records created. tms_identity: {} rp_id: {} rp_account {} client_id: {}",
              parms.tms_identity, parms.rp_id, parms.rp_account, parms.client_id);
    }
    Ok(insert_count)
}
