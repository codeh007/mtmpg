use jaws::key::JsonWebKey;
use p256::ecdsa::SigningKey;
use pggomtm::database_auth::{
    AuthMethod, AuthenticatedActor, DatabaseProfile, DatabaseTokenPolicy, DatabaseTokenVerifier,
};
use serde_json::{Value, json};

use mtmpg_executor::issuer::{
    DATABASE_TOKEN_TTL_SECONDS, DatabaseTokenIssuer, IssuerConfig, IssuerError,
};
use mtmpg_executor::protocol::DelegatedPrincipal;

const NOW: i64 = 1_800_000_000;
const ISSUER: &str = "https://auth.example.test/database";
const AUDIENCE: &str = "https://postgres.example.test/database/main";
const KID: &str = "executor-es256-test";

fn signing_key() -> SigningKey {
    SigningKey::from_slice(&[9_u8; 32]).expect("fixed synthetic signing key")
}

fn verifier(key: &SigningKey) -> DatabaseTokenVerifier {
    let mut jwk =
        serde_json::to_value(JsonWebKey::build(key.verifying_key())).expect("serialize public JWK");
    let object = jwk.as_object_mut().expect("JWK object");
    object.insert("alg".into(), json!("ES256"));
    object.insert("key_ops".into(), json!(["verify"]));
    object.insert("kid".into(), json!(KID));
    object.insert("use".into(), json!("sig"));
    let jwks = serde_json::to_string(&json!({"keys": [jwk]})).expect("serialize JWKS");
    let policy = DatabaseTokenPolicy::new(ISSUER, AUDIENCE).expect("valid policy");
    DatabaseTokenVerifier::from_jwks(&jwks, policy).expect("valid verifier")
}

fn issuer() -> DatabaseTokenIssuer {
    let config = IssuerConfig::new(ISSUER, AUDIENCE, KID).expect("valid issuer config");
    DatabaseTokenIssuer::new(config, signing_key())
}

fn principal(method: AuthMethod, profile: DatabaseProfile) -> DelegatedPrincipal {
    let (client_id, credential_id) = match method {
        AuthMethod::OAuth => (Some("cli_01".into()), None),
        AuthMethod::ApiKey => (None, Some("crd_01".into())),
    };
    DelegatedPrincipal {
        user_id: "usr_01".into(),
        client_id,
        credential_id,
        delegation_id: "dlg_01".into(),
        auth_method: method,
        authority_version: 7,
        database_scope: "database".into(),
        profile,
        credential_expires_at: Some(NOW + 300),
    }
}

#[test]
fn issues_exact_thirty_second_tokens_for_every_actor_and_generic_profile() {
    let key = signing_key();
    let verifier = verifier(&key);
    let issuer = {
        let config = IssuerConfig::new(ISSUER, AUDIENCE, KID).expect("valid issuer config");
        DatabaseTokenIssuer::new(config, key)
    };

    for method in [AuthMethod::OAuth, AuthMethod::ApiKey] {
        for profile in [
            DatabaseProfile::Ordinary,
            DatabaseProfile::BusinessAdmin,
            DatabaseProfile::DatabaseDeveloper,
        ] {
            let principal = principal(method, profile);
            let token = issuer.issue(&principal, NOW).expect("database token");
            let verified = verifier
                .verify(token.as_str(), profile.database_role(), NOW)
                .expect("validator accepts executor token");

            assert_eq!(verified.claims.issuer, ISSUER);
            assert_eq!(verified.claims.audience, AUDIENCE);
            assert_eq!(verified.claims.subject, principal.user_id);
            assert_eq!(verified.claims.issued_at, NOW);
            assert_eq!(verified.claims.expires_at, NOW + DATABASE_TOKEN_TTL_SECONDS);
            assert_eq!(verified.claims.scope, "database");
            assert_eq!(verified.claims.delegation_id, principal.delegation_id);
            assert_eq!(verified.claims.auth_method, method);
            assert_eq!(
                verified.claims.authority_version,
                principal.authority_version
            );
            assert_eq!(verified.claims.db_profile, profile);
            assert_eq!(verified.claims.db_role, profile.database_role());
            assert_eq!(
                verified.identity.actor,
                match method {
                    AuthMethod::OAuth => AuthenticatedActor::OAuthClient("cli_01".into()),
                    AuthMethod::ApiKey => {
                        AuthenticatedActor::ApiKeyCredential("crd_01".into())
                    }
                }
            );
            assert_eq!(verified.claims.token_id.len(), 32);
            assert!(
                verified
                    .claims
                    .token_id
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            );
        }
    }
}

#[test]
fn credential_must_cover_the_complete_token_lifetime() {
    let issuer = issuer();
    let mut too_short = principal(AuthMethod::OAuth, DatabaseProfile::Ordinary);
    too_short.credential_expires_at = Some(NOW + DATABASE_TOKEN_TTL_SECONDS - 1);
    assert_eq!(
        issuer.issue(&too_short, NOW),
        Err(IssuerError::CredentialExpiresTooSoon)
    );

    let mut exact = too_short;
    exact.credential_expires_at = Some(NOW + DATABASE_TOKEN_TTL_SECONDS);
    assert!(issuer.issue(&exact, NOW).is_ok());
}

#[test]
fn non_expiring_api_key_can_issue_but_oauth_cannot_omit_expiry() {
    let issuer = issuer();
    let mut api_key = principal(AuthMethod::ApiKey, DatabaseProfile::Ordinary);
    api_key.credential_expires_at = None;
    assert!(issuer.issue(&api_key, NOW).is_ok());

    let mut oauth = principal(AuthMethod::OAuth, DatabaseProfile::Ordinary);
    oauth.credential_expires_at = None;
    assert_eq!(
        issuer.issue(&oauth, NOW),
        Err(IssuerError::InvalidPrincipal)
    );
}

#[test]
fn invalid_actor_or_caller_claim_shape_never_reaches_signing() {
    let issuer = issuer();
    let mut both = principal(AuthMethod::OAuth, DatabaseProfile::Ordinary);
    both.credential_id = Some("crd_01".into());
    assert_eq!(issuer.issue(&both, NOW), Err(IssuerError::InvalidPrincipal));

    let mut wrong_scope = principal(AuthMethod::OAuth, DatabaseProfile::Ordinary);
    wrong_scope.database_scope = "administrator".into();
    assert_eq!(
        issuer.issue(&wrong_scope, NOW),
        Err(IssuerError::InvalidPrincipal)
    );
}

#[test]
fn issuer_configuration_is_strict_and_does_not_accept_ambiguous_resources() {
    for (issuer, audience, kid) in [
        ("http://auth.example.test/database", AUDIENCE, KID),
        (ISSUER, ISSUER, KID),
        (ISSUER, AUDIENCE, ""),
        (ISSUER, AUDIENCE, "kid with spaces"),
    ] {
        assert_eq!(
            IssuerConfig::new(issuer, audience, kid),
            Err(IssuerError::InvalidConfiguration)
        );
    }
}

#[test]
fn issued_token_debug_output_is_redacted() {
    let token = issuer()
        .issue(
            &principal(AuthMethod::OAuth, DatabaseProfile::Ordinary),
            NOW,
        )
        .expect("database token");
    let rendered = format!("{token:?}");
    assert!(!rendered.contains(token.as_str()));
    assert!(!rendered.contains("eyJ"));
}

#[test]
fn public_jwk_fixture_contains_no_private_material() {
    let value = serde_json::to_value(JsonWebKey::build(signing_key().verifying_key()))
        .expect("serialize public key");
    let object = value.as_object().expect("public JWK object");
    assert_eq!(object.get("kty"), Some(&Value::String("EC".into())));
    assert!(!object.contains_key("d"));
}
