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

    /** Check that the tenant value in the request is the same as the 
     * header tenant value.
     */
    pub fn check_hdr_tenant(&self, req_tenant: &String) -> bool {
        // Guard for unauthorized calls, which should never happen.
        if !self.authorized {return false;} 

        // Test whether the header and request tenants are the same.
        match &self.hdr_tenant {
            Some(hdr_tenant) => hdr_tenant == req_tenant,
            None => false
        }
    }

    /** Check that the authorized ID passed via header matches the ID passed in the path or 
     * body of a request.  For example, this consistency check can be used to guarentee that 
     * client ID  provided in the http request header is the same as that provided in the 
     * request's path parameters or payload parameter.  Processing is authz type specific. 
     *  
     * Return FALSE if the original authorization failed or the request ID is different from
     * the header ID.  Return TRUE only if the authorization type depends on non-administrative
     * IDs and the header and request IDs match.
     */
    pub fn check_hdr_id(&self, req_id: &String) -> bool {
        // Guard for unauthorized calls, which should never happen.
        if !self.authorized {return false;} 

        // ----------- AuthzTypes-specific checks
        match &self.authz_type {
            Some(atype) => {
                match atype {
                    AuthzTypes::ClientOwn => self.check_client_own(req_id),
                    _ => true  // non-client authorization (ex: admin)
                }
            },
            None => false,
        }
    }

    /** Check that if client credentials were used to authorize a request that the client id 
     * used in the credentials is the same as that provided in the request.  This consistency
     * check guarentees that client id provided in the http request header are the same as 
     * that provided in the request's path parameters or payload.
     *  
     * Return FALSE if the client_ids don't match, return TRUE otherwise.
     */
    fn check_client_own(&self, req_client_id: &String) -> bool {
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
// get_tenant_header:
// ---------------------------------------------------------------------------
pub fn get_tenant_header(http_req: &Request) -> Result<String> {
    get_header(http_req, X_TMS_TENANT)
}

// ---------------------------------------------------------------------------
// get_client_id_header:
// ---------------------------------------------------------------------------
pub fn get_client_id_header(http_req: &Request) -> Result<String> {
    get_header(http_req, X_TMS_CLIENT_ID)
}

// ---------------------------------------------------------------------------
// get_client_id_header_string:
// ---------------------------------------------------------------------------
/** Get the value of the X_TMS_CLIENT_ID as a string.  Return the empty string
 * if the value is not set or cannot be converted to a string.
 */
#[allow(dead_code)]
pub fn get_client_id_header_string(http_req: &Request) -> String {
    match http_req.headers().get(X_TMS_CLIENT_ID) {
        Some(v) => {
            match v.to_str() {
                Ok(s) => s.to_string(),
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", X_TMS_CLIENT_ID, e);
                    "".to_string()
                }
            }
        },
        None => "".to_string(),
    }
}

// ---------------------------------------------------------------------------
// authorize:
// ---------------------------------------------------------------------------
pub fn authorize(http_req: &Request, allowed: &[AuthzTypes]) -> AuthzResult {
    // Get the required tenant header once.
    let hdr_tenant = match get_tenant_header_str(http_req) {
        Ok(t) => t,
        Err(_) => return AuthzResult::new_unauthorized(),
    };

    // For each authz type, validate the required headers.
    for authz_type in allowed {
        let result = match authz_type {
            AuthzTypes::ClientOwn => authorize_by_type(http_req, hdr_tenant, AuthzTypes::ClientOwn),
            AuthzTypes::TenantAdmin => authorize_by_type(http_req, hdr_tenant, AuthzTypes::TenantAdmin),
            AuthzTypes::TmsctlHost => authorize_by_type(http_req, hdr_tenant, AuthzTypes::TmsctlHost),
            AuthzTypes::UserOwn => authorize_by_type(http_req, hdr_tenant, AuthzTypes::UserOwn),
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
// get_header:
// ---------------------------------------------------------------------------
/** Get the TMS value from its http header. This function logs its errors
 * so the caller does not have to.
 */
fn get_header(http_req: &Request, header_key: &str) -> Result<String> {
    match http_req.headers().get(header_key) {
        Some(v) => {
            match v.to_str() {
                Ok(s) => Ok(s.to_string()),
                Err(e) => {
                    let e2 = anyhow!("Invalid string assigned to header {}: {}", header_key, e);
                    error!("{}", e2);
                    Err(e2)
                }
            }
        },
        None => {
            let e = anyhow!("Required '{}' HTTP header not found", header_key);
            error!("{}", e);
            Err(e)
        }
    }
}

// ---------------------------------------------------------------------------
// authorize_by_type:
// ---------------------------------------------------------------------------
fn authorize_by_type(http_req: &Request, hdr_tenant: &str, authz_type: AuthzTypes) -> AuthzResult {
    // Get the runtime parameters for this authz type.  If the spec isn't found in the 
    // RUNTIME environment, then a compile time initialization value is missing.
    let spec = match RUNTIME_CTX.authz.specs.get(&authz_type) {
        Some(s) => s,
        None => {
            error!("ERROR: Authorization type not implemented: {:?}", authz_type);
            return AuthzResult::new_unauthorized();
        },
    };

    // Get the ID header for this authz type.  Use the empty string
    // to indicate header-not-found or an invalid header value. 
    let hdr_id = match http_req.headers().get(spec.id) {
        Some(hdr_val) => {
            match hdr_val.to_str() {
                Ok(str_val) => str_val,
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", spec.id, e);
                    ""
                }
            } 
        },
        None => "",
    };

    // Get the secret header for this authz type.  Use the empty string
    // to indicate header-not-found or an invalid header value.  
    let hdr_secret = match http_req.headers().get(spec.secret) {
        Some(hdr_val) => {
            match hdr_val.to_str() {
                Ok(str_val) => str_val,
                Err(e) => {
                    error!("Invalid string assigned to header {}: {}", spec.secret, e);
                    ""
                }
            } 
        },
        None => "",
    };

    // Do we have the complete set of tenant, id and secret?
    if hdr_tenant.is_empty() || hdr_id.is_empty() || hdr_secret.is_empty() {
        debug!("Missing header information for {} id {}", spec.display_name, hdr_id);
        return AuthzResult::new_unauthorized();
    }

    // TODO LOADTEST skip db query for secret
    // // Query the database for the client secret.
    // let db_secret_hash = match block_on(get_authz_secret(hdr_id, hdr_tenant, spec.sql_query)) {
    //     Ok(s) => s,
    //     Err(e) => {
    //         error!("Unable to retrieve secret for {} ID '{}': {}", spec.display_name, hdr_id, e);
    //         return AuthzResult::new_unauthorized();
    //     },
    // };

    // // Compare the header secret to the hashed secret from the database.
    // let hdr_secret_hash = hash_hex_secret(&hdr_secret.to_string());
    // if hdr_secret_hash == db_secret_hash {
    //     AuthzResult::new_authorized(authz_type, hdr_id.to_string(), hdr_tenant.to_string())  // Authorized
    // } else {
    //     error!("Invalid secret given for {} {} in tenant {}", spec.display_name, hdr_id, hdr_tenant);
    //     AuthzResult::new_unauthorized() // Not authorized
    // }
    AuthzResult::new_authorized(authz_type, hdr_id.to_string(), hdr_tenant.to_string())  // Authorized
}

// ---------------------------------------------------------------------------
// get_tenant_header_str:
// ---------------------------------------------------------------------------
/** Get the TMS tenant value from its http header.  This function logs its errors
 * so the caller does not have to.   
 */
fn get_tenant_header_str(http_req: &Request) -> Result<&str> {
    match http_req.headers().get(X_TMS_TENANT) {
        Some(v) => {
            match v.to_str() {
                Ok(s) => Ok(s),
                Err(e) => {
                    let msg = format!("Invalid string assigned to header {}: {}", X_TMS_TENANT, e);
                    error!("{}", msg);
                    Err(anyhow!(msg))
                }
            }
        },
        None => {
            let msg = format!("Required '{}' HTTP header not found", X_TMS_TENANT);
            error!("{}", msg);
            Err(anyhow!(msg))
        }
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


