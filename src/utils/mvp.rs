#![forbid(unsafe_code)]

use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_types::{DelegationInput, UserMfaInput, UserHostInput};
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_to_str, MAX_TMS_UTC};
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
