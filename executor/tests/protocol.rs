use mtmpg_executor::protocol::{
    BindValue, ExecutionIntent, MAX_BIND_COUNT, MAX_BIND_VALUE_BYTES, MAX_REQUEST_BODY_BYTES,
    MAX_STATEMENT_BYTES, ProtocolError, parse_execute_request,
};
use pggomtm::database_auth::{AuthMethod, DatabaseProfile};
use serde_json::{Map, Value, json};

const EXPIRY: i64 = 1_800_000_300;

fn principal(method: &str, profile: &str) -> Value {
    let mut value = json!({
        "user_id": "usr_01",
        "delegation_id": "dlg_01",
        "auth_method": method,
        "authority_version": 7,
        "database_scope": "database",
        "profile": profile,
        "credential_expires_at": EXPIRY,
    });
    let object = value.as_object_mut().expect("principal object");
    match method {
        "oauth" => {
            object.insert("client_id".into(), json!("cli_01"));
        }
        "api_key" => {
            object.insert("credential_id".into(), json!("crd_01"));
        }
        _ => {}
    }
    value
}

fn request_value(principal: Value) -> Value {
    json!({
        "principal": principal,
        "statement": "SELECT $1::text, $2::bigint, $3::boolean, $4::jsonb, $5::text",
        "binds": [
            {"type": "text", "value": "alpha"},
            {"type": "int64", "value": 42},
            {"type": "boolean", "value": true},
            {"type": "json", "value": {"key": "value"}},
            {"type": "null"}
        ],
        "intent": "read",
        "change_confirmed": false,
        "correlation_id": "req_01"
    })
}

fn parse_value(value: &Value) -> Result<mtmpg_executor::protocol::ExecuteRequest, ProtocolError> {
    parse_execute_request(&serde_json::to_vec(value).expect("serialize request"))
}

#[test]
fn oauth_and_api_key_principals_use_one_strict_request_shape() {
    for (method, actor, profile) in [
        ("oauth", "cli_01", "ordinary"),
        ("api_key", "crd_01", "business_admin"),
        ("oauth", "cli_01", "database_developer"),
    ] {
        let parsed = parse_value(&request_value(principal(method, profile)))
            .expect("valid delegated request");
        assert_eq!(
            parsed.principal.auth_method,
            if method == "oauth" {
                AuthMethod::OAuth
            } else {
                AuthMethod::ApiKey
            }
        );
        assert_eq!(parsed.principal.actor_id(), actor);
        assert_eq!(parsed.principal.profile.database_role(), profile);
        assert_eq!(parsed.principal.database_scope, "database");
        assert_eq!(parsed.intent, ExecutionIntent::Read);
        assert!(!parsed.change_confirmed);
        assert_eq!(
            parsed.binds,
            vec![
                BindValue::Text("alpha".into()),
                BindValue::Int64(42),
                BindValue::Boolean(true),
                BindValue::Json(json!({"key": "value"})),
                BindValue::Null,
            ]
        );
    }
}

#[test]
fn unknown_fields_and_all_caller_supplied_credentials_or_claims_are_rejected() {
    let base = request_value(principal("oauth", "ordinary"));
    let forbidden_top_level = ["statements", "database_jwt", "connection_string"];
    for field in forbidden_top_level {
        let mut request = base.clone();
        request
            .as_object_mut()
            .expect("request object")
            .insert(field.into(), json!("forbidden"));
        assert_eq!(parse_value(&request), Err(ProtocolError::InvalidRequest));
    }

    let forbidden_principal = [
        "bearer_token",
        "api_key",
        "password",
        "db_role",
        "issuer",
        "audience",
        "claims",
        "unknown",
    ];
    for field in forbidden_principal {
        let mut request = base.clone();
        request["principal"]
            .as_object_mut()
            .expect("principal object")
            .insert(field.into(), json!("forbidden"));
        assert_eq!(parse_value(&request), Err(ProtocolError::InvalidRequest));
    }
}

#[test]
fn principal_actor_method_scope_and_profile_must_be_canonical() {
    let mut invalid_principals = Vec::new();

    let mut both = principal("oauth", "ordinary");
    both.as_object_mut()
        .expect("principal object")
        .insert("credential_id".into(), json!("crd_01"));
    invalid_principals.push(both);

    let mut no_actor = principal("oauth", "ordinary");
    no_actor
        .as_object_mut()
        .expect("principal object")
        .remove("client_id");
    invalid_principals.push(no_actor);

    let mut wrong_actor = principal("oauth", "ordinary");
    let object = wrong_actor.as_object_mut().expect("principal object");
    object.remove("client_id");
    object.insert("credential_id".into(), json!("crd_01"));
    invalid_principals.push(wrong_actor);

    let mut wrong_scope = principal("oauth", "ordinary");
    wrong_scope["database_scope"] = json!("admin");
    invalid_principals.push(wrong_scope);

    invalid_principals.push(principal("oauth", "admin"));
    invalid_principals.push(principal("interactive", "ordinary"));

    for invalid in invalid_principals {
        assert_eq!(
            parse_value(&request_value(invalid)),
            Err(ProtocolError::InvalidRequest)
        );
    }
}

#[test]
fn statement_bind_and_body_limits_are_enforced_before_execution() {
    let mut empty = request_value(principal("oauth", "ordinary"));
    empty["statement"] = json!(" \n\t");
    assert_eq!(parse_value(&empty), Err(ProtocolError::InvalidRequest));

    let mut statement_limit = request_value(principal("oauth", "ordinary"));
    statement_limit["statement"] = json!("x".repeat(MAX_STATEMENT_BYTES));
    statement_limit["binds"] = json!([]);
    assert!(parse_value(&statement_limit).is_ok());
    statement_limit["statement"] = json!("x".repeat(MAX_STATEMENT_BYTES + 1));
    assert_eq!(
        parse_value(&statement_limit),
        Err(ProtocolError::LimitExceeded)
    );

    let mut bind_count = request_value(principal("oauth", "ordinary"));
    bind_count["statement"] = json!("SELECT 1");
    bind_count["binds"] = Value::Array(vec![json!({"type": "null"}); MAX_BIND_COUNT]);
    assert!(parse_value(&bind_count).is_ok());
    bind_count["binds"] =
        Value::Array(vec![json!({"type": "null"}); MAX_BIND_COUNT + 1]);
    assert_eq!(
        parse_value(&bind_count),
        Err(ProtocolError::LimitExceeded)
    );

    let mut bind_value = request_value(principal("oauth", "ordinary"));
    bind_value["statement"] = json!("SELECT $1::text");
    bind_value["binds"] = json!([
        {"type": "text", "value": "x".repeat(MAX_BIND_VALUE_BYTES + 1)}
    ]);
    assert_eq!(
        parse_value(&bind_value),
        Err(ProtocolError::LimitExceeded)
    );

    assert_eq!(
        parse_execute_request(&vec![b' '; MAX_REQUEST_BODY_BYTES + 1]),
        Err(ProtocolError::LimitExceeded)
    );
}

#[test]
fn change_requires_current_confirmation_and_read_rejects_change_confirmation() {
    let mut change = request_value(principal("api_key", "business_admin"));
    change["intent"] = json!("change");
    assert_eq!(
        parse_value(&change),
        Err(ProtocolError::ConfirmationRequired)
    );
    change["change_confirmed"] = json!(true);
    assert!(parse_value(&change).is_ok());

    let mut read = request_value(principal("oauth", "ordinary"));
    read["change_confirmed"] = json!(true);
    assert_eq!(parse_value(&read), Err(ProtocolError::InvalidRequest));
}

#[test]
fn malformed_bind_and_correlation_shapes_are_rejected() {
    let mut invalid_bind = request_value(principal("oauth", "ordinary"));
    invalid_bind["binds"] = json!([{"type": "text", "value": "ok", "extra": true}]);
    assert_eq!(
        parse_value(&invalid_bind),
        Err(ProtocolError::InvalidRequest)
    );

    let too_long = "x".repeat(129);
    for correlation_id in ["", "contains space", "x/y", too_long.as_str()] {
        let mut request = request_value(principal("oauth", "ordinary"));
        request["correlation_id"] = json!(correlation_id);
        assert_eq!(parse_value(&request), Err(ProtocolError::InvalidRequest));
    }

    let mut duplicate_shape = request_value(principal("oauth", "ordinary"));
    let object: &mut Map<String, Value> = duplicate_shape
        .as_object_mut()
        .expect("request object");
    object.insert("statement".into(), Value::Array(vec![json!("SELECT 1")]));
    assert_eq!(
        parse_value(&duplicate_shape),
        Err(ProtocolError::InvalidRequest)
    );
}

#[test]
fn shared_contract_types_keep_the_three_generic_database_profiles() {
    assert_eq!(
        serde_json::to_value(AuthMethod::OAuth).expect("serialize OAuth method"),
        json!("oauth")
    );
    assert_eq!(
        serde_json::to_value(AuthMethod::ApiKey).expect("serialize API key method"),
        json!("api_key")
    );
    assert_eq!(DatabaseProfile::Ordinary.database_role(), "ordinary");
    assert_eq!(
        DatabaseProfile::BusinessAdmin.database_role(),
        "business_admin"
    );
    assert_eq!(
        DatabaseProfile::DatabaseDeveloper.database_role(),
        "database_developer"
    );
}
