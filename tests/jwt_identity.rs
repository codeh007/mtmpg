use std::collections::BTreeMap;

use base64ct::{Base64UrlUnpadded, Encoding};
use jaws::Token;
use jaws::key::JsonWebKey;
use p256::ecdsa::{Signature, SigningKey};
use pggomtm::database_auth::{
    AuthMethod, AuthenticatedActor, AuthenticatedIdentity, DatabaseProfile, DatabaseTokenClaims,
    DatabaseTokenPolicy, DatabaseTokenVerifier, JwtValidationError, MAX_AUTHN_ID_BYTES,
    decode_authn_id, decode_system_user,
};
use serde::Serialize;
use serde_json::{Value, json};

const ISSUER: &str = "https://candidate.example.test/oauth/database";
const AUDIENCE: &str = "https://candidate.example.test/resources/database/gomtm-test";
const NOW: i64 = 1_800_000_000;
const KID: &str = "candidate-es256-2026-07";

fn signing_key() -> SigningKey {
    SigningKey::from_slice(&[7_u8; 32]).expect("fixed test signing key")
}

fn jwk_value(key: &SigningKey, kid: &str) -> Value {
    let mut value =
        serde_json::to_value(JsonWebKey::build(key.verifying_key())).expect("serialize public JWK");
    let object = value.as_object_mut().expect("JWK object");
    object.insert("alg".into(), json!("ES256"));
    object.insert("key_ops".into(), json!(["verify"]));
    object.insert("kid".into(), json!(kid));
    object.insert("use".into(), json!("sig"));
    value
}

fn jwks_with(keys: Vec<Value>) -> String {
    serde_json::to_string(&json!({ "keys": keys })).expect("serialize JWKS")
}

fn verifier(jwks: &str) -> DatabaseTokenVerifier {
    let policy = DatabaseTokenPolicy::new(ISSUER, AUDIENCE).expect("absolute token policy");
    DatabaseTokenVerifier::from_jwks(jwks, policy).expect("valid verifier")
}

#[test]
fn policy_requires_distinct_absolute_https_resources() {
    assert_eq!(
        DatabaseTokenPolicy::new("/relative", AUDIENCE),
        Err(JwtValidationError::InvalidPolicy)
    );
    assert_eq!(
        DatabaseTokenPolicy::new("http://candidate.example.test/issuer", AUDIENCE),
        Err(JwtValidationError::InvalidPolicy)
    );
    assert_eq!(
        DatabaseTokenPolicy::new(ISSUER, ISSUER),
        Err(JwtValidationError::InvalidPolicy)
    );
}

fn valid_oauth_claims() -> DatabaseTokenClaims {
    DatabaseTokenClaims {
        issuer: ISSUER.into(),
        audience: AUDIENCE.into(),
        subject: "usr_01J00000000000000000000000".into(),
        issued_at: NOW,
        expires_at: NOW + 120,
        token_id: "jti_01J00000000000000000000000".into(),
        scope: "database".into(),
        delegation_id: "dlg_01J00000000000000000000000".into(),
        auth_method: AuthMethod::OAuth,
        authority_version: 7,
        db_profile: DatabaseProfile::Ordinary,
        db_role: DatabaseProfile::Ordinary.database_role().into(),
        client_id: Some("cli_01J00000000000000000000000".into()),
        credential_id: None,
    }
}

fn sign_payload(payload: impl Serialize, kid: &str, key: &SigningKey) -> String {
    let mut token = Token::compact((), payload);
    *token.header_mut().key_id() = Some(kid.into());
    token
        .sign::<_, Signature>(key)
        .expect("sign token")
        .rendered()
        .expect("render compact token")
}

fn mutate_header(token: &str, mutate: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
    let mut segments = token.split('.').map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(segments.len(), 3);
    let decoded = Base64UrlUnpadded::decode_vec(&segments[0]).expect("decode header");
    let mut header: Value = serde_json::from_slice(&decoded).expect("header JSON");
    mutate(header.as_object_mut().expect("header object"));
    segments[0] = Base64UrlUnpadded::encode_string(
        &serde_json::to_vec(&header).expect("serialize mutated header"),
    );
    segments.join(".")
}

#[test]
fn valid_es256_oauth_token_round_trips_versioned_identity() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let claims = valid_oauth_claims();
    let token = sign_payload(claims.clone(), KID, &key);

    let verified = verifier
        .verify(&token, claims.db_profile.database_role(), NOW + 1)
        .expect("valid database token");

    assert_eq!(verified.claims, claims);
    assert_eq!(
        verified.identity.actor,
        AuthenticatedActor::OAuthClient("cli_01J00000000000000000000000".into())
    );
    assert_eq!(
        decode_authn_id(&verified.authn_id),
        Ok(verified.identity.clone())
    );
    assert_eq!(
        decode_system_user(&format!("oauth:{}", verified.authn_id)),
        Ok(verified.identity)
    );
    assert!(verified.authn_id.len() <= MAX_AUTHN_ID_BYTES);
}

#[test]
fn valid_api_key_token_preserves_credential_attribution() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let mut claims = valid_oauth_claims();
    claims.auth_method = AuthMethod::ApiKey;
    claims.client_id = None;
    claims.credential_id = Some("crd_01J00000000000000000000000".into());
    let token = sign_payload(claims.clone(), KID, &key);

    let verified = verifier
        .verify(&token, claims.db_profile.database_role(), NOW + 1)
        .expect("valid API-key-derived database token");

    assert_eq!(
        verified.identity.actor,
        AuthenticatedActor::ApiKeyCredential("crd_01J00000000000000000000000".into())
    );
    assert_eq!(verified.identity.auth_method, AuthMethod::ApiKey);
}

#[test]
fn jwks_rejects_duplicate_private_or_non_es256_keys() {
    let key = signing_key();
    let valid = jwk_value(&key, KID);
    let policy = || DatabaseTokenPolicy::new(ISSUER, AUDIENCE).expect("policy");

    assert!(matches!(
        DatabaseTokenVerifier::from_jwks(&jwks_with(vec![valid.clone(), valid.clone()]), policy(),),
        Err(JwtValidationError::DuplicateKeyId)
    ));

    for (field, value) in [
        ("d", json!("private-material-must-not-load")),
        ("kty", json!("RSA")),
        ("crv", json!("P-384")),
        ("alg", json!("ES384")),
        ("use", json!("enc")),
        ("key_ops", json!(["sign"])),
        ("x", json!("not-base64url=")),
    ] {
        let mut invalid = valid.clone();
        invalid
            .as_object_mut()
            .expect("JWK object")
            .insert(field.into(), value);
        assert!(
            matches!(
                DatabaseTokenVerifier::from_jwks(&jwks_with(vec![invalid]), policy()),
                Err(JwtValidationError::InvalidJwks)
            ),
            "field {field} must fail closed"
        );
    }
}

#[test]
fn token_header_rejects_missing_kid_embedded_keys_urls_and_custom_fields() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let claims = valid_oauth_claims();
    let valid = sign_payload(claims.clone(), KID, &key);

    for (field, value) in [
        ("jku", json!("https://attacker.example.test/jwks.json")),
        ("jwk", jwk_value(&key, KID)),
        ("unexpected", json!(true)),
    ] {
        let invalid = mutate_header(&valid, |header| {
            header.insert(field.into(), value);
        });
        assert_eq!(
            verifier.verify(&invalid, claims.db_profile.database_role(), NOW + 1),
            Err(JwtValidationError::InvalidHeader)
        );
    }

    let missing_kid = mutate_header(&valid, |header| {
        header.remove("kid");
    });
    assert_eq!(
        verifier.verify(&missing_kid, claims.db_profile.database_role(), NOW + 1,),
        Err(JwtValidationError::InvalidHeader)
    );
}

#[test]
fn token_rejects_unknown_kid_wrong_algorithm_and_tampered_signature() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let claims = valid_oauth_claims();

    let unknown_kid = sign_payload(claims.clone(), "unknown-kid", &key);
    assert_eq!(
        verifier.verify(&unknown_kid, claims.db_profile.database_role(), NOW + 1),
        Err(JwtValidationError::UnknownKeyId)
    );

    let valid = sign_payload(claims.clone(), KID, &key);
    let wrong_algorithm = mutate_header(&valid, |header| {
        header.insert("alg".into(), json!("RS256"));
    });
    assert_eq!(
        verifier.verify(&wrong_algorithm, claims.db_profile.database_role(), NOW + 1,),
        Err(JwtValidationError::InvalidHeader)
    );

    let mut segments = valid.split('.').map(str::to_owned).collect::<Vec<_>>();
    let replacement = if segments[2].starts_with('A') {
        "B"
    } else {
        "A"
    };
    segments[2].replace_range(..1, replacement);
    assert_eq!(
        verifier.verify(
            &segments.join("."),
            claims.db_profile.database_role(),
            NOW + 1,
        ),
        Err(JwtValidationError::InvalidSignature)
    );
}

#[test]
fn claims_reject_wrong_resource_time_actor_and_requested_role() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_oauth_claims();
    let expected_role = base.db_profile.database_role();

    let mut invalid_claims = Vec::new();

    let mut wrong_issuer = base.clone();
    wrong_issuer.issuer = "https://attacker.example.test".into();
    invalid_claims.push(wrong_issuer);

    let mut wrong_audience = base.clone();
    wrong_audience.audience = "https://candidate.example.test/resources/mcp".into();
    invalid_claims.push(wrong_audience);

    let mut wrong_scope = base.clone();
    wrong_scope.scope = "mcp".into();
    invalid_claims.push(wrong_scope);

    let mut future_iat = base.clone();
    future_iat.issued_at = NOW + 1;
    future_iat.expires_at = NOW + 121;
    invalid_claims.push(future_iat);

    let mut expired = base.clone();
    expired.issued_at = NOW - 121;
    expired.expires_at = NOW - 1;
    invalid_claims.push(expired);

    let mut ttl_too_long = base.clone();
    ttl_too_long.expires_at = NOW + 301;
    invalid_claims.push(ttl_too_long);

    let mut ttl_too_short = base.clone();
    ttl_too_short.expires_at = NOW + 29;
    invalid_claims.push(ttl_too_short);

    let mut both_actors = base.clone();
    both_actors.credential_id = Some("crd_01J00000000000000000000000".into());
    invalid_claims.push(both_actors);

    let mut method_mismatch = base.clone();
    method_mismatch.auth_method = AuthMethod::ApiKey;
    invalid_claims.push(method_mismatch);

    for claims in invalid_claims {
        let token = sign_payload(claims, KID, &key);
        assert_eq!(
            verifier.verify(&token, expected_role, NOW),
            Err(JwtValidationError::InvalidClaims)
        );
    }

    let token = sign_payload(base.clone(), KID, &key);
    assert_eq!(
        verifier.verify(&token, DatabaseProfile::BusinessAdmin.database_role(), NOW),
        Err(JwtValidationError::RequestedRoleMismatch)
    );
}

#[test]
fn claims_schema_rejects_missing_unknown_and_illegal_identity_fields() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_oauth_claims();
    let expected_role = base.db_profile.database_role();

    let mut missing_claim = serde_json::to_value(base.clone()).expect("claims JSON");
    missing_claim
        .as_object_mut()
        .expect("claims object")
        .remove("delegation_id");
    let token = sign_payload(missing_claim, KID, &key);
    assert_eq!(
        verifier.verify(&token, expected_role, NOW),
        Err(JwtValidationError::InvalidToken)
    );

    let mut unknown_claim = serde_json::to_value(base.clone()).expect("claims JSON");
    unknown_claim
        .as_object_mut()
        .expect("claims object")
        .insert("unexpected".into(), json!(true));
    let token = sign_payload(unknown_claim, KID, &key);
    assert_eq!(
        verifier.verify(&token, expected_role, NOW),
        Err(JwtValidationError::InvalidToken)
    );

    let too_long = "a".repeat(65);
    for subject in ["contains:delimiter", "含有非ASCII", "", too_long.as_str()] {
        let mut claims = base.clone();
        claims.subject = subject.into();
        let token = sign_payload(claims, KID, &key);
        assert_eq!(
            verifier.verify(&token, expected_role, NOW),
            Err(JwtValidationError::InvalidIdentity)
        );
    }
}

#[test]
fn identity_codec_rejects_ambiguity_unknown_versions_and_oversize_values() {
    let identity = AuthenticatedIdentity {
        user_id: "usr_01J00000000000000000000000".into(),
        actor: AuthenticatedActor::OAuthClient("cli_01J00000000000000000000000".into()),
        delegation_id: "dlg_01J00000000000000000000000".into(),
        auth_method: AuthMethod::OAuth,
        authority_version: 7,
        profile: DatabaseProfile::Ordinary,
    };
    let encoded = identity.encode_authn_id().expect("encode identity");

    assert_eq!(decode_authn_id(&encoded), Ok(identity));
    assert!(decode_authn_id("pggomtm:v2:u=x").is_err());
    assert!(decode_authn_id("pggomtm:v1:u=x;actor=client:y").is_err());
    assert!(decode_system_user(&format!("scram:{}", encoded)).is_err());
    assert!(decode_authn_id(&"x".repeat(MAX_AUTHN_ID_BYTES + 1)).is_err());
}

#[test]
fn profile_role_mapping_is_closed_and_non_inheriting() {
    let mappings = BTreeMap::from([
        (DatabaseProfile::Ordinary, "gomtm_candidate_ordinary"),
        (
            DatabaseProfile::BusinessAdmin,
            "gomtm_candidate_business_admin",
        ),
        (
            DatabaseProfile::DatabaseDeveloper,
            "gomtm_candidate_database_developer",
        ),
    ]);

    for (profile, role) in mappings {
        assert_eq!(profile.database_role(), role);
    }
}
