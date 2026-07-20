#[cfg(all(
    feature = "pg18",
    any(feature = "abi-runtime-gate", feature = "pgx-oauth-gate")
))]
use std::ffi::CStr;
#[cfg(all(feature = "pg18", feature = "pgx-oauth-gate"))]
use std::ffi::CString;
#[cfg(all(
    feature = "pg18",
    not(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    ))
))]
use std::ffi::{CStr, CString};
#[cfg(feature = "pg18")]
use std::ffi::{c_char, c_void};
#[cfg(feature = "pg18")]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(feature = "pg18")]
use std::ptr;

#[cfg(feature = "pg18")]
use auth_failure::AuthenticationFailureReason;
#[cfg(all(
    feature = "pg18",
    not(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    ))
))]
use runtime_config::{ValidatorSnapshot, load_validator_snapshot};

#[cfg(feature = "pg18")]
pgrx::pg_module_magic!();

#[cfg(feature = "database-token-contract")]
pub mod auth_failure;
#[cfg(feature = "database-token-contract")]
pub mod database_auth;
#[cfg(feature = "pg18")]
pub mod oauth_abi;
#[cfg(feature = "pg18")]
pub mod runtime_config;

#[cfg(feature = "pg18")]
pub use oauth_abi::{
    OAuthValidatorCallbacks, OAuthValidatorModuleInit, PG_OAUTH_VALIDATOR_MAGIC,
    ValidatorModuleResult, ValidatorModuleState, ValidatorShutdownCB, ValidatorStartupCB,
    ValidatorValidateCB,
};

#[cfg(feature = "pg18")]
pub const PG18_VERSION_NUM: i32 = 180_000;

#[cfg(all(feature = "pg18", feature = "abi-runtime-gate"))]
const ABI_RUNTIME_PANIC_SENTINEL: usize = 1;
#[cfg(all(feature = "pg18", feature = "abi-runtime-gate"))]
const ABI_RUNTIME_ERROR_SENTINEL: usize = 2;

#[cfg(all(feature = "pg18", feature = "pgx-oauth-gate"))]
const PGX_OAUTH_GATE_JWKS: &str = r#"{"keys":[{"kty":"EC","crv":"P-256","alg":"ES256","use":"sig","key_ops":["verify"],"kid":"candidate-es256-pgx-gate","x":"HhhTL9R1TALzBB2cdc6zO4P_2BrHzk_ogsyxyYvFiW4","y":"pGwxHE4v9A3ZajZT5uRURdMt_khuztdcepDGoYiBwKM"}]}"#;

#[cfg(all(feature = "pg18", feature = "pgx-oauth-gate"))]
fn verify_pgx_gate_token(
    token: &str,
    requested_role: &str,
    now: i64,
) -> Result<String, database_auth::JwtValidationError> {
    let policy = database_auth::DatabaseTokenPolicy::new(
        "https://candidate.example.test/oauth/database",
        "https://candidate.example.test/resources/database/gomtm-test",
    )?;
    let verifier = database_auth::DatabaseTokenVerifier::from_jwks(PGX_OAUTH_GATE_JWKS, policy)?;
    Ok(verifier.verify(token, requested_role, now)?.authn_id)
}

#[cfg(feature = "pg18")]
static OAUTH_CALLBACKS: OAuthValidatorCallbacks = OAuthValidatorCallbacks {
    magic: PG_OAUTH_VALIDATOR_MAGIC,
    startup_cb: Some(validator_startup),
    shutdown_cb: Some(validator_shutdown),
    validate_cb: Some(validate_token),
};

#[must_use]
#[cfg(feature = "pg18")]
pub const fn server_version_is_supported(server_version: i32) -> bool {
    server_version / 10_000 == 18
}

#[must_use]
#[cfg(feature = "pg18")]
pub fn oauth_callbacks() -> &'static OAuthValidatorCallbacks {
    &OAUTH_CALLBACKS
}

#[cfg(feature = "pg18")]
#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
unsafe extern "C-unwind" fn validator_startup(state: *mut ValidatorModuleState) {
    if state.is_null() {
        raise_authentication_error(AuthenticationFailureReason::InvalidCallbackInput);
    }

    let state = unsafe { &mut *state };
    #[cfg(feature = "abi-runtime-gate")]
    let initial_private_data = state.private_data;

    #[cfg(not(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    )))]
    if !state.private_data.is_null() {
        raise_authentication_error(AuthenticationFailureReason::InvalidCallbackState);
    }

    state.private_data = ptr::null_mut();

    #[cfg(feature = "abi-runtime-gate")]
    match initial_private_data.addr() {
        ABI_RUNTIME_PANIC_SENTINEL => panic!(
            "pggomtm authentication failed: reason={}",
            AuthenticationFailureReason::InternalPanic.code()
        ),
        ABI_RUNTIME_ERROR_SENTINEL => {
            raise_authentication_error(AuthenticationFailureReason::PostgresError)
        }
        _ => {}
    }

    if !server_version_is_supported(state.sversion) {
        raise_authentication_error(AuthenticationFailureReason::UnsupportedPostgresMajor);
    }

    #[cfg(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    ))]
    {
        state.private_data = state as *mut ValidatorModuleState as *mut c_void;
    }

    #[cfg(not(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    )))]
    {
        let snapshot = load_validator_snapshot()
            .unwrap_or_else(|error| raise_authentication_error(error.reason()));
        state.private_data = Box::into_raw(Box::new(snapshot)).cast::<c_void>();
    }
}

#[cfg(feature = "pg18")]
#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
unsafe extern "C-unwind" fn validator_shutdown(state: *mut ValidatorModuleState) {
    if let Some(state) = unsafe { state.as_mut() } {
        #[cfg(any(
            feature = "abi-runtime-gate",
            not(any(
                feature = "abi-gate",
                feature = "abi-runtime-gate",
                feature = "pgx-oauth-gate"
            ))
        ))]
        let initial_private_data = state.private_data;
        state.private_data = ptr::null_mut();

        #[cfg(feature = "abi-runtime-gate")]
        match initial_private_data.addr() {
            ABI_RUNTIME_PANIC_SENTINEL => panic!(
                "pggomtm authentication failed: reason={}",
                AuthenticationFailureReason::InternalPanic.code()
            ),
            ABI_RUNTIME_ERROR_SENTINEL => {
                raise_authentication_error(AuthenticationFailureReason::PostgresError)
            }
            _ => {}
        }

        #[cfg(not(any(
            feature = "abi-gate",
            feature = "abi-runtime-gate",
            feature = "pgx-oauth-gate"
        )))]
        if !initial_private_data.is_null() {
            drop(unsafe { Box::from_raw(initial_private_data.cast::<ValidatorSnapshot>()) });
        }
    }
}

#[cfg(feature = "pg18")]
#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
unsafe extern "C-unwind" fn validate_token(
    state: *const ValidatorModuleState,
    token: *const c_char,
    role: *const c_char,
    result: *mut ValidatorModuleResult,
) -> bool {
    if result.is_null() {
        log_authentication_rejection(AuthenticationFailureReason::InvalidCallbackInput);
        return false;
    }

    unsafe {
        (*result).authorized = false;
        (*result).authn_id = ptr::null_mut();
    }

    let validation = catch_unwind(AssertUnwindSafe(|| {
        let Some(state) = (unsafe { state.as_ref() }) else {
            log_authentication_rejection(AuthenticationFailureReason::InvalidCallbackInput);
            return false;
        };

        if token.is_null() || role.is_null() {
            log_authentication_rejection(AuthenticationFailureReason::InvalidCallbackInput);
            return false;
        }
        if !server_version_is_supported(state.sversion) {
            log_authentication_rejection(AuthenticationFailureReason::UnsupportedPostgresMajor);
            return false;
        }

        #[cfg(any(
            feature = "abi-gate",
            feature = "abi-runtime-gate",
            feature = "pgx-oauth-gate"
        ))]
        if state.private_data != state as *const ValidatorModuleState as *mut c_void {
            log_authentication_rejection(AuthenticationFailureReason::InvalidCallbackState);
            return false;
        }

        #[cfg(not(any(
            feature = "abi-gate",
            feature = "abi-runtime-gate",
            feature = "pgx-oauth-gate"
        )))]
        if state.private_data.is_null() {
            log_authentication_rejection(AuthenticationFailureReason::InvalidCallbackState);
            return false;
        }

        #[cfg(feature = "abi-runtime-gate")]
        {
            let token = unsafe { CStr::from_ptr(token) };

            if token == c"pggomtm-abi-panic" {
                panic!(
                    "pggomtm authentication failed: reason={}",
                    AuthenticationFailureReason::InternalPanic.code()
                );
            }

            if token == c"pggomtm-abi-error" {
                raise_authentication_error(AuthenticationFailureReason::PostgresError);
            }

            if token == c"pggomtm-abi-allocator" {
                let result = unsafe { &mut *result };
                result.authn_id = unsafe { pgrx::pg_sys::pstrdup(token.as_ptr()) };
                return !result.authn_id.is_null();
            }
        }

        #[cfg(any(
            feature = "pgx-oauth-gate",
            not(any(
                feature = "abi-gate",
                feature = "abi-runtime-gate",
                feature = "pgx-oauth-gate"
            ))
        ))]
        {
            let token = unsafe { CStr::from_ptr(token) };
            let role = unsafe { CStr::from_ptr(role) };
            let (Ok(token), Ok(role)) = (token.to_str(), role.to_str()) else {
                log_authentication_rejection(AuthenticationFailureReason::InvalidCallbackInput);
                return true;
            };
            let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
                log_authentication_rejection(AuthenticationFailureReason::InvalidCallbackState);
                return false;
            };
            let now = i64::try_from(now.as_secs()).unwrap_or(i64::MAX);

            #[cfg(feature = "pgx-oauth-gate")]
            let authn_id = match verify_pgx_gate_token(token, role, now) {
                Ok(authn_id) => authn_id,
                Err(error) => {
                    log_authentication_rejection(error.reason());
                    return true;
                }
            };

            #[cfg(not(any(
                feature = "abi-gate",
                feature = "abi-runtime-gate",
                feature = "pgx-oauth-gate"
            )))]
            let authn_id = {
                let snapshot = unsafe { &*state.private_data.cast::<ValidatorSnapshot>() };
                let verified = match snapshot.verify(token, role, now) {
                    Ok(verified) => verified,
                    Err(error) => {
                        log_authentication_rejection(error.reason());
                        return true;
                    }
                };
                verified.authn_id
            };

            let Ok(authn_id) = CString::new(authn_id) else {
                log_authentication_rejection(AuthenticationFailureReason::InvalidIdentity);
                return false;
            };
            let result = unsafe { &mut *result };
            result.authn_id = unsafe { pgrx::pg_sys::pstrdup(authn_id.as_ptr()) };
            result.authorized = !result.authn_id.is_null();
            result.authorized
        }

        #[cfg(all(
            not(feature = "pgx-oauth-gate"),
            any(feature = "abi-gate", feature = "abi-runtime-gate")
        ))]
        true
    }));

    match validation {
        Ok(authorized) => authorized,
        Err(error)
            if error.is::<pgrx::pg_sys::panic::CaughtError>()
                || error.is::<pgrx::pg_sys::panic::ErrorReport>()
                || error.is::<pgrx::pg_sys::panic::ErrorReportWithLevel>() =>
        {
            std::panic::resume_unwind(error)
        }
        Err(_) => {
            log_authentication_rejection(AuthenticationFailureReason::InternalPanic);
            false
        }
    }
}

#[cfg(all(feature = "pg18", feature = "abi-gate"))]
fn log_authentication_rejection(_reason: AuthenticationFailureReason) {}

#[cfg(all(feature = "pg18", not(feature = "abi-gate")))]
fn log_authentication_rejection(reason: AuthenticationFailureReason) {
    pgrx::log!("pggomtm authentication rejected: reason={reason}");
}

#[cfg(all(feature = "pg18", not(feature = "abi-gate")))]
fn raise_authentication_error(reason: AuthenticationFailureReason) -> ! {
    pgrx::error!("pggomtm authentication failed: reason={reason}");
}

#[cfg(all(feature = "pg18", feature = "abi-gate"))]
fn raise_authentication_error(reason: AuthenticationFailureReason) -> ! {
    panic!("pggomtm authentication failed: reason={reason}");
}

#[cfg(feature = "pg18")]
#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
#[unsafe(no_mangle)]
pub extern "C-unwind" fn _PG_oauth_validator_module_init() -> *const OAuthValidatorCallbacks {
    oauth_callbacks() as *const OAuthValidatorCallbacks
}

#[cfg(feature = "pg18")]
const _: OAuthValidatorModuleInit = Some(_PG_oauth_validator_module_init);

#[cfg(all(feature = "pg18", feature = "abi-gate"))]
#[must_use]
pub fn panic_boundary_for_gate() -> bool {
    catch_unwind(AssertUnwindSafe(|| panic!("pggomtm ABI panic gate"))).is_ok()
}
