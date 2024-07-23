#![forbid(unsafe_code)]

use poem::Request;
use futures::executor::block_on;
use sqlx::Row;
use anyhow::{Result, anyhow};

use log::{error, debug};

use crate::utils::tms_utils::hash_hex_secret;
use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Constants and Enums
// ***************************************************************************
// Authorization headers.
pub const X_TMS_TENANT:        &str = "X-TMS-TENANT";
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
//                            Result Struct
// ***************************************************************************
#[derive(Debug)]
pub struct AuthzResult {
    pub authorized: bool,
    pub authz_type: Option<AuthzTypes>,
    pub hdr_id: Option<String>,
    pub hdr_tenant: Option<String>,
}

impl AuthzResult {
    // Complete authorized result.
    fn new_authorized( 
           authz_type: AuthzTypes,
           hdr_id: String,
           hdr_tenant: String) -> Self
    {
        Self {authorized: true, authz_type: Option::Some(authz_type), 
              hdr_id: Option::Some(hdr_id), hdr_tenant: Option::Some(hdr_tenant)}
    }

    // Complete unauthorized result.
    fn new_unauthorized() -> Self {
         Self {authorized: false, authz_type: Option::None, 
               hdr_id: Option::None, hdr_tenant: Option::None}
     }
     
    pub fn is_authorized(&self) -> bool {
        self.authorized
    }

    /** Check that if client credentials were used to authorize a request that the client id 
     * used in the credentials is the same as that provided in the request.  This consistency
     * check guarentees that client id and tenant provided in the http request header are the
     * same as those provided in the request's path parameters or payload parameters.
     *  
     * Return FALSE if the original authorization failed, is not well-defined, or the 
     * tenant/client_id pairs don't match.  Return TRUE only if authorization did not use client
     * credentials or if the tenant/client_id pairs match.
     */
    pub fn check_request_parms(&self, req_id: &String, req_tenant: &String) -> bool {
        // Guard for unauthorized calls, which should never happen.
        if !self.authorized {return false;} 

        // ----------- Tenant Check
        // Get the tenant passed in as a header and make sure it 
        // matches the request tenant.  This check is applied to 
        // all authz types.
        let hdr_tenant = match &self.hdr_tenant {
            Some(tenant) => tenant,
            None => return false,
        };
        if req_tenant != hdr_tenant {return false;} // tenant mismatch

        // ----------- AuthzTypes-specific checks
        match &self.authz_type {
            Some(atype) => {
                match atype {
                    AuthzTypes::ClientOwn => self.check_client_parms(req_id),
                    _ => true
                }
            },
            None => false,
        }
    }

    /** Check that if client credentials were used to authorize a request that the client id 
     * used in the credentials is the same as that provided in the request.  This consistency
     * check guarentees that client id and tenant (checked above) provided in the http request 
     * header are the same as those provided in the request's path parameters or payload parameters.
     *  
     * Return FALSE if the client_ids don't match, return TRUE otherwise.
     */
    fn check_client_parms(&self, req_client_id: &String) -> bool {
        // Get the authz id passed in as a header.
        let hdr_id = match &self.hdr_id {
            Some(id) => id,
            None => return false,
        };
        
        // Make sure the client id from header and request match.
        if req_client_id == hdr_id {true} // client match
         else {false} // client mismatch
    }
}

// ***************************************************************************
//                          Public Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// authorize:
// ---------------------------------------------------------------------------
pub fn authorize(http_req: &Request, allowed: &[AuthzTypes]) -> AuthzResult {
    // For each authz type, validate the required headers.
    for authz_type in allowed {
        let result = match authz_type {
            AuthzTypes::ClientOwn => authorize_by_type(http_req, AuthzTypes::ClientOwn),
            AuthzTypes::TenantAdmin => authorize_by_type(http_req, AuthzTypes::TenantAdmin),
            AuthzTypes::TmsctlHost => authorize_by_type(http_req, AuthzTypes::TmsctlHost),
            AuthzTypes::UserOwn => authorize_by_type(http_req, AuthzTypes::UserOwn),
        };

        // The first successful authorization terminates checking.
        if result.is_authorized() {return result;}
    }

    // If we get here, no authorization checks succeeded.
    AuthzResult::new_unauthorized()
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// authorize_by_type:
// ---------------------------------------------------------------------------
fn authorize_by_type(http_req: &Request, authz_type: AuthzTypes) -> AuthzResult {
    // Get the runtime parameters for this authz type.  If the spec isn't found in the 
    // RUNTIME environment, then a compile time initialization value is missing.
    let spec = match RUNTIME_CTX.authz.specs.get(&authz_type) {
        Some(s) => s,
        None => {
            error!("ERROR: Authorization type not implemented: {:?}", authz_type);
            return AuthzResult::new_unauthorized();
        },
    };

    // Initialize the header variables.
    let mut hdr_id     = "";
    let mut hdr_secret = "";
    let mut hdr_tenant = "";

    // Look for the id and secret headers.
    let it = http_req.headers().iter();
    for v in it {
        if v.0 == X_TMS_TENANT {
            if !hdr_tenant.is_empty() {continue;} // only assign once
            hdr_tenant = match v.1.to_str() {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", X_TMS_TENANT, e);
                    return AuthzResult::new_unauthorized();
                },
            };
            continue;
        }
        if v.0 == spec.id {
            if !hdr_id.is_empty() {continue;} // only assign once
            hdr_id = match v.1.to_str() {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", spec.id, e);
                    return AuthzResult::new_unauthorized();
                },
            };
            continue;
        }
        if v.0 == spec.secret {
            if !hdr_secret.is_empty() {continue;} // only assign once
            hdr_secret = match v.1.to_str() {
                Ok(v) => v,
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", spec.secret, e);
                    return AuthzResult::new_unauthorized();
                },
            };
        }
    };

    // Did we find an id and secret?
    if hdr_tenant.is_empty() || hdr_id.is_empty() || hdr_secret.is_empty() {
        debug!("Missing header information for {} id {}", spec.display_name, hdr_id);
        return AuthzResult::new_unauthorized();
    }

    // Query the database for the client secret.
    let db_secret_hash = match block_on(get_authz_secret(hdr_id, hdr_tenant, spec.sql_query)) {
        Ok(s) => s,
        Err(e) => {
            error!("Unable to retrieve secret for {} id {}: {}", spec.display_name, hdr_id, e);
            return AuthzResult::new_unauthorized();
        },
    };

    // Compare the header secret to the hashed secret from the database.
    let hdr_secret_hash = hash_hex_secret(&hdr_secret.to_string());
    if hdr_secret_hash == db_secret_hash {
        AuthzResult::new_authorized(authz_type, hdr_id.to_string(), hdr_tenant.to_string())  // Authorized
    } else {
        error!("Invalid secret given for {} {} in tenant {}", spec.display_name, hdr_id, hdr_tenant);
        AuthzResult::new_unauthorized() // Not authorized
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

