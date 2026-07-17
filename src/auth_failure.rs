use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthenticationFailureReason {
    ConfigMissing,
    JwksMissing,
    ConfigTooLarge,
    JwksTooLarge,
    UnsafeFileType,
    UnsafePermissions,
    UnsafePublicationLayout,
    InvalidConfig,
    InvalidResources,
    InvalidJwks,
    DuplicateKeyId,
    InvalidTokenPolicy,
    InvalidToken,
    InvalidTokenHeader,
    UnknownKeyId,
    InvalidSignature,
    InvalidClaims,
    RequestedRoleMismatch,
    InvalidIdentity,
    InvalidCallbackInput,
    InvalidCallbackState,
    UnsupportedPostgresMajor,
    InternalPanic,
    PostgresError,
}

impl AuthenticationFailureReason {
    pub const ALL: [Self; 24] = [
        Self::ConfigMissing,
        Self::JwksMissing,
        Self::ConfigTooLarge,
        Self::JwksTooLarge,
        Self::UnsafeFileType,
        Self::UnsafePermissions,
        Self::UnsafePublicationLayout,
        Self::InvalidConfig,
        Self::InvalidResources,
        Self::InvalidJwks,
        Self::DuplicateKeyId,
        Self::InvalidTokenPolicy,
        Self::InvalidToken,
        Self::InvalidTokenHeader,
        Self::UnknownKeyId,
        Self::InvalidSignature,
        Self::InvalidClaims,
        Self::RequestedRoleMismatch,
        Self::InvalidIdentity,
        Self::InvalidCallbackInput,
        Self::InvalidCallbackState,
        Self::UnsupportedPostgresMajor,
        Self::InternalPanic,
        Self::PostgresError,
    ];

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ConfigMissing => "pggomtm-auth/v1/config-missing",
            Self::JwksMissing => "pggomtm-auth/v1/jwks-missing",
            Self::ConfigTooLarge => "pggomtm-auth/v1/config-too-large",
            Self::JwksTooLarge => "pggomtm-auth/v1/jwks-too-large",
            Self::UnsafeFileType => "pggomtm-auth/v1/material-file-type-unsafe",
            Self::UnsafePermissions => "pggomtm-auth/v1/material-permissions-unsafe",
            Self::UnsafePublicationLayout => "pggomtm-auth/v1/material-publication-layout-unsafe",
            Self::InvalidConfig => "pggomtm-auth/v1/config-invalid",
            Self::InvalidResources => "pggomtm-auth/v1/resources-invalid",
            Self::InvalidJwks => "pggomtm-auth/v1/jwks-invalid",
            Self::DuplicateKeyId => "pggomtm-auth/v1/jwks-duplicate-kid",
            Self::InvalidTokenPolicy => "pggomtm-auth/v1/token-policy-invalid",
            Self::InvalidToken => "pggomtm-auth/v1/token-invalid",
            Self::InvalidTokenHeader => "pggomtm-auth/v1/token-header-invalid",
            Self::UnknownKeyId => "pggomtm-auth/v1/token-kid-unknown",
            Self::InvalidSignature => "pggomtm-auth/v1/token-signature-invalid",
            Self::InvalidClaims => "pggomtm-auth/v1/token-claims-invalid",
            Self::RequestedRoleMismatch => "pggomtm-auth/v1/token-role-mismatch",
            Self::InvalidIdentity => "pggomtm-auth/v1/identity-invalid",
            Self::InvalidCallbackInput => "pggomtm-auth/v1/callback-input-invalid",
            Self::InvalidCallbackState => "pggomtm-auth/v1/callback-state-invalid",
            Self::UnsupportedPostgresMajor => "pggomtm-auth/v1/postgres-major-unsupported",
            Self::InternalPanic => "pggomtm-auth/v1/internal-panic",
            Self::PostgresError => "pggomtm-auth/v1/postgres-error",
        }
    }
}

impl fmt::Display for AuthenticationFailureReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}
