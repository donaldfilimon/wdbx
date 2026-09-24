//! Loopback-only WDBX REST routing and bounded HTTP/1.1 transport.
//!
//! This is intentionally a small local-control surface, not a general web
//! framework. It accepts one request per connection, caps requests at 64 KiB,
//! optionally checks a bearer token, and always closes the connection.

use crate::rate_limit::{RateLimitStats, RateLimiter};
use crate::{HybridScorer, RecordId, TemporalCausalGraph, V2Error, VersionedError, VersionedStore};
use abi_foundation::env::WDBX_REST_TOKEN;
use abi_foundation::http::{
    DeadlineReader, MAX_REQUEST_SIZE, ReadResult, find_body, has_bearer_token,
    loopback_host_allowed, loopback_origin_allowed, read_request, reason_phrase, write_all,
    write_unauthorized,
};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const REQUEST_DEADLINE: Duration = Duration::from_secs(5);
const MAX_REQUEST_WORKERS: usize = 16;
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const OVERLOAD_READ_DEADLINE: Duration = Duration::from_millis(25);
const RESPONSE_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const OVERLOAD_WRITE_TIMEOUT: Duration = Duration::from_millis(250);
const OVERLOADED: &[u8] = b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: 29\r\nConnection: close\r\n\r\n{\"error\":\"server overloaded\"}";

/// JSON response returned by the pure router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestResponse {
    /// HTTP status code.
    pub status: u16,
    /// UTF-8 JSON body.
    pub body: String,
}

impl RestResponse {
    fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            body: value.to_string(),
        }
    }

    fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, &json!({"error": message.into()}))
    }
}

/// Route one already-framed REST request.
pub fn route(
    store: &mut VersionedStore,
    method: &str,
    path: &str,
    body: &[u8],
    now_ms: i64,
) -> RestResponse {
    match (method, path) {
        ("GET", "/health") => RestResponse::json(200, &json!({"status": "ok"})),
        ("GET", "/stats") => RestResponse::json(200, &store_stats(store)),
        ("POST", "/verify") => RestResponse::json(
            200,
            &json!({
                "chain_valid": store.snapshot().verify_audit_dag().is_ok(),
                "blocks": store.stats().blocks,
            }),
        ),
        ("POST", "/insert") => route_insert(store, body, now_ms),
        ("POST", "/query") => route_query(store, body, now_ms),
        _ => RestResponse::error(404, format!("no route for {method} {path}")),
    }
}

fn route_insert(store: &mut VersionedStore, body: &[u8], now_ms: i64) -> RestResponse {
    let object = match parsed_object(body) {
        Ok(object) => object,
        Err(response) => return response,
    };

    if let Some(profile_value) = object.get("profile") {
        let Some(profile) = profile_value.as_str() else {
            return RestResponse::error(400, "profile must be a string");
        };
        let metadata = object.get("metadata").and_then(Value::as_str).unwrap_or("");
        return match store.add_block(
            profile,
            RecordId::new_v2(),
            RecordId::new_v2(),
            metadata,
            now_ms,
        ) {
            Ok(_) => RestResponse::json(
                200,
                &json!({"inserted": "block", "blocks": store.stats().blocks}),
            ),
            Err(error) => RestResponse::error(500, error.to_string()),
        };
    }

    if let Some(vector_value) = object.get("vector") {
        let vector = match parse_vector(vector_value) {
            Ok(vector) => vector,
            Err(message) => return RestResponse::error(400, message),
        };
        return match store.put_vector(&vector) {
            Ok(id) => RestResponse::json(200, &json!({"inserted": "vector", "id": id})),
            Err(error @ VersionedError::V2(V2Error::InvalidMutation(_))) => {
                RestResponse::error(400, error.to_string())
            }
            Err(error) => RestResponse::error(500, error.to_string()),
        };
    }

    let key = object.get("key").and_then(Value::as_str);
    let value = object.get("value").and_then(Value::as_str);
    let (Some(key), Some(value)) = (key, value) else {
        return RestResponse::error(400, "need key+value or profile");
    };
    match store.put(key, value) {
        Ok(_) => RestResponse::json(200, &json!({"inserted": "kv"})),
        Err(error) => RestResponse::error(500, error.to_string()),
    }
}

fn route_query(store: &VersionedStore, body: &[u8], now_ms: i64) -> RestResponse {
    let object = match parsed_object(body) {
        Ok(object) => object,
        Err(response) => return response,
    };

    if let Some(key_value) = object.get("key") {
        let Some(key) = key_value.as_str() else {
            return RestResponse::error(400, "key must be a string");
        };
        return match store.get(key) {
            Some(value) => RestResponse::json(200, &json!({"value": value})),
            None => RestResponse::error(404, "not found"),
        };
    }

    let Some(vector_value) = object.get("vector") else {
        return RestResponse::error(400, "need key or vector");
    };
    let vector = match parse_vector(vector_value) {
        Ok(vector) => vector,
        Err(message) => return RestResponse::error(400, message),
    };
    let limit = match parse_limit(object.get("limit")) {
        Ok(limit) => limit,
        Err(message) => return RestResponse::error(400, message),
    };
    if store.stats().vectors == 0 {
        return RestResponse::json(200, &json!({"results": [], "vectors": 0}));
    }

    let snapshot = store.snapshot();
    let graph = TemporalCausalGraph::from_v2_records(&snapshot.preferred_temporal_records());
    let focus_id = snapshot
        .causal_focus_vector_id()
        .unwrap_or(RecordId::Legacy(1));
    let scorer = HybridScorer::new(now_ms);
    let mut ranked = match store.search(&vector, limit) {
        Ok(ranked) => ranked
            .into_iter()
            .map(|result| {
                let components = scorer.score(&graph, focus_id, result.id, result.score, 0.5);
                (result, components)
            })
            .collect::<Vec<_>>(),
        Err(error) => return RestResponse::error(400, error.to_string()),
    };
    ranked.sort_by(|left, right| {
        right
            .1
            .combined()
            .total_cmp(&left.1.combined())
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let mut results = String::from("[");
    for (index, (node, components)) in ranked.into_iter().enumerate() {
        if index > 0 {
            results.push(',');
        }
        write!(
            results,
            "{{\"id\":{},\"score\":{:.6},\"semantic\":{:.6},\"temporal\":{:.6},\"causal\":{:.6},\"persona\":{:.6}}}",
            serde_json::to_string(&node.id).expect("RecordId serializes"),
            components.combined(),
            components.semantic,
            components.temporal,
            components.causal,
            components.persona,
        )
        .expect("writing JSON into String cannot fail");
    }
    results.push(']');
    RestResponse {
        status: 200,
        body: format!(
            "{{\"results\":{results},\"vectors\":{},\"ranking\":\"hybrid\"}}",
            store.stats().vectors
        ),
    }
}

fn parsed_object(body: &[u8]) -> Result<serde_json::Map<String, Value>, RestResponse> {
    let value: Value =
        serde_json::from_slice(body).map_err(|_| RestResponse::error(400, "invalid json"))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| RestResponse::error(400, "expected object"))
}

fn parse_vector(value: &Value) -> Result<Vec<f32>, &'static str> {
    let Some(array) = value.as_array() else {
        return Err("vector must be an array");
    };
    if array.is_empty() {
        return Err("vector must be non-empty");
    }
    array
        .iter()
        .map(|component| {
            component
                .as_f64()
                .map(json_number_to_f32)
                .filter(|number| number.is_finite())
                .ok_or("vector elements must be numbers")
        })
        .collect()
}

#[allow(clippy::cast_possible_truncation)]
fn json_number_to_f32(number: f64) -> f32 {
    // Zig's REST parser uses @floatCast for the same wire-format boundary.
    // Non-finite results are rejected by the caller.
    number as f32
}

fn parse_limit(value: Option<&Value>) -> Result<usize, &'static str> {
    let Some(value) = value else {
        return Ok(10);
    };
    if !value.is_number() || value.as_f64().is_some_and(f64::is_nan) {
        return Err("limit must be an integer");
    }
    let Some(limit) = value.as_u64() else {
        return if value.as_i64().is_some() {
            Err("limit must be between 1 and 100")
        } else {
            Err("limit must be an integer")
        };
    };
    if !(1..=100).contains(&limit) {
        return Err("limit must be between 1 and 100");
    }
    usize::try_from(limit).map_err(|_| "limit must be between 1 and 100")
}

fn store_stats(store: &VersionedStore) -> Value {
    let stats = store.stats();
    json!({
        "kv_entries": stats.kv_entries,
        "vectors": stats.vectors,
        "blocks": stats.blocks,
        "spatial_records": stats.spatial_records,
        "temporal_nodes": stats.temporal_nodes,
        "temporal_edges": stats.temporal_edges,
        "vector_dimensions": store.snapshot().vector_dimensions(),
        "next_vector_id": Value::Null,
        "format_version": 2,
        "backend": "cpu",
        "mode": "fallback",
    })
}

/// Configuration for one local REST service.
#[derive(Debug)]
pub struct RestConfig {
    /// Optional bearer token. `None` preserves the oracle's local auth-off mode.
    pub bearer_token: Option<String>,
    /// Token bucket shared by all connections.
    pub rate_limiter: RateLimiter,
}

impl RestConfig {
    /// Load auth and rate-limit configuration from the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            bearer_token: std::env::var(WDBX_REST_TOKEN)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            rate_limiter: RateLimiter::from_env(),
        }
    }
}

/// One-request-per-connection REST server with serialized store routing.
#[derive(Debug)]
pub struct RestServer {
    listener: TcpListener,
    store: VersionedStore,
    config: RestConfig,
}

impl RestServer {
    /// Bind exactly `127.0.0.1:port`.
    pub fn bind(port: u16, store: VersionedStore, config: RestConfig) -> io::Result<Self> {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))?;
        Ok(Self {
            listener,
            store,
            config,
        })
    }

    /// Kernel-selected or configured local port.
    pub fn local_port(&self) -> io::Result<u16> {
        Ok(self.listener.local_addr()?.port())
    }

    /// Accept and handle one connection.
    pub fn serve_one(&mut self) -> io::Result<()> {
        let (stream, _) = self.listener.accept()?;
        handle_connection(&mut self.store, &self.config, stream)
    }

    /// Serve indefinitely with bounded request workers.
    pub fn run(&mut self) -> io::Result<()> {
        self.serve_until(&Arc::new(AtomicBool::new(false)))
    }

    /// Keep slow request reads and writes off the accept loop until `stop` is set.
    ///
    /// At most 16 connection workers and 16 completed requests are pending.
    /// The store is routed only on this thread, with no network I/O in routing.
    pub fn serve_until(&mut self, stop: &Arc<AtomicBool>) -> io::Result<()> {
        self.listener.set_nonblocking(true)?;
        let (ready_tx, ready_rx) = mpsc::sync_channel::<(
            ReadResult,
            mpsc::SyncSender<Option<Vec<u8>>>,
        )>(MAX_REQUEST_WORKERS);
        let mut workers = Vec::new();
        let result = loop {
            if stop.load(Ordering::SeqCst) {
                break Ok(());
            }
            if let Ok((raw, reply)) = ready_rx.try_recv() {
                let response = prepare_response(&mut self.store, &self.config, raw);
                let _ = reply.send(response);
            }
            reap_workers(&mut workers);
            match self.listener.accept() {
                Ok((mut stream, _)) if workers.len() >= MAX_REQUEST_WORKERS => {
                    if let Err(error) = reject_overloaded(&mut stream, stop) {
                        eprintln!("wdbx REST overload response failed: {error}");
                    }
                }
                Ok((stream, _)) => {
                    let sender = ready_tx.clone();
                    let stop = Arc::clone(stop);
                    let worker = thread::Builder::new()
                        .name("wdbx-rest-request".into())
                        .spawn(move || -> io::Result<()> {
                            let mut stream = stream;
                            let raw = read_request(
                                &mut DeadlineReader::with_stop(
                                    &mut stream,
                                    &stop,
                                    REQUEST_DEADLINE,
                                ),
                                MAX_REQUEST_SIZE,
                            );
                            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
                            if sender.send((raw, reply_tx)).is_err() {
                                return Ok(());
                            }
                            if let Ok(Some(response)) = reply_rx.recv() {
                                stream.set_write_timeout(Some(RESPONSE_WRITE_TIMEOUT))?;
                                write_all(&mut stream, &response)?;
                            }
                            Ok(())
                        });
                    match worker {
                        Ok(worker) => workers.push(worker),
                        Err(error) => break Err(error),
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(ACCEPT_POLL_INTERVAL);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => break Err(error),
            }
        };
        stop.store(true, Ordering::SeqCst);
        drop(ready_rx);
        drop(ready_tx);
        for worker in workers {
            if let Err(error) = worker
                .join()
                .unwrap_or_else(|_| Err(io::Error::other("request worker panicked")))
            {
                eprintln!("wdbx REST request worker error: {error}");
            }
        }
        self.listener.set_nonblocking(false)?;
        result
    }

    /// Borrow the underlying durable store.
    #[must_use]
    pub fn store(&self) -> &VersionedStore {
        &self.store
    }
}

fn reap_workers(workers: &mut Vec<thread::JoinHandle<io::Result<()>>>) {
    let mut index = 0;
    while index < workers.len() {
        if workers[index].is_finished() {
            if let Err(error) = workers
                .swap_remove(index)
                .join()
                .unwrap_or_else(|_| Err(io::Error::other("request worker panicked")))
            {
                eprintln!("wdbx REST request worker error: {error}");
            }
        } else {
            index += 1;
        }
    }
}

fn reject_overloaded(stream: &mut TcpStream, stop: &AtomicBool) -> io::Result<()> {
    let _ = read_request(
        &mut DeadlineReader::with_stop(stream, stop, OVERLOAD_READ_DEADLINE),
        MAX_REQUEST_SIZE,
    );
    stream.set_write_timeout(Some(OVERLOAD_WRITE_TIMEOUT))?;
    write_all(stream, OVERLOADED)
}

fn handle_connection(
    store: &mut VersionedStore,
    config: &RestConfig,
    stream: TcpStream,
) -> io::Result<()> {
    handle_connection_with_deadline(store, config, stream, REQUEST_DEADLINE)
}

fn handle_connection_with_deadline(
    store: &mut VersionedStore,
    config: &RestConfig,
    mut stream: TcpStream,
    deadline: Duration,
) -> io::Result<()> {
    let raw = read_request(
        &mut DeadlineReader::new(&mut stream, deadline),
        MAX_REQUEST_SIZE,
    );
    handle_read_connection(store, config, &mut stream, raw)
}

fn handle_read_connection(
    store: &mut VersionedStore,
    config: &RestConfig,
    stream: &mut TcpStream,
    raw: ReadResult,
) -> io::Result<()> {
    if let Some(response) = prepare_response(store, config, raw) {
        stream.set_write_timeout(Some(RESPONSE_WRITE_TIMEOUT))?;
        write_all(stream, &response)?;
    }
    Ok(())
}

fn prepare_response(
    store: &mut VersionedStore,
    config: &RestConfig,
    raw: ReadResult,
) -> Option<Vec<u8>> {
    let raw = match raw {
        ReadResult::Empty => return None,
        ReadResult::Malformed => {
            return Some(response_bytes(
                &RestResponse::error(400, "malformed request"),
                &[],
            ));
        }
        ReadResult::Incomplete => {
            return Some(response_bytes(
                &RestResponse::error(400, "incomplete request"),
                &[],
            ));
        }
        ReadResult::TooLarge => {
            return Some(response_bytes(
                &RestResponse::error(413, "request too large"),
                &[],
            ));
        }
        ReadResult::Request(raw) => raw,
    };
    let Ok(raw_text) = std::str::from_utf8(&raw) else {
        return Some(response_bytes(
            &RestResponse::error(400, "incomplete request"),
            &[],
        ));
    };
    if !loopback_host_allowed(raw_text) || !loopback_origin_allowed(raw_text) {
        return Some(response_bytes(
            &RestResponse::error(403, "loopback host and origin required"),
            &[],
        ));
    }
    let (method, path) = request_target(raw_text)?;

    // Ordering is part of the security contract: failed auth consumes a token.
    if !config.rate_limiter.acquire() {
        return Some(response_bytes(
            &RestResponse::error(429, "too many requests"),
            &[("Retry-After", "1")],
        ));
    }
    if let Some(token) = &config.bearer_token
        && !has_bearer_token(raw_text, token)
    {
        let mut response = Vec::new();
        write_unauthorized(&mut response, "unauthorized")
            .expect("writing an HTTP response into Vec cannot fail");
        return Some(response);
    }

    let mut response = route(
        store,
        method,
        path,
        find_body(&raw).unwrap_or_default(),
        unix_ms(),
    );
    if path == "/stats" && response.status == 200 {
        response.body = add_rate_stats(&response.body, config.rate_limiter.stats());
    }
    Some(response_bytes(&response, &[]))
}

fn request_target(raw: &str) -> Option<(&str, &str)> {
    let request_line = raw.lines().next()?.trim_end_matches('\r');
    let mut fields = request_line.split(' ');
    Some((fields.next()?, fields.next()?))
}

fn add_rate_stats(body: &str, stats: RateLimitStats) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(body) else {
        return body.to_string();
    };
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "rate_limit".to_string(),
            serde_json::to_value(stats).expect("rate-limit stats serialize"),
        );
    }
    value.to_string()
}

fn response_bytes(response: &RestResponse, extra_headers: &[(&str, &str)]) -> Vec<u8> {
    let phrase = if response.status == 413 {
        "Payload Too Large"
    } else {
        reason_phrase(response.status)
    };
    let mut header = format!(
        "HTTP/1.1 {} {phrase}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        response.status,
        response.body.len()
    );
    for (name, value) in extra_headers {
        header.push_str(name);
        header.push_str(": ");
        header.push_str(value);
        header.push_str("\r\n");
    }
    header.push_str("Connection: close\r\n\r\n");
    let mut bytes = header.into_bytes();
    bytes.extend_from_slice(response.body.as_bytes());
    bytes
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
