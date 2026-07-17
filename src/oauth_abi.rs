mod generated {
    // PostgreSQL 生成的 OAuth callback table 使用 server uint32 typedef。
    // https://docs.rs/pgrx/0.19.1/pgrx/pg_sys/type.uint32.html
    use pgrx::pg_sys::uint32;

    include!(concat!(env!("OUT_DIR"), "/pggomtm_oauth_bindings.rs"));
}

pub use generated::{
    OAuthValidatorCallbacks, OAuthValidatorModuleInit, PG_OAUTH_VALIDATOR_MAGIC,
    ValidatorModuleResult, ValidatorModuleState, ValidatorShutdownCB, ValidatorStartupCB,
    ValidatorValidateCB,
};

pub const PG_OAUTH_HEADER_SHA256: &str = env!("PG_OAUTH_HEADER_SHA256");
