use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use zeroize::Zeroizing;

use crate::protocol::{BindValue, ExecuteRequest, ExecutionIntent};
use crate::token_registry::{ConnectionId, ConnectionTokenRegistry};

pub const MAX_RESULT_ROWS: usize = 1_000;
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
pub const MAX_RESULT_VALUE_BYTES: usize = 256 * 1024;
pub const TOTAL_DEADLINE: Duration = Duration::from_secs(3);

const POLL_SLICE_MICROSECONDS: i64 = 50_000;
const JSON_OID: u32 = 114;
const JSONB_OID: u32 = 3_802;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientAbiError {
    Unavailable,
    UnsupportedPostgresMajor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientAbi {
    pub postgresql_major: u32,
    pub oauth_bearer_auth_data_hook: bool,
    pub async_cancel: bool,
    pub extended_query: bool,
    pub socket_poll: bool,
}

pub fn current_client_abi() -> Result<ClientAbi, ClientAbiError> {
    // SAFETY: PQlibVersion takes no arguments and does not retain Rust memory.
    let version = unsafe { ffi::PQlibVersion() };
    if version <= 0 {
        return Err(ClientAbiError::Unavailable);
    }
    let postgresql_major =
        u32::try_from(version / 10_000).map_err(|_| ClientAbiError::Unavailable)?;
    if postgresql_major != 18 {
        return Err(ClientAbiError::UnsupportedPostgresMajor);
    }
    Ok(ClientAbi {
        postgresql_major,
        oauth_bearer_auth_data_hook: true,
        async_cancel: true,
        extended_query: true,
        socket_poll: true,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub host: String,
    pub database: String,
    pub ca_path: String,
}

impl DatabaseConfig {
    #[must_use]
    pub fn canonical(ca_path: impl Into<String>) -> Self {
        Self {
            host: "postgres".into(),
            database: "gomtm".into(),
            ca_path: ca_path.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseErrorKind {
    InvalidRequest,
    Unavailable,
    Rejected,
    BudgetExceeded,
    DeadlineExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseError {
    pub kind: DatabaseErrorKind,
    pub sqlstate_class: Option<String>,
}

impl DatabaseError {
    fn new(kind: DatabaseErrorKind) -> Self {
        Self {
            kind,
            sqlstate_class: None,
        }
    }

    fn rejected(sqlstate: Option<String>) -> Self {
        if sqlstate.as_deref() == Some("57") {
            return Self::new(DatabaseErrorKind::DeadlineExceeded);
        }
        Self {
            kind: DatabaseErrorKind::Rejected,
            sqlstate_class: sqlstate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResultColumn {
    pub name: String,
    pub type_oid: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutionResult {
    pub columns: Vec<ResultColumn>,
    pub rows: Vec<Vec<Value>>,
    pub command_tag: String,
    pub affected_rows: u64,
    pub duration_ms: u64,
    pub correlation_id: String,
}

#[derive(Debug, Clone)]
pub struct Cancellation {
    cancelled: Arc<AtomicBool>,
}

impl Cancellation {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthHookError {
    AlreadyInstalled,
    InvalidClientAbi,
}

static AUTH_TOKENS: OnceLock<Arc<ConnectionTokenRegistry>> = OnceLock::new();

pub fn install_auth_data_hook(registry: Arc<ConnectionTokenRegistry>) -> Result<(), AuthHookError> {
    current_client_abi().map_err(|_| AuthHookError::InvalidClientAbi)?;
    AUTH_TOKENS
        .set(registry)
        .map_err(|_| AuthHookError::AlreadyInstalled)?;
    // SAFETY: the callback is process-global, has C ABI, never unwinds, and the registry is static.
    unsafe { ffi::PQsetAuthDataHook(Some(auth_data_hook)) };
    Ok(())
}

struct HookToken {
    bytes: Zeroizing<String>,
}

unsafe extern "C" fn auth_data_hook(
    auth_type: ffi::PGauthData,
    connection: *mut ffi::PGconn,
    data: *mut c_void,
) -> c_int {
    std::panic::catch_unwind(|| {
        if auth_type != ffi::PGauthData_PQAUTHDATA_OAUTH_BEARER_TOKEN
            || connection.is_null()
            || data.is_null()
        {
            return -1;
        }
        let Some(registry) = AUTH_TOKENS.get() else {
            return -1;
        };
        let Some(connection_id) = ConnectionId::new(connection.addr()) else {
            return -1;
        };
        let Ok(token) = registry.claim(connection_id) else {
            return -1;
        };

        let mut bytes = token.into_secret();
        bytes.push('\0');
        let mut token = Box::new(HookToken { bytes });
        let token_pointer = token.bytes.as_mut_ptr().cast::<c_char>();
        let user_pointer = Box::into_raw(token).cast::<c_void>();
        let request = data.cast::<ffi::PGoauthBearerRequest>();
        // SAFETY: libpq supplied data for this auth type as a writable PGoauthBearerRequest.
        unsafe {
            (*request).async_ = None;
            (*request).cleanup = Some(cleanup_auth_data);
            (*request).token = token_pointer;
            (*request).user = user_pointer;
        }
        1
    })
    .unwrap_or(-1)
}

unsafe extern "C" fn cleanup_auth_data(
    _connection: *mut ffi::PGconn,
    request: *mut ffi::PGoauthBearerRequest,
) {
    let _ = std::panic::catch_unwind(|| {
        if request.is_null() {
            return;
        }
        // SAFETY: user was created by auth_data_hook and libpq calls cleanup at most once.
        unsafe {
            if !(*request).user.is_null() {
                drop(Box::from_raw((*request).user.cast::<HookToken>()));
                (*request).user = ptr::null_mut();
            }
            (*request).token = ptr::null_mut();
            (*request).cleanup = None;
        }
    });
}

pub fn execute(
    config: &DatabaseConfig,
    registry: Arc<ConnectionTokenRegistry>,
    request: &ExecuteRequest,
    token: Zeroizing<String>,
    cancellation: &Cancellation,
) -> Result<ExecutionResult, DatabaseError> {
    let deadline = Instant::now() + TOTAL_DEADLINE;
    let mut connection = PgConnection::connect(
        config,
        Arc::clone(&registry),
        request.principal.profile.database_role(),
        token,
        deadline,
        cancellation,
    )?;
    let started = Instant::now();

    let begin = match request.intent {
        ExecutionIntent::Read => "BEGIN READ ONLY",
        ExecutionIntent::Change => "BEGIN",
    };
    connection.control(begin, deadline, cancellation)?;
    connection.control("SET LOCAL lock_timeout = '250ms'", deadline, cancellation)?;
    connection.control(
        "SET LOCAL statement_timeout = '750ms'",
        deadline,
        cancellation,
    )?;
    connection.control(
        "SET LOCAL idle_in_transaction_session_timeout = '1500ms'",
        deadline,
        cancellation,
    )?;

    let outcome = match connection.query(&request.statement, &request.binds, deadline, cancellation)
    {
        Ok(outcome) => outcome,
        Err(error) => {
            connection.rollback_best_effort();
            return Err(error);
        }
    };
    // SAFETY: the connection is live; a successful user statement must leave our transaction open.
    if unsafe { ffi::PQtransactionStatus(connection.raw) }
        != ffi::PGTransactionStatusType_PQTRANS_INTRANS
    {
        return Err(DatabaseError::new(DatabaseErrorKind::Rejected));
    }
    let result = ExecutionResult {
        columns: outcome.columns,
        rows: outcome.rows,
        command_tag: outcome.command_tag,
        affected_rows: outcome.affected_rows,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        correlation_id: request.correlation_id.clone(),
    };
    let encoded = serde_json::to_vec(&result)
        .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
    if encoded.len() > MAX_RESULT_BYTES {
        connection.rollback_best_effort();
        return Err(DatabaseError::new(DatabaseErrorKind::BudgetExceeded));
    }
    if let Err(error) = connection.control("COMMIT", deadline, cancellation) {
        connection.rollback_best_effort();
        return Err(error);
    }
    Ok(result)
}

struct QueryOutcome {
    columns: Vec<ResultColumn>,
    rows: Vec<Vec<Value>>,
    command_tag: String,
    affected_rows: u64,
}

struct PgConnection {
    raw: *mut ffi::PGconn,
    registry: Arc<ConnectionTokenRegistry>,
    connection_id: ConnectionId,
}

impl PgConnection {
    fn connect(
        config: &DatabaseConfig,
        registry: Arc<ConnectionTokenRegistry>,
        user: &str,
        token: Zeroizing<String>,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<Self, DatabaseError> {
        let keys = [
            c_string("host")?,
            c_string("dbname")?,
            c_string("user")?,
            c_string("sslmode")?,
            c_string("sslrootcert")?,
            c_string("require_auth")?,
        ];
        let values = [
            c_string(&config.host)?,
            c_string(&config.database)?,
            c_string(user)?,
            c_string("verify-full")?,
            c_string(&config.ca_path)?,
            c_string("oauth")?,
        ];
        let mut key_pointers: Vec<*const c_char> =
            keys.iter().map(|value| value.as_ptr()).collect();
        let mut value_pointers: Vec<*const c_char> =
            values.iter().map(|value| value.as_ptr()).collect();
        key_pointers.push(ptr::null());
        value_pointers.push(ptr::null());

        // SAFETY: both arrays are NULL-terminated and their CStrings outlive the call.
        let raw =
            unsafe { ffi::PQconnectStartParams(key_pointers.as_ptr(), value_pointers.as_ptr(), 0) };
        if raw.is_null() {
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        }
        // SAFETY: the callback has C ABI and intentionally ignores all notice content.
        unsafe { ffi::PQsetNoticeProcessor(raw, Some(discard_notice), ptr::null_mut()) };
        let Some(connection_id) = ConnectionId::new(raw.addr()) else {
            // SAFETY: raw is a live libpq connection returned above.
            unsafe { ffi::PQfinish(raw) };
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        };
        if registry.register_secret(connection_id, token).is_err() {
            // SAFETY: raw is a live libpq connection returned above.
            unsafe { ffi::PQfinish(raw) };
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        }
        let connection = Self {
            raw,
            registry,
            connection_id,
        };

        // SAFETY: the live connection is exclusively owned by this thread.
        if unsafe { ffi::PQsetnonblocking(connection.raw, 1) } != 0 {
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        }
        loop {
            operation_allowed(deadline, cancellation)?;
            // SAFETY: raw remains live and exclusively owned.
            let status = unsafe { ffi::PQconnectPoll(connection.raw) };
            match status {
                ffi::PostgresPollingStatusType_PGRES_POLLING_OK => return Ok(connection),
                ffi::PostgresPollingStatusType_PGRES_POLLING_READING => {
                    connection.poll(true, false, deadline, cancellation)?;
                }
                ffi::PostgresPollingStatusType_PGRES_POLLING_WRITING => {
                    connection.poll(false, true, deadline, cancellation)?;
                }
                ffi::PostgresPollingStatusType_PGRES_POLLING_ACTIVE => {}
                _ => return Err(DatabaseError::new(DatabaseErrorKind::Unavailable)),
            }
        }
    }

    fn control(
        &mut self,
        statement: &str,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<(), DatabaseError> {
        let outcome = self.query(statement, &[], deadline, cancellation)?;
        if outcome.rows.is_empty() {
            Ok(())
        } else {
            Err(DatabaseError::new(DatabaseErrorKind::Unavailable))
        }
    }

    fn query(
        &mut self,
        statement: &str,
        binds: &[BindValue],
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<QueryOutcome, DatabaseError> {
        operation_allowed(deadline, cancellation)?;
        let statement = c_string(statement)?;
        let encoded_binds = EncodedBinds::new(binds)?;
        let bind_count = c_int::try_from(encoded_binds.values.len())
            .map_err(|_| DatabaseError::new(DatabaseErrorKind::InvalidRequest))?;
        // SAFETY: all pointers refer to live owned buffers for the duration of the call.
        let sent = unsafe {
            ffi::PQsendQueryParams(
                self.raw,
                statement.as_ptr(),
                bind_count,
                ptr::null(),
                encoded_binds.values.as_ptr(),
                encoded_binds.lengths.as_ptr(),
                encoded_binds.formats.as_ptr(),
                0,
            )
        };
        if sent != 1 {
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        }
        self.flush(deadline, cancellation)?;
        self.wait_for_result(deadline, cancellation)?;

        // SAFETY: libpq owns the returned PGresult until PQclear below.
        let raw_result = unsafe { ffi::PQgetResult(self.raw) };
        if raw_result.is_null() {
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        }
        let result = ResultOwner(raw_result);
        let outcome = result.decode()?;
        // A single extended-protocol statement must produce exactly one result.
        // SAFETY: the connection remains live and no other query is active.
        let unexpected = unsafe { ffi::PQgetResult(self.raw) };
        if !unexpected.is_null() {
            // SAFETY: libpq transferred ownership of this unexpected result to the caller.
            unsafe { ffi::PQclear(unexpected) };
            return Err(DatabaseError::new(DatabaseErrorKind::Rejected));
        }
        Ok(outcome)
    }

    fn flush(&self, deadline: Instant, cancellation: &Cancellation) -> Result<(), DatabaseError> {
        loop {
            operation_allowed(deadline, cancellation).inspect_err(|_| self.cancel_query())?;
            // SAFETY: the connection is live and exclusively owned.
            match unsafe { ffi::PQflush(self.raw) } {
                0 => return Ok(()),
                1 => self.poll(false, true, deadline, cancellation)?,
                _ => return Err(DatabaseError::new(DatabaseErrorKind::Unavailable)),
            }
        }
    }

    fn wait_for_result(
        &self,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<(), DatabaseError> {
        loop {
            operation_allowed(deadline, cancellation).inspect_err(|_| self.cancel_query())?;
            // SAFETY: the connection is live and exclusively owned.
            if unsafe { ffi::PQconsumeInput(self.raw) } != 1 {
                return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
            }
            // SAFETY: the connection is live and exclusively owned.
            if unsafe { ffi::PQisBusy(self.raw) } == 0 {
                return Ok(());
            }
            self.poll(true, false, deadline, cancellation)?;
        }
    }

    fn poll(
        &self,
        read: bool,
        write: bool,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<(), DatabaseError> {
        operation_allowed(deadline, cancellation)?;
        // SAFETY: the connection is live and exclusively owned.
        let socket = unsafe { ffi::PQsocket(self.raw) };
        if socket < 0 {
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        }
        // SAFETY: this asks libpq for its monotonic-compatible current time.
        let now = unsafe { ffi::PQgetCurrentTimeUSec() };
        let poll_deadline = now.saturating_add(POLL_SLICE_MICROSECONDS);
        // SAFETY: socket belongs to the live connection and the deadline uses libpq's clock.
        let result = unsafe {
            ffi::PQsocketPoll(socket, c_int::from(read), c_int::from(write), poll_deadline)
        };
        if result < 0 {
            return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
        }
        Ok(())
    }

    fn cancel_query(&self) {
        // SAFETY: the live connection is exclusively owned; cancelConn is always finalized.
        unsafe {
            let cancel = ffi::PQcancelCreate(self.raw);
            if !cancel.is_null() {
                let _ = ffi::PQcancelBlocking(cancel);
                ffi::PQcancelFinish(cancel);
            }
        }
    }

    fn rollback_best_effort(&mut self) {
        let cancellation = Cancellation::new();
        let _ = self.control(
            "ROLLBACK",
            Instant::now() + Duration::from_millis(500),
            &cancellation,
        );
    }
}

unsafe extern "C" fn discard_notice(_argument: *mut c_void, _message: *const c_char) {}

impl Drop for PgConnection {
    fn drop(&mut self) {
        self.registry.cleanup(self.connection_id);
        // SAFETY: this object is the sole owner and calls PQfinish exactly once.
        unsafe { ffi::PQfinish(self.raw) };
    }
}

struct EncodedBinds {
    _storage: Vec<Option<CString>>,
    values: Vec<*const c_char>,
    lengths: Vec<c_int>,
    formats: Vec<c_int>,
}

impl EncodedBinds {
    fn new(binds: &[BindValue]) -> Result<Self, DatabaseError> {
        let mut storage = Vec::with_capacity(binds.len());
        for bind in binds {
            let encoded = match bind {
                BindValue::Null => None,
                BindValue::Text(value) => Some(c_string(value)?),
                BindValue::Int64(value) => Some(c_string(&value.to_string())?),
                BindValue::Boolean(value) => Some(c_string(if *value { "true" } else { "false" })?),
                BindValue::Json(value) => {
                    Some(c_string(&serde_json::to_string(value).map_err(|_| {
                        DatabaseError::new(DatabaseErrorKind::InvalidRequest)
                    })?)?)
                }
            };
            storage.push(encoded);
        }
        let values = storage
            .iter()
            .map(|value| value.as_ref().map_or(ptr::null(), |value| value.as_ptr()))
            .collect();
        Ok(Self {
            lengths: vec![0; binds.len()],
            formats: vec![0; binds.len()],
            _storage: storage,
            values,
        })
    }
}

struct ResultOwner(*mut ffi::PGresult);

impl ResultOwner {
    fn decode(&self) -> Result<QueryOutcome, DatabaseError> {
        // SAFETY: self owns a live PGresult for the duration of decoding.
        let status = unsafe { ffi::PQresultStatus(self.0) };
        match status {
            ffi::ExecStatusType_PGRES_COMMAND_OK | ffi::ExecStatusType_PGRES_TUPLES_OK => {}
            _ => return Err(DatabaseError::rejected(self.sqlstate_class())),
        }

        // SAFETY: all accessors only inspect this live PGresult.
        let row_count = usize::try_from(unsafe { ffi::PQntuples(self.0) })
            .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
        let column_count = usize::try_from(unsafe { ffi::PQnfields(self.0) })
            .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
        if row_count > MAX_RESULT_ROWS {
            return Err(DatabaseError::new(DatabaseErrorKind::BudgetExceeded));
        }

        let mut columns = Vec::with_capacity(column_count);
        for column in 0..column_count {
            let column_index = c_int::try_from(column)
                .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
            // SAFETY: column_index is within PQnfields.
            let name = unsafe { ffi::PQfname(self.0, column_index) };
            if name.is_null() {
                return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
            }
            // SAFETY: libpq returns a NUL-terminated name owned by PGresult.
            let name = unsafe { CStr::from_ptr(name) }
                .to_str()
                .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?
                .to_owned();
            // SAFETY: column_index is within PQnfields.
            let type_oid = unsafe { ffi::PQftype(self.0, column_index) };
            columns.push(ResultColumn { name, type_oid });
        }

        let mut rows = Vec::with_capacity(row_count);
        let mut value_bytes = 0_usize;
        for row in 0..row_count {
            let row_index = c_int::try_from(row)
                .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
            let mut values = Vec::with_capacity(column_count);
            for (column, column_schema) in columns.iter().enumerate() {
                let column_index = c_int::try_from(column)
                    .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
                // SAFETY: both indexes are inside the result bounds.
                if unsafe { ffi::PQgetisnull(self.0, row_index, column_index) } == 1 {
                    values.push(Value::Null);
                    continue;
                }
                // SAFETY: both indexes are inside the result bounds.
                let length =
                    usize::try_from(unsafe { ffi::PQgetlength(self.0, row_index, column_index) })
                        .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
                if length > MAX_RESULT_VALUE_BYTES {
                    return Err(DatabaseError::new(DatabaseErrorKind::BudgetExceeded));
                }
                value_bytes = value_bytes.saturating_add(length);
                if value_bytes > MAX_RESULT_BYTES {
                    return Err(DatabaseError::new(DatabaseErrorKind::BudgetExceeded));
                }
                // SAFETY: libpq returns at least length readable bytes for this cell.
                let pointer = unsafe { ffi::PQgetvalue(self.0, row_index, column_index) };
                if pointer.is_null() {
                    return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
                }
                // SAFETY: length came from PQgetlength for this pointer.
                let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), length) };
                let text = std::str::from_utf8(bytes)
                    .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?;
                let value = match column_schema.type_oid {
                    JSON_OID | JSONB_OID => serde_json::from_str(text)
                        .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?,
                    _ => Value::String(text.to_owned()),
                };
                values.push(value);
            }
            rows.push(values);
        }

        // SAFETY: command status and tuple count are NUL-terminated strings owned by PGresult.
        let command_tag = unsafe { c_string_from_libpq(ffi::PQcmdStatus(self.0)) }?;
        // SAFETY: see above; empty is valid for statements without affected rows.
        let affected = unsafe { c_string_from_libpq(ffi::PQcmdTuples(self.0)) }?;
        let affected_rows = if affected.is_empty() {
            0
        } else {
            affected
                .parse()
                .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))?
        };
        Ok(QueryOutcome {
            columns,
            rows,
            command_tag,
            affected_rows,
        })
    }

    fn sqlstate_class(&self) -> Option<String> {
        // SAFETY: self owns a live result; NULL indicates no SQLSTATE.
        let field = unsafe { ffi::PQresultErrorField(self.0, c_int::from(ffi::PG_DIAG_SQLSTATE)) };
        if field.is_null() {
            return None;
        }
        // SAFETY: libpq returns a NUL-terminated field owned by PGresult.
        let state = unsafe { CStr::from_ptr(field) }.to_str().ok()?;
        (state.len() == 5).then(|| state[..2].to_owned())
    }
}

impl Drop for ResultOwner {
    fn drop(&mut self) {
        // SAFETY: this object owns the PGresult and clears it exactly once.
        unsafe { ffi::PQclear(self.0) };
    }
}

fn c_string(value: &str) -> Result<CString, DatabaseError> {
    CString::new(value).map_err(|_| DatabaseError::new(DatabaseErrorKind::InvalidRequest))
}

unsafe fn c_string_from_libpq(pointer: *const c_char) -> Result<String, DatabaseError> {
    if pointer.is_null() {
        return Err(DatabaseError::new(DatabaseErrorKind::Unavailable));
    }
    // SAFETY: callers only pass libpq-owned NUL-terminated strings.
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| DatabaseError::new(DatabaseErrorKind::Unavailable))
}

fn operation_allowed(deadline: Instant, cancellation: &Cancellation) -> Result<(), DatabaseError> {
    if cancellation.is_cancelled() || Instant::now() >= deadline {
        return Err(DatabaseError::new(DatabaseErrorKind::DeadlineExceeded));
    }
    Ok(())
}

#[allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
mod ffi {
    include!(concat!(
        env!("OUT_DIR"),
        "/mtmpg_executor_libpq_bindings.rs"
    ));
}
