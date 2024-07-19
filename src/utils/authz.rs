#![forbid(unsafe_code)]

use poem::Request;
use futures::executor::block_on;
use sqlx::Row;
use anyhow::{Result, anyhow};

use log::error;

use crate::utils::tms_utils::hash_hex_secret;
use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Constants and Enums
// ***************************************************************************
// Authorization headers.
pub const X_TMS_ADMIN_ID:      &str = "X-TMS-ADMIN-ID";
pub const X_TMS_ADMIN_SECRET:  &str = "X-TMS-ADMIN-SECRET";
pub const X_TMS_CLIENT_ID:     &str = "X-TMS-CLIENT-ID";
pub const X_TMS_CLIENT_SECRET: &str = "X-TMS-CLIENT-SECRET";

// The different types of authorizations that can be checked.  Each implemented
// authz type is configured with a AuthzSpec that is stored in the static
// AuthzArgs component of RUNTIME_CTX (see config.rs for details).
#[derive(Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum AuthzTypes {ClientOwn, TenantAdmin, TmsctlHost, UserOwn}

// ***************************************************************************
//                          Public Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// authorize:
// ---------------------------------------------------------------------------
pub fn authorize(http_req: &Request, tenant: &str, allowed: &[AuthzTypes]) -> bool {
    // For each authz type, validate the required headers.
    for authz_type in allowed {
        let allow = match authz_type {
            AuthzTypes::ClientOwn => authorize_by_type(http_req, tenant, AuthzTypes::ClientOwn),
            AuthzTypes::TenantAdmin => authorize_by_type(http_req, tenant, AuthzTypes::TenantAdmin),
            AuthzTypes::TmsctlHost => authorize_by_type(http_req, tenant, AuthzTypes::TmsctlHost),
            AuthzTypes::UserOwn => authorize_by_type(http_req, tenant, AuthzTypes::UserOwn),
        };

        // The first successful authorization terminates checking.
        if allow {return true;}
    }

    // If we get here, no authorization checks succeeded.
    false
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// authorize_by_type:
// ---------------------------------------------------------------------------
fn authorize_by_type(http_req: &Request, tenant: &str, authz_type: AuthzTypes) -> bool {
    // Get the runtime parameters for this authz type.  If the spec isn't found in the 
    // RUNTIME environment, then a compile time initialization value is missing.
    let spec = match RUNTIME_CTX.authz.specs.get(&authz_type) {
        Some(s) => s,
        None => {
            error!("ERROR: Authorization type not implemented: {:?}", authz_type);
            return false;
        },
    };

    // Initialize the header variables.
    let mut hdr_id     = "";
    let mut hdr_secret = "";

    // Look for the client id and secret headers.
    let it = http_req.headers().iter();
    for v in it {
        if v.0 == spec.id {
            hdr_id = match v.1.to_str() {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", spec.id, e);
                    return false;
                },
            };
            continue;
        }
        if v.0 == spec.secret {
            hdr_secret = match v.1.to_str() {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", spec.secret, e);
                    return false;
                },
            };
        }
    };

    // Did we find an id and secret?
    if hdr_id.is_empty() || hdr_secret.is_empty() {return false;}

    // Query the database for the client secret.
    let db_secret_hash = match block_on(get_authz_secret(hdr_id, tenant, spec.sql_query)) {
        Ok(s) => s,
        Err(e) => {
            error!("Unable to retrieve secret for {} id {}: {}", spec.display_name, hdr_id, e);
            return false;
        },
    };

    // Compare the header secret to the hashed secret from the database.
    let hdr_secret_hash = hash_hex_secret(&hdr_secret.to_string());
    if hdr_secret_hash == db_secret_hash {
        true  // Authorized
    } else {
        error!("Invalid secret given for {} {} in tenant {}", spec.display_name, hdr_id, tenant);
        false // Not authorized
    }
}

// ***************************************************************************
//                          Database Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_client_secret:
// ---------------------------------------------------------------------------
async fn get_authz_secret(id: &str, tenant: &str, sql_query: &str) -> Result<String> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement using the query passed in for the authorization
    // type we are running.  Note that only queries that take id as the 1st parameter
    // and tenant as the 2nd parameter are supported.  If in the future different
    // query signatures are required, we can bind different query parameters based
    // on authz type. 
    let result = sqlx::query(sql_query)
        .bind(id)
        .bind(tenant)
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // Did we find the secret?
    match result {
        Some(row) => {
            Ok(row.get(0))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}

