use std::ffi::c_char;
use std::ptr;

use pggomtm::oauth_abi::{
    OAuthValidatorCallbacks, OAuthValidatorModuleInit, PG_OAUTH_BINDINGS_SHA256,
    PG_OAUTH_HEADER_SHA256, PG_OAUTH_VALIDATOR_MAGIC, ValidatorModuleResult, ValidatorModuleState,
    ValidatorShutdownCB, ValidatorStartupCB, ValidatorValidateCB,
};

unsafe extern "C-unwind" fn startup_callback(_state: *mut ValidatorModuleState) {}

unsafe extern "C-unwind" fn shutdown_callback(_state: *mut ValidatorModuleState) {}

unsafe extern "C-unwind" fn validate_callback(
    _state: *const ValidatorModuleState,
    _token: *const c_char,
    _role: *const c_char,
    _result: *mut ValidatorModuleResult,
) -> bool {
    false
}

unsafe extern "C-unwind" fn module_init() -> *const OAuthValidatorCallbacks {
    ptr::null()
}

#[test]
fn official_header_generates_the_complete_oauth_abi_surface() {
    assert_eq!(
        PG_OAUTH_HEADER_SHA256,
        "be015ae68deef28a906c8739bc653ca90a4c6966c10f0efd3bd926efb4958bcf"
    );
    assert_eq!(
        PG_OAUTH_BINDINGS_SHA256,
        "b6f8bf810c467f74a0e43f9019f00cfd517cc881c9b606175818ca1b17204beb"
    );
    assert_eq!(PG_OAUTH_VALIDATOR_MAGIC, 0x2025_0220);

    let startup: ValidatorStartupCB = Some(startup_callback);
    let shutdown: ValidatorShutdownCB = Some(shutdown_callback);
    let validate: ValidatorValidateCB = Some(validate_callback);
    let init: OAuthValidatorModuleInit = Some(module_init);
    let callbacks = OAuthValidatorCallbacks {
        magic: PG_OAUTH_VALIDATOR_MAGIC,
        startup_cb: startup,
        shutdown_cb: shutdown,
        validate_cb: validate,
    };

    assert!(callbacks.startup_cb.is_some());
    assert!(callbacks.shutdown_cb.is_some());
    assert!(callbacks.validate_cb.is_some());
    assert!(init.is_some());
}
