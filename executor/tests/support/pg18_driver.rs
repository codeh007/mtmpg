use std::error::Error;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Certificate, StatusCode};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const EXECUTE_PATH: &str = "/v1/sql/execute";
const WIRE_VERSION: &str = "v1";
const DEFAULT_BASE_URL: &str = "https://executor:8443";

type HmacSha256 = Hmac<Sha256>;

struct Harness {
    base_url: String,
    client: Client,
    secret: Arc<Vec<u8>>,
    nonce: AtomicU64,
}

impl Harness {
    fn from_environment() -> Result<Self, Box<dyn Error>> {
        let ca_path = required_environment("MTMPG_EXECUTOR_CA_PATH")?;
        let secret_path = required_environment("MTMPG_EXECUTOR_HMAC_PATH")?;
        let ca = Certificate::from_pem(&fs::read(ca_path)?)?;
        let client = Client::builder()
            .add_root_certificate(ca)
            .no_proxy()
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self {
            base_url: std::env::var("MTMPG_EXECUTOR_URL")
                .unwrap_or_else(|_| DEFAULT_BASE_URL.into()),
            client,
            secret: Arc::new(fs::read(secret_path)?),
            nonce: AtomicU64::new(1),
        })
    }

    fn ready(&self) -> Result<(), Box<dyn Error>> {
        for _ in 0..50 {
            match self.client.get(format!("{}/ready", self.base_url)).send() {
                Ok(response) if response.status() == StatusCode::OK => return Ok(()),
                Ok(_) | Err(_) => thread::sleep(Duration::from_millis(100)),
            }
        }
        Err(invalid_data("executor readiness failed").into())
    }

    fn execute(&self, body: Value) -> Result<(StatusCode, Value), Box<dyn Error>> {
        let timestamp = unix_time()?;
        let nonce = format!("{:032x}", self.nonce.fetch_add(1, Ordering::Relaxed));
        self.execute_with_envelope(body, timestamp, &nonce, None)
    }

    fn execute_with_envelope(
        &self,
        body: Value,
        timestamp: i64,
        nonce: &str,
        signature_override: Option<&str>,
    ) -> Result<(StatusCode, Value), Box<dyn Error>> {
        let body = serde_json::to_vec(&body)?;
        let signature = match signature_override {
            Some(signature) => signature.to_owned(),
            None => sign(&self.secret, timestamp, nonce, &body)?,
        };
        let response = self
            .client
            .post(format!("{}{}", self.base_url, EXECUTE_PATH))
            .headers(headers(timestamp, nonce, &signature)?)
            .body(body)
            .send()?;
        decode_response(response)
    }

    fn request(&self, profile: &str, method: &str, statement: &str) -> Value {
        let actor = if method == "oauth" {
            json!({"client_id": format!("client_{profile}")})
        } else {
            json!({"credential_id": format!("credential_{profile}")})
        };
        let mut principal = json!({
            "user_id": format!("user_{profile}"),
            "delegation_id": format!("delegation_{profile}"),
            "auth_method": method,
            "authority_version": 7,
            "database_scope": "database",
            "profile": profile,
            "credential_expires_at": unix_time().expect("system time") + 300
        });
        principal
            .as_object_mut()
            .expect("principal object")
            .extend(actor.as_object().expect("actor object").clone());
        json!({
            "principal": principal,
            "statement": statement,
            "binds": [],
            "intent": "read",
            "change_confirmed": false,
            "correlation_id": format!(
                "integration-{:08}",
                self.nonce.load(Ordering::Relaxed)
            )
        })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let harness = Arc::new(Harness::from_environment()?);
    harness.ready()?;
    verify_hmac_boundary(&harness)?;
    verify_concurrent_identity_isolation(&harness)?;
    verify_extended_protocol_and_statement_boundary(&harness)?;
    verify_transaction_and_budget_boundaries(&harness)?;
    verify_deadline_and_cancellation(&harness)?;
    verify_tls_hostname(&harness)?;
    println!("PG18 executor OAuth and SQL matrix passed");
    Ok(())
}

fn verify_hmac_boundary(harness: &Harness) -> Result<(), Box<dyn Error>> {
    let body = harness.request("ordinary", "oauth", "SELECT 1");
    let timestamp = unix_time()?;
    let nonce = "00112233445566778899aabbccddeeff";
    let encoded = serde_json::to_vec(&body)?;
    let signature = sign(&harness.secret, timestamp, nonce, &encoded)?;
    let first = harness.execute_with_envelope(body.clone(), timestamp, nonce, Some(&signature))?;
    expect_success(&first, "valid HMAC request")?;
    let replay = harness.execute_with_envelope(body.clone(), timestamp, nonce, Some(&signature))?;
    expect_error(
        &replay,
        StatusCode::UNAUTHORIZED,
        "unauthorized",
        "replayed HMAC request",
    )?;
    let tampered = harness.execute_with_envelope(
        body,
        timestamp,
        "ffeeddccbbaa99887766554433221100",
        Some(&"00".repeat(32)),
    )?;
    expect_error(
        &tampered,
        StatusCode::UNAUTHORIZED,
        "unauthorized",
        "tampered HMAC request",
    )?;
    Ok(())
}

fn verify_concurrent_identity_isolation(harness: &Arc<Harness>) -> Result<(), Box<dyn Error>> {
    let cases = [
        ("ordinary", "oauth"),
        ("business_admin", "api_key"),
        ("database_developer", "oauth"),
    ];
    let mut workers = Vec::new();
    for (profile, method) in cases {
        let harness = Arc::clone(harness);
        workers.push(thread::spawn(move || -> Result<(), String> {
            let request = harness.request(profile, method, "SELECT current_user, system_user");
            let response = harness
                .execute(request)
                .map_err(|error| error.to_string())?;
            let result = expect_success(&response, "concurrent identity request")
                .map_err(|error| error.to_string())?;
            let row = result["rows"]
                .as_array()
                .and_then(|rows| rows.first())
                .and_then(Value::as_array)
                .ok_or_else(|| "identity result row is unavailable".to_owned())?;
            if row.first().and_then(Value::as_str) != Some(profile) {
                return Err("current_user did not match profile".into());
            }
            let system_user = row
                .get(1)
                .and_then(Value::as_str)
                .ok_or_else(|| "system_user is unavailable".to_owned())?;
            if !system_user.starts_with("oauth:pggomtm:v2;")
                || !system_user.contains(&format!(";p={profile}"))
            {
                return Err("system_user did not match delegated principal".into());
            }
            Ok(())
        }));
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| invalid_data("identity worker panicked"))?
            .map_err(|message| IoError::new(ErrorKind::InvalidData, message))?;
    }
    Ok(())
}

fn verify_extended_protocol_and_statement_boundary(
    harness: &Harness,
) -> Result<(), Box<dyn Error>> {
    let mut binds = harness.request(
        "ordinary",
        "oauth",
        "SELECT $1::text, $2::bigint, $3::boolean, $4::jsonb, $5::text",
    );
    binds["binds"] = json!([
        {"type": "text", "value": "alpha"},
        {"type": "int64", "value": 42},
        {"type": "boolean", "value": true},
        {"type": "json", "value": {"key": "value"}},
        {"type": "null"}
    ]);
    let response = harness.execute(binds)?;
    let result = expect_success(&response, "parameter bind query")?;
    assert_json(
        &result["rows"][0],
        &json!(["alpha", "42", "t", {"key": "value"}, null]),
        "parameter bind result",
    )?;

    let multiple = harness.execute(harness.request(
        "ordinary",
        "oauth",
        "SELECT 1; INSERT INTO app.executor_probe(value) VALUES ('forbidden')",
    ))?;
    expect_error(
        &multiple,
        StatusCode::UNPROCESSABLE_ENTITY,
        "database_rejected",
        "multiple statement request",
    )?;

    let cte = harness.execute(harness.request(
        "ordinary",
        "oauth",
        "WITH input(value) AS (VALUES (41)) SELECT value + 1 FROM input",
    ))?;
    expect_success(&cte, "CTE query")?;

    let mut call = harness.request(
        "business_admin",
        "api_key",
        "CALL app.record_probe($1::text)",
    );
    call["binds"] = json!([{"type": "text", "value": "call"}]);
    call["intent"] = json!("change");
    call["change_confirmed"] = json!(true);
    expect_success(&harness.execute(call)?, "CALL statement")?;

    let mut do_statement = harness.request(
        "database_developer",
        "oauth",
        "DO $$ BEGIN PERFORM 1; END $$",
    );
    do_statement["intent"] = json!("change");
    do_statement["change_confirmed"] = json!(true);
    expect_success(&harness.execute(do_statement)?, "DO statement")?;
    Ok(())
}

fn verify_transaction_and_budget_boundaries(harness: &Harness) -> Result<(), Box<dyn Error>> {
    let read_write = harness.execute(harness.request(
        "ordinary",
        "oauth",
        "INSERT INTO app.executor_probe(value) VALUES ('read-write')",
    ))?;
    expect_error(
        &read_write,
        StatusCode::UNPROCESSABLE_ENTITY,
        "database_rejected",
        "read-only write request",
    )?;

    let mut confirmed = harness.request(
        "business_admin",
        "api_key",
        "INSERT INTO app.executor_probe(value) VALUES ('confirmed')",
    );
    confirmed["intent"] = json!("change");
    confirmed["change_confirmed"] = json!(true);
    let confirmed_response = harness.execute(confirmed)?;
    let confirmed_result = expect_success(&confirmed_response, "confirmed change")?;
    assert_json(
        &confirmed_result["affected_rows"],
        &json!(1),
        "affected row count",
    )?;

    let mut failure = harness.request("business_admin", "api_key", "CALL app.fail_after_insert()");
    failure["intent"] = json!("change");
    failure["change_confirmed"] = json!(true);
    expect_error(
        &harness.execute(failure)?,
        StatusCode::UNPROCESSABLE_ENTITY,
        "database_rejected",
        "failed routine request",
    )?;

    let mut oversized_change = harness.request(
        "business_admin",
        "api_key",
        "INSERT INTO app.executor_probe(value) SELECT 'budget' FROM generate_series(1, 1001) RETURNING value",
    );
    oversized_change["intent"] = json!("change");
    oversized_change["change_confirmed"] = json!(true);
    expect_error(
        &harness.execute(oversized_change)?,
        StatusCode::PAYLOAD_TOO_LARGE,
        "budget_exceeded",
        "change result budget",
    )?;

    for statement in [
        "SELECT generate_series(1, 1001)",
        "SELECT repeat('x', 262145)",
        "SELECT repeat('x', 2048) FROM generate_series(1, 1000)",
    ] {
        expect_error(
            &harness.execute(harness.request("ordinary", "oauth", statement))?,
            StatusCode::PAYLOAD_TOO_LARGE,
            "budget_exceeded",
            "read result budget",
        )?;
    }

    let count = harness.execute(harness.request(
        "ordinary",
        "oauth",
        "SELECT count(*) FROM app.executor_probe WHERE value IN ('read-write', 'failed', 'budget')",
    ))?;
    let result = expect_success(&count, "rollback count")?;
    assert_json(&result["rows"][0][0], &json!("0"), "rollback count")?;
    Ok(())
}

fn verify_deadline_and_cancellation(harness: &Harness) -> Result<(), Box<dyn Error>> {
    let deadline = harness.execute(harness.request("ordinary", "oauth", "SELECT pg_sleep(10)"))?;
    expect_error(
        &deadline,
        StatusCode::GATEWAY_TIMEOUT,
        "deadline_exceeded",
        "statement deadline",
    )?;

    let ca_path = required_environment("MTMPG_EXECUTOR_CA_PATH")?;
    let ca = Certificate::from_pem(&fs::read(ca_path)?)?;
    let short_client = Client::builder()
        .add_root_certificate(ca)
        .no_proxy()
        .timeout(Duration::from_millis(250))
        .build()?;
    let body = harness.request("ordinary", "oauth", "SELECT pg_sleep(10)");
    let encoded = serde_json::to_vec(&body)?;
    let timestamp = unix_time()?;
    let nonce = "102132435465768798a9bacbdcedfe0f";
    let signature = sign(&harness.secret, timestamp, nonce, &encoded)?;
    let cancelled = short_client
        .post(format!("{}{}", harness.base_url, EXECUTE_PATH))
        .headers(headers(timestamp, nonce, &signature)?)
        .body(encoded)
        .send();
    if cancelled.is_ok() {
        return Err(invalid_data("cancelled HTTP request unexpectedly completed").into());
    }
    thread::sleep(Duration::from_secs(1));
    expect_success(
        &harness.execute(harness.request("ordinary", "oauth", "SELECT 1"))?,
        "post-cancel query",
    )?;
    Ok(())
}

fn verify_tls_hostname(harness: &Harness) -> Result<(), Box<dyn Error>> {
    let request = harness.request("ordinary", "oauth", "SELECT 1");
    let body = serde_json::to_vec(&request)?;
    let timestamp = unix_time()?;
    let nonce = "abcdefabcdefabcdefabcdefabcdefab";
    let signature = sign(&harness.secret, timestamp, nonce, &body)?;
    let invalid_host = harness
        .client
        .post(format!("https://127.0.0.1:8443{EXECUTE_PATH}"))
        .headers(headers(timestamp, nonce, &signature)?)
        .body(body)
        .send();
    if invalid_host.is_ok() {
        return Err(invalid_data("TLS hostname mismatch was accepted").into());
    }
    Ok(())
}

fn headers(timestamp: i64, nonce: &str, signature: &str) -> Result<HeaderMap, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("x-executor-version", HeaderValue::from_static(WIRE_VERSION));
    headers.insert(
        "x-executor-timestamp",
        HeaderValue::from_str(&timestamp.to_string())?,
    );
    headers.insert("x-executor-nonce", HeaderValue::from_str(nonce)?);
    headers.insert("x-executor-signature", HeaderValue::from_str(signature)?);
    Ok(headers)
}

fn sign(secret: &[u8], timestamp: i64, nonce: &str, body: &[u8]) -> Result<String, IoError> {
    let digest = Sha256::digest(body);
    let canonical =
        format!("{WIRE_VERSION}\nPOST\n{EXECUTE_PATH}\n{timestamp}\n{nonce}\n{digest:x}");
    let mut mac =
        HmacSha256::new_from_slice(secret).map_err(|_| invalid_data("invalid HMAC test secret"))?;
    mac.update(canonical.as_bytes());
    Ok(format!("{:x}", mac.finalize().into_bytes()))
}

fn decode_response(response: Response) -> Result<(StatusCode, Value), Box<dyn Error>> {
    let status = response.status();
    let body = response.json::<Value>()?;
    Ok((status, body))
}

fn expect_success<'a>(
    response: &'a (StatusCode, Value),
    context: &str,
) -> Result<&'a Value, IoError> {
    if response.0 != StatusCode::OK {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            format!("{context} returned HTTP {}", response.0),
        ));
    }
    response.1.get("result").ok_or_else(|| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("{context} omitted the result"),
        )
    })
}

fn expect_error(
    response: &(StatusCode, Value),
    status: StatusCode,
    category: &str,
    context: &str,
) -> Result<(), IoError> {
    if response.0 != status || response.1["error"]["category"] != category {
        let actual_category = response.1["error"]["category"]
            .as_str()
            .unwrap_or("missing");
        return Err(IoError::new(
            ErrorKind::InvalidData,
            format!(
                "{context} expected HTTP {status} category {category}, got HTTP {} category {actual_category}",
                response.0
            ),
        ));
    }
    Ok(())
}

fn assert_json(actual: &Value, expected: &Value, context: &str) -> Result<(), IoError> {
    if actual != expected {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            format!("{context} did not match"),
        ));
    }
    Ok(())
}

fn required_environment(name: &str) -> Result<String, IoError> {
    std::env::var(name).map_err(|_| {
        IoError::new(
            ErrorKind::InvalidInput,
            format!("required environment is unavailable: {name}"),
        )
    })
}

fn unix_time() -> Result<i64, IoError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| invalid_data("system clock is before Unix epoch"))?
        .as_secs();
    i64::try_from(seconds).map_err(|_| invalid_data("system clock is out of range"))
}

fn invalid_data(message: &'static str) -> IoError {
    IoError::new(ErrorKind::InvalidData, message)
}
