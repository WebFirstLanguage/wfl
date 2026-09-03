#![deny(clippy::await_holding_refcell_ref)]
mod assertion_helpers;
pub mod bounded_buffer;
pub mod command_sanitizer;
pub mod control_flow;
pub mod database;
pub mod environment;
pub mod error;
pub(crate) mod io_capture;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod op_refactor_error_tests;
#[cfg(test)]
mod op_refactor_tests;
pub mod sessions;
#[cfg(test)]
mod tests;
mod tls;
pub mod value;

use self::control_flow::ControlFlow;

use self::environment::Environment;
use self::error::{ErrorKind, RuntimeError};
use self::value::{
    ContainerDefinitionValue, ContainerEventValue, ContainerInstanceValue, ContainerMethodValue,
    EventHandler, FunctionValue, InterfaceDefinitionValue, OverloadedFunction, StaticMethodContext,
    Value,
};
use crate::builtins::get_function_arity;
use crate::config::WflConfig;
use crate::debug_report::CallFrame;
use crate::exec::budget::{BudgetExceeded, ExecutionBudget};
#[cfg(debug_assertions)]
use crate::exec_block_enter;
#[cfg(debug_assertions)]
use crate::exec_block_exit;
#[cfg(debug_assertions)]
use crate::exec_control_flow;
#[cfg(debug_assertions)]
use crate::exec_function_call;
#[cfg(debug_assertions)]
use crate::exec_function_return;
use crate::exec_trace;
#[cfg(debug_assertions)]
use crate::exec_var_assign;
#[cfg(debug_assertions)]
use crate::exec_var_declare;
#[cfg(debug_assertions)]
use crate::logging::IndentGuard;
use crate::parser::ast::{
    Assertion, ExitScope, Expression, FileOpenMode, Literal, Operator, Program, Statement, Type,
    UnaryOperator, WsHandlerEvent,
};
use crate::pattern::CompiledPattern;
use crate::stdlib;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

// Type alias for complex pending response type
type PendingResponseSender = Arc<tokio::sync::Mutex<Option<oneshot::Sender<HandlerReply>>>>;

/// An open server response stream: the bounded body-chunk sender, the running
/// total of body bytes written (enforced against `max_response_bytes`), and an
/// optional oneshot that the transport fires when it has finished with the body.
///
/// The finished signal is how `close out` (and end-of-handler drain) wait for the
/// terminating chunk to actually leave the process — dropping the sender only
/// *signals* end-of-body; under load the warp/hyper connection task can still be
/// flushing when the program returns and the runtime is dropped, which surfaces
/// as a truncated chunked body on the client (#680).
struct ServerResponseStream {
    tx: mpsc::Sender<Vec<u8>>,
    bytes_written: usize,
    /// Taken and awaited by close/drain. `None` once already consumed, or when
    /// the stream was closed via a sync Drop path that cannot await.
    finished: Option<oneshot::Receiver<()>>,
}

/// Chunk body stream that notifies when hyper is done with it.
///
/// Dropped by the connection task after the response is fully written (or the
/// client disconnects). That Drop is the acknowledgement `close out` awaits so
/// the program does not finish before the terminating chunk is delivered.
/// Public only because it appears on [`HandlerReply::Streaming`]; fields are
/// private and construction stays inside the interpreter.
pub struct NotifyingChunkStream {
    rx: mpsc::Receiver<Vec<u8>>,
    done: Option<oneshot::Sender<()>>,
}

impl futures_util::Stream for NotifyingChunkStream {
    type Item = Result<Vec<u8>, std::io::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            std::task::Poll::Ready(Some(chunk)) => std::task::Poll::Ready(Some(Ok(chunk))),
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl Drop for NotifyingChunkStream {
    fn drop(&mut self) {
        if let Some(done) = self.done.take() {
            let _ = done.send(());
        }
    }
}

/// A dequeued HTTP request parked in `pending_responses` awaiting a `respond`.
///
/// Holds only the response channel. The request's in-flight admission slot is
/// **not** parked here — it lives with the warp transport task, which stays
/// alive awaiting this response, so the slot is released when that task
/// completes (a delivered response, a response timeout, or a client
/// disconnect) independently of any later admitted request. Parking the slot in
/// this map instead pinned it until a *future* dequeued request pruned it, which
/// permanently wedged admission once the cap was full (all route tasks timed out
/// but no new request could be admitted to trigger the prune).
struct PendingResponse {
    sender: PendingResponseSender,
}
use uuid;
use warp::Filter;

/// How often (in charged operations) execution cooperatively yields to the async
/// runtime. A tight CPU-bound `count`/`while`/`repeat` loop otherwise never
/// returns control to the executor, so a `select!` waiting to deliver
/// cooperative cancellation (e.g. the REPL's Ctrl-C → `budget.cancel()`) could
/// not be polled until the run happened to hit real async work. Power of two so
/// `count & (STRIDE - 1)` is exact; large enough that the yield is negligible.
const COOP_YIELD_STRIDE: u64 = 1024;

/// Bounded capacity of a server response stream's body-chunk channel. A slow
/// client fills this and then backpressures the handler's `write` (it awaits a
/// free slot) rather than letting queued chunks grow without bound.
const RESPONSE_STREAM_BUFFER: usize = 64;

/// How long `close out` / end-of-handler drain wait for the transport to finish
/// the body after the sender is dropped (#680).
///
/// Short on purpose and **independent** of `web_server_response_timeout_seconds`
/// (the write-path / admission bound, default 300s, with `0` = unbounded). Close
/// only needs enough time for hyper to flush the terminating chunk under load;
/// tying it to the write timeout would let a client that stops reading pin a
/// serial `main loop` for up to five minutes. After this ceiling, close returns
/// anyway — the sender is already dropped so the body will still end when the
/// transport can flush or the connection is later torn down.
const RESPONSE_STREAM_FINISH_WAIT: Duration = Duration::from_secs(2);

/// Maximum number of `main loop concurrently:` iterations in flight at once. The
/// transport already bounds the request *queue*; this bounds concurrent *handler*
/// execution so a burst cannot spawn unbounded cooperative tasks. Requests
/// beyond this run as handlers free up (and the transport sheds 503 if its queue
/// also fills).
const CONCURRENT_HANDLER_LIMIT: usize = 256;
const MAX_CONSECUTIVE_HANDLER_FAILURES: u32 = 256;
const REQUEST_WAIT_TIMEOUT_PREFIX: &str = "Timeout waiting for request";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConcurrentHandlerDisposition {
    RequestLocal,
    Structural,
}

fn classify_concurrent_handler_error(
    error: &RuntimeError,
    accepted_request: bool,
) -> ConcurrentHandlerDisposition {
    // `exit program` is a deliberate stop, so it must leave the loop and unwind
    // to the top of the run — never be absorbed as a request-local outcome,
    // even for a handler that already accepted a request.
    if error.is_exit_program() {
        return ConcurrentHandlerDisposition::Structural;
    }
    let request_wait_timeout =
        error.kind == ErrorKind::Timeout && error.message.starts_with(REQUEST_WAIT_TIMEOUT_PREFIX);
    if accepted_request || error.kind == ErrorKind::Cancelled || request_wait_timeout {
        ConcurrentHandlerDisposition::RequestLocal
    } else {
        ConcurrentHandlerDisposition::Structural
    }
}

// Web server data structures
#[derive(Debug)]
pub struct WflHttpRequest {
    pub id: String,
    pub method: String,
    pub path: String,
    /// Raw query string without the leading `?` (empty when the URL has none).
    /// Exposed as the `query` request property / loop-scoped variable so
    /// handlers can feed it to `parse_query_string`.
    pub query: String,
    pub client_ip: String,
    /// Raw request body bytes. Exposed to WFL both as a lossy-UTF-8 `body`
    /// text variable (backward compatible) and as a lossless `body_bytes`
    /// binary value, so binary uploads survive intact.
    pub body: Vec<u8>,
    pub headers: HashMap<String, String>,
    pub response_sender: Arc<tokio::sync::Mutex<Option<oneshot::Sender<HandlerReply>>>>,
}

#[derive(Debug, Clone)]
pub struct WflHttpResponse {
    /// Raw response body bytes. Text responses store their UTF-8 encoding;
    /// binary responses (`Value::Binary`) store their bytes verbatim, so
    /// fonts/images/etc. are served losslessly.
    pub content: Vec<u8>,
    pub status: u16,
    pub content_type: String,
    pub headers: HashMap<String, String>,
}

/// What a request handler delivers back to its warp transport task over the
/// per-request `oneshot`.
///
/// `respond` sends a fully-buffered `Buffered` reply; `start streaming response`
/// sends a `Streaming` reply whose head (status/headers) is written immediately
/// and whose body is fed chunk-by-chunk over a bounded channel by `write
/// line|chunk`. A bounded channel gives backpressure: a slow client slows the
/// handler's writes. Dropping the sender (handler end, `close`, or a caught
/// error) closes the body stream and finalizes the response. The body stream
/// notifies when the transport is finished with it so `close out` can await
/// delivery (#680).
pub enum HandlerReply {
    Buffered(WflHttpResponse),
    Streaming {
        status: u16,
        content_type: String,
        headers: HashMap<String, String>,
        body: NotifyingChunkStream,
    },
}

impl std::fmt::Debug for HandlerReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerReply::Buffered(response) => f.debug_tuple("Buffered").field(response).finish(),
            HandlerReply::Streaming {
                status,
                content_type,
                headers,
                body: _,
            } => f
                .debug_struct("Streaming")
                .field("status", status)
                .field("content_type", content_type)
                .field("headers", headers)
                .field("body", &"<stream>")
                .finish(),
        }
    }
}

/// Ensures an HTTP `respond` always resolves its request after the commit point.
/// Fallible response expressions are evaluated while the pending sender remains
/// available as a disconnect signal. At commit, the sender is atomically removed
/// and held here; if delivery then exits early, `Drop` answers 500 rather than
/// leaving the client hanging. Successful delivery calls
/// [`ResponseCompletion::take_sender`] to disarm that fallback.
struct ResponseCompletion {
    sender: Option<oneshot::Sender<HandlerReply>>,
}

/// Interpreter state that must survive cancellation of fallible response
/// expressions. The evaluation future is explicitly dropped before this state
/// is restored, so partially-entered actions and loops cannot leak into a
/// handler that catches `Cancelled` and continues.
struct ResponsePrecommitSnapshot {
    call_stack: Vec<CallFrame>,
    call_depth: usize,
    current_count: Option<f64>,
    in_count_loop: bool,
    http_owner: StreamOwner,
    http_streams: HashSet<String>,
    response_streams: HashSet<String>,
    pending_requests: HashSet<String>,
}

impl ResponseCompletion {
    fn take_sender(&mut self) -> Option<oneshot::Sender<HandlerReply>> {
        self.sender.take()
    }
}

impl Drop for ResponseCompletion {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(HandlerReply::Buffered(WflHttpResponse {
                content: b"Internal Server Error\n".to_vec(),
                status: 500,
                content_type: "text/plain; charset=utf-8".to_string(),
                headers: HashMap::new(),
            }));
        }
    }
}

/// Look up an HTTP header by name in a request headers map.
///
/// Header names are case-insensitive (RFC 7230). Warp stores them lowercase,
/// while WFL programs typically use canonical forms like `User-Agent`, so an
/// exact-key miss falls back to a case-insensitive scan. Extracted as a pure
/// function so unit tests pin the regression without spinning up a server.
pub(crate) fn lookup_header_case_insensitive(
    headers: &HashMap<String, Value>,
    header_name: &str,
) -> Option<Value> {
    headers.get(header_name).cloned().or_else(|| {
        let lowered = header_name.to_lowercase();
        headers
            .iter()
            .find(|(key, _)| key.to_lowercase() == lowered)
            .map(|(_, value)| value.clone())
    })
}

fn cookie_value_from_request(request: &Value, cookie_name: &str) -> Option<String> {
    let Value::Object(obj) = request else {
        return None;
    };
    let obj = obj.borrow();
    let headers = match obj.get("headers") {
        Some(Value::Object(headers)) => headers.borrow(),
        _ => return None,
    };
    let raw = lookup_header_case_insensitive(&headers, "cookie")?;
    let header = match raw {
        Value::Text(s) => s.to_string(),
        _ => return None,
    };
    for part in header.split(';') {
        let part = part.trim();
        if let Some((name, value)) = part.split_once('=')
            && name.trim() == cookie_name
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

#[derive(Debug)]
pub struct WflWebServer {
    // Bounded transport→interpreter queue (Phase 0, PR-0c): a full queue sheds
    // new requests with 503 rather than growing memory without bound.
    pub request_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<WflHttpRequest>>>,
    pub request_sender: mpsc::Sender<WflHttpRequest>,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    pub sessions: Option<Rc<sessions::SessionManager>>,
}

impl Drop for WflWebServer {
    fn drop(&mut self) {
        // Safety net for every non-`close server` teardown (interpreter drop, map
        // replacement, a caught setup error): abort the accept task so a bound
        // listener is never orphaned. `close server` moves `server_handle` out
        // first, so this does not double-abort.
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

/// Build the 503 response returned when the transport→interpreter request queue
/// is full (Phase 0, PR-0c). A free function so the shed path can be tested
/// without standing up a live server.
pub fn overloaded_response() -> warp::http::Response<Vec<u8>> {
    let body = b"Service Unavailable: the server is overloaded, please retry later\n".to_vec();
    let content_length = body.len();
    warp::http::Response::builder()
        .status(warp::http::StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Content-Length", content_length)
        .header("Retry-After", "1")
        .body(body)
        .expect("static 503 response is always valid")
}

/// Build a static `text/plain` response with the given status and message.
fn plain_status_response(
    status: warp::http::StatusCode,
    message: &str,
) -> warp::http::Response<Vec<u8>> {
    let body = message.as_bytes().to_vec();
    let content_length = body.len();
    warp::http::Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Content-Length", content_length)
        .body(body)
        .expect("static status response is always valid")
}

/// 413 returned when a request body exceeds `web_server_max_body_size`. Because
/// it is enforced while streaming, a chunked body with no `Content-Length` is
/// bounded too.
fn payload_too_large_response() -> warp::http::Response<Vec<u8>> {
    plain_status_response(
        warp::http::StatusCode::PAYLOAD_TOO_LARGE,
        "Payload Too Large: request body exceeds the configured limit\n",
    )
}

/// 504 returned when a handler does not answer an accepted request within
/// `web_server_response_timeout_seconds`, freeing its in-flight slot.
fn gateway_timeout_response() -> warp::http::Response<Vec<u8>> {
    plain_status_response(
        warp::http::StatusCode::GATEWAY_TIMEOUT,
        "Gateway Timeout: the request handler did not respond in time\n",
    )
}

/// 408 returned when a client does not finish sending its request body within
/// `web_server_response_timeout_seconds`, so a slow "trickle" upload cannot pin
/// its in-flight slot indefinitely.
fn request_timeout_response() -> warp::http::Response<Vec<u8>> {
    plain_status_response(
        warp::http::StatusCode::REQUEST_TIMEOUT,
        "Request Timeout: the request body was not received in time\n",
    )
}

/// Error from [`read_body_capped`].
enum BodyReadError {
    /// The streamed body exceeded the byte ceiling.
    TooLarge,
    /// The transport failed while reading the body.
    Io,
}

/// Read a streamed request body into a `Vec`, aborting as soon as it exceeds
/// `max` bytes. Unlike buffering the whole body first, this bounds memory for
/// chunked requests (which carry no `Content-Length`): the buffer never grows
/// past `max + 1` bytes before the limit trips.
async fn read_body_capped<S, B>(stream: S, max: usize) -> Result<Vec<u8>, BodyReadError>
where
    S: futures_util::Stream<Item = Result<B, warp::Error>>,
    B: bytes::Buf,
{
    use futures_util::StreamExt;

    futures_util::pin_mut!(stream);
    let mut out: Vec<u8> = Vec::new();
    while let Some(item) = stream.next().await {
        let mut chunk = item.map_err(|_| BodyReadError::Io)?;
        while chunk.has_remaining() {
            let slice = chunk.chunk();
            // Copy at most enough to reach `max + 1` (one sentinel byte past the
            // limit), so a single huge transport chunk cannot grow `out` to
            // `max + chunk_len` before the check — the buffer never exceeds
            // `max + 1` bytes.
            let allowance = max.saturating_sub(out.len()).saturating_add(1);
            if slice.len() >= allowance {
                out.extend_from_slice(&slice[..allowance]);
                return Err(BodyReadError::TooLarge);
            }
            let take = slice.len();
            out.extend_from_slice(slice);
            chunk.advance(take);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// WebSocket support
//
// WebSockets reuse the HTTP server's single-threaded design: warp runs each
// socket in background tokio tasks, which push lifecycle events to the
// interpreter over an unbounded channel and receive outbound frames back over a
// per-connection channel. The interpreter drains those events and runs the
// matching `on websocket ...` handler block while the program sits inside a
// `wait for <duration>`, so all WFL code still executes on one thread.
// ---------------------------------------------------------------------------

/// An outbound frame queued for a single connection's writer task. A `Text`
/// frame carries a [`WsBytePermit`] reserving its payload against the global
/// WebSocket queued-byte budget; the permit releases when the frame is sent
/// (consumed) or shed on a full/closed queue (dropped).
#[derive(Debug)]
enum WsOutbound {
    Text {
        text: String,
        _permit: crate::exec::budget::WsBytePermit,
    },
    Close,
}

/// Which lifecycle event a `WflWsEvent` represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WsEventKind {
    Connect,
    Message,
    Disconnect,
}

/// A lifecycle event pushed from a warp WebSocket task to the interpreter.
#[derive(Debug)]
struct WflWsEvent {
    kind: WsEventKind,
    connection_id: String,
    client_ip: String,
    /// Text payload for `Message` events; `None` for connect/disconnect.
    content: Option<String>,
    /// For `Message` events, the reservation of this payload's bytes against
    /// the global WebSocket queued-byte budget; released when the interpreter
    /// consumes (drops) the event. `None` for the payloadless connect/disconnect.
    _permit: Option<crate::exec::budget::WsBytePermit>,
}

/// Outbound-sender registry keyed by connection id, shared with warp tasks.
/// Each sender is a *bounded* channel (sized from the shared budget's
/// `ws_queue_bound`), so a slow client cannot make the server buffer frames
/// without bound — sends `try_send` and shed on `Full`.
type WsConnectionRegistry = Arc<std::sync::Mutex<HashMap<String, mpsc::Sender<WsOutbound>>>>;

/// A registered `on websocket ...` handler block plus its captured environment.
struct WsRegisteredHandler {
    binding: String,
    body: Vec<Statement>,
    env: Rc<RefCell<Environment>>,
}

/// The connect/message/disconnect handlers registered for one server.
#[derive(Default)]
struct WsHandlerSet {
    connect: Option<WsRegisteredHandler>,
    message: Option<WsRegisteredHandler>,
    disconnect: Option<WsRegisteredHandler>,
}

/// A running WebSocket server: an event stream in, the set of live connection
/// ids (for broadcast), the registered handler blocks, and the server task.
struct WflWebSocketServer {
    event_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<WflWsEvent>>>,
    connection_ids: Arc<std::sync::Mutex<Vec<String>>>,
    handlers: RefCell<WsHandlerSet>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Per-server cancellation: flipping (or dropping) this wakes every live
    /// connection's reader `select!` so `close server` terminates each socket
    /// task even if the peer ignores the close handshake.
    close_tx: tokio::sync::watch::Sender<bool>,
}

impl Drop for WflWebSocketServer {
    fn drop(&mut self) {
        // Safety net for every non-`close server` teardown (interpreter drop, map
        // replacement, a caught setup error): wake connections and abort the
        // accept task so a bound listener is never orphaned. `close server` moves
        // `server_handle` out first, so this does not double-abort.
        let _ = self.close_tx.send(true);
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

/// Drives one upgraded WebSocket connection: registers its outbound channel,
/// emits connect/message/disconnect events, and forwards queued frames to the
/// socket. Runs as an independent tokio task per connection.
async fn handle_ws_connection(
    socket: warp::ws::WebSocket,
    remote_addr: Option<std::net::SocketAddr>,
    events: mpsc::Sender<WflWsEvent>,
    connections: WsConnectionRegistry,
    connection_ids: Arc<std::sync::Mutex<Vec<String>>>,
    budget: Arc<ExecutionBudget>,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) {
    use futures_util::{SinkExt, StreamExt};

    let client_ip = remote_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Enforce the shared connection ceiling before doing any per-connection
    // work. The guard releases the slot when this task ends (any exit path).
    let _conn_guard = match budget.try_acquire_ws_connection() {
        Some(guard) => guard,
        None => {
            log::warn!(
                "WebSocket connection limit ({}) reached; refusing connection from {client_ip}",
                budget.limits().max_ws_connections
            );
            let (mut ws_tx, _ws_rx) = socket.split();
            let _ = ws_tx.send(warp::ws::Message::close()).await;
            let _ = ws_tx.flush().await;
            return;
        }
    };

    let conn_id = uuid::Uuid::new_v4().to_string();

    let (mut ws_tx, mut ws_rx) = socket.split();
    // Bounded outbound queue (sized from the budget): a slow client sheds
    // frames on `Full` instead of growing memory without bound.
    let (out_tx, mut out_rx) = mpsc::channel::<WsOutbound>(budget.ws_queue_bound());

    if let Ok(mut map) = connections.lock() {
        map.insert(conn_id.clone(), out_tx);
    }
    if let Ok(mut ids) = connection_ids.lock() {
        ids.push(conn_id.clone());
    }

    // The connect event MUST be delivered before this socket is allowed to emit
    // messages: it runs the `on websocket connect` handler (app init/auth). If it
    // cannot be admitted (queue full or closed), fail closed — unregister the
    // connection and close the socket — rather than leaving a live socket whose
    // connect handler never ran but which could still emit Message events once
    // the queue drains.
    if let Err(err) = events.try_send(WflWsEvent {
        kind: WsEventKind::Connect,
        connection_id: conn_id.clone(),
        client_ip: client_ip.clone(),
        content: None,
        _permit: None,
    }) {
        log::warn!(
            "WebSocket connect event for {conn_id} could not be admitted ({err}); closing the connection"
        );
        if let Ok(mut map) = connections.lock() {
            map.remove(&conn_id);
        }
        if let Ok(mut ids) = connection_ids.lock() {
            ids.retain(|c| c != &conn_id);
        }
        let _ = ws_tx.send(warp::ws::Message::close()).await;
        let _ = ws_tx.flush().await;
        return; // `_conn_guard` drops here, releasing the connection slot.
    }

    // Writer task: drains queued frames to the socket until an explicit close,
    // the peer going away, or the channel closing.
    let mut writer = tokio::spawn(async move {
        let mut peer_gone = false;
        while let Some(out) = out_rx.recv().await {
            match out {
                // `_permit` drops at the end of this arm, releasing the frame's
                // reserved bytes back to the global queued-byte budget once sent.
                WsOutbound::Text { text, _permit } => {
                    if ws_tx.send(warp::ws::Message::text(text)).await.is_err() {
                        peer_gone = true;
                        break;
                    }
                }
                // Fall through to the unconditional close below.
                WsOutbound::Close => break,
            }
        }
        // Always send a best-effort close frame on exit — whether from an
        // explicit `Close`, an empty channel (`close server` dropped every
        // sender), or a `Full` queue that could not carry the `Close` message.
        // This guarantees `close server` terminates the socket even under
        // backpressure, instead of leaving the reader task (and its connection
        // slot) alive waiting on the peer.
        if !peer_gone {
            let _ = ws_tx.send(warp::ws::Message::close()).await;
            let _ = ws_tx.flush().await;
        }
    });

    // Reader loop: forward inbound text frames as Message events; stop on close.
    // A `select!` on the per-server cancellation receiver means `close server`
    // (which flips/drops the watch channel) wakes a reader that would otherwise
    // block on `ws_rx.next()` forever waiting on a peer that ignores the close
    // handshake — so the task and its connection slot always terminate.
    loop {
        let result = tokio::select! {
            biased;
            changed = cancel.changed() => {
                // Ok(()) => close requested; Err(_) => the server (its watch
                // sender) was dropped. Either way, stop reading.
                let _ = changed;
                break;
            }
            next = ws_rx.next() => match next {
                Some(r) => r,
                None => break,
            },
        };
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.to_str() {
                        // Bound the frame: reject oversized payloads and reserve
                        // this payload's bytes against the global queued-byte
                        // budget, so the event queue holds bounded memory, not
                        // `ws_queue_bound` arbitrarily-large messages.
                        match budget.try_reserve_ws_bytes(text.len()) {
                            Some(permit) => {
                                if let Err(err) = events.try_send(WflWsEvent {
                                    kind: WsEventKind::Message,
                                    connection_id: conn_id.clone(),
                                    client_ip: client_ip.clone(),
                                    content: Some(text.to_string()),
                                    _permit: Some(permit),
                                }) {
                                    log::warn!(
                                        "WebSocket event queue full; dropping message event for {conn_id}: {err}"
                                    );
                                }
                            }
                            None => {
                                log::warn!(
                                    "WebSocket message from {conn_id} ({} bytes) exceeds the per-message or global queued-byte limit; dropping frame",
                                    text.len()
                                );
                            }
                        }
                    }
                } else if msg.is_close() {
                    break;
                }
                // Binary/ping/pong frames are ignored in this MVP.
            }
            Err(_) => break,
        }
    }

    if let Ok(mut map) = connections.lock() {
        map.remove(&conn_id);
    }
    if let Ok(mut ids) = connection_ids.lock() {
        ids.retain(|c| c != &conn_id);
    }
    // The disconnect event MUST be delivered. This connection's connect event
    // was delivered (connect is fail-closed above), so its `on websocket
    // connect` handler may have initialized per-connection application state,
    // and the paired `on websocket disconnect` handler is the only place that
    // state is cleaned up. A lossy `try_send` here could drop that cleanup under
    // a momentarily-full queue, leaking application state and leaving a queued
    // Connect with no matching Disconnect. Use a blocking send so every admitted
    // Connect gets its paired Disconnect: it resolves as the interpreter drains
    // the queue, and returns `Err` only if the server has shut down (its
    // receiver dropped), in which case no disconnect handler remains to run.
    if let Err(err) = events
        .send(WflWsEvent {
            kind: WsEventKind::Disconnect,
            connection_id: conn_id.clone(),
            client_ip,
            content: None,
            _permit: None,
        })
        .await
    {
        log::warn!(
            "WebSocket disconnect event for {conn_id} could not be delivered (server shut down): {err}"
        );
    }
    // Bounded close handshake: give the writer a moment to flush its close frame,
    // then force teardown so a peer that ignores the handshake cannot keep this
    // task alive. The writer exits once its channel drains/closes; on timeout we
    // abort it (dropping a JoinHandle would only detach, not stop, the task).
    if tokio::time::timeout(Duration::from_millis(250), &mut writer)
        .await
        .is_err()
    {
        writer.abort();
    }
}

impl WsEventKind {
    fn as_str(self) -> &'static str {
        match self {
            WsEventKind::Connect => "connect",
            WsEventKind::Message => "message",
            WsEventKind::Disconnect => "disconnect",
        }
    }
}

/// Builds the WFL object a handler binds to for a connection (`id`, `ip`). The
/// same shape is used for the connect/disconnect binding and for a message's
/// `sender`, so `id of conn` / `ip of conn` and `send ... to conn` all work.
fn build_ws_connection_object(id: &str, ip: &str) -> Value {
    let mut map = HashMap::new();
    map.insert("id".to_string(), Value::Text(Arc::from(id)));
    map.insert("ip".to_string(), Value::Text(Arc::from(ip)));
    Value::Object(Rc::new(RefCell::new(map)))
}

/// Builds the WFL object bound by a websocket handler for one event. Connect and
/// disconnect bindings expose `id`/`ip`; message bindings additionally expose
/// `body` (the text payload) and a `sender` connection object. The payload uses
/// `body` — mirroring `body of request` on the HTTP server — because `content`
/// is a reserved keyword and cannot appear before `of`. The message object's own
/// `id`/`ip` are its sender's, so `send ... to msg` replies to the sender.
fn build_ws_event_object(event: &WflWsEvent) -> Value {
    let mut map = HashMap::new();
    map.insert(
        "id".to_string(),
        Value::Text(Arc::from(event.connection_id.as_str())),
    );
    map.insert(
        "ip".to_string(),
        Value::Text(Arc::from(event.client_ip.as_str())),
    );
    if event.kind == WsEventKind::Message {
        let body = event.content.clone().unwrap_or_default();
        map.insert("body".to_string(), Value::Text(Arc::from(body.as_str())));
        map.insert(
            "sender".to_string(),
            build_ws_connection_object(&event.connection_id, &event.client_ip),
        );
    }
    Value::Object(Rc::new(RefCell::new(map)))
}

// Custom error type for warp rejections
#[derive(Debug)]
#[allow(dead_code)]
pub struct ServerError(String);

impl warp::reject::Reject for ServerError {}

/// Rejection raised when the server is at its in-flight request capacity, so a
/// request is shed *before* its body is buffered (Phase 0, PR-0c). Mapped to a
/// 503 by [`handle_overloaded`].
#[derive(Debug)]
struct Overloaded;

impl warp::reject::Reject for Overloaded {}

/// Rejection raised when a request's advertised `Content-Length` exceeds the
/// body ceiling, so it is refused before any body is read. Mapped to a 413 by
/// [`handle_overloaded`] (the streaming path returns the 413 directly).
#[derive(Debug)]
struct PayloadTooLarge;

impl warp::reject::Reject for PayloadTooLarge {}

/// Warp recover handler: turn an [`Overloaded`] rejection into a 503 and a
/// [`PayloadTooLarge`] rejection into a 413, and re-raise every other rejection
/// so warp's default handling still applies.
async fn handle_overloaded(
    err: warp::Rejection,
) -> Result<warp::http::Response<Vec<u8>>, warp::Rejection> {
    if err.find::<Overloaded>().is_some() {
        Ok(overloaded_response())
    } else if err.find::<PayloadTooLarge>().is_some() {
        Ok(payload_too_large_response())
    } else {
        Err(err)
    }
}

/// Strips the port from an HTTP Host header value, preserving IPv6 brackets
/// (`example.com:8080` -> `example.com`, `[::1]:8080` -> `[::1]`).
fn strip_host_port(host: &str) -> &str {
    if let Some(end) = host.rfind(']') {
        // Bracketed IPv6 literal, with or without a trailing :port
        &host[..=end]
    } else if let Some(idx) = host.rfind(':') {
        &host[..idx]
    } else {
        host
    }
}

/// Resolves a request's accepted peer address on both server transports.
///
/// Warp's normal server path stores the address in its private route state.
/// The custom Rustls/Hyper path cannot call Warp's private `call_with_addr`,
/// so it places the same address in the request extensions instead.
fn request_remote_addr()
-> impl warp::Filter<Extract = (Option<std::net::SocketAddr>,), Error = std::convert::Infallible> + Clone
{
    warp::addr::remote()
        .and(warp::ext::optional::<std::net::SocketAddr>())
        .map(
            |warp_addr: Option<std::net::SocketAddr>,
             extension_addr: Option<std::net::SocketAddr>| {
                warp_addr.or(extension_addr)
            },
        )
}

/// RAII guard for the interpreter's live recursion depth. Increments on
/// `enter`, decrements on drop (normal return *or* error unwind), so the depth
/// counter always reflects the real call nesting — independent of the
/// diagnostic `call_stack`, which may be force-cleared on a terminal timeout.
struct CallDepthGuard<'a> {
    cell: &'a Cell<usize>,
}

impl<'a> CallDepthGuard<'a> {
    fn enter(cell: &'a Cell<usize>) -> Self {
        cell.set(cell.get() + 1);
        Self { cell }
    }
}

impl Drop for CallDepthGuard<'_> {
    fn drop(&mut self) {
        self.cell.set(self.cell.get().saturating_sub(1));
    }
}

/// A snapshot of the interpreter's per-execution "run state" — the mutable
/// bookkeeping that belongs to a single in-flight execution rather than to the
/// interpreter as a whole: the count-loop variable, recursion depth, the
/// diagnostic call stack, and the current block's overload-dup set.
///
/// Under serial execution this state lives directly on `Interpreter` and is
/// never contended. Under `main loop concurrently:` several handler futures are
/// interleaved cooperatively on one thread, so at every `await` point one
/// handler's run state must not be visible to (or clobbered by) another. Each
/// handler owns a `RunState` that is swapped into the interpreter only while
/// that handler is actively being polled (see [`IsolatedHandler`]).
#[derive(Default)]
struct RunState {
    /// This handler's transaction scope (see [`Interpreter::tx_scope`]).
    tx_scope: u64,
    current_count: Option<f64>,
    in_count_loop: bool,
    call_depth: usize,
    call_stack: Vec<CallFrame>,
    /// Static-method environments active in this handler. Like `call_stack`,
    /// these contexts may remain live across an `.await`, so concurrent
    /// handlers must park them independently between polls.
    active_static_method_contexts: Vec<Rc<StaticMethodContext>>,
    block_overload_dups: Option<Rc<std::collections::HashSet<String>>>,
    /// Server response streams opened by this handler and not yet explicitly
    /// closed. Closed automatically when the handler ends on any path (see
    /// `IsolatedHandler`'s `Drop` and the serial main loop's per-iteration
    /// drain), so a handler that forgets `close out` never hangs the client.
    open_response_streams: Vec<String>,
    /// Request ids this handler dequeued (`wait for request comes in`) and has
    /// not yet answered. If the handler ends on any path without responding, each
    /// is answered 500 immediately instead of leaving the client to wait out the
    /// request timeout (see `fail_unanswered_requests`).
    open_pending_requests: Vec<String>,
    /// Outbound streaming-response handle ids (`... stream response as <name>`)
    /// this handler opened and has not yet closed/exhausted. Handler-OWNED: when
    /// the handler ends on any path (normal, error, panic, cancellation, loop
    /// exit) these are dropped from `IoClient.stream_handles`, which cancels the
    /// in-flight upstream request — so an abandoned proxy read never leaks an
    /// upstream connection or handle past the handler's lifetime.
    open_http_streams: StreamOwner,
    /// Sticky: this handler successfully dequeued at least one request via
    /// `wait for request`. Used by the concurrent main loop to distinguish
    /// structural pre-request failures (feed the consecutive-failure breaker)
    /// from request-local outcomes (must never tear the server down because one
    /// accepted request failed). Survives respond clearing `open_pending_requests`.
    accepted_request: bool,
    /// This handler's `execute file ... and read output` capture stack. The
    /// capture routing itself is a thread-local (see `io_capture`), so it is
    /// swapped — not just saved — per poll: while this handler runs, the
    /// thread-local holds this stack; while it is suspended, the ambient stack
    /// is restored so a sibling's output cannot land in this handler's capture
    /// buffer (#642). Starts as a clone of the ambient stack so handler output
    /// still reaches an enclosing capture.
    capture_stack: Vec<Rc<RefCell<String>>>,
    /// This handler's module-resolution base (`current_source_file`). Swapped
    /// per poll so a sibling parked mid-`include` cannot make this handler
    /// resolve relative module paths against the sibling's module directory
    /// (#642). Starts as the calling context's source file.
    current_source_file: Option<PathBuf>,
    /// This handler's in-flight module-load stack (circular-dependency
    /// detection and import-depth accounting). Swapped per poll so a sibling's
    /// in-flight load of the same module is not misreported as a cycle (#642).
    /// Starts as a clone of the calling context's stack, so a handler
    /// re-including a module the enclosing context is still loading is still
    /// (correctly) a cycle.
    loading_stack: Vec<PathBuf>,
}

#[cfg(test)]
impl RunState {
    /// Build a fresh run state seeded with the given call depth; every other
    /// scratch field starts empty/default. Used by the concurrent-handler unit
    /// tests to stand up an isolated [`RunState`] without a live handler.
    fn fresh(call_depth: usize) -> Self {
        RunState {
            call_depth,
            ..RunState::default()
        }
    }
}

/// Installs one handler's parked [`RunState`] in the interpreter until drop.
///
/// Besides reducing duplicated swap calls, the guard makes restoration
/// unwind-safe if polling or dropping an inner future panics.
struct InstalledRunState<'a> {
    interp: &'a Interpreter,
    parked: &'a mut RunState,
}

impl<'a> InstalledRunState<'a> {
    fn new(interp: &'a Interpreter, parked: &'a mut RunState) -> Self {
        interp.swap_run_state(parked);
        interp.refresh_active_static_method_context();
        Self { interp, parked }
    }
}

impl Drop for InstalledRunState<'_> {
    fn drop(&mut self) {
        // A static method may suspend after mutating its lexical property
        // mirror. Publish those changes before another handler is polled, then
        // refresh from shared state when this handler is installed again. This
        // prevents two interleaved handlers from later writing back stale,
        // full-property snapshots over each other.
        self.interp.persist_active_static_method_context();
        self.interp.swap_run_state(self.parked);
    }
}

/// Wraps a handler future so its [`RunState`] is swapped into the interpreter
/// for the duration of each `poll` and swapped back out again the instant the
/// poll returns (ready **or** pending). This makes the interpreter's run-state
/// fields effectively poll-local: while handler A is suspended at an `await`,
/// its count-loop variable, recursion depth, and call stack are parked in A's
/// own `RunState`, so handler B — polled next — neither sees nor corrupts them.
///
/// `inner` is a boxed handler future (already wrapped in `catch_unwind`); a
/// panic therefore surfaces as `Poll::Ready` and the swap-back still runs,
/// leaving the interpreter's scratch fields restored for the next sibling.
///
/// After the handler future completes, any still-open server response streams
/// are drained **asynchronously** (drop sender + await transport finish) before
/// this wrapper reports `Ready`. Without that step, a concurrent handler that
/// streams without an explicit `close out` and then `break`s / `return`s would
/// only nowait-close in [`Drop`], racing runtime teardown the same way the
/// serial path did before #680. Cancellation (dropping this wrapper mid-flight)
/// still falls back to sync nowait close.
///
/// The wrapper's output is `(inner_output, accepted_request)` so the concurrent
/// loop can classify request-local vs structural failures after the handler's
/// run state has been swapped out (and its pending list drained by `Drop`).
struct IsolatedHandler<'a, T> {
    interp: &'a Interpreter,
    state: RunState,
    /// Kept in an `Option` so cancellation can explicitly drop the suspended
    /// future while this handler's state is installed. Any RAII guards inside
    /// the future then unwind against their own call/static-context stacks.
    inner: Option<std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a>>>,
    /// After `inner` completes: await body finish for streams the handler left
    /// open. Polled without installing `RunState` (ids are already taken).
    drain: Option<std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>>>,
    /// Output of `inner`, held until `drain` completes. Boxed so this wrapper
    /// stays `Unpin` regardless of `T` (required for `Pin::get_mut` in `poll`).
    pending_output: Option<Box<(T, bool)>>,
    /// Stream ids currently being drained (or waiting to start). Drop falls
    /// back to nowait-close for these if the wrapper is cancelled mid-drain.
    pending_finish_ids: Vec<String>,
}

impl<'a, T> std::future::Future for IsolatedHandler<'a, T> {
    type Output = (T, bool);

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<(T, bool)> {
        // Every field is `Unpin` (`&`, `RunState`, and `Pin<Box<..>>`), so the
        // wrapper itself is `Unpin` and `get_mut` is sound.
        let this = self.get_mut();

        // Phase 2: finish any open response bodies before reporting Ready so a
        // concurrent Break/Exit/Return cannot tear down the runtime mid-flush.
        if this.drain.is_some() {
            match this
                .drain
                .as_mut()
                .expect("drain present")
                .as_mut()
                .poll(cx)
            {
                std::task::Poll::Ready(()) => {
                    this.drain = None;
                    this.pending_finish_ids.clear();
                    return std::task::Poll::Ready(
                        *this
                            .pending_output
                            .take()
                            .expect("drained handler must retain its output"),
                    );
                }
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }

        // Phase 1: run the handler body with its RunState installed.
        let completed = {
            let _installed = InstalledRunState::new(this.interp, &mut this.state);
            let inner = this
                .inner
                .as_mut()
                .expect("isolated handler must not be polled after its inner future is dropped");
            match inner.as_mut().poll(cx) {
                std::task::Poll::Pending => None,
                std::task::Poll::Ready(value) => {
                    // Capture while installed: `accepted_request` and open stream
                    // ids live on the interpreter for this poll. Taking the ids
                    // clears the list so swap-out parks an empty vec (Drop will
                    // not double-close them via `state.open_response_streams`).
                    let accepted = this.interp.accepted_request.get();
                    let stream_ids =
                        std::mem::take(&mut *this.interp.open_response_streams.borrow_mut());
                    Some((value, accepted, stream_ids))
                }
            }
        };

        let Some((value, accepted, stream_ids)) = completed else {
            return std::task::Poll::Pending;
        };

        // Drop the completed inner future with RunState installed so any RAII
        // guards still live at the top frame unwind against this handler.
        if let Some(inner) = this.inner.take() {
            let _installed = InstalledRunState::new(this.interp, &mut this.state);
            drop(inner);
        }

        if stream_ids.is_empty() {
            return std::task::Poll::Ready((value, accepted));
        }

        this.pending_output = Some(Box::new((value, accepted)));
        this.pending_finish_ids = stream_ids.clone();
        let interp = this.interp;
        this.drain = Some(Box::pin(async move {
            interp.close_response_streams(&stream_ids).await;
        }));
        match this
            .drain
            .as_mut()
            .expect("drain just set")
            .as_mut()
            .poll(cx)
        {
            std::task::Poll::Ready(()) => {
                this.drain = None;
                this.pending_finish_ids.clear();
                std::task::Poll::Ready(
                    *this
                        .pending_output
                        .take()
                        .expect("drained handler must retain its output"),
                )
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl<'a, T> Drop for IsolatedHandler<'a, T> {
    fn drop(&mut self) {
        // Drop the handler future FIRST, with this handler's run state
        // installed: a future dropped while suspended still runs the `Drop` of
        // every live RAII guard inside it (call-depth, capture, module-load,
        // and `StaticMethodCallScope`), and those guards must unwind against
        // the handler's own state, not the ambient context installed at
        // teardown (#642). The `InstalledRunState` guard also persists this
        // handler's static-method contexts on swap-out.
        if let Some(inner) = self.inner.take() {
            let _installed = InstalledRunState::new(self.interp, &mut self.state);
            drop(inner);
        }
        // Abandon an in-flight finish wait (cancellation path).
        self.drain.take();

        // The handler is finished (normal return, error, panic contained by
        // `catch_unwind`, or cancellation as the loop tears down). After the
        // final swap-out, `state` holds any streams it opened but never
        // closed and any requests it dequeued but never answered. Close the
        // streams (finalizing the client's body) and 500 the unanswered requests,
        // so every exit path resolves the client instead of leaving it hanging.
        //
        // Sync nowait: Drop cannot await. Normal completion already awaited
        // finish via `drain` and cleared these lists; this path covers
        // cancellation and any streams still open when the wrapper is dropped
        // mid-drain (senders not yet removed) or never drained.
        let mut ids = std::mem::take(&mut self.state.open_response_streams);
        ids.append(&mut self.pending_finish_ids);
        self.interp.close_response_streams_nowait(&ids);
        self.interp
            .fail_unanswered_requests(&self.state.open_pending_requests);
        // Drop any outbound streams this handler still owns, cancelling their
        // in-flight upstream requests so an abandoned proxy read never leaks.
        self.interp
            .close_http_streams(&self.state.open_http_streams);
    }
}

/// RAII guard that finalizes the interpreter's still-open server-request state
/// when the running `interpret()` future ends — crucially including when that
/// future is *dropped* (an embedder cancels the run) before the normal
/// handler-exit / program-cleanup sites execute. It shares the interpreter's
/// tracking lists and maps via `Rc`, so its `Drop` runs even as the interpreter
/// itself stays alive (e.g. a reused REPL). It covers, for the top-level/serial
/// context whose state lives directly on the interpreter (concurrent handlers
/// finalize their own via `IsolatedHandler`'s `Drop`):
///
/// - **outbound streams** — dropped from `IoClient.stream_handles`, cancelling
///   the in-flight upstream request;
/// - **server response streams** — dropped from `server_response_streams`, ending
///   the client's body so a `start streaming response` that never `close`d does
///   not leave the connection hanging;
/// - **pending requests** — answered 500 so a dequeued-but-unanswered request is
///   resolved instead of waiting out the request timeout.
///
/// On a normal run the cleanup sites have already drained the lists, so this is a
/// no-op. All work is synchronous and best-effort (`try_lock` never blocks in a
/// `Drop`).
struct OutboundStreamCleanup {
    io_client: Rc<IoClient>,
    open_http_streams: StreamOwner,
    open_response_streams: Rc<RefCell<Vec<String>>>,
    server_response_streams: Rc<RefCell<HashMap<String, ServerResponseStream>>>,
    open_pending_requests: Rc<RefCell<Vec<String>>>,
    pending_responses: Rc<RefCell<HashMap<String, PendingResponse>>>,
}

impl Drop for OutboundStreamCleanup {
    fn drop(&mut self) {
        // Outbound upstream streams: removing a handle drops its reqwest stream,
        // cancelling the in-flight upstream request.
        self.io_client.close_stream_owner(&self.open_http_streams);

        // Server response streams: dropping the sender ends the client's body.
        let stream_ids = std::mem::take(&mut *self.open_response_streams.borrow_mut());
        if !stream_ids.is_empty() {
            let mut map = self.server_response_streams.borrow_mut();
            for id in &stream_ids {
                map.remove(id);
            }
        }

        // Pending requests dequeued but never answered: resolve each with 500 so
        // the client is not left waiting out its request timeout. Mirrors
        // `fail_unanswered_requests`, but operates on the shared `Rc` maps so it
        // runs even when the interpreter's own methods are unreachable (future
        // dropped). The sender mutex is only held during `respond`, which a
        // dropped run is no longer inside, so `try_lock` succeeds.
        let request_ids = std::mem::take(&mut *self.open_pending_requests.borrow_mut());
        if !request_ids.is_empty() {
            let mut pending = self.pending_responses.borrow_mut();
            for id in &request_ids {
                if let Some(entry) = pending.remove(id)
                    && let Ok(mut guard) = entry.sender.try_lock()
                    && let Some(sender) = guard.take()
                {
                    let _ = sender.send(HandlerReply::Buffered(WflHttpResponse {
                        content: b"Internal Server Error\n".to_vec(),
                        status: 500,
                        content_type: "text/plain; charset=utf-8".to_string(),
                        headers: HashMap::new(),
                    }));
                }
            }
        }
    }
}

/// RAII guard that ensures module loading context is restored on scope exit.
/// Automatically removes its loading_stack entry and restores
/// current_source_file when dropped.
///
/// Restoration is identity-checked rather than a blind pop/overwrite: under
/// `main loop concurrently:` the loading context is handler-local (swapped per
/// poll), so a guard that drops while its handler's context is parked (the
/// handler future was dropped mid-suspend) sees the *ambient* context and must
/// not clobber it (#642).
struct ModuleLoadGuard<'a> {
    interpreter: &'a Interpreter,
    module_path: PathBuf,
    previous_source: Option<PathBuf>,
    should_restore: bool,
}

impl<'a> ModuleLoadGuard<'a> {
    fn new(
        interpreter: &'a Interpreter,
        module_path: PathBuf,
        previous_source: Option<PathBuf>,
    ) -> Self {
        interpreter
            .loading_stack
            .borrow_mut()
            .push(module_path.clone());
        *interpreter.current_source_file.borrow_mut() = Some(module_path.clone());

        Self {
            interpreter,
            module_path,
            previous_source,
            should_restore: true,
        }
    }

    /// Get the current loading chain before cleanup (for error reporting)
    fn get_chain(&self) -> Vec<String> {
        self.interpreter
            .loading_stack
            .borrow()
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    }
}

impl<'a> Drop for ModuleLoadGuard<'a> {
    fn drop(&mut self) {
        if self.should_restore {
            {
                let mut current = self.interpreter.current_source_file.borrow_mut();
                if current.as_ref() == Some(&self.module_path) {
                    *current = self.previous_source.clone();
                }
            }
            let mut stack = self.interpreter.loading_stack.borrow_mut();
            if let Some(pos) = stack.iter().rposition(|p| p == &self.module_path) {
                stack.remove(pos);
            }
        }
    }
}

// Helper functions for execution logging
#[cfg(debug_assertions)]
fn stmt_type(stmt: &Statement) -> String {
    match stmt {
        Statement::VariableDeclaration { name, .. } => format!("VariableDeclaration '{name}'"),
        Statement::Assignment { name, .. } => format!("Assignment to '{name}'"),
        Statement::IfStatement { .. } => "IfStatement".to_string(),
        Statement::SingleLineIf { .. } => "SingleLineIf".to_string(),
        Statement::DisplayStatement { .. } => "DisplayStatement".to_string(),
        Statement::ActionDefinition { name, .. } => format!("ActionDefinition '{name}'"),
        Statement::ReturnStatement { .. } => "ReturnStatement".to_string(),
        Statement::ExpressionStatement { .. } => "ExpressionStatement".to_string(),
        Statement::CountLoop { .. } => "CountLoop".to_string(),
        Statement::ForEachLoop { item_name, .. } => format!("ForEachLoop '{item_name}'"),
        Statement::WhileLoop { .. } => "WhileLoop".to_string(),
        Statement::RepeatUntilLoop { .. } => "RepeatUntilLoop".to_string(),
        Statement::RepeatWhileLoop { .. } => "RepeatWhileLoop".to_string(),
        Statement::ForeverLoop { .. } => "ForeverLoop".to_string(),
        Statement::MainLoop { .. } => "MainLoop".to_string(),
        Statement::BreakStatement { .. } => "BreakStatement".to_string(),
        Statement::ContinueStatement { .. } => "ContinueStatement".to_string(),
        Statement::ExitStatement { .. } => "ExitStatement".to_string(),
        Statement::OpenFileStatement { variable_name, .. } => {
            format!("OpenFileStatement '{variable_name}'")
        }
        Statement::ReadFileStatement { variable_name, .. } => {
            format!("ReadFileStatement '{variable_name}'")
        }
        Statement::WriteFileStatement { .. } => "WriteFileStatement".to_string(),
        Statement::WriteToStatement { .. } => "WriteToStatement".to_string(),
        Statement::WriteContentStatement { .. } => "WriteContentStatement".to_string(),
        Statement::WriteBinaryStatement { .. } => "WriteBinaryStatement".to_string(),
        Statement::CloseFileStatement { .. } => "CloseFileStatement".to_string(),
        Statement::OpenDatabaseStatement { variable_name, .. } => {
            format!("OpenDatabaseStatement '{variable_name}'")
        }
        Statement::DatabaseQueryStatement { variable_name, .. } => {
            format!("DatabaseQueryStatement '{variable_name}'")
        }
        Statement::CloseDatabaseStatement { .. } => "CloseDatabaseStatement".to_string(),
        Statement::TransactionStatement { .. } => "TransactionStatement".to_string(),
        Statement::CreateDirectoryStatement { .. } => "CreateDirectoryStatement".to_string(),
        Statement::CreateFileStatement { .. } => "CreateFileStatement".to_string(),
        Statement::DeleteFileStatement { .. } => "DeleteFileStatement".to_string(),
        Statement::DeleteDirectoryStatement { .. } => "DeleteDirectoryStatement".to_string(),
        Statement::LoadModuleStatement { path, .. } => {
            format!("LoadModuleStatement from '{:?}'", path)
        }
        Statement::IncludeStatement { path, .. } => {
            format!("IncludeStatement from '{:?}'", path)
        }
        Statement::ExportStatement {
            export_type, name, ..
        } => {
            format!("ExportStatement {:?} {}", export_type, name)
        }
        Statement::ExecuteCommandStatement { variable_name, .. } => {
            if let Some(var) = variable_name {
                format!("ExecuteCommandStatement '{var}'")
            } else {
                "ExecuteCommandStatement".to_string()
            }
        }
        Statement::ExecuteFileStatement { variable_name, .. } => {
            if let Some(var) = variable_name {
                format!("ExecuteFileStatement '{var}'")
            } else {
                "ExecuteFileStatement".to_string()
            }
        }
        Statement::SpawnProcessStatement { variable_name, .. } => {
            format!("SpawnProcessStatement '{variable_name}'")
        }
        Statement::ReadProcessOutputStatement { variable_name, .. } => {
            format!("ReadProcessOutputStatement '{variable_name}'")
        }
        Statement::KillProcessStatement { .. } => "KillProcessStatement".to_string(),
        Statement::WaitForProcessStatement { variable_name, .. } => {
            if let Some(var) = variable_name {
                format!("WaitForProcessStatement '{var}'")
            } else {
                "WaitForProcessStatement".to_string()
            }
        }
        Statement::WaitForStatement { .. } => "WaitForStatement".to_string(),
        Statement::WaitForDurationStatement { .. } => "WaitForDurationStatement".to_string(),
        Statement::TryStatement { .. } => "TryStatement".to_string(),
        Statement::HttpGetStatement { variable_name, .. } => {
            format!("HttpGetStatement '{variable_name}'")
        }
        Statement::HttpPostStatement { variable_name, .. } => {
            format!("HttpPostStatement '{variable_name}'")
        }
        Statement::HttpRequestStatement { variable_name, .. } => {
            format!("HttpRequestStatement '{variable_name}'")
        }
        Statement::HttpStreamStatement { variable_name, .. } => {
            format!("HttpStreamStatement '{variable_name}'")
        }
        Statement::WaitForNextChunkStatement { variable_name, .. } => {
            format!("WaitForNextChunkStatement '{variable_name}'")
        }
        Statement::WaitForNextLineStatement { variable_name, .. } => {
            format!("WaitForNextLineStatement '{variable_name}'")
        }
        Statement::StartStreamingResponseStatement { variable_name, .. } => {
            format!("StartStreamingResponseStatement '{variable_name}'")
        }
        Statement::StreamWriteStatement { is_line, .. } => {
            format!("StreamWriteStatement (line={is_line})")
        }
        Statement::FlushStreamStatement { .. } => "FlushStreamStatement".to_string(),
        Statement::PushStatement { .. } => "PushStatement to list".to_string(),
        Statement::CreateListStatement { name, .. } => format!("CreateListStatement '{name}'"),
        Statement::MapCreation { name, .. } => format!("MapCreation '{name}'"),
        Statement::CreateDateStatement { name, .. } => format!("CreateDateStatement '{name}'"),
        Statement::CreateTimeStatement { name, .. } => format!("CreateTimeStatement '{name}'"),
        Statement::AddToListStatement { list_name, .. } => {
            format!("AddToListStatement to '{list_name}'")
        }
        Statement::RemoveFromListStatement { list_name, .. } => {
            format!("RemoveFromListStatement from '{list_name}'")
        }
        Statement::ClearListStatement { list_name, .. } => {
            format!("ClearListStatement '{list_name}'")
        }
        // Container-related statements
        Statement::ContainerDefinition { name, .. } => format!("ContainerDefinition '{name}'"),
        Statement::ContainerInstantiation {
            container_type,
            instance_name,
            ..
        } => format!("ContainerInstantiation '{container_type}' as '{instance_name}'"),
        Statement::InterfaceDefinition { name, .. } => format!("InterfaceDefinition '{name}'"),
        Statement::EventDefinition { name, .. } => format!("EventDefinition '{name}'"),
        Statement::EventTrigger { name, .. } => format!("EventTrigger '{name}'"),
        Statement::EventHandler { event_name, .. } => format!("EventHandler '{event_name}'"),
        Statement::ParentMethodCall { method_name, .. } => {
            format!("ParentMethodCall '{method_name}'")
        }
        Statement::PatternDefinition { name, .. } => {
            format!("PatternDefinition '{name}'")
        }
        Statement::ListenStatement { server_name, .. } => {
            format!("ListenStatement '{server_name}'")
        }
        Statement::WaitForRequestStatement { request_name, .. } => {
            format!("WaitForRequestStatement '{request_name}'")
        }
        Statement::RespondStatement { .. } => "RespondStatement".to_string(),
        Statement::RegisterSignalHandlerStatement {
            signal_type,
            handler_name,
            ..
        } => {
            format!(
                "RegisterSignalHandlerStatement '{}' -> '{}'",
                signal_type, handler_name
            )
        }
        Statement::StopAcceptingConnectionsStatement { .. } => {
            "StopAcceptingConnectionsStatement".to_string()
        }
        Statement::CloseServerStatement { .. } => "CloseServerStatement".to_string(),
        Statement::ListenWebSocketStatement { server_name, .. } => {
            format!("ListenWebSocketStatement '{server_name}'")
        }
        Statement::WebSocketHandlerStatement { event, .. } => {
            format!("WebSocketHandlerStatement '{}'", event.as_str())
        }
        Statement::SendWebSocketMessageStatement { .. } => {
            "SendWebSocketMessageStatement".to_string()
        }
        Statement::BroadcastWebSocketMessageStatement { .. } => {
            "BroadcastWebSocketMessageStatement".to_string()
        }
        // Test framework statements
        Statement::DescribeBlock { description, .. } => {
            format!("DescribeBlock '{description}'")
        }
        Statement::TestBlock { description, .. } => {
            format!("TestBlock '{description}'")
        }
        Statement::ExpectStatement { .. } => "ExpectStatement".to_string(),
        Statement::ConfigureSessionsStatement { .. } => "ConfigureSessionsStatement".to_string(),
        Statement::EnableCsrfProtectionStatement { .. } => {
            "EnableCsrfProtectionStatement".to_string()
        }
        Statement::EnableSecureCookiesStatement { .. } => {
            "EnableSecureCookiesStatement".to_string()
        }
        Statement::SetSessionValueStatement { .. } => "SetSessionValueStatement".to_string(),
        Statement::DestroySessionStatement { .. } => "DestroySessionStatement".to_string(),
        Statement::StoreSessionDataStatement { .. } => "StoreSessionDataStatement".to_string(),
        Statement::DeleteSessionDataStatement { .. } => "DeleteSessionDataStatement".to_string(),
    }
}

#[cfg(debug_assertions)]
fn expr_type(expr: &Expression) -> String {
    match expr {
        Expression::Literal(lit, ..) => match lit {
            Literal::String(s) => format!("StringLiteral \"{s}\""),
            Literal::Integer(i) => format!("IntegerLiteral {i}"),
            Literal::Float(f) => format!("FloatLiteral {f}"),
            Literal::Boolean(b) => format!("BooleanLiteral {b}"),
            Literal::Nothing => "NullLiteral".to_string(),
            Literal::Pattern(p) => format!("PatternLiteral \"{p}\""),
            Literal::List(_) => "ListLiteral".to_string(),
        },
        Expression::Variable(name, ..) => format!("Variable '{name}'"),
        Expression::BinaryOperation { operator, .. } => format!("BinaryOperation '{operator:?}'"),
        Expression::UnaryOperation { operator, .. } => format!("UnaryOperation '{operator:?}'"),
        Expression::FunctionCall { function, .. } => match function.as_ref() {
            Expression::Variable(name, ..) => format!("FunctionCall '{name}'"),
            _ => "FunctionCall".to_string(),
        },
        Expression::ActionCall { name, .. } => format!("ActionCall '{name}'"),
        Expression::MemberAccess { property, .. } => format!("MemberAccess '{property}'"),
        Expression::IndexAccess { .. } => "IndexAccess".to_string(),
        Expression::Concatenation { .. } => "Concatenation".to_string(),
        Expression::PatternMatch { .. } => "PatternMatch".to_string(),
        Expression::PatternFind { .. } => "PatternFind".to_string(),
        Expression::PatternReplace { .. } => "PatternReplace".to_string(),
        Expression::PatternSplit { .. } => "PatternSplit".to_string(),
        Expression::StringSplit { .. } => "StringSplit".to_string(),
        Expression::AwaitExpression { .. } => "AwaitExpression".to_string(),
        // Container-related expressions
        Expression::StaticMemberAccess {
            container, member, ..
        } => format!("StaticMemberAccess '{container}' member '{member}'"),
        Expression::MethodCall { method, .. } => format!("MethodCall '{method}'"),
        Expression::PropertyAccess { property, .. } => format!("PropertyAccess '{property}'"),
        Expression::FileExists { .. } => "FileExists".to_string(),
        Expression::DirectoryExists { .. } => "DirectoryExists".to_string(),
        Expression::ListFiles { .. } => "ListFiles".to_string(),
        Expression::ReadContent { .. } => "ReadContent".to_string(),
        Expression::ReadBinaryContent { .. } => "ReadBinaryContent".to_string(),
        Expression::ReadBinaryN { .. } => "ReadBinaryN".to_string(),
        Expression::FileSizeOf { .. } => "FileSizeOf".to_string(),
        Expression::ListFilesRecursive { .. } => "ListFilesRecursive".to_string(),
        Expression::ListFilesFiltered { .. } => "ListFilesFiltered".to_string(),
        Expression::HeaderAccess { header_name, .. } => format!("HeaderAccess '{header_name}'"),
        Expression::CurrentTimeMilliseconds { .. } => "CurrentTimeMilliseconds".to_string(),
        Expression::CurrentTimeFormatted { format, .. } => {
            format!("CurrentTimeFormatted '{format}'")
        }
        Expression::ProcessRunning { .. } => "ProcessRunning".to_string(),
        Expression::DatabaseQuery { .. } => "DatabaseQuery".to_string(),
        Expression::CreateSession { .. } => "CreateSession".to_string(),
        Expression::GetSession { .. } => "GetSession".to_string(),
        Expression::GetSessionValue { .. } => "GetSessionValue".to_string(),
        Expression::GenerateCsrfTokenForSession { .. } => "GenerateCsrfTokenForSession".to_string(),
        Expression::FindExpiredSessions { .. } => "FindExpiredSessions".to_string(),
        Expression::GetSessionStatistics { .. } => "GetSessionStatistics".to_string(),
        Expression::LoadSessionData { .. } => "LoadSessionData".to_string(),
    }
}

use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::Mutex;
// use self::value::FutureValue;

/// The WFL tree-walking interpreter.
///
/// # Stack safety (embedders)
///
/// The interpreter recurses through several async frames per WFL call, so deep
/// WFL recursion is stack-heavy: an ordinary 8 MiB thread stack overflows near
/// depth ~40 in debug builds. The budget's `max_call_depth` only turns runaway
/// recursion into a clean, catchable `ResourceLimit` error when the stack is
/// large enough to *reach* that limit first.
///
/// The default public path is therefore **safe by default**:
/// [`Interpreter::new`] — which promises nothing about the thread stack it runs
/// on — caps recursion at the conservative [`Interpreter::DEFAULT_EMBED_CALL_DEPTH`]
/// (not the config-file default of 1000), so a deep WFL program returns a
/// catchable depth error instead of aborting the host process on an ordinary
/// stack. To recurse deeper, opt into both a higher `max_call_depth` (via
/// [`Interpreter::with_config`] / [`Interpreter::with_config_and_budget`]) **and**
/// the large stack from [`crate::run_with_interpreter_stack`] — the combination
/// the WFL CLI uses to reach the full configured 1000. Those config-taking
/// constructors honor the caller's `max_call_depth` verbatim precisely so the
/// CLI (and any embedder that has arranged the stack) can raise it.
pub struct Interpreter {
    global_env: Rc<RefCell<Environment>>,
    current_count: RefCell<Option<f64>>,
    in_count_loop: RefCell<bool>,
    // Main-loop state (deadline exemption) now lives on the shared budget as a
    // depth counter with an RAII guard (see `ExecutionBudget::enter_main_loop`),
    // so it restores on every exit and nests correctly across `execute file`.
    /// Monotonic per-statement counter driving the cooperative-yield stride.
    /// Increments on every executed statement regardless of the deadline
    /// exemption (unlike the operation counter, which a `main loop` skips), so a
    /// CPU-bound `main loop` body still yields to the runtime and lets a
    /// `select!` deliver cooperative cancellation (the REPL's Ctrl-C).
    sched_counter: Cell<u64>,
    /// The single shared execution budget: deadline/cancellation, operation
    /// ceiling, recursion/import/execute-file depth, pattern steps/states, byte
    /// caps, pending-request and WebSocket limits. Held behind `Arc` so the
    /// multi-threaded web transport (warp accept tasks, per-connection
    /// WebSocket tasks) can read it without any `Rc`/`RefCell` crossing a
    /// thread boundary. Replaces the old `op_count`/`started`/`max_duration`
    /// fields and the scattered per-subsystem constants.
    budget: Arc<ExecutionBudget>,
    /// Action names defined more than once in the *currently executing*
    /// statement block (scanned at block entry), so even the *first*
    /// definition of an overloaded name enforces its declared parameter
    /// types during the window before the later definitions execute. Scoped
    /// per block — a sibling block's lone definition of the same name keeps
    /// legacy dynamic-call leniency. `None` when the current block overloads
    /// nothing (the common case; no allocation). Overloads the block scan
    /// cannot see (includes, cross-block merges) are flagged at merge time
    /// by `Environment::define_or_merge_action`.
    current_block_overload_dups: RefCell<Option<Rc<std::collections::HashSet<String>>>>,
    call_stack: RefCell<Vec<CallFrame>>,
    /// Live recursion depth for enforcement, kept **separate** from `call_stack`
    /// (which is diagnostic and gets force-cleared on a terminal timeout). A
    /// dedicated RAII counter means a caught `ResourceLimit` can never leave the
    /// enforcement depth under-counted, so catch-and-recurse stays bounded.
    call_depth: Cell<usize>,
    /// Identifies the execution context ("who") that owns a transaction.
    ///
    /// Under `main loop concurrently:` every handler runs against the *same*
    /// `IoClient` and can name the same global database handle. Keying open
    /// transactions by handle alone would mean one handler's ordinary query
    /// silently joined another handler's transaction — and got committed or
    /// rolled back by it. Transactions are therefore keyed by (scope, handle).
    ///
    /// Swapped per poll by [`InstalledRunState`], exactly like the other
    /// handler-local run state, so the value read here always belongs to the
    /// handler currently executing. `0` is the serial/top-level scope.
    tx_scope: Cell<u64>,
    /// Source of unique [`Self::tx_scope`] ids for concurrent handlers.
    next_tx_scope: Cell<u64>,
    /// Static methods execute against lexical environments that mirror shared
    /// container properties. This stack synchronizes those mirrors at nested
    /// call boundaries so a re-entrant static call sees mutations made by its
    /// caller and the caller resumes with mutations made by its callee.
    active_static_method_contexts: RefCell<Vec<Rc<StaticMethodContext>>>,
    /// The depth `call_depth` resets to at the start of a run. Normally 0, but a
    /// child interpreter spawned by `execute file` inherits the parent's live
    /// depth here, so recursion accounting *spans* the execute-file boundary: a
    /// parent already near `max_call_depth` cannot run a child that consumes
    /// another full allowance and multiplies the native stack toward overflow.
    base_call_depth: usize,
    #[allow(dead_code)]
    io_client: Rc<IoClient>,
    step_mode: bool,          // Controls single-step execution mode
    script_args: Vec<String>, // Command-line arguments passed to the script
    web_servers: RefCell<HashMap<String, WflWebServer>>, // Web servers by name
    web_socket_servers: RefCell<HashMap<String, WflWebSocketServer>>, // WebSocket servers keyed by address
    ws_connections: WsConnectionRegistry, // Outbound senders for all live WebSocket connections
    pending_responses: Rc<RefCell<HashMap<String, PendingResponse>>>, // Pending responses (channel + admission slot) by request ID
    /// Open server response streams (`start streaming response`), keyed by
    /// handle id ("respstream1", ...). Each holds the bounded body-chunk sender,
    /// the running total of body bytes written (enforced against
    /// `max_response_bytes`), and a oneshot the transport fires when it has
    /// finished the body. `write line|chunk`/`flush` push chunks; `close`
    /// drops the sender and awaits that finish signal (#680).
    server_response_streams: Rc<RefCell<HashMap<String, ServerResponseStream>>>,
    next_response_stream_id: std::cell::Cell<usize>,
    /// Handle ids of server response streams opened by the currently executing
    /// handler and not yet explicitly closed. Part of the per-handler `RunState`
    /// (swapped in/out per poll under `main loop concurrently:`) so each handler
    /// tracks only its own streams; drained and closed when the handler ends on
    /// any path so the client's body is always finalized (see
    /// `close_response_streams`).
    /// `Rc<RefCell<..>>` (not a bare `RefCell`) so the `interpret()`-scoped
    /// cleanup guard can share it and finalize any still-open server response
    /// bodies if the future is dropped before its normal exit sites run (see
    /// `OutboundStreamCleanup`).
    open_response_streams: Rc<RefCell<Vec<String>>>,
    /// Request ids the currently executing handler dequeued but has not yet
    /// answered. Part of the per-handler `RunState` (swapped per poll) so each
    /// handler tracks only its own requests; any still unanswered when the handler
    /// ends are answered 500 immediately (see `fail_unanswered_requests`).
    /// `Rc<RefCell<..>>` (not a bare `RefCell`) so the `interpret()`-scoped
    /// cleanup guard can share it and 500 any still-unanswered requests if the
    /// future is dropped before its normal exit sites run (see
    /// `OutboundStreamCleanup`).
    open_pending_requests: Rc<RefCell<Vec<String>>>,
    /// Outbound stream handle ids (`... stream response as <name>`) the currently
    /// executing handler opened and has not yet closed/exhausted. Part of the
    /// per-handler `RunState` (swapped per poll); any still open when the handler
    /// ends are dropped, cancelling their upstream requests (see
    /// `close_http_streams`).
    /// `Rc<RefCell<..>>` (not a bare `RefCell`) so an RAII cleanup guard tied to
    /// the `interpret()` future can share the list and close these handles if the
    /// future is dropped/cancelled before its normal exit sites run (see
    /// `OutboundStreamCleanup`).
    open_http_streams: Rc<RefCell<StreamOwner>>,
    /// Sticky per-handler flag: at least one request was dequeued and parked.
    /// Part of `RunState` (swapped per poll); see `RunState::accepted_request`.
    accepted_request: Cell<bool>,
    #[allow(dead_code)] // Used for future security features
    config: Arc<WflConfig>, // Configuration for security and other settings
    current_source_file: RefCell<Option<PathBuf>>, // Currently executing source file (for path resolution)
    loading_stack: RefCell<Vec<PathBuf>>, // Stack of currently loading files (for circular dependency detection)
    execute_depth: usize, // Nesting depth of `execute file` runs (guards against circular execution)
    // Test execution state
    test_mode: RefCell<bool>,
    test_results: RefCell<TestResults>,
    current_describe_stack: RefCell<Vec<String>>,
    current_test_name: RefCell<Option<String>>,
}

/// Synchronizes one static-method property environment with the shared
/// container definitions for the full lifetime of its call.
///
/// Keeping this as an RAII scope is important: dropping an in-flight
/// interpretation future must not strand an active context on the stack or
/// discard mutations that completed before cancellation.
struct StaticMethodCallScope<'a> {
    active_contexts: &'a RefCell<Vec<Rc<StaticMethodContext>>>,
    context: Rc<StaticMethodContext>,
}

impl<'a> StaticMethodCallScope<'a> {
    fn enter(interpreter: &'a Interpreter, context: Rc<StaticMethodContext>) -> Self {
        interpreter.persist_active_static_method_context();
        Interpreter::refresh_static_method_context(&context);
        interpreter
            .active_static_method_contexts
            .borrow_mut()
            .push(Rc::clone(&context));
        Self {
            active_contexts: &interpreter.active_static_method_contexts,
            context,
        }
    }
}

impl Drop for StaticMethodCallScope<'_> {
    fn drop(&mut self) {
        Interpreter::persist_static_method_context(&self.context);

        let parent_context = {
            let mut active_contexts = self.active_contexts.borrow_mut();
            let popped = active_contexts.pop();
            debug_assert!(
                popped
                    .as_ref()
                    .is_some_and(|context| Rc::ptr_eq(context, &self.context)),
                "static method contexts must unwind in call order"
            );
            active_contexts.last().cloned()
        };

        if let Some(parent_context) = parent_context {
            Interpreter::refresh_static_method_context(&parent_context);
        }
    }
}

// Test framework data structures
#[derive(Debug, Default, Clone)]
pub struct TestResults {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub describe_context: Vec<String>,
    pub test_name: String,
    pub assertion_message: String,
    pub line: usize,
    pub column: usize,
}

/// RAII scope for [`Interpreter::current_block_overload_dups`]: installs the
/// executing block's overload-duplicate set and restores the enclosing
/// block's on drop — surviving `?` early returns and `.await` points.
struct BlockDupsScope<'a> {
    slot: &'a RefCell<Option<Rc<std::collections::HashSet<String>>>>,
    prev: Option<Rc<std::collections::HashSet<String>>>,
}

impl<'a> BlockDupsScope<'a> {
    fn enter(
        slot: &'a RefCell<Option<Rc<std::collections::HashSet<String>>>>,
        current: Option<Rc<std::collections::HashSet<String>>>,
    ) -> Self {
        let prev = slot.replace(current);
        Self { slot, prev }
    }
}

impl Drop for BlockDupsScope<'_> {
    fn drop(&mut self) {
        *self.slot.borrow_mut() = self.prev.take();
    }
}

/// Members speculatively armed at block entry (`enforce_param_types` set
/// because a later definition in the block will merge with them). If the
/// merge never executes — the run errors or returns first — the promise is
/// broken and Drop restores each member's prior flag, so a still-single
/// action keeps its legacy dynamic-call behavior in later runs (a REPL
/// interpreter is reused across snippets). A member that did merge is part
/// of an overload set by drop time and keeps enforcement. Entries restore
/// in reverse order so re-armed duplicates unwind to the oldest prior flag.
struct ArmedMember {
    env: Rc<RefCell<Environment>>,
    name: String,
    member: Rc<FunctionValue>,
    prior_enforce: bool,
}

struct ArmedEnforcementGuard {
    armed: Vec<ArmedMember>,
}

impl Drop for ArmedEnforcementGuard {
    fn drop(&mut self) {
        for entry in self.armed.drain(..).rev() {
            let merged = matches!(
                entry.env.borrow().get_local(&entry.name),
                Some(Value::Overloaded(set))
                    if set.overloads.iter().any(|m| Rc::ptr_eq(m, &entry.member))
            );
            if !merged {
                entry.member.enforce_param_types.set(entry.prior_enforce);
            }
        }
    }
}

// Process handle for managing subprocess state
#[allow(dead_code)]
pub struct ProcessHandle {
    child: tokio::process::Child,
    command: String,
    args: Vec<String>,
    started_at: Instant,
    completed_at: Option<Instant>,
    exit_code: Option<i32>,
    stdout_buffer: Arc<tokio::sync::Mutex<bounded_buffer::BoundedBuffer>>,
    stderr_buffer: Arc<tokio::sync::Mutex<bounded_buffer::BoundedBuffer>>,
}

/// Failure from a foreground `execute command`. Budget breaches stay typed so
/// the interpreter can preserve timeout/resource-limit error kinds instead of
/// flattening them into a generic subprocess error.
#[derive(Debug)]
enum ExecuteCommandError {
    Budget(BudgetExceeded),
    Timeout { seconds: u64 },
    Other(String),
}

impl std::fmt::Display for ExecuteCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Budget(exceeded) => std::fmt::Display::fmt(exceeded, formatter),
            Self::Timeout { seconds } => {
                write!(
                    formatter,
                    "Subprocess execution exceeded timeout ({seconds}s)"
                )
            }
            Self::Other(message) => formatter.write_str(message),
        }
    }
}

/// Bytes retained from one subprocess stream plus the amount discarded after
/// the configured per-stream ceiling was reached.
struct CapturedProcessStream {
    bytes: Vec<u8>,
    bytes_dropped: usize,
}

/// Abort detached pipe readers on every non-success path, including external
/// cancellation that drops the whole `execute_command` future before its
/// explicit interruption branch can run.
struct ProcessCaptureAbortGuard {
    handles: [tokio::task::AbortHandle; 2],
    armed: bool,
}

impl ProcessCaptureAbortGuard {
    fn new(stdout: tokio::task::AbortHandle, stderr: tokio::task::AbortHandle) -> Self {
        Self {
            handles: [stdout, stderr],
            armed: true,
        }
    }

    fn abort(&mut self) {
        if self.armed {
            for handle in &self.handles {
                handle.abort();
            }
            self.armed = false;
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ProcessCaptureAbortGuard {
    fn drop(&mut self) {
        self.abort();
    }
}

/// Which wall-clock rule applies to one foreground command. Long-lived
/// `main loop`s remain exempt from the run-wide deadline, but each blocking
/// subprocess operation still receives a fresh configured timeout.
#[derive(Debug, Clone, Copy)]
enum ForegroundCommandDeadline {
    None,
    Execution,
    MainLoop { started: Instant, timeout: Duration },
}

const SUBPROCESS_BUDGET_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Drain one subprocess pipe without ever retaining more than `max_size`
/// bytes. Keeping the most recent bytes matches background-process capture.
async fn capture_process_stream<R>(
    mut stream: R,
    max_size: usize,
) -> io::Result<CapturedProcessStream>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buffer = bounded_buffer::BoundedBuffer::new(max_size);
    let mut chunk = [0_u8; 8192];

    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.push(&chunk[..read]);
    }

    let bytes_dropped = buffer.stats().bytes_dropped;
    Ok(CapturedProcessStream {
        bytes: buffer.read_all(),
        bytes_dropped,
    })
}

/// Select the finite wall-clock rule for one foreground execution and reject a
/// launch when its shared budget is already expired or cancelled.
fn foreground_command_deadline(
    budget: &ExecutionBudget,
    configured_timeout: Duration,
) -> Result<ForegroundCommandDeadline, ExecuteCommandError> {
    budget
        .check_cancelled()
        .map_err(ExecuteCommandError::Budget)?;

    if budget.is_deadline_exempt() {
        return Ok(ForegroundCommandDeadline::MainLoop {
            started: Instant::now(),
            timeout: configured_timeout,
        });
    }

    budget
        .check_deadline()
        .map_err(ExecuteCommandError::Budget)?;
    if budget.limits().max_duration.is_some() {
        Ok(ForegroundCommandDeadline::Execution)
    } else {
        Ok(ForegroundCommandDeadline::None)
    }
}

/// Wait until cancellation or the applicable deadline interrupts the complete
/// foreground operation. This monitor remains active after the direct child
/// exits because descendants can inherit its pipes and withhold EOF forever.
async fn foreground_command_interrupt(
    budget: &ExecutionBudget,
    deadline: ForegroundCommandDeadline,
) -> ExecuteCommandError {
    loop {
        if let Err(exceeded) = budget.check_cancelled() {
            return ExecuteCommandError::Budget(exceeded);
        }

        match deadline {
            ForegroundCommandDeadline::None => {}
            ForegroundCommandDeadline::Execution => {
                if let Err(exceeded) = budget.check_deadline() {
                    return ExecuteCommandError::Budget(exceeded);
                }
            }
            ForegroundCommandDeadline::MainLoop { started, timeout } => {
                if started.elapsed() >= timeout {
                    return ExecuteCommandError::Timeout {
                        seconds: timeout.as_secs(),
                    };
                }
            }
        }

        tokio::time::sleep(SUBPROCESS_BUDGET_POLL_INTERVAL).await;
    }
}

/// Kill and reap the direct child unless it has already completed. A second
/// status check handles the normal race where it exits between inspection and
/// the kill request.
async fn terminate_foreground_child(child: &mut tokio::process::Child) -> Result<(), String> {
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(error) => return Err(format!("failed to inspect subprocess: {error}")),
    }

    match child.kill().await {
        Ok(()) => Ok(()),
        Err(kill_error) => match child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err(format!("failed to terminate subprocess: {kill_error}")),
            Err(wait_error) => Err(format!(
                "failed to terminate subprocess: {kill_error}; status check failed: {wait_error}"
            )),
        },
    }
}

#[allow(dead_code)]
/// An open transaction behind its own lock, so the transaction map's lock is
/// never held across SQL I/O. `None` marks a handle reserved by an in-flight
/// `begin` (see [`IoClient::db_transactions`]).
type SharedTransaction = Arc<Mutex<Option<database::DbTransaction>>>;

pub struct IoClient {
    /// Shared outbound HTTP client, built on first use.
    ///
    /// Deliberately lazy: `reqwest::Client::new()` panics when the TLS builder
    /// fails, and the builder fails on any system whose trust store holds no CA
    /// certificates (a minimal container image, for instance). Building it in
    /// [`IoClient::new`] therefore aborted the whole process at interpreter
    /// start-up on such a system, even for a program that never makes a
    /// request. See [`IoClient::http_client`].
    http_client: OnceCell<reqwest::Client>,
    file_handles: Mutex<HashMap<String, (PathBuf, tokio::fs::File)>>,
    next_file_id: Mutex<usize>,
    process_handles: Mutex<HashMap<String, ProcessHandle>>,
    next_process_id: Mutex<usize>,
    db_handles: Mutex<HashMap<String, database::DbPool>>,
    next_db_id: Mutex<usize>,
    /// Transactions currently open, keyed by the same handle id as
    /// `db_handles`. While an entry is present, every `query`/`execute` on that
    /// handle runs on the transaction's pinned connection instead of taking a
    /// fresh one from the pool — which is the whole point of issue #664.
    ///
    /// Each transaction lives behind its **own** lock rather than inside the
    /// map's. Statements within one transaction are necessarily serial (a
    /// transaction owns a single connection), but holding the map's lock across
    /// the SQL await would extend that to every database operation in the
    /// program, so one slow statement would stall unrelated handles. Callers
    /// therefore lock the map only long enough to clone the `Arc`.
    ///
    /// Keyed by `(scope, handle)` rather than handle alone: see
    /// [`Interpreter::tx_scope`]. Two concurrent handlers naming the same global
    /// database handle therefore get separate entries, and a handler with no
    /// transaction of its own takes the pool as it always did.
    ///
    /// The slot is `Option` so a handle can be *reserved* under the map lock
    /// before the `begin` round-trip completes: two concurrent begins on one
    /// handle would otherwise both pass a `contains_key` check and the second
    /// would silently replace — and drop — the first transaction.
    db_transactions: Mutex<HashMap<(u64, String), SharedTransaction>>,
    /// Live outbound streaming response bodies, keyed by handle id
    /// ("httpstream1", ...). See [`StreamSlot`] / [`HttpStreamHandle`].
    ///
    /// Uses a **std** mutex so Drop/cleanup paths can lock reliably (tokio's
    /// async mutex only offers `try_lock` from sync Drop, which previously
    /// abandoned handles when the map was briefly held). Critical sections are
    /// short (no `.await` while held).
    stream_handles: Arc<std::sync::Mutex<StreamRegistry>>,
    next_stream_id: Mutex<usize>,
    /// Test-only live-task accounting. The production build carries no
    /// instrumentation; unit tests retain a runtime and assert that closing a
    /// stream actually drops its sleeping reaper instead of relying on runtime
    /// shutdown to hide leaked timers.
    #[cfg(test)]
    active_stream_reapers: Arc<std::sync::atomic::AtomicUsize>,
    config: Arc<WflConfig>,
}

#[cfg(test)]
struct ActiveStreamReaperGuard {
    active: Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl ActiveStreamReaperGuard {
    fn new(active: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        active.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self { active }
    }
}

#[cfg(test)]
impl Drop for ActiveStreamReaperGuard {
    fn drop(&mut self) {
        self.active
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Hard ceiling on `outbound_stream_max_seconds` when converting to an
/// `Instant` deadline. Extreme `u64` values must not panic
/// `Instant::now() + Duration::from_secs(secs)` (which can overflow).
const MAX_OUTBOUND_STREAM_DEADLINE_SECS: u64 = 365 * 24 * 60 * 60; // 1 year

/// Compute the absolute stream deadline from a configured second cap.
/// `0` is the documented sentinel for "no absolute total cap". Values above
/// [`MAX_OUTBOUND_STREAM_DEADLINE_SECS`] are clamped before `Instant`
/// arithmetic so even an extreme configuration remains finite and cannot
/// panic.
fn outbound_stream_deadline(secs: u64) -> Option<Instant> {
    let effective = outbound_stream_effective_seconds(secs)?;
    Instant::now().checked_add(Duration::from_secs(effective))
}

fn outbound_stream_effective_seconds(secs: u64) -> Option<u64> {
    (secs != 0).then(|| secs.min(MAX_OUTBOUND_STREAM_DEADLINE_SECS))
}

/// Shared per-stream cancellation: close, expire, and EOF all trip this so an
/// active body read can select against it and drop the upstream promptly
/// (rather than only noticing when `put_stream` finds a missing slot).
struct StreamCancel {
    terminal: tokio::sync::watch::Sender<Option<StreamTerminal>>,
}

impl StreamCancel {
    fn new() -> Arc<Self> {
        let (terminal, _receiver) = tokio::sync::watch::channel(None);
        Arc::new(Self { terminal })
    }

    fn terminal(&self) -> Option<StreamTerminal> {
        *self.terminal.borrow()
    }

    fn terminate(&self, reason: StreamTerminal) -> StreamTerminal {
        self.terminal.send_if_modified(|terminal| {
            if terminal.is_none() {
                *terminal = Some(reason);
                true
            } else {
                false
            }
        });
        self.terminal()
            .expect("stream terminal reason must be set after terminate")
    }

    fn subscribe(&self) -> tokio::sync::watch::Receiver<Option<StreamTerminal>> {
        self.terminal.subscribe()
    }
}

/// Why a stream slot was terminated. Active readers observe the shared
/// first-wins signal; clean EOF and unread expiry are retained briefly in the
/// bounded recent queue so the next read can consume the terminal outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamTerminal {
    CleanEof,
    Timeout,
    Closed,
}

type StreamOwner = Arc<std::sync::Mutex<HashSet<String>>>;

/// Keep a short, bounded window of terminal outcomes after removing the live
/// body. This preserves one clean-EOF read or a typed timeout without retaining
/// the request body, cancel channel, owner, or sleeping task.
const MAX_RECENT_STREAM_TERMINALS: usize = 64;
const RECENT_STREAM_TERMINAL_TTL: Duration = Duration::from_secs(60);

#[derive(Default)]
struct StreamRegistry {
    live: HashMap<String, StreamSlot>,
    recent: VecDeque<RecentStreamTerminal>,
}

struct RecentStreamTerminal {
    id: String,
    reason: StreamTerminal,
    expires_at: Instant,
}

impl StreamRegistry {
    fn prune_recent(&mut self, now: Instant) {
        while self
            .recent
            .front()
            .is_some_and(|entry| entry.expires_at <= now)
        {
            self.recent.pop_front();
        }
    }

    fn remember_recent(&mut self, id: String, reason: StreamTerminal, now: Instant) {
        self.prune_recent(now);
        self.recent.retain(|entry| entry.id != id);
        while self.recent.len() >= MAX_RECENT_STREAM_TERMINALS {
            self.recent.pop_front();
        }
        self.recent.push_back(RecentStreamTerminal {
            id,
            reason,
            expires_at: now + RECENT_STREAM_TERMINAL_TTL,
        });
    }

    fn take_recent(&mut self, id: &str, now: Instant) -> Option<StreamTerminal> {
        self.prune_recent(now);
        let index = self.recent.iter().position(|entry| entry.id == id)?;
        self.recent.remove(index).map(|entry| entry.reason)
    }

    fn forget_recent(&mut self, id: &str) -> bool {
        let before = self.recent.len();
        self.recent.retain(|entry| entry.id != id);
        self.recent.len() != before
    }
}

/// Per-handle shared lifecycle for an outbound stream.
///
/// Reads take the inner [`HttpStreamHandle`] out for the duration of the await
/// (so the global map lock is not held across the network). Explicit
/// finish/close removes the slot after signalling [`StreamCancel`]; expiry
/// drops the parked body, removes the live slot, and records a bounded recent
/// terminal so mid-read work aborts without losing the reason. Clean EOF uses
/// the same queue for its one follow-up read.
struct StreamSlot {
    /// The live body handle. `None` while a body read owns it.
    handle: Option<HttpStreamHandle>,
    /// Absolute deadline for the whole stream (`None` = no absolute total cap).
    deadline: Option<Instant>,
    /// Shared cancel signal for active-read races.
    cancel: Arc<StreamCancel>,
    /// Abort handle for the reaper timer. Cancelled on EOF, error, or explicit
    /// close so rapid open/close cycles do not accumulate sleeping tasks.
    reaper_abort: Option<tokio::task::AbortHandle>,
    /// Handler ownership is stored with the live slot so the reaper can remove
    /// the id immediately. Recent terminal records never retain an owner.
    owner: Option<StreamOwner>,
}

fn remove_stream_owner(slot: &mut StreamSlot, handle_id: &str) {
    if let Some(owner) = slot.owner.take() {
        owner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(handle_id);
    }
}

/// A live, parked outbound streaming response body.
///
/// The status and headers were already handed to the WFL program by
/// `open url ... and stream response as <name>`; this holds the still-open body
/// stream so `wait for next chunk|line` can pull it incrementally without ever
/// buffering the whole body. Dropping the handle — on clean EOF, a mid-stream
/// error, an explicit `close`, or interpreter teardown — drops the underlying
/// reqwest body future and thereby cancels the in-flight upstream request.
struct HttpStreamHandle {
    /// The response body as a stream of raw byte chunks. `Vec<u8>` (not
    /// `bytes::Bytes`) so the boxed trait object stays nameable here.
    stream: std::pin::Pin<Box<dyn futures_util::Stream<Item = reqwest::Result<Vec<u8>>> + Send>>,
    /// Bytes read from the network but not yet handed to the program: the
    /// remainder after a line split, plus bytes accumulated while scanning for
    /// the next newline. Bounded by the run's `max_response_bytes` because
    /// every byte placed here was counted against that ceiling as it was read.
    buffer: Vec<u8>,
    /// True once the underlying stream has yielded a clean end of stream.
    done: bool,
    /// Total body bytes pulled from the network so far, enforced against
    /// `max_response_bytes`.
    bytes_read: usize,
    /// Absolute deadline for the whole stream, set from
    /// `outbound_stream_max_seconds` at open time. Distinct from the per-read
    /// idle timeout: an upstream that trickles a byte before every idle timeout
    /// still cannot run past this. `None` disables the total cap.
    total_deadline: Option<Instant>,
}

/// A body read in progress: the handle plus a cancel watch shared with close/reaper.
struct TakenStream {
    handle: HttpStreamHandle,
    cancel: Arc<StreamCancel>,
}

/// Errors raised while an outbound HTTP request is in flight.
///
/// Budget failures stay structured until the interpreter can attach source
/// location and the appropriate [`ErrorKind`]. Keeping them out of strings is
/// important for `try`/`when` handlers that distinguish timeouts from resource
/// limits.
#[derive(Debug)]
enum HttpClientError {
    Request(String),
    Budget(BudgetExceeded),
    Timeout {
        seconds: u64,
    },
    /// The stream was explicitly closed (or finished) while a body read was in
    /// flight. Distinct from absolute-lifetime [`Self::Timeout`].
    Closed,
    /// The downstream (browser) client disconnected while a proxy handler was
    /// blocked on this upstream read, so the read was cancelled cooperatively.
    /// A normal, expected event — distinct from a fault — surfaced with
    /// `ErrorKind::Cancelled` so the concurrent loop does not count it as a
    /// handler failure.
    Disconnected,
}

impl From<BudgetExceeded> for HttpClientError {
    fn from(exceeded: BudgetExceeded) -> Self {
        Self::Budget(exceeded)
    }
}

/// Which finite wall-clock limit applies to an outbound request.
#[derive(Debug, Clone, Copy)]
enum OutboundHttpDeadline {
    /// An explicitly-unlimited non-server run has no wall-clock deadline.
    None,
    /// Outside a `main loop`, an outbound request consumes the run's remaining
    /// global execution time.
    Execution {
        remaining: Duration,
        limit_secs: u64,
    },
    /// A `main loop` is lifetime-exempt, but each individual outbound request
    /// still gets a fresh finite timeout so a stalled peer cannot wedge the
    /// server forever.
    MainLoop { duration: Duration },
}

/// Polling is used because cooperative cancellation is represented by an
/// atomic flag rather than a notification primitive. This interval bounds how
/// quickly an in-flight socket operation observes `ExecutionBudget::cancel()`.
const HTTP_CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// How often a blocked upstream operation polls its handler's pending requests
/// for a client disconnect (the transport drops the oneshot receiver). Polled
/// because the sender lives behind an `Arc<Mutex<Option<..>>>` shared with the
/// transport rather than an awaitable primitive; small so a disconnect is
/// observed promptly.
const REQUEST_DISCONNECT_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// How long an idle keep-alive connection may sit in the outbound pool before
/// it is dropped instead of reused.
///
/// reqwest's own default is 90 seconds, which is longer than most peers keep an
/// idle connection alive: Node closes at 5 seconds, and proxies commonly sit
/// between 5 and 15. Reusing a socket the peer has already closed turns the
/// next request into a spurious failure, so the pool is kept well under that
/// floor. It does not *close* the race — the peer can still close between the
/// pool's check and the write, which is what [`IoClient::send_with_reuse_retry`]
/// is for — it just makes the race rare instead of routine. Connections are
/// still reused for back-to-back requests, where the win actually is.
const HTTP_POOL_IDLE_TIMEOUT_SECONDS: u64 = 3;

/// Total attempts (initial send + retries) for an outbound request that never
/// reached a response head. See [`IoClient::send_with_reuse_retry`].
const HTTP_SEND_MAX_ATTEMPTS: u32 = 2;

/// Headers with which a caller states that repeating an outbound request is
/// safe: the upstream deduplicates on the key, so a second delivery of the same
/// request is not a second side effect. `Idempotency-Key` is the name the
/// payment and API ecosystem settled on; the `X-` form is accepted because
/// plenty of services still publish it. The promise is in the *value*, so only a
/// non-empty one counts: an empty key gives the upstream nothing to deduplicate
/// on, and is usually an unset variable rather than a deliberate opt-in.
const HTTP_IDEMPOTENCY_KEY_HEADERS: [&str; 2] = ["idempotency-key", "x-idempotency-key"];

/// Whether re-sending `request` after a failure that produced no response head
/// can only ever duplicate work the upstream is prepared for.
///
/// `reqwest::Error::is_request()` establishes that the *caller* saw no response.
/// It does not establish that the upstream never received the request: hyper
/// reports the same error for a socket the peer closed before reading anything
/// and for one it closed after reading — and possibly committing — the body, and
/// the two are indistinguishable from the client side. Replaying the second case
/// would deliver a non-idempotent request twice, duplicating a charge or a write
/// the program never asked to repeat, so replay requires a request that carries
/// a promise it is safe:
///
/// * an idempotent method (RFC 9110 §9.2.2), where repeating the request is
///   defined to have the same effect on the server as issuing it once, or
/// * an explicit idempotency-key header with a non-empty value, the caller's own
///   statement that the upstream collapses retries of this exact request.
///
/// Every other request surfaces the send failure instead. A bare `POST` is not
/// left unprotected: [`HTTP_POOL_IDLE_TIMEOUT_SECONDS`] keeps a connection from
/// being reused once it has been idle long enough for the peer to have dropped
/// it, which is the case that made these failures routine.
fn request_may_be_replayed(request: &reqwest::Request) -> bool {
    if request.method().is_idempotent() {
        return true;
    }

    HTTP_IDEMPOTENCY_KEY_HEADERS.iter().any(|name| {
        request
            .headers()
            .get(*name)
            .is_some_and(|value| !value.is_empty())
    })
}

#[derive(Debug)]
enum FileReadError {
    Io(String),
    Budget(BudgetExceeded),
}

impl std::fmt::Display for FileReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => f.write_str(message),
            Self::Budget(exceeded) => std::fmt::Display::fmt(exceeded, f),
        }
    }
}

/// Buffer one file read under the run's file-byte ceiling. Reading through a
/// `Take` capped at `limit + 1` is intentional: metadata is not reliable for
/// special files and streams (for example `/dev/zero`), so the limit must be
/// enforced while bytes arrive rather than after an unbounded `read_to_end`.
async fn read_to_end_capped<R>(
    reader: &mut R,
    budget: &ExecutionBudget,
    operation: &str,
) -> Result<Vec<u8>, FileReadError>
where
    R: AsyncRead + Unpin,
{
    let limit = budget.max_file_read_bytes();
    let probe_size = limit.saturating_add(1);
    let probe_size_u64 = u64::try_from(probe_size).unwrap_or(u64::MAX);
    let mut bytes = Vec::with_capacity(limit.min(8 * 1024));
    let mut bounded = reader.take(probe_size_u64);
    bounded
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| FileReadError::Io(format!("{operation}: {e}")))?;

    budget
        .check_file_read_bytes(bytes.len())
        .map_err(FileReadError::Budget)?;
    Ok(bytes)
}

impl IoClient {
    /// The shared outbound HTTP client, built on first use.
    ///
    /// `reqwest::Client::new()` unwraps the builder, so it *panics* rather than
    /// returning an error when no CA certificates can be loaded from the system
    /// trust store. Constructing it eagerly meant a machine with an empty trust
    /// store could not run **any** WFL program — the interpreter aborted before
    /// the first statement, whether or not the program touched the network.
    /// Building it here instead keeps start-up independent of the trust store,
    /// and turns a genuinely unusable TLS stack into an ordinary WFL runtime
    /// error raised at the request that needs it.
    fn http_client(&self) -> Result<&reqwest::Client, HttpClientError> {
        self.http_client.get_or_try_init(|| {
            reqwest::Client::builder()
                // Defence in depth against reusing a socket the peer has
                // already closed: see HTTP_POOL_IDLE_TIMEOUT_SECONDS.
                .pool_idle_timeout(Duration::from_secs(HTTP_POOL_IDLE_TIMEOUT_SECONDS))
                .build()
                .map_err(|error| {
                    HttpClientError::Request(format!("Could not create HTTP client: {error}"))
                })
        })
    }

    /// Send a request, re-sending it once on a fresh connection if the first
    /// attempt never reached a response head.
    ///
    /// Keep-alive connections are pooled, and the peer may close an idle one at
    /// any moment — Node closes at 5 seconds by default, many proxies between 5
    /// and 15. When a request is written to a socket the peer has already
    /// closed, hyper reports a send failure and does *not* replay it: replaying
    /// a non-idempotent request is not hyper's call to make. Left alone, that
    /// surfaces to the WFL program as a hard error for a request the server
    /// never even saw — an intermittent failure that looks exactly like the
    /// upstream being down, and that gets more likely the longer a program idles
    /// between calls.
    ///
    /// A re-send needs *both* of two conditions, because either one alone would
    /// duplicate work:
    ///
    /// 1. The connection was lost — see [`Self::connection_was_lost`]. A peer
    ///    that answered, even with something unparseable, has demonstrably
    ///    handled the request, so replaying it could duplicate whatever it did
    ///    with the first copy. Only a connection that died before any of the
    ///    response arrived leaves the caller nothing to have acted on.
    /// 2. Repeating the request is safe — see [`request_may_be_replayed`]. A
    ///    lost connection says nothing about whether the *upstream* acted: it can
    ///    also be lost after a server read and committed the body, and nothing on
    ///    this side can tell that apart from a socket the peer abandoned before
    ///    the request arrived. So the replay is restricted to requests that carry
    ///    a promise they can be repeated — an idempotent method, or a non-empty
    ///    idempotency-key header — and a bare `POST` relies on the pool idle
    ///    timeout above instead.
    ///
    /// The retry is deliberately bounded at one: a peer that is genuinely gone
    /// must surface as an error promptly rather than being replayed in a loop. A
    /// body that cannot be cloned (a streaming body) is never retried, since it
    /// cannot be re-sent faithfully.
    ///
    /// Both send sites use this, so the buffered and streaming paths recover
    /// identically. The whole thing runs inside the caller's timeout/budget
    /// wrapper, so the retry cannot extend a request past its deadline.
    ///
    /// The future is boxed rather than left inline, which is a stack-size
    /// decision, not a style one. Holding a request builder, its replay clone,
    /// and an in-flight `send()` inline would embed all of that in the async
    /// state machine of every statement that can make a request — and a
    /// `execute file` chain nests one such state machine per level. Measured on
    /// the self-executing-file depth guard, the inline form cost roughly 150 KiB
    /// of stack headroom (survived a 2048 KiB thread stack, overflowed at 1920
    /// KiB, where the same test survived 1920 KiB before this change). One heap
    /// allocation per outbound request is not measurable next to a network
    /// round trip; the stack headroom is worth keeping.
    fn send_with_reuse_retry<'a>(
        request: reqwest::RequestBuilder,
        method: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<reqwest::Response, HttpClientError>> + Send + 'a>> {
        // Decided once, before anything goes out: whether this request may be
        // repeated at all. Building a clone is also what proves the body can be
        // reproduced faithfully, so a streaming body drops out here too.
        let replayable = request
            .try_clone()
            .and_then(|probe| probe.build().ok())
            .is_some_and(|probe| request_may_be_replayed(&probe));

        Box::pin(async move {
            let mut attempt = request;
            let mut attempts_left = HTTP_SEND_MAX_ATTEMPTS;

            loop {
                attempts_left -= 1;
                let replay = if replayable && attempts_left > 0 {
                    attempt.try_clone()
                } else {
                    None
                };

                match attempt.send().await {
                    Ok(response) => return Ok(response),
                    Err(error) => match replay {
                        // The connection died with nothing to observe, and this
                        // request is safe to repeat: try once more on a
                        // connection that is known to be fresh.
                        Some(retry) if Self::connection_was_lost(&error) => attempt = retry,
                        _ => {
                            return Err(HttpClientError::Request(format!(
                                "Failed to send HTTP {method} request: {error}"
                            )));
                        }
                    },
                }
            }
        })
    }

    /// Did this send failure mean the connection died before any of the response
    /// arrived — as opposed to the peer answering, or never being reachable at
    /// all?
    ///
    /// This is one half of the safety argument for a re-send (the other is
    /// [`request_may_be_replayed`]), so it tests the specific evidence rather
    /// than the general shape of the failure. `reqwest::Error::is_request()` is
    /// far too broad: it
    /// also covers a peer that replied with something unparseable (which handled
    /// the request) and a peer that refused the connection (which cannot be
    /// fixed by trying again immediately). The two signals that do mean the
    /// connection was lost mid-request:
    ///
    /// * `hyper::Error::is_incomplete_message` — the connection closed before
    ///   the response head, which is exactly what a peer's expired keep-alive
    ///   socket produces;
    /// * an I/O error whose kind says the socket was reset, aborted, broken, or
    ///   read to an unexpected end — the same event seen at the syscall layer,
    ///   which is what some platforms surface instead.
    ///
    /// Anything else — a parse failure, a refused connection, a TLS error — is
    /// reported to the program as it stands.
    fn connection_was_lost(error: &reqwest::Error) -> bool {
        use std::io::ErrorKind as IoErrorKind;

        let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(error);
        while let Some(current) = cause {
            if let Some(hyper_error) = current.downcast_ref::<hyper::Error>()
                && hyper_error.is_incomplete_message()
            {
                return true;
            }
            if let Some(io_error) = current.downcast_ref::<io::Error>()
                && matches!(
                    io_error.kind(),
                    IoErrorKind::ConnectionReset
                        | IoErrorKind::ConnectionAborted
                        | IoErrorKind::BrokenPipe
                        | IoErrorKind::NotConnected
                        | IoErrorKind::UnexpectedEof
                )
            {
                return true;
            }
            cause = current.source();
        }
        false
    }

    fn new(config: Arc<WflConfig>) -> Self {
        Self {
            http_client: OnceCell::new(),
            file_handles: Mutex::new(HashMap::new()),
            next_file_id: Mutex::new(1),
            process_handles: Mutex::new(HashMap::new()),
            next_process_id: Mutex::new(1),
            db_handles: Mutex::new(HashMap::new()),
            next_db_id: Mutex::new(1),
            db_transactions: Mutex::new(HashMap::new()),
            stream_handles: Arc::new(std::sync::Mutex::new(StreamRegistry::default())),
            next_stream_id: Mutex::new(1),
            #[cfg(test)]
            active_stream_reapers: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            config,
        }
    }

    /// Open a database connection pool and return its WFL handle ("db1", ...).
    async fn open_database(&self, url: &str) -> Result<String, String> {
        let pool = database::connect(url).await?;

        let handle_id = {
            let mut next_id = self.next_db_id.lock().await;
            let id = format!("db{}", *next_id);
            *next_id += 1;
            id
        };

        self.db_handles.lock().await.insert(handle_id.clone(), pool);
        Ok(handle_id)
    }

    /// Look up a database pool by handle; pools are cheap to clone (Arc inside).
    async fn get_database(&self, handle_id: &str) -> Result<database::DbPool, String> {
        self.db_handles
            .lock()
            .await
            .get(handle_id)
            .cloned()
            .ok_or_else(|| format!("Invalid or closed database handle: {handle_id}"))
    }

    /// Close a database pool and drop its handle.
    ///
    /// Refuses while a transaction is open on the handle: closing the pool
    /// under an in-flight transaction would discard the work with no report of
    /// what happened to it.
    async fn close_database(&self, handle_id: &str) -> Result<(), String> {
        // Any scope's transaction blocks the close, not just the caller's.
        // Handles are plain text, so two concurrent handlers can name the same
        // one; checking only the caller's scope would let handler B close the
        // pool out from under handler A's live transaction — the exact outcome
        // this guard exists to prevent.
        if self
            .db_transactions
            .lock()
            .await
            .keys()
            .any(|(_, handle)| handle == handle_id)
        {
            return Err(format!(
                "Cannot close database '{handle_id}' while a transaction is open on it. \
                 Let the `in transaction on ...` block finish first — it commits at \
                 `end transaction`, or rolls back if something inside it fails."
            ));
        }

        let pool = self
            .db_handles
            .lock()
            .await
            .remove(handle_id)
            .ok_or_else(|| format!("Invalid or closed database handle: {handle_id}"))?;
        database::close(pool).await;
        Ok(())
    }

    /// Open a transaction on `handle_id`, pinning one pooled connection to it.
    async fn begin_transaction(&self, scope: u64, handle_id: &str) -> Result<(), String> {
        // Nesting would need savepoints, which are not implemented; say so
        // rather than quietly flattening the inner block into the outer one.
        // Reserve the handle and take the slot's lock *before* awaiting the
        // database, so a concurrent begin on the same handle loses the race
        // here rather than replacing a live transaction later.
        let slot: SharedTransaction = {
            let mut transactions = self.db_transactions.lock().await;
            if transactions.contains_key(&(scope, handle_id.to_string())) {
                return Err(format!(
                    "A transaction is already open on database '{handle_id}'. Transaction \
                     blocks cannot be nested on the same database — finish the current one \
                     before starting another."
                ));
            }
            let slot: SharedTransaction = Arc::new(Mutex::new(None));
            transactions.insert((scope, handle_id.to_string()), Arc::clone(&slot));
            slot
        };
        // Held across the begin, so a statement that arrives meanwhile waits for
        // the transaction rather than seeing an empty slot and taking the pool.
        let mut open = slot.lock().await;

        // The reservation must not outlive a failed begin, or the handle would
        // be stuck refusing every later transaction.
        let pool = match self.get_database(handle_id).await {
            Ok(pool) => pool,
            Err(err) => {
                self.db_transactions
                    .lock()
                    .await
                    .remove(&(scope, handle_id.to_string()));
                return Err(err);
            }
        };
        match database::begin(&pool).await {
            Ok(tx) => {
                *open = Some(tx);
                Ok(())
            }
            Err(err) => {
                self.db_transactions
                    .lock()
                    .await
                    .remove(&(scope, handle_id.to_string()));
                Err(err)
            }
        }
    }

    /// Commit the open transaction on `handle_id`.
    async fn commit_transaction(&self, scope: u64, handle_id: &str) -> Result<(), String> {
        let slot = self
            .db_transactions
            .lock()
            .await
            .remove(&(scope, handle_id.to_string()))
            .ok_or_else(|| format!("No transaction is open on database '{handle_id}'"))?;
        let tx = slot
            .lock()
            .await
            .take()
            .ok_or_else(|| format!("No transaction is open on database '{handle_id}'"))?;
        database::commit(tx).await
    }

    /// Roll back the open transaction on `handle_id`.
    ///
    /// A missing transaction is not an error here: this runs on the failure
    /// path, where the transaction may already have been resolved.
    async fn rollback_transaction(&self, scope: u64, handle_id: &str) -> Result<(), String> {
        let slot = self
            .db_transactions
            .lock()
            .await
            .remove(&(scope, handle_id.to_string()));
        let tx = match slot {
            Some(slot) => slot.lock().await.take(),
            None => None,
        };
        match tx {
            Some(tx) => database::rollback(tx).await,
            None => Ok(()),
        }
    }

    /// Run a row-returning statement, routed to the handle's open transaction
    /// when it has one.
    async fn db_query(
        &self,
        scope: u64,
        handle_id: &str,
        sql: &str,
        params: &[database::SqlParam],
    ) -> Result<Value, String> {
        // Lock the map only long enough to clone the handle's slot; the SQL
        // below is awaited under the slot's own lock, never the map's.
        let slot = self
            .db_transactions
            .lock()
            .await
            .get(&(scope, handle_id.to_string()))
            .cloned();
        if let Some(slot) = slot {
            let mut open = slot.lock().await;
            if let Some(tx) = open.as_mut() {
                return database::run_query(database::DbTarget::Transaction(tx), sql, params).await;
            }
        }
        let pool = self.get_database(handle_id).await?;
        database::run_query(database::DbTarget::Pool(&pool), sql, params).await
    }

    /// Run a non-returning statement, routed to the handle's open transaction
    /// when it has one.
    async fn db_execute(
        &self,
        scope: u64,
        handle_id: &str,
        sql: &str,
        params: &[database::SqlParam],
    ) -> Result<Value, String> {
        let slot = self
            .db_transactions
            .lock()
            .await
            .get(&(scope, handle_id.to_string()))
            .cloned();
        if let Some(slot) = slot {
            let mut open = slot.lock().await;
            if let Some(tx) = open.as_mut() {
                return database::run_execute(database::DbTarget::Transaction(tx), sql, params)
                    .await;
            }
        }
        let pool = self.get_database(handle_id).await?;
        database::run_execute(database::DbTarget::Pool(&pool), sql, params).await
    }

    #[allow(dead_code)]
    async fn http_get(
        &self,
        url: &str,
        budget: Arc<ExecutionBudget>,
    ) -> Result<String, HttpClientError> {
        let (_, _, body) = self
            .send_http_request(self.http_client()?.get(url), "GET", budget)
            .await?;
        Ok(body)
    }

    #[allow(dead_code)]
    async fn http_post(
        &self,
        url: &str,
        data: &str,
        budget: Arc<ExecutionBudget>,
    ) -> Result<String, HttpClientError> {
        let (_, _, body) = self
            .send_http_request(
                self.http_client()?.post(url).body(data.to_string()),
                "POST",
                budget,
            )
            .await?;
        Ok(body)
    }

    /// Perform an HTTP request with an arbitrary method, optional headers,
    /// and an optional body. Returns (status, response headers, body text).
    /// Non-2xx statuses are not errors: callers inspect the status themselves.
    async fn http_request(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
        budget: Arc<ExecutionBudget>,
    ) -> Result<(u16, Vec<(String, String)>, String), HttpClientError> {
        let parsed_method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| HttpClientError::Request(format!("Invalid HTTP method: {method}")))?;

        let mut request = self.http_client()?.request(parsed_method, url);
        for (name, value) in headers {
            request = request.header(name.as_str(), value.as_str());
        }
        if let Some(body) = body {
            request = request.body(body);
        }

        self.send_http_request(request, method, budget).await
    }

    /// Open a streaming outbound request: send it and return
    /// `(status, response headers, stream handle id)` as soon as the response
    /// head arrives, WITHOUT buffering the body. The body stays open behind the
    /// returned handle id for incremental `wait for next chunk|line` reads.
    ///
    /// The head phase (connect + headers) is bounded by the same finite
    /// deadline and cooperative-cancellation machinery as a buffered request;
    /// each later body read is bounded per-chunk in [`Self::stream_pull`].
    async fn open_http_stream(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
        budget: Arc<ExecutionBudget>,
    ) -> Result<(u16, Vec<(String, String)>, String), HttpClientError> {
        use futures_util::StreamExt;

        let parsed_method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| HttpClientError::Request(format!("Invalid HTTP method: {method}")))?;
        let mut request = self.http_client()?.request(parsed_method, url);
        for (name, value) in headers {
            request = request.header(name.as_str(), value.as_str());
        }
        if let Some(body) = body {
            request = request.body(body);
        }

        let method_owned = method.to_string();
        // Start the absolute total-lifetime clock at request initiation — BEFORE
        // `send()` — so connect + header time counts toward
        // `outbound_stream_max_seconds`, matching the documented "total lifetime
        // measured from when the stream is opened". The head phase below is then
        // bounded by the remaining time to that deadline as well as the idle
        // timeout, so a stalled connect/header handshake cannot outlive the total.
        // `outbound_stream_deadline` clamps extreme config values so Instant
        // arithmetic cannot panic.
        let total_deadline = outbound_stream_deadline(self.config.outbound_stream_max_seconds);
        let idle_timeout = Duration::from_secs(self.config.timeout_seconds.max(1));
        let configured_timeout = match total_deadline {
            Some(deadline) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(self.outbound_stream_timeout_error());
                }
                idle_timeout.min(remaining)
            }
            None => idle_timeout,
        };
        let op = async move { Self::send_with_reuse_retry(request, &method_owned).await };
        // Only the head is awaited here; dropping this future on
        // timeout/cancel aborts the connection cleanly.
        let response =
            Self::run_http_with_budget(Arc::clone(&budget), configured_timeout, op).await?;

        let status = response.status().as_u16();
        let response_headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_ascii_lowercase(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();

        // Reject an over-ceiling body up front when the length is advertised;
        // per-chunk reads enforce the same ceiling for chunked/unknown lengths.
        let max_response_bytes = budget.limits().max_response_bytes;
        if let Some(content_length) = response.content_length() {
            let max_as_u64 = u64::try_from(max_response_bytes).unwrap_or(u64::MAX);
            if content_length > max_as_u64 {
                return Err(HttpClientError::Budget(BudgetExceeded::ResponseBytes {
                    limit: max_response_bytes,
                    actual: usize::try_from(content_length).unwrap_or(usize::MAX),
                }));
            }
        }

        let stream = response
            .bytes_stream()
            .map(|chunk| chunk.map(|b| b.to_vec()));
        // `total_deadline` was started at request initiation above (before the
        // head was sent) so it covers connect/header time too.
        let handle = HttpStreamHandle {
            stream: Box::pin(stream),
            buffer: Vec::new(),
            done: false,
            bytes_read: 0,
            total_deadline,
        };
        let handle_id = {
            let mut next_id = self.next_stream_id.lock().await;
            let id = format!("httpstream{}", *next_id);
            *next_id += 1;
            id
        };

        // Insert the slot FIRST, then arm the reaper under the same lock so the
        // reaper can never fire before the slot exists (and so close/finish that
        // races with open still sees a consistent cancel handle).
        let cancel = StreamCancel::new();
        {
            let mut registry = self
                .stream_handles
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            registry.prune_recent(Instant::now());
            registry.live.insert(
                handle_id.clone(),
                StreamSlot {
                    handle: Some(handle),
                    deadline: total_deadline,
                    cancel: Arc::clone(&cancel),
                    reaper_abort: None,
                    owner: None,
                },
            );
            if let Some(deadline) = total_deadline {
                let handles = Arc::clone(&self.stream_handles);
                let reap_id = handle_id.clone();
                let cancel_reap = Arc::clone(&cancel);
                #[cfg(test)]
                let reaper_guard =
                    ActiveStreamReaperGuard::new(Arc::clone(&self.active_stream_reapers));
                let join = tokio::spawn(async move {
                    #[cfg(test)]
                    let _reaper_guard = reaper_guard;
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    tokio::time::sleep(remaining).await;
                    // Remove the heavy live slot and preserve only a bounded,
                    // lightweight typed terminal record for one later read.
                    let now = Instant::now();
                    let mut registry = handles.lock().unwrap_or_else(|e| e.into_inner());
                    registry.prune_recent(now);
                    if let Some(mut slot) = registry.live.remove(&reap_id) {
                        let terminal = cancel_reap.terminate(StreamTerminal::Timeout);
                        slot.reaper_abort = None; // we are the reaper
                        drop(slot.handle.take());
                        remove_stream_owner(&mut slot, &reap_id);
                        registry.remember_recent(reap_id, terminal, now);
                    }
                });
                if let Some(slot) = registry.live.get_mut(&handle_id) {
                    slot.reaper_abort = Some(join.abort_handle());
                } else {
                    // Already finished before we armed — cancel the timer.
                    join.abort();
                }
            }
        }

        Ok((status, response_headers, handle_id))
    }

    /// Atomically attach a handler owner to a freshly opened stream. If expiry
    /// won the race, return its typed outcome without creating stale ownership.
    fn claim_stream_owner(
        &self,
        handle_id: &str,
        owner: &StreamOwner,
    ) -> Result<(), HttpClientError> {
        let now = Instant::now();
        let mut registry = self
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        registry.prune_recent(now);
        if let Some(slot) = registry.live.get_mut(handle_id) {
            owner
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .insert(handle_id.to_string());
            slot.owner = Some(Arc::clone(owner));
            return Ok(());
        }
        if let Some(terminal) = registry.take_recent(handle_id, now) {
            return Err(self.stream_terminal_error(terminal));
        }
        Err(HttpClientError::Closed)
    }

    /// Close every live stream owned by one handler. The owner lock is released
    /// before the registry lock is acquired, preserving the registry->owner
    /// nesting order used by the reaper.
    fn close_stream_owner(&self, owner: &StreamOwner) {
        let ids: Vec<String> = {
            let mut owned = owner.lock().unwrap_or_else(|error| error.into_inner());
            owned.drain().collect()
        };
        self.close_stream_ids(&ids);
    }

    /// Close a selected set of live streams without disturbing other handles
    /// owned by the same handler.
    fn close_stream_ids(&self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }

        let mut registry = self
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        registry.prune_recent(Instant::now());
        for id in ids {
            if let Some(mut slot) = registry.live.remove(id) {
                slot.cancel.terminate(StreamTerminal::Closed);
                if let Some(abort) = slot.reaper_abort.take() {
                    abort.abort();
                }
                drop(slot.handle.take());
                remove_stream_owner(&mut slot, id);
            }
            registry.forget_recent(id);
        }
    }

    /// Signal cancel, abort the reaper, drop any parked handle, and remove the
    /// slot. Guaranteed (std mutex) — usable from Drop. Returns whether a slot
    /// was present.
    fn finish_stream_slot_sync(&self, handle_id: &str, terminal: StreamTerminal) -> bool {
        let mut registry = self
            .stream_handles
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        registry.prune_recent(Instant::now());
        let had_recent = registry.forget_recent(handle_id);
        if let Some(mut slot) = registry.live.remove(handle_id) {
            slot.cancel.terminate(terminal);
            if let Some(abort) = slot.reaper_abort.take() {
                abort.abort();
            }
            drop(slot.handle.take());
            remove_stream_owner(&mut slot, handle_id);
            true
        } else {
            had_recent
        }
    }

    async fn finish_stream_slot(&self, handle_id: &str) -> bool {
        self.finish_stream_slot_sync(handle_id, StreamTerminal::Closed)
    }

    /// Remove a stream handle from its slot so a body read can await without
    /// holding the global handle lock. The cancel watch stays alive so close/
    /// expire aborts the read. A recent clean EOF yields `Ok(None)`; unknown,
    /// closed, and past-deadline handles remain errors.
    fn take_stream(&self, handle_id: &str) -> Result<Option<TakenStream>, HttpClientError> {
        let now = Instant::now();
        let mut registry = self
            .stream_handles
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        registry.prune_recent(now);
        if !registry.live.contains_key(handle_id) {
            if let Some(terminal) = registry.take_recent(handle_id, now) {
                return match terminal {
                    StreamTerminal::CleanEof => Ok(None),
                    terminal => Err(self.stream_terminal_error(terminal)),
                };
            }
            return Err(HttpClientError::Request(format!(
                "Unknown or already-closed stream handle '{handle_id}'"
            )));
        }
        if let Some(terminal) = registry
            .live
            .get(handle_id)
            .and_then(|slot| slot.cancel.terminal())
        {
            if let Some(mut slot) = registry.live.remove(handle_id) {
                if let Some(abort) = slot.reaper_abort.take() {
                    abort.abort();
                }
                drop(slot.handle.take());
                remove_stream_owner(&mut slot, handle_id);
            }
            return Err(self.stream_terminal_error(terminal));
        }
        let past_deadline = registry
            .live
            .get(handle_id)
            .expect("live stream checked above")
            .deadline
            .is_some_and(|deadline| deadline <= now);
        if past_deadline {
            if let Some(mut slot) = registry.live.remove(handle_id) {
                let terminal = slot.cancel.terminate(StreamTerminal::Timeout);
                if let Some(abort) = slot.reaper_abort.take() {
                    abort.abort();
                }
                drop(slot.handle.take());
                remove_stream_owner(&mut slot, handle_id);
                return Err(self.stream_terminal_error(terminal));
            }
            return Err(self.outbound_stream_timeout_error());
        }
        let slot = registry
            .live
            .get_mut(handle_id)
            .expect("live stream checked above");
        let cancel = Arc::clone(&slot.cancel);
        match slot.handle.take() {
            Some(handle) => Ok(Some(TakenStream { handle, cancel })),
            None => Err(HttpClientError::Request(format!(
                "Unknown or already-closed stream handle '{handle_id}'"
            ))),
        }
    }

    /// Return a still-open stream handle after a body read. Refuses reinsertion
    /// if the stream was closed/expired mid-read (cancel flag or missing slot).
    fn put_stream(
        &self,
        handle_id: &str,
        handle: HttpStreamHandle,
        cancel: &StreamCancel,
    ) -> Result<(), HttpClientError> {
        // Observing upstream EOF is the linearization point for clean
        // completion. Finalize that already-latched result before consulting
        // the wall clock or generic terminal rejection below: a reaper/close
        // may have removed the slot after EOF won, but it must not erase the
        // one follow-up `nothing` read.
        if handle.done {
            let terminal = cancel.terminate(StreamTerminal::CleanEof);
            if terminal == StreamTerminal::CleanEof {
                drop(handle);
                let now = Instant::now();
                let mut registry = self
                    .stream_handles
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                registry.prune_recent(now);
                if let Some(mut slot) = registry.live.remove(handle_id) {
                    slot.cancel.terminate(StreamTerminal::CleanEof);
                    if let Some(abort) = slot.reaper_abort.take() {
                        abort.abort();
                    }
                    drop(slot.handle.take());
                    remove_stream_owner(&mut slot, handle_id);
                }
                // `remember_recent` replaces an existing record for this ID,
                // so terminalization leaves exactly one bounded one-shot
                // result even when the racing reaper already recorded EOF.
                registry.remember_recent(handle_id.to_string(), StreamTerminal::CleanEof, now);
                return Ok(());
            }
        }

        if let Some(terminal) = cancel.terminal() {
            drop(handle);
            // Ensure the slot is gone (reaper/close may already have removed it).
            let _ = self.finish_stream_slot_sync(handle_id, terminal);
            return Err(self.stream_terminal_error(terminal));
        }
        let now = Instant::now();
        let mut registry = self
            .stream_handles
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        registry.prune_recent(now);
        if !registry.live.contains_key(handle_id) {
            drop(handle);
            let terminal = cancel
                .terminal()
                .or_else(|| registry.take_recent(handle_id, now));
            return Err(terminal
                .map(|reason| self.stream_terminal_error(reason))
                .unwrap_or(HttpClientError::Closed));
        }
        let terminal = registry.live.get(handle_id).and_then(|slot| {
            slot.cancel.terminal().or_else(|| {
                if slot.deadline.is_some_and(|deadline| deadline <= now) {
                    Some(slot.cancel.terminate(StreamTerminal::Timeout))
                } else {
                    None
                }
            })
        });
        if let Some(terminal) = terminal {
            drop(handle);
            if let Some(mut slot) = registry.live.remove(handle_id) {
                if let Some(abort) = slot.reaper_abort.take() {
                    abort.abort();
                }
                drop(slot.handle.take());
                remove_stream_owner(&mut slot, handle_id);
            }
            return Err(self.stream_terminal_error(terminal));
        }
        let slot = registry
            .live
            .get_mut(handle_id)
            .expect("live stream checked above");
        slot.handle = Some(handle);
        Ok(())
    }

    fn stream_terminal_error(&self, terminal: StreamTerminal) -> HttpClientError {
        match terminal {
            StreamTerminal::CleanEof => HttpClientError::Closed,
            StreamTerminal::Timeout => self.outbound_stream_timeout_error(),
            StreamTerminal::Closed => HttpClientError::Closed,
        }
    }

    fn outbound_stream_timeout_error(&self) -> HttpClientError {
        HttpClientError::Timeout {
            seconds: outbound_stream_effective_seconds(self.config.outbound_stream_max_seconds)
                .unwrap_or(self.config.timeout_seconds.max(1)),
        }
    }

    /// Pull one network chunk into `handle.buffer`, bounded by the per-chunk
    /// read deadline, the absolute total, and cooperative stream cancellation
    /// (close/expire while reading). Returns `Ok(true)` when bytes were added,
    /// `Ok(false)` at clean EOF (sets `handle.done`).
    async fn stream_pull(
        &self,
        handle: &mut HttpStreamHandle,
        budget: &Arc<ExecutionBudget>,
        cancel: &StreamCancel,
    ) -> Result<bool, HttpClientError> {
        use futures_util::StreamExt;

        if handle.done {
            let terminal = cancel.terminate(StreamTerminal::CleanEof);
            return if terminal == StreamTerminal::CleanEof {
                Ok(false)
            } else {
                Err(self.stream_terminal_error(terminal))
            };
        }
        let mut terminal_rx = cancel.subscribe();
        if let Some(terminal) = *terminal_rx.borrow() {
            return Err(self.stream_terminal_error(terminal));
        }
        let max_response_bytes = budget.limits().max_response_bytes;
        let idle_timeout = Duration::from_secs(self.config.timeout_seconds.max(1));
        let configured_timeout = match handle.total_deadline {
            Some(deadline) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(self.outbound_stream_timeout_error());
                }
                idle_timeout.min(remaining)
            }
            None => idle_timeout,
        };

        // Race the network read against stream cancellation so close-during-
        // active-read aborts the upstream promptly.
        let op = Self::run_http_with_budget(Arc::clone(budget), configured_timeout, async {
            Ok::<Option<reqwest::Result<Vec<u8>>>, HttpClientError>(handle.stream.next().await)
        });
        tokio::pin!(op);
        let next = tokio::select! {
            // Prefer a simultaneously-ready body result so the terminal
            // re-check below is the single deterministic arbiter: expiry still
            // wins, and a ready chunk can never be reinserted after the reaper.
            biased;
            result = &mut op => result?,
            changed = terminal_rx.changed() => {
                let _ = changed;
                let terminal = (*terminal_rx.borrow()).unwrap_or(StreamTerminal::Closed);
                return Err(self.stream_terminal_error(terminal));
            }
        };
        // Re-check after select so a simultaneously-ready body chunk cannot win
        // over an already-recorded hard deadline and be reinserted.
        if let Some(terminal) = cancel.terminal() {
            return Err(self.stream_terminal_error(terminal));
        }

        match next {
            Some(Ok(bytes)) => {
                let actual = handle.bytes_read.saturating_add(bytes.len());
                if bytes.len() > max_response_bytes.saturating_sub(handle.bytes_read) {
                    return Err(HttpClientError::Budget(BudgetExceeded::ResponseBytes {
                        limit: max_response_bytes,
                        actual,
                    }));
                }
                handle.bytes_read = actual;
                handle.buffer.extend_from_slice(&bytes);
                Ok(true)
            }
            Some(Err(e)) => Err(HttpClientError::Request(format!(
                "Failed to read response chunk: {e}"
            ))),
            None => {
                let terminal = cancel.terminate(StreamTerminal::CleanEof);
                if terminal == StreamTerminal::CleanEof {
                    handle.done = true;
                    Ok(false)
                } else {
                    Err(self.stream_terminal_error(terminal))
                }
            }
        }
    }

    /// Return a `Timeout` error if the handle's absolute stream lifetime
    /// (`outbound_stream_max_seconds`, tracked as `total_deadline`) has elapsed.
    /// Called before serving locally-buffered bytes so the absolute lifetime is
    /// enforced even when no network read is performed; `stream_pull` performs the
    /// same check before each network read.
    fn check_stream_deadline(&self, handle: &HttpStreamHandle) -> Result<(), HttpClientError> {
        if let Some(deadline) = handle.total_deadline
            && deadline.saturating_duration_since(Instant::now()).is_zero()
        {
            return Err(self.outbound_stream_timeout_error());
        }
        Ok(())
    }

    /// Pull the next raw byte chunk from a streaming response. Returns
    /// `Ok(None)` at clean end of stream (handle is finished). On error or EOF
    /// the slot is removed and the reaper aborted so the upstream is released
    /// and no sleeping timer remains.
    async fn next_chunk(
        &self,
        handle_id: &str,
        budget: Arc<ExecutionBudget>,
    ) -> Result<Option<Vec<u8>>, HttpClientError> {
        let Some(TakenStream { mut handle, cancel }) = self.take_stream(handle_id)? else {
            return Ok(None);
        };

        if let Err(e) = self.check_stream_deadline(&handle) {
            let _ = self.finish_stream_slot(handle_id).await;
            return Err(e);
        }

        if !handle.buffer.is_empty() {
            let chunk = std::mem::take(&mut handle.buffer);
            self.put_stream(handle_id, handle, &cancel)?;
            return Ok(Some(chunk));
        }

        match self.stream_pull(&mut handle, &budget, &cancel).await {
            Ok(true) => {
                let chunk = std::mem::take(&mut handle.buffer);
                self.put_stream(handle_id, handle, &cancel)?;
                Ok(Some(chunk))
            }
            Ok(false) => {
                let _ = self.finish_stream_slot(handle_id).await;
                Ok(None)
            }
            Err(e) => {
                let _ = self.finish_stream_slot(handle_id).await;
                Err(e)
            }
        }
    }

    /// Pull the next newline-delimited line (trailing `\n`, and a paired `\r`,
    /// stripped) from a streaming response. A final unterminated line is
    /// returned before EOF. Returns `Ok(None)` at clean end of stream.
    async fn next_line(
        &self,
        handle_id: &str,
        budget: Arc<ExecutionBudget>,
    ) -> Result<Option<String>, HttpClientError> {
        let Some(TakenStream { mut handle, cancel }) = self.take_stream(handle_id)? else {
            return Ok(None);
        };

        loop {
            let clean_eof_latched =
                handle.done && cancel.terminal() == Some(StreamTerminal::CleanEof);
            if !clean_eof_latched && let Err(e) = self.check_stream_deadline(&handle) {
                let _ = self.finish_stream_slot(handle_id).await;
                return Err(e);
            }

            if let Some(pos) = handle.buffer.iter().position(|&b| b == b'\n') {
                let mut line: Vec<u8> = handle.buffer.drain(..=pos).collect();
                line.pop(); // drop '\n'
                if line.last() == Some(&b'\r') {
                    line.pop(); // drop paired '\r' (CRLF)
                }
                self.put_stream(handle_id, handle, &cancel)?;
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }

            if handle.done {
                // Final unterminated line (if any), then fully finish — do NOT
                // leave a done slot + reaper parked for a subsequent read.
                if handle.buffer.is_empty() {
                    let _ = self.finish_stream_slot(handle_id).await;
                    return Ok(None);
                }
                let mut line = std::mem::take(&mut handle.buffer);
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                // Preserve one lightweight clean-EOF result so the next wait
                // binds `nothing`; `put_stream` removes all live stream state.
                self.put_stream(handle_id, handle, &cancel)?;
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }

            if let Err(e) = self.stream_pull(&mut handle, &budget, &cancel).await {
                let _ = self.finish_stream_slot(handle_id).await;
                return Err(e);
            }
        }
    }

    /// Close a streaming response handle if present. Signals cancel (aborting
    /// any active read), drops the handle, and aborts the reaper timer.
    /// Idempotent.
    async fn close_stream(&self, handle_id: &str) -> bool {
        self.finish_stream_slot(handle_id).await
    }

    /// Send a request and consume its body without ever buffering more than the
    /// configured response ceiling. The budget passed here is deliberately the
    /// interpreter's *live* budget, not construction-time IoClient state: the
    /// REPL replaces its budget for every command.
    async fn send_http_request(
        &self,
        request: reqwest::RequestBuilder,
        method: &str,
        budget: Arc<ExecutionBudget>,
    ) -> Result<(u16, Vec<(String, String)>, String), HttpClientError> {
        let method = method.to_string();
        let operation_budget = Arc::clone(&budget);
        let operation = async move {
            use futures_util::StreamExt;

            let response = Self::send_with_reuse_retry(request, &method).await?;

            let status = response.status().as_u16();
            // Header names are normalized to lowercase for consistent access
            // from WFL (e.g. resp.headers["content-type"]), and non-UTF8
            // values are converted lossily instead of dropped.
            let response_headers = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_ascii_lowercase(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect();
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);

            let max_response_bytes = operation_budget.limits().max_response_bytes;
            if let Some(content_length) = response.content_length() {
                let max_as_u64 = u64::try_from(max_response_bytes).unwrap_or(u64::MAX);
                if content_length > max_as_u64 {
                    return Err(HttpClientError::Budget(BudgetExceeded::ResponseBytes {
                        limit: max_response_bytes,
                        actual: usize::try_from(content_length).unwrap_or(usize::MAX),
                    }));
                }
            }

            // Do not retain a full raw byte buffer and then allocate a second,
            // potentially larger UTF-8 string. Decode each network chunk into
            // bounded scratch space, and enforce the same ceiling on both wire
            // bytes and decoded UTF-8 bytes. Invalid UTF-8 alone can expand 3x
            // when replaced with U+FFFD.
            let initial_capacity = response
                .content_length()
                .and_then(|len| usize::try_from(len).ok())
                .unwrap_or(0)
                .min(max_response_bytes)
                .min(64 * 1024);
            let encoding = Self::http_text_encoding(content_type.as_deref());
            let mut decoder = encoding.new_decoder();
            let mut body = String::with_capacity(initial_capacity);
            let mut wire_bytes = 0_usize;
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| {
                    HttpClientError::Request(format!("Failed to read response body: {e}"))
                })?;
                let actual = wire_bytes.saturating_add(chunk.len());
                if chunk.len() > max_response_bytes.saturating_sub(wire_bytes) {
                    return Err(HttpClientError::Budget(BudgetExceeded::ResponseBytes {
                        limit: max_response_bytes,
                        actual,
                    }));
                }
                wire_bytes = actual;
                Self::decode_http_chunk(
                    &mut decoder,
                    &chunk,
                    false,
                    &mut body,
                    max_response_bytes,
                )?;
            }
            Self::decode_http_chunk(&mut decoder, &[], true, &mut body, max_response_bytes)?;

            Ok((status, response_headers, body))
        };

        // A custom live budget may deliberately have no run-wide deadline. In
        // a lifetime-exempt main loop, fall back to the interpreter's
        // configured `timeout_seconds` (minimum one second) so the individual
        // network operation is still finite and user-configurable.
        let configured_timeout = Duration::from_secs(self.config.timeout_seconds.max(1));
        Self::run_http_with_budget(budget, configured_timeout, operation).await
    }

    /// Select the response encoding while preserving reqwest's text behavior:
    /// honor a declared charset and default to UTF-8.
    fn http_text_encoding(content_type: Option<&str>) -> &'static encoding_rs::Encoding {
        let charset = content_type.and_then(|value| {
            value.split(';').skip(1).find_map(|parameter| {
                let (name, value) = parameter.trim().split_once('=')?;
                name.trim()
                    .eq_ignore_ascii_case("charset")
                    .then(|| value.trim().trim_matches(|ch| ch == '\'' || ch == '"'))
            })
        });
        charset
            .and_then(|name| encoding_rs::Encoding::for_label(name.as_bytes()))
            .unwrap_or(encoding_rs::UTF_8)
    }

    /// Incrementally decode one response chunk into caller-owned text. The
    /// decoder writes only into fixed scratch storage; the destination reserves
    /// exactly the accepted addition before appending, avoiding Vec/String
    /// geometric-growth spikes near the configured ceiling.
    fn decode_http_chunk(
        decoder: &mut encoding_rs::Decoder,
        mut input: &[u8],
        last: bool,
        output: &mut String,
        max_response_bytes: usize,
    ) -> Result<(), HttpClientError> {
        let mut decoded = [0_u8; 8 * 1024];
        loop {
            let (result, read, written, _) = decoder.decode_to_utf8(input, &mut decoded, last);
            let actual = output.len().saturating_add(written);
            if written > max_response_bytes.saturating_sub(output.len()) {
                return Err(HttpClientError::Budget(BudgetExceeded::ResponseBytes {
                    limit: max_response_bytes,
                    actual,
                }));
            }
            if written > 0 {
                output.try_reserve_exact(written).map_err(|error| {
                    HttpClientError::Request(format!(
                        "Failed to allocate bounded HTTP response buffer: {error}"
                    ))
                })?;
                let text = std::str::from_utf8(&decoded[..written])
                    .expect("encoding_rs must emit valid UTF-8");
                output.push_str(text);
            }
            input = &input[read..];

            match result {
                encoding_rs::CoderResult::InputEmpty => return Ok(()),
                encoding_rs::CoderResult::OutputFull => {}
            }
        }
    }

    fn outbound_http_deadline(
        budget: &ExecutionBudget,
        configured_timeout: Duration,
    ) -> Result<OutboundHttpDeadline, HttpClientError> {
        budget.check_cancelled()?;

        if budget.is_deadline_exempt() {
            return Ok(OutboundHttpDeadline::MainLoop {
                duration: budget.limits().max_duration.unwrap_or(configured_timeout),
            });
        }

        let Some(limit) = budget.limits().max_duration else {
            return Ok(OutboundHttpDeadline::None);
        };
        let Some(remaining) = limit.checked_sub(budget.elapsed()) else {
            return Err(HttpClientError::Budget(BudgetExceeded::Deadline {
                limit_secs: limit.as_secs(),
            }));
        };
        if remaining.is_zero() {
            return Err(HttpClientError::Budget(BudgetExceeded::Deadline {
                limit_secs: limit.as_secs(),
            }));
        }
        Ok(OutboundHttpDeadline::Execution {
            remaining,
            limit_secs: limit.as_secs(),
        })
    }

    /// Race the complete network operation (connect, headers, and streamed
    /// body) against both cooperative cancellation and the applicable finite
    /// deadline. Dropping reqwest's future closes/cancels the in-flight work.
    async fn run_http_with_budget<T, F>(
        budget: Arc<ExecutionBudget>,
        configured_timeout: Duration,
        operation: F,
    ) -> Result<T, HttpClientError>
    where
        F: std::future::Future<Output = Result<T, HttpClientError>>,
    {
        let deadline = Self::outbound_http_deadline(&budget, configured_timeout)?;
        // The budget/run-derived finite duration, if any.
        let budget_duration = match deadline {
            OutboundHttpDeadline::None => None,
            OutboundHttpDeadline::Execution { remaining, .. } => Some(remaining),
            OutboundHttpDeadline::MainLoop { duration } => Some(duration),
        };
        // The operation is bounded by the SHORTER of the caller's configured
        // timeout — which already encodes the stream's idle timeout AND its
        // remaining absolute-total deadline (see `stream_pull`/`open_http_stream`)
        // — and the run/budget deadline. Whichever is smaller decides both how
        // long we wait and which error we report. `configured_timeout` is always
        // finite, so the operation is bounded even when the budget has no
        // run-wide deadline (previously that path discarded the stream deadline
        // entirely and could wait out the whole run).
        let budget_is_binding = matches!(budget_duration, Some(bd) if bd <= configured_timeout);
        let timeout_duration = match budget_duration {
            Some(bd) if budget_is_binding => bd,
            _ => configured_timeout,
        };

        let cancellation_budget = Arc::clone(&budget);
        let cancellation = async move {
            loop {
                if cancellation_budget.is_cancelled() {
                    break;
                }
                tokio::time::sleep(HTTP_CANCELLATION_POLL_INTERVAL).await;
            }
        };
        let timeout = async move { tokio::time::sleep(timeout_duration).await };

        tokio::pin!(operation);
        tokio::pin!(cancellation);
        tokio::pin!(timeout);
        tokio::select! {
            result = &mut operation => result,
            _ = &mut cancellation => Err(HttpClientError::Budget(BudgetExceeded::Cancelled)),
            _ = &mut timeout => {
                if budget_is_binding {
                    // The run/budget deadline was the shorter bound.
                    match deadline {
                        OutboundHttpDeadline::Execution { limit_secs, .. } => {
                            Err(HttpClientError::Budget(BudgetExceeded::Deadline { limit_secs }))
                        }
                        OutboundHttpDeadline::MainLoop { duration } => {
                            Err(HttpClientError::Timeout { seconds: duration.as_secs() })
                        }
                        OutboundHttpDeadline::None => {
                            unreachable!("budget cannot be binding when there is no budget deadline")
                        }
                    }
                } else {
                    // The caller's configured timeout (stream idle / absolute
                    // total) was the shorter bound.
                    Err(HttpClientError::Timeout {
                        seconds: configured_timeout.as_secs().max(1),
                    })
                }
            }
        }
    }

    #[allow(dead_code)]
    async fn open_file(&self, path: &str) -> Result<String, String> {
        let handle_id = {
            let mut next_id = self.next_file_id.lock().await;
            let id = format!("file{}", *next_id);
            *next_id += 1;
            id
        };

        let path_buf = PathBuf::from(path);

        match tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false) // Explicitly preserve file content on open
            .open(path)
            .await
        {
            Ok(file) => {
                let mut file_handles = self.file_handles.lock().await;

                // Check if the file is already open, but don't error - just use a new handle
                file_handles.insert(handle_id.clone(), (path_buf, file));
                Ok(handle_id)
            }
            Err(e) => Err(format!("Failed to open file {path}: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn open_file_with_mode(
        &self,
        path: &str,
        mode: FileOpenMode,
    ) -> Result<String, RuntimeError> {
        let handle_id = {
            let mut next_id = self.next_file_id.lock().await;
            let id = format!("file{}", *next_id);
            *next_id += 1;
            id
        };

        let path_buf = PathBuf::from(path);

        let mut options = tokio::fs::OpenOptions::new();
        match mode {
            FileOpenMode::Read => {
                options.read(true).write(false).create(false);
            }
            FileOpenMode::Write => {
                options.read(false).write(true).create(true).truncate(true);
            }
            FileOpenMode::Append => {
                options.read(false).write(true).create(true).append(true);
            }
            FileOpenMode::ReadBinary => {
                options.read(true).write(false).create(false);
            }
            FileOpenMode::WriteBinary => {
                options.read(false).write(true).create(true).truncate(true);
            }
        }

        match options.open(path).await {
            Ok(file) => {
                let mut file_handles = self.file_handles.lock().await;
                file_handles.insert(handle_id.clone(), (path_buf, file));
                Ok(handle_id)
            }
            Err(e) => {
                let error_kind = match e.kind() {
                    std::io::ErrorKind::NotFound => ErrorKind::FileNotFound,
                    std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
                    _ => ErrorKind::General,
                };
                Err(RuntimeError::with_kind(
                    format!("Failed to open file {path}: {e}"),
                    0,
                    0,
                    error_kind,
                ))
            }
        }
    }

    #[allow(dead_code)]
    async fn read_file(
        &self,
        handle_id: &str,
        budget: &ExecutionBudget,
    ) -> Result<String, FileReadError> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            drop(file_handles);

            match self.open_file(handle_id).await {
                Ok(new_handle) => {
                    // Now read from the new handle - use Box::pin to handle recursion in async fn
                    let future = Box::pin(self.read_file(&new_handle, budget));
                    let result = future.await;
                    let _ = self.close_file(&new_handle).await;
                    return result;
                }
                Err(e) => {
                    return Err(FileReadError::Io(format!(
                        "Invalid file handle or path: {handle_id}: {e}"
                    )));
                }
            }
        }

        let mut file_clone = match file_handles.get_mut(handle_id).unwrap().1.try_clone().await {
            Ok(clone) => clone,
            Err(e) => {
                return Err(FileReadError::Io(format!(
                    "Failed to clone file handle: {e}"
                )));
            }
        };

        drop(file_handles);

        let bytes = read_to_end_capped(&mut file_clone, budget, "Failed to read file").await?;
        String::from_utf8(bytes).map_err(|e| {
            FileReadError::Io(format!(
                "Failed to read file: stream did not contain valid UTF-8: {}",
                e.utf8_error()
            ))
        })
    }

    /// Syncs file to disk with Windows-specific error handling.
    ///
    /// # Windows Behavior
    /// On Windows, `sync_all()` can return spurious `PermissionDenied` errors when:
    /// - Multiple processes/threads access the same file
    /// - File locking or antivirus software interferes
    /// - The filesystem has concurrent access patterns
    ///
    /// This is a known Windows limitation (not a real permission error). Since `flush()`
    /// has already ensured data reaches OS buffers, it's safe to ignore PermissionDenied.
    ///
    /// # Error Handling
    /// - Windows: Suppress ONLY PermissionDenied; propagate all other errors
    /// - Unix: Propagate all errors
    ///
    /// # Why Other Errors Must Propagate
    /// Errors like `StorageFull`, `IoUnavailable`, `ReadOnlyFilesystem` indicate real
    /// I/O failures that the user must be notified about. Silently ignoring these would
    /// cause data loss or corruption.
    ///
    /// # Parameters
    /// - `file`: The file to sync
    /// - `operation`: Description of the operation (for error messages)
    async fn sync_file_with_windows_handling(
        file: &mut tokio::fs::File,
        operation: &str,
    ) -> Result<(), String> {
        match file.sync_all().await {
            Ok(_) => Ok(()),
            Err(e) => {
                // On Windows, selectively suppress only PermissionDenied errors
                #[cfg(windows)]
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    eprintln!(
                        "Warning: Windows file sync encountered spurious PermissionDenied during {} (data already flushed)",
                        operation
                    );
                    return Ok(());
                }

                // All other errors must be propagated on all platforms
                Err(format!("Failed to sync file during {}: {e}", operation))
            }
        }
    }

    #[allow(dead_code)]
    async fn write_file(&self, handle_id: &str, content: &str) -> Result<(), String> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            drop(file_handles);

            match self.open_file(handle_id).await {
                Ok(new_handle) => {
                    // Now write to the new handle - use Box::pin to handle recursion in async fn
                    let future = Box::pin(self.write_file(&new_handle, content));
                    let result = future.await;
                    let _ = self.close_file(&new_handle).await;
                    return result;
                }
                Err(e) => return Err(format!("Invalid file handle or path: {handle_id}: {e}")),
            }
        }

        let mut file_clone = match file_handles.get_mut(handle_id).unwrap().1.try_clone().await {
            Ok(clone) => clone,
            Err(e) => return Err(format!("Failed to clone file handle: {e}")),
        };

        drop(file_handles);

        match AsyncSeekExt::seek(&mut file_clone, std::io::SeekFrom::Start(0)).await {
            Ok(_) => match file_clone.set_len(0).await {
                Ok(_) => {
                    match AsyncWriteExt::write_all(&mut file_clone, content.as_bytes()).await {
                        Ok(_) => {
                            // Flush the data to ensure it's written to disk
                            match file_clone.flush().await {
                                Ok(_) => {
                                    // Platform-specific sync behavior
                                    // Sync file to disk with Windows-aware error handling
                                    Self::sync_file_with_windows_handling(&mut file_clone, "write")
                                        .await
                                }
                                Err(e) => Err(format!("Failed to flush file: {e}")),
                            }
                        }
                        Err(e) => Err(format!("Failed to write to file: {e}")),
                    }
                }
                Err(e) => Err(format!("Failed to truncate file: {e}")),
            },
            Err(e) => Err(format!("Failed to seek in file: {e}")),
        }
    }

    async fn read_binary(
        &self,
        handle_id: &str,
        budget: &ExecutionBudget,
    ) -> Result<Vec<u8>, FileReadError> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            return Err(FileReadError::Io(format!(
                "Invalid file handle: {handle_id}"
            )));
        }

        let mut file_clone = match file_handles.get_mut(handle_id).unwrap().1.try_clone().await {
            Ok(clone) => clone,
            Err(e) => {
                return Err(FileReadError::Io(format!(
                    "Failed to clone file handle: {e}"
                )));
            }
        };

        drop(file_handles);

        // Seek to start before reading all
        match AsyncSeekExt::seek(&mut file_clone, std::io::SeekFrom::Start(0)).await {
            Ok(_) => {}
            Err(e) => return Err(FileReadError::Io(format!("Failed to seek in file: {e}"))),
        }

        read_to_end_capped(&mut file_clone, budget, "Failed to read binary file").await
    }

    async fn read_binary_n(
        &self,
        handle_id: &str,
        count: usize,
        budget: &ExecutionBudget,
    ) -> Result<Vec<u8>, FileReadError> {
        budget
            .check_file_read_bytes(count)
            .map_err(FileReadError::Budget)?;

        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            return Err(FileReadError::Io(format!(
                "Invalid file handle: {handle_id}"
            )));
        }

        let mut file_clone = match file_handles.get_mut(handle_id).unwrap().1.try_clone().await {
            Ok(clone) => clone,
            Err(e) => {
                return Err(FileReadError::Io(format!(
                    "Failed to clone file handle: {e}"
                )));
            }
        };

        drop(file_handles);

        let mut buf = vec![0u8; count];
        match AsyncReadExt::read(&mut file_clone, &mut buf).await {
            Ok(n) => {
                buf.truncate(n);
                Ok(buf)
            }
            Err(e) => Err(FileReadError::Io(format!(
                "Failed to read binary bytes: {e}"
            ))),
        }
    }

    async fn write_binary(&self, handle_id: &str, data: &[u8]) -> Result<(), String> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            return Err(format!("Invalid file handle: {handle_id}"));
        }

        let mut file_clone = match file_handles.get_mut(handle_id).unwrap().1.try_clone().await {
            Ok(clone) => clone,
            Err(e) => return Err(format!("Failed to clone file handle: {e}")),
        };

        drop(file_handles);

        match AsyncWriteExt::write_all(&mut file_clone, data).await {
            Ok(_) => match file_clone.flush().await {
                Ok(_) => {
                    Self::sync_file_with_windows_handling(&mut file_clone, "write_binary").await
                }
                Err(e) => Err(format!("Failed to flush binary file: {e}")),
            },
            Err(e) => Err(format!("Failed to write binary data: {e}")),
        }
    }

    async fn file_size(&self, handle_id: &str) -> Result<u64, String> {
        let file_handles = self.file_handles.lock().await;

        if let Some((path, _)) = file_handles.get(handle_id) {
            let path = path.clone();
            drop(file_handles);
            match tokio::fs::metadata(&path).await {
                Ok(meta) => Ok(meta.len()),
                Err(e) => Err(format!("Failed to get file size: {e}")),
            }
        } else {
            // Fallback: try using handle_id as a filesystem path
            match tokio::fs::metadata(handle_id).await {
                Ok(meta) => Ok(meta.len()),
                Err(e) => Err(format!(
                    "Invalid file handle '{handle_id}' and filesystem lookup failed: {e}"
                )),
            }
        }
    }

    /// Improved file append operation - directly appends content without reading the whole file first
    /// This is much more memory efficient, especially for large log files
    #[allow(dead_code)]
    async fn close_file(&self, handle_id: &str) -> Result<(), String> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            return Ok(());
        }

        // Get the file handle before removing it
        if let Some((_, mut file)) = file_handles.remove(handle_id) {
            // Flush the file before closing to ensure all data is written to disk
            match file.flush().await {
                Ok(_) => {
                    // Sync file to disk with Windows-aware error handling
                    Self::sync_file_with_windows_handling(&mut file, "close").await
                }
                Err(e) => Err(format!("Failed to flush file during close: {e}")),
            }
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    async fn append_file(&self, handle_id: &str, content: &str) -> Result<(), String> {
        let mut file_handles = self.file_handles.lock().await;

        let (_, file) = match file_handles.get_mut(handle_id) {
            Some(entry) => entry,
            None => return Err(format!("Invalid file handle: {handle_id}")),
        };

        match AsyncSeekExt::seek(file, std::io::SeekFrom::End(0)).await {
            Ok(_) => match AsyncWriteExt::write_all(file, content.as_bytes()).await {
                Ok(_) => {
                    // Flush the data to ensure it's written to disk
                    match file.flush().await {
                        Ok(_) => {
                            // Sync file to disk with Windows-aware error handling
                            Self::sync_file_with_windows_handling(file, "append").await
                        }
                        Err(e) => Err(format!("Failed to flush appended data: {e}")),
                    }
                }
                Err(e) => Err(format!("Failed to append to file: {e}")),
            },
            Err(e) => Err(format!("Failed to seek to end of file: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn create_directory(&self, path: &str) -> Result<(), String> {
        match tokio::fs::create_dir_all(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to create directory: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn create_file(&self, path: &str, content: &str) -> Result<(), String> {
        match tokio::fs::write(path, content).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to create file: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn delete_file(&self, path: &str) -> Result<(), String> {
        match tokio::fs::remove_file(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete file: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn delete_directory(&self, path: &str) -> Result<(), String> {
        match tokio::fs::remove_dir_all(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete directory: {e}")),
        }
    }

    // Subprocess management methods

    /// Shared subprocess policy gate for execute + spawn (shell and direct-exec).
    fn authorize_subprocess(
        &self,
        command: &str,
        args: &[&str],
        use_shell: bool,
        line: usize,
        column: usize,
    ) -> Result<bool, String> {
        use crate::interpreter::command_sanitizer::{CommandSanitizer, ValidationResult};

        let needs_shell = use_shell
            || (args.is_empty() && CommandSanitizer::contains_shell_metacharacters(command));

        // Resolve program name for policy (must happen before any Command::new)
        let program = if args.is_empty() && !needs_shell {
            CommandSanitizer::parse_command(command)
                .map(|(p, _)| p)
                .unwrap_or_else(|_| command.to_string())
        } else if args.is_empty() && needs_shell {
            // Shell path: policy uses the first token for allowlist matching
            command
                .split_whitespace()
                .next()
                .unwrap_or(command)
                .to_string()
        } else {
            command.to_string()
        };

        let sanitizer = CommandSanitizer::new(Arc::clone(&self.config));
        match sanitizer.authorize_process_execution(&program, needs_shell, command)? {
            ValidationResult::Safe => Ok(needs_shell),
            ValidationResult::RequiresShell { warnings, .. } => {
                if self.config.warn_on_shell_execution {
                    eprintln!("⚠️  Security Warning (line {}, column {}):", line, column);
                    eprintln!("   Shell execution enabled for command: {}", command);
                    for warning in warnings {
                        eprintln!("   - {}", warning);
                    }
                    eprintln!(
                        "   Prefer: execute command \"program\" with arguments [\"arg1\", \"arg2\"] \
                         after allowing the program in .wflcfg."
                    );
                }
                Ok(needs_shell)
            }
            ValidationResult::Blocked { reason } => Err(format!(
                "Command blocked by security policy: {}\n\
                 Subprocess execution is disabled by default. To allow it, update .wflcfg:\n\
                   allow_shell_execution = true\n\
                   shell_execution_mode = allowlist_only   # or sanitized / unrestricted\n\
                   allowed_shell_commands = echo, ls       # required for allowlist_only\n\
                 (line {}, column {})",
                reason, line, column
            )),
        }
    }

    /// Execute a command and wait for it to complete, returning (stdout, stderr, exit_code)
    #[allow(dead_code)]
    async fn execute_command(
        &self,
        command: &str,
        args: &[&str],
        use_shell: bool,
        line: usize,
        column: usize,
    ) -> Result<(String, String, i32), ExecuteCommandError> {
        use crate::interpreter::command_sanitizer::CommandSanitizer;
        use tokio::process::Command;

        let needs_shell = self
            .authorize_subprocess(command, args, use_shell, line, column)
            .map_err(ExecuteCommandError::Other)?;

        let budget = ExecutionBudget::current_or_default();
        let configured_timeout = Duration::from_secs(self.config.timeout_seconds.max(1));
        let deadline = foreground_command_deadline(&budget, configured_timeout)?;

        // Build the command
        let mut cmd = if needs_shell && (use_shell || args.is_empty()) {
            // Shell execution path
            #[cfg(target_os = "windows")]
            {
                let mut cmd = Command::new("cmd.exe");
                cmd.args(["/C", command]);
                cmd
            }

            #[cfg(not(target_os = "windows"))]
            {
                let mut cmd = Command::new("sh");
                cmd.args(["-c", command]);
                cmd
            }
        } else {
            // Direct-exec path (still policy-gated above)
            let (program, parsed_args) = if args.is_empty() {
                CommandSanitizer::parse_command(command).map_err(ExecuteCommandError::Other)?
            } else {
                (
                    command.to_string(),
                    args.iter().map(|s| s.to_string()).collect(),
                )
            };

            let mut cmd = Command::new(program);
            cmd.args(parsed_args);
            cmd
        };

        // `Command::output` accumulates both streams into unbounded Vecs and
        // cannot observe WFL's cooperative cancellation while the child is
        // stalled. Pipe and drain both streams concurrently under the existing
        // per-stream buffer ceiling instead. `kill_on_drop` is a final safety
        // net if this future itself is abandoned by its caller.
        let mut child = cmd
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                ExecuteCommandError::Other(format!(
                    "Failed to execute command '{}': {}",
                    command, e
                ))
            })?;

        let stdout_pipe = child.stdout.take().ok_or_else(|| {
            ExecuteCommandError::Other("Failed to capture command stdout".to_string())
        })?;
        let stderr_pipe = child.stderr.take().ok_or_else(|| {
            ExecuteCommandError::Other("Failed to capture command stderr".to_string())
        })?;
        let buffer_size = self.config.subprocess_config.max_buffer_size_bytes;

        let stdout_task = tokio::spawn(capture_process_stream(stdout_pipe, buffer_size));
        let stderr_task = tokio::spawn(capture_process_stream(stderr_pipe, buffer_size));
        let mut capture_abort =
            ProcessCaptureAbortGuard::new(stdout_task.abort_handle(), stderr_task.abort_handle());

        // The guarded operation includes pipe EOF, not just direct-child exit.
        // A command can spawn a descendant that inherits stdout/stderr and then
        // exit; without this wider guard, collectors would await that
        // descendant forever even though `child.wait()` already succeeded.
        let completed = {
            let operation_child = &mut child;
            let operation = async move {
                let status = operation_child.wait().await.map_err(|error| {
                    ExecuteCommandError::Other(format!("Failed to wait for command: {error}"))
                })?;
                let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);
                let stdout_capture = stdout_result
                    .map_err(|error| {
                        ExecuteCommandError::Other(format!(
                            "Failed to join stdout collector: {error}"
                        ))
                    })?
                    .map_err(|error| {
                        ExecuteCommandError::Other(format!(
                            "Failed to read command stdout: {error}"
                        ))
                    })?;
                let stderr_capture = stderr_result
                    .map_err(|error| {
                        ExecuteCommandError::Other(format!(
                            "Failed to join stderr collector: {error}"
                        ))
                    })?
                    .map_err(|error| {
                        ExecuteCommandError::Other(format!(
                            "Failed to read command stderr: {error}"
                        ))
                    })?;

                Ok((status, stdout_capture, stderr_capture))
            };
            let interrupt = foreground_command_interrupt(&budget, deadline);
            tokio::pin!(operation);
            tokio::pin!(interrupt);

            tokio::select! {
                biased;
                result = &mut operation => Ok(result),
                error = &mut interrupt => Err(error),
            }
        };

        let (status, stdout_capture, stderr_capture) = match completed {
            Ok(Ok(result)) => {
                capture_abort.disarm();
                result
            }
            Ok(Err(interruption)) | Err(interruption) => {
                // Stop retaining output immediately, then kill/reap the direct
                // child if it is still alive. Closing its readers also prevents
                // a chatty child from blocking forever on a full pipe.
                capture_abort.abort();
                let termination = terminate_foreground_child(&mut child).await;

                if let Err(termination_error) = termination {
                    return Err(ExecuteCommandError::Other(format!(
                        "{interruption}; {termination_error}"
                    )));
                }
                return Err(interruption);
            }
        };

        if stdout_capture.bytes_dropped > 0 {
            eprintln!(
                "⚠️  WARNING: Command stdout exceeded max_buffer_size_bytes; \
                 {} oldest byte(s) were discarded.",
                stdout_capture.bytes_dropped
            );
        }
        if stderr_capture.bytes_dropped > 0 {
            eprintln!(
                "⚠️  WARNING: Command stderr exceeded max_buffer_size_bytes; \
                 {} oldest byte(s) were discarded.",
                stderr_capture.bytes_dropped
            );
        }

        let stdout = String::from_utf8_lossy(&stdout_capture.bytes).to_string();
        let stderr = String::from_utf8_lossy(&stderr_capture.bytes).to_string();
        let exit_code = status.code().unwrap_or(-1);

        Ok((stdout, stderr, exit_code))
    }

    /// Spawn a background process and return a process ID
    #[allow(dead_code)]
    async fn spawn_process(
        &self,
        command: &str,
        args: &[&str],
        use_shell: bool,
        line: usize,
        column: usize,
    ) -> Result<String, String> {
        use crate::interpreter::command_sanitizer::CommandSanitizer;
        use tokio::io::AsyncReadExt;
        use tokio::process::Command;

        // Clean up completed processes before spawning new one
        // self.cleanup_completed_processes().await;

        // Check process limit
        {
            let handles = self.process_handles.lock().await;
            if handles.len() >= self.config.subprocess_config.max_concurrent_processes {
                return Err(format!(
                    "Process limit reached: {} processes currently running (max: {}). \
                     Consider waiting for processes to complete or increasing max_concurrent_processes in .wflcfg",
                    handles.len(),
                    self.config.subprocess_config.max_concurrent_processes
                ));
            }
        }

        let needs_shell = self.authorize_subprocess(command, args, use_shell, line, column)?;

        // Build the command
        let mut cmd = if needs_shell && (use_shell || args.is_empty()) {
            // Shell execution path
            #[cfg(target_os = "windows")]
            {
                let mut cmd = Command::new("cmd.exe");
                cmd.args(["/C", command]);
                cmd
            }

            #[cfg(not(target_os = "windows"))]
            {
                let mut cmd = Command::new("sh");
                cmd.args(["-c", command]);
                cmd
            }
        } else {
            // Direct-exec path (still policy-gated above)
            let (program, parsed_args) = if args.is_empty() {
                CommandSanitizer::parse_command(command)?
            } else {
                (
                    command.to_string(),
                    args.iter().map(|s| s.to_string()).collect(),
                )
            };

            let mut cmd = Command::new(program);
            cmd.args(parsed_args);
            cmd
        };

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn process '{}': {}", command, e))?;

        // Generate process ID
        let process_id = {
            let mut next_id = self.next_process_id.lock().await;
            let id = format!("proc{}", *next_id);
            *next_id += 1;
            id
        };

        // Create buffers for stdout and stderr with configurable size
        let buffer_size = self.config.subprocess_config.max_buffer_size_bytes;
        let stdout_buffer = Arc::new(tokio::sync::Mutex::new(bounded_buffer::BoundedBuffer::new(
            buffer_size,
        )));
        let stderr_buffer = Arc::new(tokio::sync::Mutex::new(bounded_buffer::BoundedBuffer::new(
            buffer_size,
        )));

        // Spawn background tasks to collect stdout and stderr
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if let Some(mut stdout) = stdout {
            let buffer = Arc::clone(&stdout_buffer);
            let cmd = command.to_string();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let mut warning_shown = false;
                loop {
                    match stdout.read(&mut buf).await {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            let mut locked_buffer = buffer.lock().await;
                            locked_buffer.push(&buf[..n]);

                            // Warn once if data is being dropped
                            if locked_buffer.stats().bytes_dropped > 0 && !warning_shown {
                                eprintln!(
                                    "⚠️  WARNING: Process '{}' stdout buffer overflow. \
                                     Data is being dropped. Consider reading output more frequently.",
                                    cmd
                                );
                                warning_shown = true;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        if let Some(mut stderr) = stderr {
            let buffer = Arc::clone(&stderr_buffer);
            let cmd = command.to_string();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let mut warning_shown = false;
                loop {
                    match stderr.read(&mut buf).await {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            let mut locked_buffer = buffer.lock().await;
                            locked_buffer.push(&buf[..n]);

                            // Warn once if data is being dropped
                            if locked_buffer.stats().bytes_dropped > 0 && !warning_shown {
                                eprintln!(
                                    "⚠️  WARNING: Process '{}' stderr buffer overflow. \
                                     Data is being dropped. Consider reading output more frequently.",
                                    cmd
                                );
                                warning_shown = true;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        // Store process handle
        let handle = ProcessHandle {
            child,
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            started_at: Instant::now(),
            completed_at: None,
            exit_code: None,
            stdout_buffer,
            stderr_buffer,
        };

        self.process_handles
            .lock()
            .await
            .insert(process_id.clone(), handle);

        Ok(process_id)
    }

    /// Clean up completed processes (lazy cleanup)
    /// This prevents memory leaks by removing process handles for completed processes
    #[allow(dead_code)]
    async fn cleanup_completed_processes(&self) {
        let mut handles = self.process_handles.lock().await;
        handles.retain(|_id, handle| {
            match handle.child.try_wait() {
                Ok(Some(_exit_status)) => {
                    // Process has completed - remove it
                    false
                }
                Ok(None) => {
                    // Process is still running - keep it
                    true
                }
                Err(_) => {
                    // Error checking status - remove it
                    false
                }
            }
        });
    }

    /// Read accumulated output from a process
    #[allow(dead_code)]
    async fn read_process_output(&self, process_id: &str) -> Result<String, String> {
        // Don't cleanup here - user may want to read output from completed processes
        let handles = self.process_handles.lock().await;
        let handle = handles
            .get(process_id)
            .ok_or_else(|| format!("Invalid process ID: {}", process_id))?;

        let mut buffer = handle.stdout_buffer.lock().await;
        let output = String::from_utf8_lossy(&buffer.read_all()).to_string();
        Ok(output)
    }

    /// Kill a running process
    #[allow(dead_code)]
    async fn kill_process(&self, process_id: &str) -> Result<(), String> {
        {
            let mut handles = self.process_handles.lock().await;
            let handle = handles
                .get_mut(process_id)
                .ok_or_else(|| format!("Invalid process ID: {}", process_id))?;

            handle
                .child
                .kill()
                .await
                .map_err(|e| format!("Failed to kill process: {}", e))?;
        }

        // Clean up killed and other completed processes
        self.cleanup_completed_processes().await;

        Ok(())
    }

    /// Wait for a process to complete and return its exit code
    #[allow(dead_code)]
    async fn wait_for_process(&self, process_id: &str) -> Result<i32, String> {
        let mut handle = {
            let mut handles = self.process_handles.lock().await;
            handles
                .remove(process_id)
                .ok_or_else(|| format!("Invalid process ID: {}", process_id))?
        };

        let status = handle
            .child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for process: {}", e))?;

        Ok(status.code().unwrap_or(-1))
    }

    /// Check if a process is still running
    #[allow(dead_code)]
    async fn is_process_running(&self, process_id: &str) -> bool {
        let mut handles = self.process_handles.lock().await;
        if let Some(handle) = handles.get_mut(process_id) {
            matches!(handle.child.try_wait(), Ok(None))
        } else {
            false
        }
        // Note: Cleanup happens in spawn_process and kill_process
    }
}

impl Drop for IoClient {
    fn drop(&mut self) {
        // Try to acquire lock without blocking (Drop can't be async)
        if let Ok(mut handles) = self.process_handles.try_lock() {
            let running_count = handles.len();

            if running_count > 0 && self.config.subprocess_config.warn_on_orphan {
                eprintln!(
                    "⚠️  WARNING: {} subprocess(es) still running at interpreter shutdown",
                    running_count
                );
            }

            // Optionally kill all running processes on shutdown
            if self.config.subprocess_config.kill_on_shutdown {
                for handle in handles.values_mut() {
                    let _ = handle.child.start_kill();
                }
            }

            handles.clear();
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve `path` against `cwd` and normalize it lexically, mirroring Python's
/// `os.path.abspath` (`normpath(join(cwd, path))`): `.` components are dropped
/// and `..` collapses without touching the filesystem (no symlink resolution).
fn lexical_abspath(path: &std::path::Path, cwd: &std::path::Path) -> String {
    use std::path::Component;

    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    let mut root = PathBuf::new();
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for component in joined.components() {
        match component {
            Component::Prefix(prefix) => root.push(prefix.as_os_str()),
            Component::RootDir => root.push(std::path::MAIN_SEPARATOR_STR),
            Component::CurDir => {}
            // Like os.path.normpath, ".." at the root is dropped
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(part) => parts.push(part.to_os_string()),
        }
    }

    let mut result = root;
    for part in parts {
        result.push(part);
    }
    result.to_string_lossy().into_owned()
}

impl Interpreter {
    /// Conservative WFL call/recursion ceiling for an interpreter built via
    /// [`Interpreter::new`], the zero-config default path that makes no promise
    /// about the thread stack it runs on. WFL's async tree-walker costs enough
    /// debug stack per WFL call that an ordinary (e.g. 8 MiB) thread overflows
    /// after only a few dozen frames, so this default is kept well below that so
    /// the budget's catchable "maximum call depth exceeded" error fires *before*
    /// a native stack overflow on such a stack. It is deliberately shallow:
    /// programs that recurse deeper must opt into both a higher `max_call_depth`
    /// and [`crate::run_with_interpreter_stack`] (see the type-level "Stack
    /// safety" docs); the config-taking constructors honor the configured depth,
    /// so the CLI reaches the full 1000 on its dedicated 1 GiB stack.
    pub const DEFAULT_EMBED_CALL_DEPTH: usize = 12;

    pub fn new() -> Self {
        // The default path makes no stack guarantee, so cap recursion
        // conservatively (see `DEFAULT_EMBED_CALL_DEPTH`) rather than inheriting
        // the config-file default of 1000, which only stays catchable on the
        // CLI's dedicated large stack.
        let config = WflConfig {
            max_call_depth: Self::DEFAULT_EMBED_CALL_DEPTH,
            ..WflConfig::default()
        };
        Self::with_config(Arc::new(config))
    }

    pub fn with_config(config: Arc<WflConfig>) -> Self {
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        Self::with_config_and_budget(config, budget)
    }

    /// Construct an interpreter that shares a caller-supplied
    /// [`ExecutionBudget`], so one budget can govern a whole run — the CLI's
    /// pre-parse source check, lexing/parsing, interpretation, and any nested
    /// `execute file` all charge the same deadline, operation ceiling, and
    /// cancellation flag. Use [`Interpreter::budget`] to obtain the handle for
    /// cancellation.
    pub fn with_config_and_budget(config: Arc<WflConfig>, budget: Arc<ExecutionBudget>) -> Self {
        let global_env = Environment::new_global();

        {
            let mut env = global_env.borrow_mut();
            let _ = env.define(
                "display",
                Value::NativeFunction("display", Self::native_display),
            );

            stdlib::register_stdlib(&mut env);
        }

        Interpreter {
            global_env,
            current_count: RefCell::new(None),
            in_count_loop: RefCell::new(false),
            sched_counter: Cell::new(0),
            budget,
            current_block_overload_dups: RefCell::new(None),
            call_stack: RefCell::new(Vec::new()),
            call_depth: Cell::new(0),
            tx_scope: Cell::new(0),
            next_tx_scope: Cell::new(1),
            active_static_method_contexts: RefCell::new(Vec::new()),
            base_call_depth: 0,
            io_client: Rc::new(IoClient::new(Arc::clone(&config))),
            step_mode: false,                          // Default to non-step mode
            script_args: Vec::new(),                   // Initialize empty, will be set later
            web_servers: RefCell::new(HashMap::new()), // Initialize empty web servers map
            web_socket_servers: RefCell::new(HashMap::new()), // Initialize empty WebSocket servers map
            ws_connections: Arc::new(std::sync::Mutex::new(HashMap::new())), // Live WebSocket connections
            pending_responses: Rc::new(RefCell::new(HashMap::new())), // Initialize empty pending responses map
            server_response_streams: Rc::new(RefCell::new(HashMap::new())),
            open_response_streams: Rc::new(RefCell::new(Vec::new())),
            open_pending_requests: Rc::new(RefCell::new(Vec::new())),
            open_http_streams: Rc::new(RefCell::new(Arc::new(std::sync::Mutex::new(
                HashSet::new(),
            )))),
            accepted_request: Cell::new(false),
            next_response_stream_id: std::cell::Cell::new(1),
            config,
            current_source_file: RefCell::new(None), // No source file initially
            loading_stack: RefCell::new(Vec::new()), // Empty loading stack
            execute_depth: 0,
            // Test execution state
            test_mode: RefCell::new(false),
            test_results: RefCell::new(TestResults::default()),
            current_describe_stack: RefCell::new(Vec::new()),
            current_test_name: RefCell::new(None),
        }
    }

    pub fn with_timeout(seconds: u64) -> Self {
        let config = WflConfig {
            timeout_seconds: if seconds > 300 { 300 } else { seconds },
            ..Default::default()
        };
        Self::with_config(Arc::new(config))
    }

    pub fn set_step_mode(&mut self, step_mode: bool) {
        self.step_mode = step_mode;
    }

    /// The shared [`ExecutionBudget`] governing this run. Clone the handle to
    /// observe usage or to request cooperative cancellation via
    /// [`ExecutionBudget::cancel`] from another task/thread (the budget is
    /// `Send + Sync`).
    pub fn budget(&self) -> Arc<ExecutionBudget> {
        Arc::clone(&self.budget)
    }

    /// Install a fresh run budget while keeping the rest of the interpreter
    /// state (environment, definitions) intact. Used by the REPL to give each
    /// command its own wall-clock deadline and cancellation flag without
    /// discarding the session's variables.
    pub fn set_budget(&mut self, budget: Arc<ExecutionBudget>) {
        self.budget = budget;
    }

    /// A read-only handle to this interpreter's configuration.
    pub fn config(&self) -> &Arc<WflConfig> {
        &self.config
    }

    pub fn set_script_args(&mut self, args: Vec<String>) {
        self.script_args = args;
    }

    pub fn set_source_file(&mut self, path: PathBuf) {
        *self.current_source_file.borrow_mut() = Some(path);
    }

    /// Enable or disable test mode
    pub fn set_test_mode(&self, enabled: bool) {
        *self.test_mode.borrow_mut() = enabled;
    }

    /// Get test results after running in test mode
    pub fn get_test_results(&self) -> TestResults {
        self.test_results.borrow().clone()
    }

    /// Extract variables from the environment for module analyzer
    /// Returns a HashMap of variable names to (inferred type, is_mutable)
    fn extract_parent_variables(
        env: &Rc<RefCell<Environment>>,
    ) -> HashMap<String, (crate::parser::ast::Type, bool)> {
        let mut vars = HashMap::new();
        let env_borrowed = env.borrow();

        for (name, value) in &env_borrowed.values {
            // Skip native builtins (e.g. `year`, `month`, `day`, `length`, ...).
            // The analyzer already resolves these through `is_builtin_function`,
            // so seeding them as parent *variables* only makes an included file
            // stricter than the main file: an action-local `store year as ...`
            // would fatally conflict with the builtin's outer-scope binding even
            // though the same code runs fine in a main program (#557). Leaving
            // them out lets locals shadow builtins consistently in both paths.
            if matches!(value, Value::NativeFunction(_, _)) {
                continue;
            }
            let inferred_type = Self::infer_type_from_value(value);
            // Check if this variable is a constant (immutable)
            let is_mutable = !env_borrowed.constants.contains(name);
            vars.insert(name.clone(), (inferred_type, is_mutable));
        }

        // Also extract from parent scopes
        if let Some(parent_weak) = &env_borrowed.parent
            && let Some(parent_rc) = parent_weak.upgrade()
        {
            drop(env_borrowed); // Release borrow before recursive call
            let parent_vars = Self::extract_parent_variables(&parent_rc);
            // Parent variables are added first, can be shadowed by current scope
            for (name, (ty, is_mut)) in parent_vars {
                vars.entry(name).or_insert((ty, is_mut));
            }
        }

        vars
    }

    /// Infer AST Type from runtime Value
    fn infer_type_from_value(value: &Value) -> crate::parser::ast::Type {
        use crate::parser::ast::Type;

        match value {
            Value::Number(_) => Type::Number,
            Value::Text(_) => Type::Text,
            Value::Bool(_) => Type::Boolean,
            Value::List(_) => Type::List(Box::new(Type::Unknown)),
            Value::Object(_) => Type::Map(Box::new(Type::Text), Box::new(Type::Unknown)),
            Value::Function(_) | Value::Overloaded(_) => Type::Function {
                parameters: vec![],
                return_type: Box::new(Type::Unknown),
            },
            Value::Pattern(_) => Type::Pattern,
            Value::Date(_) => Type::Date,
            Value::Time(_) => Type::Time,
            Value::DateTime(_) => Type::DateTime,
            Value::ContainerDefinition(def) => Type::Container(def.name.clone()),
            Value::ContainerInstance(inst) => {
                Type::ContainerInstance(inst.borrow().container_type.clone())
            }
            Value::Null | Value::Nothing => Type::Nothing,
            _ => Type::Unknown,
        }
    }

    async fn resolve_module_path(
        &self,
        relative_path: &str,
        line: usize,
        column: usize,
    ) -> Result<PathBuf, RuntimeError> {
        // Extract and clone the Option<PathBuf> to avoid holding the borrow across await
        let opt_path = self.current_source_file.borrow().as_ref().cloned();

        let resolved = if let Some(source_path) = opt_path {
            let base_dir = source_path.parent().ok_or_else(|| {
                RuntimeError::new(
                    "Cannot determine parent directory of current file".to_string(),
                    line,
                    column,
                )
            })?;
            base_dir.join(relative_path)
        } else {
            let cwd = std::env::current_dir().map_err(|e| {
                RuntimeError::new(
                    format!("Cannot determine current directory: {}", e),
                    line,
                    column,
                )
            })?;
            cwd.join(relative_path)
        };

        // Canonicalize to handle . and .. and detect duplicates
        let canonical = tokio::fs::canonicalize(&resolved).await.map_err(|e| {
            RuntimeError::new(
                format!("Cannot resolve module path '{}': {}", relative_path, e),
                line,
                column,
            )
        })?;

        Ok(canonical)
    }

    fn check_circular_dependency(
        &self,
        path: &PathBuf,
        line: usize,
        column: usize,
    ) -> Result<(), RuntimeError> {
        let stack = self.loading_stack.borrow();

        if stack.contains(path) {
            let mut chain: Vec<String> = stack.iter().map(|p| p.display().to_string()).collect();
            chain.push(path.display().to_string());

            return Err(RuntimeError::new(
                format!("Circular dependency detected: {}", chain.join(" → ")),
                line,
                column,
            ));
        }

        Ok(())
    }

    fn dump_state(
        &self,
        stmt: &Statement,
        line: usize,
        _column: usize,
        env_before: &HashMap<String, Value>,
    ) {
        println!("Line {}: {}", line, Self::get_statement_text(stmt));

        let current_env = self.global_env.borrow();
        let mut changes = Vec::new();

        for (name, value) in current_env.values.iter() {
            if let Some(old_value) = env_before.get(name) {
                if !value.eq(old_value) {
                    changes.push(format!("{name} = {old_value} -> {value}"));
                }
            } else {
                changes.push(format!("{name} = {value}"));
            }
        }

        if !changes.is_empty() {
            println!("Variables changed:");
            for change in changes {
                println!("  {change}");
            }
        }

        let call_stack = self.get_call_stack();
        if !call_stack.is_empty() {
            println!("Call stack:");
            for frame in &call_stack {
                println!("  {} (line {})", frame.func_name, frame.call_line);
            }
        }
    }

    fn get_statement_text(stmt: &Statement) -> String {
        format!("{stmt:?}")
    }

    pub fn prompt_continue(&self) -> bool {
        loop {
            print!("continue (y/n)? ");
            if let Err(e) = io::stdout().flush() {
                eprintln!("Error flushing stdout: {e}");
            }

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let input = input.trim().to_lowercase();
                    match input.as_str() {
                        "y" => return true,
                        "n" => return false,
                        _ => {
                            println!("Invalid input. Please enter 'y' or 'n'.");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading input: {e}");
                    return false;
                }
            }
        }
    }

    pub fn get_call_stack(&self) -> Vec<CallFrame> {
        self.call_stack.borrow().clone()
    }

    pub fn clear_call_stack(&self) {
        self.call_stack.borrow_mut().clear();
    }
    pub fn global_env(&self) -> &Rc<RefCell<Environment>> {
        &self.global_env
    }

    /// Charge one interpreter operation against the shared budget. This counts
    /// the operation, honours cooperative cancellation, and — outside a
    /// `main loop` — enforces the operation ceiling and wall-clock deadline. The
    /// `main loop` exemption preserves the historic rule that a long-lived
    /// server loop never times out; cancellation still applies so a server can
    /// be stopped. Clock reads stay throttled to one per 1024 operations inside
    /// the budget, matching the previous `op_count & 1023` optimization.
    fn check_time(&self) -> Result<(), RuntimeError> {
        // Deadline/operation-ceiling exemption is driven by the shared budget's
        // main-loop depth (see `ExecutionBudget::enter_main_loop`): exempt while
        // any `main loop` is active on this run, so a long-lived server never
        // times out on its own uptime; cancellation still applies everywhere.
        let enforce_limits = !self.budget.is_deadline_exempt();
        match self.budget.charge_operation(enforce_limits) {
            Ok(()) => Ok(()),
            Err(exceeded) => Err(self.budget_error(exceeded, 0, 0)),
        }
    }

    /// Map a [`BudgetExceeded`] onto a `RuntimeError`.
    ///
    /// The deadline keeps its historic `[Timeout]` kind (and verbatim message)
    /// so existing timeout handling still matches; every other budget breach is
    /// a resource-limit error.
    ///
    /// This maps the breach to a kind but intentionally performs **no** state
    /// mutation. Every budget breach — including the deadline — is catchable by
    /// a general `try`/`when`, so the call stack, count-loop flags, and recursion
    /// depth must unwind *naturally*: `call_function` pops each frame as the
    /// error propagates and the RAII `CallDepthGuard` restores `call_depth`.
    /// Force-clearing state here would corrupt an enclosing count loop or
    /// under-count depth after a catch; `interpret()` resets everything for the
    /// next top-level run, so an uncaught terminal breach is fine too.
    fn budget_error(&self, exceeded: BudgetExceeded, line: usize, column: usize) -> RuntimeError {
        // Do NOT mutate interpreter state here. Every budget breach — deadline,
        // operation ceiling, recursion/import/execute-file depth, byte caps — is
        // catchable by a general `try`/`when`, so the call stack, count-loop
        // flags, and recursion depth must unwind *naturally* to leave a
        // consistent, resumable interpreter when the error is caught:
        // `call_function` pops each frame and the RAII `CallDepthGuard` restores
        // `call_depth` as the error propagates. Force-clearing that state (as the
        // historic timeout path did) corrupts an enclosing count loop or
        // under-counts depth after a catch. `interpret()` resets everything for
        // the next top-level run, so an *uncaught* terminal breach is fine too.
        let kind = match exceeded {
            BudgetExceeded::Deadline { .. } => ErrorKind::Timeout,
            _ => ErrorKind::ResourceLimit,
        };
        RuntimeError::with_kind(exceeded.message(), line, column, kind)
    }

    /// Attach WFL source information to an outbound HTTP error while
    /// preserving structured timeout/resource-limit kinds for `try`/`when`.
    fn http_client_error(
        &self,
        error: HttpClientError,
        line: usize,
        column: usize,
    ) -> RuntimeError {
        match error {
            HttpClientError::Request(message) => RuntimeError::new(message, line, column),
            HttpClientError::Budget(exceeded) => self.budget_error(exceeded, line, column),
            HttpClientError::Timeout { seconds } => RuntimeError::with_kind(
                format!("Outbound HTTP request exceeded timeout ({seconds}s)"),
                line,
                column,
                ErrorKind::Timeout,
            ),
            HttpClientError::Closed => {
                RuntimeError::new("Stream was closed while reading".to_string(), line, column)
            }
            HttpClientError::Disconnected => RuntimeError::with_kind(
                "Client disconnected; upstream read cancelled".to_string(),
                line,
                column,
                ErrorKind::Cancelled,
            ),
        }
    }

    /// Resolve a `wait for next chunk|line from <source>` operand to a stream
    /// handle id. Requires the streaming-response object bound by
    /// `stream response as <name>` (reads its internal `_stream` id); a bare text
    /// id is rejected so an arbitrary string can't be aimed at a stream handle.
    async fn resolve_stream_handle(
        &self,
        source: &Expression,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<String, RuntimeError> {
        let value = self.evaluate_expression(source, Rc::clone(env)).await?;
        match &value {
            Value::Object(obj) => match obj.borrow().get("_stream") {
                Some(Value::Text(id)) => Ok(id.to_string()),
                _ => Err(RuntimeError::new(
                    "Expected a streaming response handle (from `stream response as ...`), \
                     but this value has no open stream"
                        .to_string(),
                    line,
                    column,
                )),
            },
            _ => Err(RuntimeError::new(
                format!(
                    "Expected a streaming response handle (from `stream response as ...`), got {}",
                    value.type_name()
                ),
                line,
                column,
            )),
        }
    }

    /// Resolve a `write line|chunk`/`flush` operand to a server response-stream
    /// handle id. Requires the object bound by `start streaming response as ...`
    /// (reads its internal `_server_stream` id); a bare text id is rejected so an
    /// arbitrary string can't be aimed at a server response stream.
    async fn resolve_server_stream_handle(
        &self,
        target: &Expression,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<String, RuntimeError> {
        let value = self.evaluate_expression(target, Rc::clone(env)).await?;
        match &value {
            Value::Object(obj) => match obj.borrow().get("_server_stream") {
                Some(Value::Text(id)) => Ok(id.to_string()),
                _ => Err(RuntimeError::new(
                    "Expected a server response stream (from `start streaming response as ...`), \
                     but this value has no open stream"
                        .to_string(),
                    line,
                    column,
                )),
            },
            _ => Err(RuntimeError::new(
                format!(
                    "Expected a server response stream (from `start streaming response as ...`), got {}",
                    value.type_name()
                ),
                line,
                column,
            )),
        }
    }

    /// Whether the current execution context owns response stream `handle_id`:
    /// it appears in the currently-installed per-handler open list (the
    /// interpreter's own list under serial execution). Stream verbs require
    /// ownership while the stream is LIVE, so a handler holding a sibling's
    /// stream handle (e.g. through a shared global) cannot inject into, flush,
    /// or truncate the sibling's response (#642). Once a stream is closed
    /// (removed from the map and every open list) ownership is no longer
    /// determinable — write/flush then fail with the closed-stream error,
    /// while `close` is an idempotent no-op for any holder of the stale
    /// handle (there is no live response left to protect).
    fn owns_response_stream(&self, handle_id: &str) -> bool {
        self.open_response_streams
            .borrow()
            .iter()
            .any(|s| s == handle_id)
    }

    /// The ownership-failure error for a stream verb: distinguishes a stream
    /// that is live but owned by another handler from one that is simply no
    /// longer open (closed earlier, or its owner exited).
    fn response_stream_ownership_error(
        &self,
        verb: &str,
        handle_id: &str,
        line: usize,
        column: usize,
    ) -> RuntimeError {
        if self
            .server_response_streams
            .borrow()
            .contains_key(handle_id)
        {
            RuntimeError::new(
                format!("Cannot {verb} a response stream owned by another handler"),
                line,
                column,
            )
        } else {
            RuntimeError::new(
                format!("Cannot {verb} a closed response stream"),
                line,
                column,
            )
        }
    }

    /// Swap the interpreter's per-execution run-state fields with `state`.
    ///
    /// Used by [`IsolatedHandler`] to make the run state poll-local under
    /// concurrent execution: the interpreter's live fields and the parked
    /// snapshot trade places, so exactly one handler's run state is installed at
    /// a time. A plain field-by-field `mem::swap`, so it is its own inverse.
    fn swap_run_state(&self, state: &mut RunState) {
        std::mem::swap(
            &mut *self.current_count.borrow_mut(),
            &mut state.current_count,
        );
        std::mem::swap(
            &mut *self.in_count_loop.borrow_mut(),
            &mut state.in_count_loop,
        );
        let depth = self.call_depth.replace(state.call_depth);
        state.call_depth = depth;
        let scope = self.tx_scope.replace(state.tx_scope);
        state.tx_scope = scope;
        std::mem::swap(&mut *self.call_stack.borrow_mut(), &mut state.call_stack);
        std::mem::swap(
            &mut *self.active_static_method_contexts.borrow_mut(),
            &mut state.active_static_method_contexts,
        );
        std::mem::swap(
            &mut *self.current_block_overload_dups.borrow_mut(),
            &mut state.block_overload_dups,
        );
        std::mem::swap(
            &mut *self.open_response_streams.borrow_mut(),
            &mut state.open_response_streams,
        );
        std::mem::swap(
            &mut *self.open_pending_requests.borrow_mut(),
            &mut state.open_pending_requests,
        );
        std::mem::swap(
            &mut *self.open_http_streams.borrow_mut(),
            &mut state.open_http_streams,
        );
        let accepted = self.accepted_request.replace(state.accepted_request);
        state.accepted_request = accepted;
        io_capture::swap_stack(&mut state.capture_stack);
        std::mem::swap(
            &mut *self.current_source_file.borrow_mut(),
            &mut state.current_source_file,
        );
        std::mem::swap(
            &mut *self.loading_stack.borrow_mut(),
            &mut state.loading_stack,
        );
    }

    /// Build the initial [`RunState`] for a new concurrent handler: recursion
    /// accounting starts at the LIVE depth at loop entry — the enclosing
    /// action frames beneath a nested `main loop concurrently:` stay counted,
    /// mirroring `execute file`'s child seeding — and the handler inherits
    /// (clones of) the calling context's capture stack and module-loading
    /// context so captures, relative module paths, and cycle/import-depth
    /// checks behave as they would in the enclosing context (#642).
    fn fresh_handler_run_state(&self) -> RunState {
        let scope = self.next_tx_scope.get();
        self.next_tx_scope.set(scope.wrapping_add(1).max(1));
        RunState {
            // A fresh scope per handler: its transactions are its own, and an
            // unrelated handler naming the same database handle takes the pool.
            tx_scope: scope,
            call_depth: self.call_depth.get(),
            capture_stack: io_capture::snapshot_stack(),
            current_source_file: self.current_source_file.borrow().clone(),
            loading_stack: self.loading_stack.borrow().clone(),
            ..RunState::default()
        }
    }

    /// Drop each outbound streaming handle whose id is in `ids` from
    /// `IoClient.stream_handles`, cancelling its in-flight upstream request
    /// (dropping the reqwest body stream aborts the connection). Best-effort and
    /// synchronous (usable from `Drop`): if the async lock is momentarily held,
    /// the handles remain and are reclaimed at interpreter teardown. Idempotent —
    /// an id already removed by EOF/error/explicit `close` is a no-op.
    fn close_http_streams(&self, owner: &StreamOwner) {
        self.io_client.close_stream_owner(owner);
    }

    /// Build an RAII guard that closes any outbound stream handles still tracked
    /// as open when the guard drops — including when the `interpret()` future is
    /// dropped/cancelled before reaching its normal handler-exit/program-cleanup
    /// sites. On a normal run those sites have already drained the list, so the
    /// guard is a no-op; on a dropped future it releases the leaked upstreams
    /// (instead of leaking them until the interpreter itself is torn down).
    fn outbound_stream_cleanup_guard(&self) -> OutboundStreamCleanup {
        OutboundStreamCleanup {
            io_client: Rc::clone(&self.io_client),
            open_http_streams: Arc::clone(&self.open_http_streams.borrow()),
            open_response_streams: Rc::clone(&self.open_response_streams),
            server_response_streams: Rc::clone(&self.server_response_streams),
            open_pending_requests: Rc::clone(&self.open_pending_requests),
            pending_responses: Rc::clone(&self.pending_responses),
        }
    }

    /// Drain and drop every outbound stream the current (serial) handler left
    /// open. Called at the end of each serial `main loop` iteration and at program
    /// exit, mirroring the concurrent path's per-handler `Drop`.
    fn close_open_http_streams(&self) {
        let owner = Arc::clone(&self.open_http_streams.borrow());
        self.close_http_streams(&owner);
    }

    /// Stop tracking an outbound stream id as handler-owned — it has already left
    /// `IoClient.stream_handles` (EOF, error, or an explicit `close`), so the
    /// handler-exit cleanup must not try to (re-)drop it.
    fn untrack_http_stream(&self, handle_id: &str) {
        self.open_http_streams
            .borrow_mut()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(handle_id);
    }

    /// Clone the sender of every downstream response stream this handler owns. A
    /// client disconnect drops the stream's `Receiver`, so `Sender::closed()` on
    /// a clone resolves — a proactive disconnect signal that a blocked upstream
    /// read can select against, so a proxy handler wakes and cancels its upstream
    /// the moment the browser goes away, instead of only discovering it at the
    /// next downstream `write` or when the absolute stream deadline elapses.
    /// Returns owned clones (no `RefCell` borrow is held across the later await).
    fn downstream_disconnect_senders(&self) -> Vec<mpsc::Sender<Vec<u8>>> {
        let open = self.open_response_streams.borrow();
        if open.is_empty() {
            return Vec::new();
        }
        let map = self.server_response_streams.borrow();
        open.iter()
            .filter_map(|id| map.get(id).map(|stream| stream.tx.clone()))
            .collect()
    }

    /// Await until ANY of `senders`' downstream clients has disconnected (its
    /// `Receiver` dropped). Never resolves if `senders` is empty — the caller
    /// guards that case so `select!` still has a live branch.
    async fn any_downstream_disconnected(senders: Vec<mpsc::Sender<Vec<u8>>>) {
        if senders.is_empty() {
            std::future::pending::<()>().await;
            return;
        }
        let closes: Vec<_> = senders.iter().map(|tx| Box::pin(tx.closed())).collect();
        let _ = futures_util::future::select_all(closes).await;
    }

    /// Await until any of this handler's open pending requests has had its client
    /// disconnect — the transport route task drops the oneshot receiver when the
    /// client goes away, so the parked sender reports `is_closed()`. This is the
    /// disconnect signal that is valid BEFORE `start streaming response` (when no
    /// downstream response stream exists yet), so a handler blocked opening an
    /// upstream head can still be cancelled by a browser disconnect. Polled (the
    /// sender lives behind an `Arc<Mutex<Option<..>>>` shared with the transport,
    /// not an awaitable primitive); never resolves when the handler holds no
    /// pending request, so the caller's `select!` still has a live branch.
    async fn any_pending_request_disconnected(&self) {
        loop {
            // Compute in a tight scope so no `RefCell`/`Mutex` guard is held across
            // the await below. `None` => nothing to watch.
            let any_closed = {
                let open = self.open_pending_requests.borrow();
                if open.is_empty() {
                    None
                } else {
                    let pending = self.pending_responses.borrow();
                    Some(open.iter().any(|id| match pending.get(id) {
                        Some(p) => match p.sender.try_lock() {
                            Ok(guard) => guard.as_ref().is_some_and(|s| s.is_closed()),
                            // Being responded to right now — not a disconnect.
                            Err(_) => false,
                        },
                        // Owned by this handler yet absent from the map: the only
                        // removal that leaves an id in `open_pending_requests` is a
                        // sibling handler's `wait for request` global prune, which
                        // deletes ONLY closed (disconnected) senders — and a handler
                        // that answered its own request drops the id from
                        // `open_pending_requests` in the same step. So a missing
                        // owned id is a terminal disconnect, not "still connected";
                        // reporting it as connected here is exactly the bug where a
                        // parked handler waits out its timeout after a sibling pruned
                        // its since-disconnected entry.
                        None => true,
                    }))
                }
            };
            match any_closed {
                None => {
                    std::future::pending::<()>().await;
                    return;
                }
                Some(true) => return,
                Some(false) => {
                    tokio::time::sleep(REQUEST_DISCONNECT_POLL_INTERVAL).await;
                }
            }
        }
    }

    /// Await until this handler's client has disconnected by EITHER signal: an
    /// open downstream response stream's receiver dropped (post-`start streaming
    /// response`, event-driven) OR an open pending request's oneshot receiver
    /// dropped (pre-`start streaming response`, polled). Racing an upstream head
    /// open / body read against this cancels it the moment the browser goes away,
    /// whichever phase the handler is in.
    async fn any_client_disconnected(&self, senders: Vec<mpsc::Sender<Vec<u8>>>) {
        tokio::select! {
            _ = Self::any_downstream_disconnected(senders) => {}
            _ = self.any_pending_request_disconnected() => {}
        }
    }

    /// How long `close out` / end-of-handler drain will wait for the transport
    /// to finish the body after the sender is dropped. See
    /// [`RESPONSE_STREAM_FINISH_WAIT`] — a short fixed ceiling, not the write-
    /// path timeout (so a non-reading client cannot pin a serial main loop).
    fn response_stream_finish_timeout(&self) -> Duration {
        RESPONSE_STREAM_FINISH_WAIT
    }

    /// Drop the body sender for each id (ending the stream) without waiting for
    /// the transport to finish. Used only from `Drop` impls that cannot await.
    /// Prefer [`Self::close_response_streams`] on every async path so the
    /// terminating chunk is delivered before the program returns (#680).
    fn close_response_streams_nowait(&self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        let mut map = self.server_response_streams.borrow_mut();
        for id in ids {
            map.remove(id);
        }
    }

    /// Close each server response stream whose handle id is in `ids`: drop the
    /// sender (ending the body) and await the transport's finished signal so the
    /// terminating chunk has left the process before we return. Idempotent — an
    /// id already closed by an explicit `close out` (or a disconnect) is a
    /// no-op — so it is safe to call over a handler's full opened-stream list on
    /// exit.
    async fn close_response_streams(&self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        let mut finished = Vec::new();
        {
            let mut map = self.server_response_streams.borrow_mut();
            for id in ids {
                if let Some(stream) = map.remove(id)
                    && let Some(rx) = stream.finished
                {
                    finished.push(rx);
                }
                // Dropping `stream.tx` (via `stream` going out of scope) ends
                // the body; the transport then fires `finished` on Drop of the
                // body stream.
            }
        }
        if finished.is_empty() {
            return;
        }
        let timeout = self.response_stream_finish_timeout();
        // Wait for every body in parallel so multi-stream handlers don't pay
        // sequential timeouts, then bound the whole set.
        let _ = tokio::time::timeout(
            timeout,
            futures_util::future::join_all(finished.into_iter().map(|rx| async move {
                let _ = rx.await;
            })),
        )
        .await;
    }

    /// Drain and close every server response stream the current (serial) handler
    /// left open. Called at the end of each serial `main loop` iteration and at
    /// program exit, mirroring the concurrent path's per-handler `Drop` — but
    /// awaits body completion so a `break` right after the last chunk still
    /// delivers a clean end-of-stream (#680).
    async fn close_open_response_streams(&self) {
        let ids = std::mem::take(&mut *self.open_response_streams.borrow_mut());
        self.close_response_streams(&ids).await;
    }

    /// `execute [wfl] file at <path> [with <request>] [and read output as <var>]`.
    ///
    /// Kept as its own async fn (and only `Box::pin`ned from
    /// `_execute_statement`) so the nested lex→parse→analyze→interpret pipeline
    /// is not part of the giant shared statement state machine. Nesting this
    /// statement multiplies only this smaller future on the poll stack (#681).
    async fn execute_wfl_file(
        &self,
        path: &Expression,
        request: Option<&Expression>,
        variable_name: Option<&str>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Guard against a file that (directly or indirectly) executes itself,
        // using the shared budget's execute-file depth ceiling.
        if let Err(exceeded) = self.budget.check_execute_file_depth(self.execute_depth) {
            return Err(self.budget_error(exceeded, line, column));
        }

        // Evaluate path expression to string
        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str: String = match &path_value {
            Value::Text(s) => s.to_string(),
            _ => {
                return Err(RuntimeError::new(
                    format!(
                        "Execute file path must be text, got {}",
                        path_value.type_name()
                    ),
                    line,
                    column,
                ));
            }
        };

        // Resolve relative to the current script's directory (like load module),
        // mapping a missing file to FileNotFound so `when file not found` works
        let opt_source = self.current_source_file.borrow().as_ref().cloned();
        let joined = if let Some(source_path) = opt_source {
            source_path
                .parent()
                .map(|dir| dir.join(&path_str))
                .unwrap_or_else(|| PathBuf::from(&path_str))
        } else {
            let cwd = std::env::current_dir().map_err(|e| {
                RuntimeError::new(
                    format!("Cannot determine current directory: {e}"),
                    line,
                    column,
                )
            })?;
            cwd.join(&path_str)
        };
        let map_io_error = |e: std::io::Error| {
            let kind = match e.kind() {
                std::io::ErrorKind::NotFound => ErrorKind::FileNotFound,
                std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
                _ => ErrorKind::General,
            };
            RuntimeError::with_kind(
                format!("Cannot execute wfl file '{path_str}': {e}"),
                line,
                column,
                kind,
            )
        };
        let resolved_path = tokio::fs::canonicalize(&joined)
            .await
            .map_err(map_io_error)?;
        // Read under the shared source-size ceiling (bounded read).
        let content = self
            .read_source_bounded(&resolved_path, line, column)
            .await?;

        // Evaluate the optional request context and extract the variables that
        // `wait for request` defines, so the executed file sees the same names.
        // Validate the shape upfront so a wrong object fails here with a clear
        // message instead of as confusing undefined variable errors inside the
        // executed file.
        let request_vars: Vec<(String, Value)> = if let Some(request_expr) = request {
            let request_value = self
                .evaluate_expression(request_expr, Rc::clone(&env))
                .await?;
            match &request_value {
                Value::Object(props) => {
                    let props = props.borrow();
                    let mut vars = Vec::new();
                    for key in ["method", "path", "query", "client_ip", "body", "headers"] {
                        let value = props.get(key).ok_or_else(|| {
                            RuntimeError::new(
                                format!(
                                    "Execute file request context is missing '{key}' - pass the request object from 'wait for request'"
                                ),
                                line,
                                column,
                            )
                        })?;
                        let type_ok = match key {
                            "headers" => matches!(value, Value::Object(_)),
                            _ => matches!(value, Value::Text(_)),
                        };
                        if !type_ok {
                            return Err(RuntimeError::new(
                                format!(
                                    "Execute file request context field '{key}' must be {}, got {}",
                                    if key == "headers" {
                                        "an object"
                                    } else {
                                        "text"
                                    },
                                    value.type_name()
                                ),
                                line,
                                column,
                            ));
                        }
                        // Deep clone so the executed file cannot mutate the
                        // parent's request data (e.g. the headers object)
                        vars.push((key.to_string(), value.deep_clone()));
                    }
                    vars
                }
                _ => {
                    return Err(RuntimeError::new(
                        format!(
                            "Execute file request context must be a request object, got {}",
                            request_value.type_name()
                        ),
                        line,
                        column,
                    ));
                }
            }
        } else {
            Vec::new()
        };

        // Parse the file; errors are catchable in the parent
        use crate::lexer::lex_wfl_with_positions_checked;
        use crate::parser::Parser;

        // Lex under the shared run budget: a deadline / cancellation /
        // operation breach during nested source loading surfaces as a
        // typed, catchable runtime error instead of a truncated token
        // stream that could execute as if it were the whole file.
        let tokens = lex_wfl_with_positions_checked(&content)
            .map_err(|exceeded| self.budget_error(exceeded, line, column))?;
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().map_err(|errors| {
            let first_error = errors.first();
            RuntimeError::new(
                format!(
                    "Parse error in executed file '{}' (line {}, column {}): {}",
                    resolved_path.display(),
                    first_error.map(|e| e.line).unwrap_or(1),
                    first_error.map(|e| e.column).unwrap_or(1),
                    first_error.map(|e| e.message.as_str()).unwrap_or("unknown")
                ),
                line,
                column,
            )
        })?;

        // Analyze semantics, seeding the injected request variable names.
        // The type checker is intentionally skipped: main.rs treats type
        // errors as warnings only, so a hard gate here would reject files
        // that run fine standalone.
        use crate::analyzer::Analyzer;

        let mut seeded_vars: HashMap<String, (crate::parser::ast::Type, bool)> = HashMap::new();
        for (name, value) in &request_vars {
            seeded_vars.insert(name.clone(), (Self::infer_type_from_value(value), true));
        }
        let mut analyzer = Analyzer::with_parent_variables(seeded_vars);
        if let Err(errors) = analyzer.analyze(&program) {
            let first_error = errors.first();
            return Err(RuntimeError::new(
                format!(
                    "Semantic error in executed file '{}': {}",
                    resolved_path.display(),
                    first_error.map(|e| e.to_string()).unwrap_or_default()
                ),
                line,
                column,
            ));
        }

        // Run the file in a fresh nested interpreter (own global env and
        // stdlib, inherits the parent's config) with request context injected
        let mut child = Interpreter::with_config(Arc::clone(&self.config));
        child.set_source_file(resolved_path.clone());
        child.execute_depth = self.execute_depth + 1;
        // Share the parent's budget so the deadline, operation ceiling,
        // and cancellation span the whole run — otherwise splitting work
        // across `execute file` calls would reset them and evade the cap.
        child.budget = Arc::clone(&self.budget);
        // Seed the child's recursion accounting with the parent's live
        // depth so the combined WFL call depth across nested `execute
        // file` runs is bounded by `max_call_depth` (not multiplied per
        // level), preventing native-stack overflow before the guard fires.
        child.base_call_depth = self.call_depth.get();

        {
            let mut child_env = child.global_env().borrow_mut();
            for (name, value) in request_vars {
                if let Err(msg) = child_env.define(&name, value) {
                    return Err(RuntimeError::new(msg, line, column));
                }
            }
        }

        // With an output clause, capture the child's display/print output;
        // without one, child output flows to the current sink (stdout, or
        // the parent's own capture buffer if the parent is being captured)
        let capture_buffer = variable_name.map(|_| Rc::new(RefCell::new(String::new())));
        // The child shares this budget, so the parent's active main-loop
        // exemption (a depth counter, not a flag) naturally covers the
        // child and the nested front end — `execute file` from inside a
        // server's `main loop` handler inherits the exemption instead of
        // spuriously timing out, and the RAII guard needs no save/restore.
        let run_result = {
            let _guard = capture_buffer
                .as_ref()
                .map(|buffer| io_capture::push_capture(Rc::clone(buffer)));
            // Box::pin breaks the recursive future (this method awaits a full
            // nested interpret), keeping the future finitely sized
            Box::pin(child.interpret(&program)).await
        };

        if let Err(errors) = run_result {
            let first = errors
                .into_iter()
                .next()
                .unwrap_or_else(|| RuntimeError::new("unknown error".to_string(), line, column));
            // Position the error at the parent's execute statement, keep the
            // child's error kind so typed `when` clauses still match
            return Err(RuntimeError::with_kind(
                format!(
                    "Error in executed file '{}' (line {}): {}",
                    resolved_path.display(),
                    first.line,
                    first.message
                ),
                line,
                column,
                first.kind,
            ));
        }

        if let (Some(var_name), Some(buffer)) = (variable_name, capture_buffer) {
            let output = buffer.borrow();
            env.borrow_mut()
                .define_direct(var_name, Value::Text(Arc::from(output.as_str())))
                .map_err(|e| RuntimeError::new(e, line, column))?;
        }

        Ok((Value::Null, ControlFlow::None))
    }

    fn pending_response_disconnected_now(&self, request_id: &str) -> bool {
        let owned = self
            .open_pending_requests
            .borrow()
            .iter()
            .any(|id| id == request_id);
        if !owned {
            return false;
        }

        let pending = self.pending_responses.borrow();
        match pending.get(request_id) {
            Some(entry) => match entry.sender.try_lock() {
                Ok(sender) => sender.as_ref().is_none_or(|sender| sender.is_closed()),
                Err(_) => false,
            },
            None => true,
        }
    }

    /// Wait for one specific still-owned request to disconnect. Losing ownership
    /// is deliberately not treated as cancellation: duplicate/forged responses
    /// must retain their established general-error classification.
    async fn pending_response_disconnected(&self, request_id: &str) {
        loop {
            if self.pending_response_disconnected_now(request_id) {
                return;
            }
            if !self
                .open_pending_requests
                .borrow()
                .iter()
                .any(|id| id == request_id)
            {
                std::future::pending::<()>().await;
                return;
            }
            tokio::time::sleep(REQUEST_DISCONNECT_POLL_INTERVAL).await;
        }
    }

    fn first_pending_response_disconnected_now(&self, request_ids: &[String]) -> Option<String> {
        request_ids
            .iter()
            .find(|request_id| self.pending_response_disconnected_now(request_id))
            .cloned()
    }

    /// A response request operand can itself be asynchronous, so its target id
    /// is not available until evaluation finishes. Watch only the pending
    /// requests owned when evaluation starts; requests accepted by nested work
    /// are cleaned as newly-created resources if this evaluation is cancelled.
    async fn first_pending_response_disconnected(&self, request_ids: &[String]) -> String {
        loop {
            if let Some(request_id) = self.first_pending_response_disconnected_now(request_ids) {
                return request_id;
            }
            let any_still_owned = {
                let owned = self.open_pending_requests.borrow();
                request_ids
                    .iter()
                    .any(|request_id| owned.iter().any(|id| id == request_id))
            };
            if !any_still_owned {
                std::future::pending::<()>().await;
            }
            tokio::time::sleep(REQUEST_DISCONNECT_POLL_INTERVAL).await;
        }
    }

    fn response_precommit_snapshot(&self) -> ResponsePrecommitSnapshot {
        let http_owner = Arc::clone(&self.open_http_streams.borrow());
        let http_streams = http_owner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        ResponsePrecommitSnapshot {
            call_stack: self.call_stack.borrow().clone(),
            call_depth: self.call_depth.get(),
            current_count: *self.current_count.borrow(),
            in_count_loop: *self.in_count_loop.borrow(),
            http_owner,
            http_streams,
            response_streams: self
                .open_response_streams
                .borrow()
                .iter()
                .cloned()
                .collect(),
            pending_requests: self
                .open_pending_requests
                .borrow()
                .iter()
                .cloned()
                .collect(),
        }
    }

    /// Restore poll-local state and release only resources opened by the
    /// cancelled evaluation. Existing handler resources remain owned.
    fn cancel_response_precommit(&self, request_id: &str, snapshot: ResponsePrecommitSnapshot) {
        *self.call_stack.borrow_mut() = snapshot.call_stack;
        self.call_depth.set(snapshot.call_depth);
        *self.current_count.borrow_mut() = snapshot.current_count;
        *self.in_count_loop.borrow_mut() = snapshot.in_count_loop;

        let new_http_streams: Vec<String> = snapshot
            .http_owner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .filter(|id| !snapshot.http_streams.contains(*id))
            .cloned()
            .collect();
        self.io_client.close_stream_ids(&new_http_streams);

        let new_response_streams: Vec<String> = self
            .open_response_streams
            .borrow()
            .iter()
            .filter(|id| !snapshot.response_streams.contains(*id))
            .cloned()
            .collect();
        // Cancel path: drop senders immediately (client is already gone or the
        // pre-commit work is abandoned). No finish wait — there is no successful
        // body left to deliver.
        self.close_response_streams_nowait(&new_response_streams);
        self.open_response_streams
            .borrow_mut()
            .retain(|id| snapshot.response_streams.contains(id));

        let new_pending_requests: Vec<String> = self
            .open_pending_requests
            .borrow()
            .iter()
            .filter(|id| !snapshot.pending_requests.contains(*id))
            .cloned()
            .collect();
        self.fail_unanswered_requests(&new_pending_requests);
        self.open_pending_requests
            .borrow_mut()
            .retain(|id| id != request_id && snapshot.pending_requests.contains(id));
        self.pending_responses.borrow_mut().remove(request_id);
    }

    /// Evaluate the response request operand while racing every request this
    /// handler already owns. The target request id is not known until the
    /// operand resolves, so this establishes the response-attempt snapshot that
    /// is carried through field evaluation and commit.
    async fn evaluate_response_request<T, F>(
        &self,
        line: usize,
        column: usize,
        disconnect_message: &'static str,
        evaluation: F,
    ) -> Result<(T, ResponsePrecommitSnapshot), RuntimeError>
    where
        F: std::future::Future<Output = Result<T, RuntimeError>>,
    {
        let snapshot = self.response_precommit_snapshot();
        let mut watched_requests: Vec<String> = snapshot.pending_requests.iter().cloned().collect();
        watched_requests.sort();
        let mut evaluation = Box::pin(evaluation);
        let mut disconnected =
            Box::pin(self.first_pending_response_disconnected(&watched_requests));
        let outcome = tokio::select! {
            biased;
            request_id = disconnected.as_mut() => (None, Some(request_id)),
            result = evaluation.as_mut() => (Some(result), None),
        };
        let disconnected_request = outcome
            .1
            .or_else(|| self.first_pending_response_disconnected_now(&watched_requests));

        if let Some(result) = outcome.0
            && disconnected_request.is_none()
        {
            drop(evaluation);
            drop(disconnected);
            return result.map(|value| (value, snapshot));
        }

        // Drop first so action/loop RAII runs before restoring the baseline.
        drop(evaluation);
        drop(disconnected);
        let request_id =
            disconnected_request.expect("disconnect watcher completed without a request id");
        self.cancel_response_precommit(&request_id, snapshot);
        Err(RuntimeError::with_kind(
            disconnect_message.to_string(),
            line,
            column,
            ErrorKind::Cancelled,
        ))
    }

    /// Evaluate every fallible response field while racing the target request's
    /// disconnect signal. Disconnect wins ties, and the evaluation future is
    /// dropped before partially-entered interpreter state is restored.
    async fn evaluate_response_precommit<T, F>(
        &self,
        request_id: &str,
        line: usize,
        column: usize,
        disconnect_message: &'static str,
        snapshot: ResponsePrecommitSnapshot,
        evaluation: F,
    ) -> Result<(T, ResponsePrecommitSnapshot), RuntimeError>
    where
        F: std::future::Future<Output = Result<T, RuntimeError>>,
    {
        if let Err(error) = self
            .ensure_pending_response_owned(request_id, line, column)
            .await
        {
            if error.kind == ErrorKind::Cancelled {
                self.cancel_response_precommit(request_id, snapshot);
            }
            return Err(error);
        }
        let mut evaluation = Box::pin(evaluation);
        let mut disconnected = Box::pin(self.pending_response_disconnected(request_id));
        let outcome = tokio::select! {
            biased;
            _ = disconnected.as_mut() => None,
            result = evaluation.as_mut() => Some(result),
        };
        let disconnected_now = self.pending_response_disconnected_now(request_id);

        if let Some(result) = outcome
            && !disconnected_now
        {
            drop(evaluation);
            drop(disconnected);
            return result.map(|value| (value, snapshot));
        }

        // This ordering is intentional: dropping the future runs RAII guards
        // before we overwrite any manually-restored run-state fields.
        drop(evaluation);
        drop(disconnected);
        self.cancel_response_precommit(request_id, snapshot);
        Err(RuntimeError::with_kind(
            disconnect_message.to_string(),
            line,
            column,
            ErrorKind::Cancelled,
        ))
    }

    /// Take a pending response sender into an RAII completion guard for
    /// `respond` / `start streaming response`.
    ///
    /// Ownership is checked against `open_pending_requests` *before* clearing the
    /// id: a sibling `wait for request` may globally prune a closed (client-
    /// disconnected) sender, so the map entry is missing while this handler still
    /// owns the request. That path is `ErrorKind::Cancelled`. A missing entry when
    /// the handler no longer owns the id (duplicate respond after a successful
    /// one, or a forged request id) remains a general error.
    /// Check that this handler still owns `request_id` and a pending entry is
    /// present (or was pruned as disconnected → Cancelled), WITHOUT taking the
    /// sender. Used before evaluating respond/stream-head expressions so the
    /// parked sender remains a disconnect signal for upstream work.
    async fn ensure_pending_response_owned(
        &self,
        request_id: &str,
        line: usize,
        column: usize,
    ) -> Result<(), RuntimeError> {
        let was_owned = self
            .open_pending_requests
            .borrow()
            .iter()
            .any(|id| id == request_id);
        if !was_owned {
            return Err(RuntimeError::new(
                "Request ID not found - response may have already been sent".to_string(),
                line,
                column,
            ));
        }
        let present = self.pending_responses.borrow().contains_key(request_id);
        if !present {
            // Sibling prune of a closed sender while we still own the request.
            return Err(RuntimeError::with_kind(
                "Client disconnected before the response was sent".to_string(),
                line,
                column,
                ErrorKind::Cancelled,
            ));
        }
        // Optionally check is_closed early for a clearer cancel before heavy eval.
        let closed = {
            let pending = self.pending_responses.borrow();
            match pending.get(request_id) {
                Some(p) => match p.sender.try_lock() {
                    Ok(g) => g.as_ref().is_none_or(|s| s.is_closed()),
                    Err(_) => false,
                },
                None => true,
            }
        };
        if closed {
            return Err(RuntimeError::with_kind(
                "Client disconnected before the response was sent".to_string(),
                line,
                column,
                ErrorKind::Cancelled,
            ));
        }
        Ok(())
    }

    async fn take_pending_response_completion(
        &self,
        request_id: &str,
        line: usize,
        column: usize,
    ) -> Result<ResponseCompletion, RuntimeError> {
        let was_owned = self
            .open_pending_requests
            .borrow()
            .iter()
            .any(|id| id == request_id);
        let pending_entry = {
            let mut pending = self.pending_responses.borrow_mut();
            pending.remove(request_id)
        };
        // Answered (or definitively cancelled) now: drop it from the handler's
        // unanswered-request tracking so the exit-time 500 fallback skips it.
        self.open_pending_requests
            .borrow_mut()
            .retain(|id| id != request_id);
        match pending_entry {
            // The admission slot is released by the transport task when it
            // finishes delivering this response (or on its timeout), so the
            // completion guard carries only the response channel.
            Some(entry) => match entry.sender.lock().await.take() {
                Some(sender) => Ok(ResponseCompletion {
                    sender: Some(sender),
                }),
                None => Err(RuntimeError::new(
                    "Response already sent for this request".to_string(),
                    line,
                    column,
                )),
            },
            None if was_owned => {
                // Sibling prune removed a closed sender, or the entry otherwise
                // vanished while this handler still owned the request — treat as
                // client disconnect / cooperative cancellation.
                Err(RuntimeError::with_kind(
                    "Client disconnected before the response was sent".to_string(),
                    line,
                    column,
                    ErrorKind::Cancelled,
                ))
            }
            None => Err(RuntimeError::new(
                "Request ID not found - response may have already been sent".to_string(),
                line,
                column,
            )),
        }
    }

    /// Answer 500 for each request id in `ids` that is still unanswered (its
    /// sender is still parked in `pending_responses`). A request the handler
    /// already answered is gone from the map, so its `remove` is a no-op —
    /// idempotent and safe to call over a handler's full dequeued list on exit.
    ///
    /// Fully synchronous (a non-blocking `try_lock` plus a `oneshot` send) so it
    /// runs from `IsolatedHandler`'s `Drop`. The sender's mutex is only ever held
    /// during `respond`, which the finished handler is no longer inside, so the
    /// `try_lock` succeeds; if it somehow does not, the transport's request
    /// timeout remains the backstop.
    fn fail_unanswered_requests(&self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        let mut pending = self.pending_responses.borrow_mut();
        for id in ids {
            if let Some(entry) = pending.remove(id)
                && let Ok(mut guard) = entry.sender.try_lock()
                && let Some(sender) = guard.take()
            {
                let _ = sender.send(HandlerReply::Buffered(WflHttpResponse {
                    content: b"Internal Server Error\n".to_vec(),
                    status: 500,
                    content_type: "text/plain; charset=utf-8".to_string(),
                    headers: HashMap::new(),
                }));
            }
        }
    }

    /// Drain and 500 any requests the current (serial) handler dequeued but never
    /// answered. Called at the end of each serial `main loop` iteration and at
    /// program exit, mirroring the concurrent path's per-handler `Drop`.
    fn fail_open_pending_requests(&self) {
        let ids = std::mem::take(&mut *self.open_pending_requests.borrow_mut());
        self.fail_unanswered_requests(&ids);
    }

    /// Execute a `main loop concurrently:` body. Keeps up to
    /// `CONCURRENT_HANDLER_LIMIT` iterations of `body` in flight at once, each in
    /// its own isolated child scope, driven cooperatively on this single thread
    /// (no threads, no `Send`/`Arc` across the interpreter core). A handler that
    /// errors or panics is contained — its request is resolved with 500 by the
    /// response-completion drop guard — and its siblings keep running.
    ///
    /// Each handler also carries an isolated [`RunState`] (count-loop variable,
    /// recursion depth, call stack, block overload set) via [`IsolatedHandler`],
    /// so one handler's loop/recursion bookkeeping can never leak into another
    /// across an `await`.
    async fn execute_concurrent_main_loop(
        &self,
        body: &[Statement],
        env: &Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        use futures_util::FutureExt;
        use futures_util::stream::{FuturesUnordered, StreamExt};

        let cap = CONCURRENT_HANDLER_LIMIT.max(1);
        let mut futs = FuturesUnordered::new();
        let mut last_value = Value::Null;

        // A handler that fails *before* it ever awaits (a deterministic bad
        // expression, or `wait for request` on a server that was closed without
        // the loop breaking) completes instantly, so a naive refill-and-repoll
        // would spin the CPU (and the log) forever — and this loop is exempt from
        // the wall-clock deadline. Count consecutive failures with no successful
        // iteration in between: back off between them, and terminate the loop once
        // it's clearly a structural failure rather than incidental handler errors.
        let mut consecutive_failures: u32 = 0;

        loop {
            self.check_time()?;

            // Refill to the concurrency cap. Each iteration gets a fresh isolated
            // scope so concurrent requests never clobber each other's variables,
            // and a fresh `RunState` (wrapped by `IsolatedHandler`) so their
            // count-loop/recursion/call-stack bookkeeping stays poll-local.
            while futs.len() < cap {
                let scope = Environment::new_child_env(env);
                let handler =
                    std::panic::AssertUnwindSafe(self.execute_block(body, scope)).catch_unwind();
                futs.push(IsolatedHandler {
                    interp: self,
                    state: self.fresh_handler_run_state(),
                    inner: Some(Box::pin(handler)),
                    drain: None,
                    pending_output: None,
                    pending_finish_ids: Vec::new(),
                });
            }

            // With cap >= 1 the set is never empty, so `next()` never returns a
            // `Ready(None)` that would busy-spin the loop.
            match futs.next().await {
                // `IsolatedHandler` yields `(inner, accepted_request)` so we can
                // tell request-local outcomes from structural pre-request failures
                // after the handler's run state (and open-pending list) is gone.
                Some((Ok(Ok((value, flow))), _accepted)) => {
                    // A completed iteration (request handled) — not a failure.
                    consecutive_failures = 0;
                    last_value = value;
                    match flow {
                        ControlFlow::Break => break,
                        ControlFlow::Exit => return Ok((last_value, ControlFlow::Exit)),
                        ControlFlow::Return(val) => {
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::Continue | ControlFlow::None => {}
                    }
                }
                // Expected / request-local outcomes must NEVER feed the structural
                // consecutive-failure breaker:
                // - `Cancelled`: client disconnect (cooperative cancellation)
                // - finite `wait for request ... with timeout` expiry ONLY
                //   (not every ErrorKind::Timeout — e.g. pre-request structural
                //   timeouts still feed the breaker)
                // - any error from a handler that already accepted a request
                //
                // Structural pre-request failures feed the breaker.
                Some((Ok(Err(err)), accepted)) => {
                    // `exit program` stops the server, so it unwinds out of the
                    // loop instead of being logged and retried.
                    if err.is_exit_program() {
                        return Err(err);
                    }
                    match classify_concurrent_handler_error(&err, accepted) {
                        ConcurrentHandlerDisposition::RequestLocal => {
                            log::debug!(
                                "concurrent main loop: non-structural handler outcome \
                                 (kind={:?}, accepted_request={accepted}): {err}",
                                err.kind
                            );
                        }
                        ConcurrentHandlerDisposition::Structural => {
                            log::warn!("concurrent main loop: structural handler error: {err}");
                            if self
                                .backoff_or_break_concurrent(
                                    &mut consecutive_failures,
                                    MAX_CONSECUTIVE_HANDLER_FAILURES,
                                )
                                .await
                            {
                                break;
                            }
                        }
                    }
                }
                // Panic after accepting a request is contained and the request is
                // answered 500 by the drop guard — request-local, not structural.
                // Panic before accepting a request can hot-spin; count it toward
                // the structural breaker.
                Some((Err(_panic), accepted)) => {
                    if accepted {
                        log::warn!(
                            "concurrent main loop: handler panicked after accepting a request; \
                             request answered 500"
                        );
                    } else {
                        log::warn!(
                            "concurrent main loop: handler panicked before accepting a request"
                        );
                        if self
                            .backoff_or_break_concurrent(
                                &mut consecutive_failures,
                                MAX_CONSECUTIVE_HANDLER_FAILURES,
                            )
                            .await
                        {
                            break;
                        }
                    }
                }
                None => break, // unreachable while cap >= 1; end cleanly if reached
            }
        }

        Ok((last_value, ControlFlow::None))
    }

    /// Handle a failed concurrent-handler iteration: bump the consecutive-failure
    /// counter, back off proportionally (capped) so instant failures cannot hot-
    /// spin the CPU/log while the loop is deadline-exempt, and report whether the
    /// caller should break the loop because failures have crossed the structural-
    /// failure threshold (e.g. the server was closed without the loop breaking).
    /// Returns `true` to break.
    async fn backoff_or_break_concurrent(&self, consecutive_failures: &mut u32, max: u32) -> bool {
        *consecutive_failures += 1;
        if *consecutive_failures >= max {
            log::error!(
                "concurrent main loop: {consecutive_failures} consecutive handler failures with no successful request; stopping the loop to avoid a hot spin"
            );
            return true;
        }
        // Small, capped backoff so a burst of instant failures yields the thread
        // (and rate-limits the log) instead of spinning.
        let backoff_ms = (*consecutive_failures).min(50) as u64;
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        false
    }

    /// Preserve ordinary file I/O failures while classifying byte-ceiling
    /// breaches as catchable execution-budget resource errors.
    fn file_read_error(&self, error: FileReadError, line: usize, column: usize) -> RuntimeError {
        match error {
            FileReadError::Io(message) => RuntimeError::new(message, line, column),
            FileReadError::Budget(exceeded) => self.budget_error(exceeded, line, column),
        }
    }

    /// Map a pattern-VM error onto a `RuntimeError`. Budget breaches (step/state
    /// ceilings, cancellation) surface as catchable `ResourceLimit` errors so a
    /// ReDoS/cancellation during matching is not silently collapsed into a
    /// non-match; structural pattern errors surface as general runtime errors.
    fn pattern_error(
        &self,
        err: crate::pattern::PatternError,
        line: usize,
        column: usize,
    ) -> RuntimeError {
        use crate::pattern::PatternError;
        let kind = match err {
            // A pattern that outruns the wall-clock deadline is a timeout, with
            // the historic `[Timeout]` kind and message, so existing timeout
            // handling/tests keep matching.
            PatternError::Timeout { .. } => ErrorKind::Timeout,
            PatternError::StepLimitExceeded
            | PatternError::StateLimitExceeded
            | PatternError::Cancelled => ErrorKind::ResourceLimit,
            _ => ErrorKind::General,
        };
        RuntimeError::with_kind(err.to_string(), line, column, kind)
    }

    /// Read a WFL source file (`load module`, `include from`, `execute file`)
    /// under the shared source-size ceiling. Reads at most `max_source_size + 1`
    /// bytes, so an oversized file is refused without ever allocating the whole
    /// thing — this holds even when the file's metadata is unavailable, stale,
    /// or reports `0` (special files), which a metadata-only check would miss.
    async fn read_source_bounded(
        &self,
        path: &std::path::Path,
        line: usize,
        column: usize,
    ) -> Result<String, RuntimeError> {
        use tokio::io::AsyncReadExt;

        let io_err = |e: std::io::Error| {
            let kind = match e.kind() {
                std::io::ErrorKind::NotFound => ErrorKind::FileNotFound,
                std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
                _ => ErrorKind::General,
            };
            RuntimeError::with_kind(
                format!("Cannot read source file '{}': {e}", path.display()),
                line,
                column,
                kind,
            )
        };

        let max = self.budget.max_source_bytes();
        // Read one byte past the limit so exceeding it is detectable; the buffer
        // never grows beyond `max + 1`.
        let read_cap = (max as u64).saturating_add(1);
        let file = tokio::fs::File::open(path).await.map_err(io_err)?;
        let mut buf = Vec::new();
        file.take(read_cap)
            .read_to_end(&mut buf)
            .await
            .map_err(io_err)?;

        if let Err(exceeded) = self.budget.check_source_bytes(buf.len()) {
            return Err(self.budget_error(exceeded, line, column));
        }

        String::from_utf8(buf).map_err(|_| {
            RuntimeError::new(
                format!("Source file '{}' is not valid UTF-8", path.display()),
                line,
                column,
            )
        })
    }

    fn assert_invariants(&self) {
        debug_assert_eq!(
            *self.in_count_loop.borrow(),
            self.current_count.borrow().is_some()
        );

        debug_assert!(self.call_stack.borrow().len() < 10_000);
    }

    fn native_display(args: Vec<Value>) -> Result<Value, RuntimeError> {
        let mut line = String::new();
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                line.push(' ');
            }
            line.push_str(&arg.to_string());
        }
        io_capture::emit_line(&line);
        Ok(Value::Null)
    }

    pub async fn interpret(&mut self, program: &Program) -> Result<Value, Vec<RuntimeError>> {
        // Scope this run's budget as the TASK-local current budget, so leaf
        // helpers with no budget parameter (the stdlib pattern builtins in
        // particular) match under the run's configured ceilings and shared
        // meters — and, crucially, so a library embedder that interleaves two
        // interpreter futures on one thread never sees the other's budget or
        // restores stale state across an `.await`. An `execute file` child that
        // calls `interpret` again nests its own scope (same or child budget).
        ExecutionBudget::scope(Arc::clone(&self.budget), self.interpret_inner(program)).await
    }

    /// Action names defined more than once in `statements` — the immediate
    /// slice only, matching the same-scope overloading rule: two same-name
    /// definitions in different blocks are independent actions, not
    /// overloads. Returns `None` (no allocation kept) when nothing in the
    /// slice is overloaded.
    fn scan_block_overload_dups(
        statements: &[Statement],
    ) -> Option<Rc<std::collections::HashSet<String>>> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        let mut dups: std::collections::HashSet<String> = std::collections::HashSet::new();
        for statement in statements {
            if let Statement::ActionDefinition { name, .. } = statement {
                let entry = counts.entry(name.as_str()).or_insert(0);
                *entry += 1;
                if *entry == 2 {
                    dups.insert(name.clone());
                }
            }
        }
        if dups.is_empty() {
            None
        } else {
            Some(Rc::new(dups))
        }
    }

    /// Speculatively arms `enforce_param_types` on any same-scope function
    /// that `statements` will merge a new definition into — the merge makes
    /// that member overloaded, so its temporal window starts when the block
    /// starts executing, not at the later merge. Inherited (parent-scope)
    /// names are not mergeable — defining over them is a shadowing error —
    /// so only the local scope is consulted. The returned guard reverts the
    /// arming on drop for members whose merge never actually executed.
    fn arm_block_members(
        statements: &[Statement],
        env: &Rc<RefCell<Environment>>,
    ) -> ArmedEnforcementGuard {
        let mut armed = Vec::new();
        for statement in statements {
            if let Statement::ActionDefinition { name, .. } = statement
                && let Some(Value::Function(existing)) = env.borrow().get_local(name)
            {
                let prior_enforce = existing.enforce_param_types.get();
                existing.enforce_param_types.set(true);
                armed.push(ArmedMember {
                    env: Rc::clone(env),
                    name: name.clone(),
                    member: existing,
                    prior_enforce,
                });
            }
        }
        ArmedEnforcementGuard { armed }
    }

    /// Enters a statement block for overload enforcement: computes the
    /// block's own duplicate set and speculatively arms mergeable members
    /// (see [`Self::arm_block_members`]). Both guards restore on drop.
    fn enter_block_overloads(
        &self,
        statements: &[Statement],
        env: &Rc<RefCell<Environment>>,
    ) -> (BlockDupsScope<'_>, ArmedEnforcementGuard) {
        let armed = Self::arm_block_members(statements, env);
        (
            BlockDupsScope::enter(
                &self.current_block_overload_dups,
                Self::scan_block_overload_dups(statements),
            ),
            armed,
        )
    }

    /// The interpreter run body, executed inside the task-local budget scope
    /// established by [`Interpreter::interpret`].
    async fn interpret_inner(&mut self, program: &Program) -> Result<Value, Vec<RuntimeError>> {
        // Reset per-run enforcement/loop state first, so a prior *terminal*
        // budget breach (e.g. an uncaught timeout that unwound to the top) can't
        // leak stale count-loop or depth state into this run — matters when one
        // interpreter is reused (the REPL). Done before `assert_invariants` so
        // the invariant holds regardless of how the previous run ended.
        *self.in_count_loop.borrow_mut() = false;
        *self.current_count.borrow_mut() = None;
        // A prior run that ended while a stream was open or a request was
        // unanswered (REPL reuse) must not leave dangling entries. Actually CLOSE
        // those streams and 500 those requests (draining the tracking), rather
        // than only clearing the id lists — clearing alone would strand the
        // still-open sender/receiver in `server_response_streams` /
        // `pending_responses`, hanging the client and leaking the entry.
        self.close_open_response_streams().await;
        self.fail_open_pending_requests();
        self.close_open_http_streams();
        // RAII: if THIS run's future is dropped/cancelled before its normal exit
        // sites run, still close any outbound handles it opened (they would
        // otherwise leak the upstream until the interpreter itself is dropped).
        // On a normal run the exit sites drain the list first, so this is a no-op.
        let _outbound_cleanup = self.outbound_stream_cleanup_guard();
        // Reset to the inherited base depth (0 for a top-level run/REPL; the
        // parent's live depth for an `execute file` child) so recursion
        // accounting spans the execute-file boundary instead of granting the
        // child a fresh full allowance.
        self.call_depth.set(self.base_call_depth);
        // NOTE: do NOT touch the shared budget's main-loop depth here. It is
        // managed entirely by the RAII `MainLoopGuard`, so it never leaks (a
        // mid-loop unwind drops the guard); and for an `execute file` child that
        // shares the parent's budget, clearing it would wrongly cancel the
        // parent's still-active main-loop exemption.
        self.assert_invariants();
        // Names that will become overload sets in the top-level block enforce
        // their declared types from the first definition on. Nested blocks
        // get their own scan at block entry (`BlockDupsScope`), so
        // enforcement stays scoped to the block that actually overloads.
        // Same-scope members an incoming definition will merge with (a REPL
        // interpreter reused across snippets) are armed the same way — and
        // reverted on run exit if this run never executed the merge.
        let _armed_members = Self::arm_block_members(&program.statements, &self.global_env);
        *self.current_block_overload_dups.borrow_mut() =
            Self::scan_block_overload_dups(&program.statements);
        self.call_stack.borrow_mut().clear();

        // Set up script arguments in the global environment
        {
            let mut env = self.global_env.borrow_mut();

            // Create args list with all arguments
            let args_list: Vec<Value> = self
                .script_args
                .iter()
                .map(|arg| Value::Text(Arc::from(arg.as_str())))
                .collect();
            let _ = env.define("args", Value::List(Rc::new(RefCell::new(args_list))));

            // Parse and set up flags (arguments starting with - or --)
            let mut flags = HashMap::new();
            let mut positional_args = Vec::new();
            let mut i = 0;

            while i < self.script_args.len() {
                let arg = &self.script_args[i];
                if arg.starts_with("--") {
                    let flag_name = arg.trim_start_matches("--");
                    // Check if next argument is a value for this flag
                    if i + 1 < self.script_args.len() && !self.script_args[i + 1].starts_with("-") {
                        flags.insert(
                            flag_name.to_string(),
                            Value::Text(Arc::from(self.script_args[i + 1].as_str())),
                        );
                        i += 2;
                    } else {
                        flags.insert(flag_name.to_string(), Value::Bool(true));
                        i += 1;
                    }
                } else if arg.starts_with("-") && arg.len() > 1 {
                    // Handle short flags like -f
                    let flag_name = arg.trim_start_matches("-");
                    // Check if next argument is a value for this flag
                    if i + 1 < self.script_args.len() && !self.script_args[i + 1].starts_with("-") {
                        flags.insert(
                            flag_name.to_string(),
                            Value::Text(Arc::from(self.script_args[i + 1].as_str())),
                        );
                        i += 2;
                    } else {
                        flags.insert(flag_name.to_string(), Value::Bool(true));
                        i += 1;
                    }
                } else {
                    positional_args.push(Value::Text(Arc::from(arg.as_str())));
                    i += 1;
                }
            }

            // Convert flags HashMap to Value
            let mut flags_map = HashMap::new();
            for (key, value) in flags {
                flags_map.insert(key, value);
            }

            // Store positional arguments
            let _ = env.define(
                "positional_args",
                Value::List(Rc::new(RefCell::new(positional_args.clone()))),
            );

            // Store argument count
            let _ = env.define("arg_count", Value::Number(self.script_args.len() as f64));

            // Store program name (first argument or empty string)
            let program_name = if self.script_args.is_empty() {
                "wfl".to_string()
            } else {
                // Extract just the filename from the path
                std::path::Path::new(&self.script_args[0])
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            };
            let _ = env.define("program_name", Value::Text(Arc::from(program_name)));

            // Store current directory
            let current_dir = std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let _ = env.define("current_directory", Value::Text(Arc::from(current_dir)));

            // Store the running script's absolute path and directory,
            // the equivalent of Python's __file__ / dirname(abspath(__file__)).
            // Always absolute or empty: empty when no script file is running,
            // or when a relative script path can't be resolved because the
            // current directory is unavailable.
            let (script_path, script_directory) = match self.current_source_file.borrow().as_ref() {
                Some(source_file) => {
                    let cwd = if source_file.is_absolute() {
                        // lexical_abspath ignores cwd for absolute paths
                        Some(PathBuf::new())
                    } else {
                        std::env::current_dir().ok()
                    };
                    match cwd {
                        Some(cwd) => {
                            let abs = lexical_abspath(source_file, &cwd);
                            let dir = std::path::Path::new(&abs)
                                .parent()
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            (abs, dir)
                        }
                        None => (String::new(), String::new()),
                    }
                }
                None => (String::new(), String::new()),
            };
            let _ = env.define("script_path", Value::Text(Arc::from(script_path)));
            let _ = env.define("script_directory", Value::Text(Arc::from(script_directory)));

            // Store flags as individual variables with flag_ prefix
            for (key, value) in flags_map {
                let _ = env.define(&format!("flag_{key}"), value);
            }
        }

        // Use exec_trace for execution logs instead of println
        if !self.step_mode {
            exec_trace!(
                "Starting script execution with {} statements...",
                program.statements.len()
            );
        }
        exec_trace!("=== Starting program execution ===");

        let mut last_value = Value::Null;
        let mut errors = Vec::new();
        // Set when an `exit program` statement asked the run to stop: the
        // remaining top-level statements and the conventional `main` action are
        // skipped, but the cleanup below still runs and the run still succeeds.
        let mut exited = false;

        #[allow(unused_variables)]
        for (i, statement) in program.statements.iter().enumerate() {
            if !self.step_mode {
                exec_trace!(
                    "Executing statement {}/{}...",
                    i + 1,
                    program.statements.len()
                );
            }
            exec_trace!("Executing statement {}/{}", i + 1, program.statements.len());

            if let Err(err) = self.check_time() {
                if !self.step_mode {
                    exec_trace!(
                        "Timeout reached at statement {}/{}",
                        i + 1,
                        program.statements.len()
                    );
                }
                errors.push(err);
                // A mid-run timeout is still an exit path: finalize any open
                // top-level streams and unanswered requests before returning.
                self.close_open_response_streams().await;
                self.fail_open_pending_requests();
                self.close_open_http_streams();
                return Err(errors);
            }

            match self
                .execute_statement(statement, Rc::clone(&self.global_env))
                .await
            {
                Ok((value, control_flow)) => {
                    last_value = value;
                    if !self.step_mode {
                        exec_trace!(
                            "Statement {}/{} completed successfully",
                            i + 1,
                            program.statements.len()
                        );
                    }

                    match control_flow {
                        ControlFlow::Break | ControlFlow::Continue | ControlFlow::Exit => {
                            exec_trace!("Warning: {:?} at top level ignored", control_flow);
                        }
                        ControlFlow::Return(val) => {
                            exec_trace!("Return at top level with value: {:?}", val);
                            last_value = val;
                            break;
                        }
                        ControlFlow::None => {}
                    }
                }
                // `exit program`: a successful stop, not a failure. Nothing is
                // reported and nothing further runs.
                Err(err) if err.is_exit_program() => {
                    exec_trace!("Program exited via `exit program`");
                    exited = true;
                    break;
                }
                Err(err) => {
                    if !self.step_mode {
                        exec_trace!(
                            "Error at statement {}/{}: {:?}",
                            i + 1,
                            program.statements.len(),
                            err
                        );
                    }
                    errors.push(err);
                    break; // Stop on first runtime error
                }
            }
        }

        // Run the conventional `main` action (if any) before cleanup, so a
        // stream/request opened by `main` is finalized by the drain below rather
        // than leaking.
        if errors.is_empty() && !exited {
            let main_func_opt = {
                match self.global_env.borrow().get("main") {
                    Some(Value::Function(main_func)) => Some(main_func.clone()),
                    // An overloaded `main` runs its zero-argument overload.
                    Some(Value::Overloaded(overloaded)) => overloaded
                        .overloads
                        .iter()
                        .find(|func| func.params.is_empty())
                        .cloned(),
                    _ => None,
                }
            };

            if let Some(main_func) = main_func_opt {
                exec_trace!("Calling main function");
                match self.call_function(&main_func, vec![], 0, 0).await {
                    Ok(value) => {
                        exec_trace!("Main function returned: {:?}", value);
                        last_value = value
                    }
                    // `exit program` inside `main` finishes the run cleanly,
                    // exactly as it does at top level.
                    Err(err) if err.is_exit_program() => {
                        exec_trace!("Program exited from main via `exit program`");
                    }
                    Err(err) => {
                        exec_trace!("Main function failed: {}", err);
                        errors.push(err);
                    }
                }
            }
        }

        // Close any server response streams opened directly at top level or by
        // `main` (outside a `main loop`, which already closes
        // per-iteration/per-handler), and 500 any top-level request dequeued but
        // never answered — on EVERY exit path (normal end, statement error, or a
        // failing `main`), so a script that exits without `close` still finalizes
        // the client's body rather than leaving it hanging until process death.
        self.close_open_response_streams().await;
        self.fail_open_pending_requests();
        self.close_open_http_streams();

        self.assert_invariants();
        if errors.is_empty() {
            Ok(last_value)
        } else {
            Err(errors)
        }
    }

    /// Run an `in transaction on <db>: ... end transaction` block.
    ///
    /// Split out of [`Self::execute_statement`] and awaited behind a `Box::pin`
    /// so its locals do not inflate that function's state machine for every
    /// other statement kind — see the call site.
    async fn execute_transaction_statement(
        &self,
        db: &Expression,
        body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let db_value = self.evaluate_expression(db, Rc::clone(&env)).await?;
        let handle = match &db_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected a database handle, got {db_value:?}"),
                    line,
                    column,
                ));
            }
        };

        self.io_client
            .begin_transaction(self.tx_scope.get(), &handle)
            .await
            .map_err(|e| RuntimeError::new(e, line, column))?;

        // Statements inside share the enclosing scope, like `try:`, so a
        // variable set inside the block is still readable after it.
        let outcome = self.execute_block(body, Rc::clone(&env)).await;

        match outcome {
            // `exit` stops the program where it stands rather than
            // finishing the block, so its work is abandoned — matching
            // what already happens when a program ends with a
            // transaction still open. Committing a half-finished block
            // on an abrupt stop is the surprise worth avoiding.
            Ok((value, ControlFlow::Exit)) => {
                self.io_client
                    .rollback_transaction(self.tx_scope.get(), &handle)
                    .await
                    .map_err(|e| RuntimeError::new(e, line, column))?;
                Ok((value, ControlFlow::Exit))
            }
            Ok((value, control_flow)) => {
                // The block finished without an error. `break`, `continue`
                // and `return` are ordinary exits, not failures, so they
                // commit too — the work inside completed.
                self.io_client
                    .commit_transaction(self.tx_scope.get(), &handle)
                    .await
                    .map_err(|e| RuntimeError::new(e, line, column))?;
                Ok((value, control_flow))
            }
            Err(err) => {
                // Roll back and report the original failure. A rollback
                // that itself fails is appended rather than replacing the
                // cause, which is what the user actually needs to see.
                match self
                    .io_client
                    .rollback_transaction(self.tx_scope.get(), &handle)
                    .await
                {
                    Ok(()) => Err(err),
                    // Keep the original kind. `Cancelled`, `Timeout` and
                    // `ResourceLimit` select `when` clauses and drive concurrent
                    // handler classification, so flattening them to `General`
                    // would turn a client disconnect inside a transaction into a
                    // structural handler failure.
                    Err(rollback_error) => Err(RuntimeError::with_kind(
                        format!("{} (rollback also failed: {rollback_error})", err.message),
                        err.line,
                        err.column,
                        err.kind,
                    )),
                }
            }
        }
    }

    async fn execute_statement(
        &self,
        stmt: &Statement,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        exec_trace!("Executing statement: {}", stmt_type(stmt));
        Box::pin(self._execute_statement(stmt, env)).await
    }

    async fn _execute_statement(
        &self,
        stmt: &Statement,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        self.check_time()?;

        // Cooperatively yield to the async runtime on a throttled stride so a
        // tight CPU-bound loop periodically returns control to the executor,
        // letting a `select!` deliver cooperative cancellation (e.g. the REPL's
        // Ctrl-C → `budget.cancel()`). Driven by a dedicated per-statement
        // counter that advances even inside a `main loop` (whose operation
        // counter is exempt and whose body is not guaranteed to await anything
        // that returns `Pending`), so a CPU-only main loop still yields.
        let sched = self.sched_counter.get().wrapping_add(1);
        self.sched_counter.set(sched);
        if sched & (COOP_YIELD_STRIDE - 1) == 0 {
            tokio::task::yield_now().await;
        }

        let env_before = if self.step_mode {
            self.global_env.borrow().values.clone()
        } else {
            HashMap::new()
        };

        let (line, column) = match stmt {
            Statement::VariableDeclaration { line, column, .. } => (*line, *column),
            Statement::Assignment { line, column, .. } => (*line, *column),
            Statement::IfStatement { line, column, .. } => (*line, *column),
            Statement::SingleLineIf { line, column, .. } => (*line, *column),
            Statement::DisplayStatement { line, column, .. } => (*line, *column),
            Statement::ActionDefinition { line, column, .. } => (*line, *column),
            Statement::ReturnStatement { line, column, .. } => (*line, *column),
            Statement::ExpressionStatement { line, column, .. } => (*line, *column),
            Statement::CountLoop { line, column, .. } => (*line, *column),
            Statement::ForEachLoop { line, column, .. } => (*line, *column),
            Statement::WhileLoop { line, column, .. } => (*line, *column),
            Statement::RepeatUntilLoop { line, column, .. } => (*line, *column),
            Statement::RepeatWhileLoop { line, column, .. } => (*line, *column),
            Statement::ForeverLoop { line, column, .. } => (*line, *column),
            Statement::MainLoop { line, column, .. } => (*line, *column),
            Statement::BreakStatement { line, column, .. } => (*line, *column),
            Statement::ContinueStatement { line, column, .. } => (*line, *column),
            Statement::ExitStatement { line, column, .. } => (*line, *column),
            Statement::OpenFileStatement { line, column, .. } => (*line, *column),
            Statement::ReadFileStatement { line, column, .. } => (*line, *column),
            Statement::WriteFileStatement { line, column, .. } => (*line, *column),
            Statement::WriteToStatement { line, column, .. } => (*line, *column),
            Statement::WriteContentStatement { line, column, .. } => (*line, *column),
            Statement::WriteBinaryStatement { line, column, .. } => (*line, *column),
            Statement::CloseFileStatement { line, column, .. } => (*line, *column),
            Statement::OpenDatabaseStatement { line, column, .. } => (*line, *column),
            Statement::DatabaseQueryStatement { line, column, .. } => (*line, *column),
            Statement::CloseDatabaseStatement { line, column, .. } => (*line, *column),
            Statement::TransactionStatement { line, column, .. } => (*line, *column),
            Statement::CreateDirectoryStatement { line, column, .. } => (*line, *column),
            Statement::CreateFileStatement { line, column, .. } => (*line, *column),
            Statement::DeleteFileStatement { line, column, .. } => (*line, *column),
            Statement::DeleteDirectoryStatement { line, column, .. } => (*line, *column),
            Statement::LoadModuleStatement { line, column, .. } => (*line, *column),
            Statement::IncludeStatement { line, column, .. } => (*line, *column),
            Statement::ExportStatement { line, column, .. } => (*line, *column),
            Statement::WaitForStatement { line, column, .. } => (*line, *column),
            Statement::WaitForDurationStatement { line, column, .. } => (*line, *column),
            Statement::TryStatement { line, column, .. } => (*line, *column),
            Statement::HttpGetStatement { line, column, .. } => (*line, *column),
            Statement::HttpPostStatement { line, column, .. } => (*line, *column),
            Statement::HttpRequestStatement { line, column, .. } => (*line, *column),
            Statement::HttpStreamStatement { line, column, .. } => (*line, *column),
            Statement::WaitForNextChunkStatement { line, column, .. } => (*line, *column),
            Statement::WaitForNextLineStatement { line, column, .. } => (*line, *column),
            Statement::StartStreamingResponseStatement { line, column, .. } => (*line, *column),
            Statement::StreamWriteStatement { line, column, .. } => (*line, *column),
            Statement::FlushStreamStatement { line, column, .. } => (*line, *column),
            Statement::PushStatement { line, column, .. } => (*line, *column),
            Statement::CreateListStatement { line, column, .. } => (*line, *column),
            Statement::MapCreation { line, column, .. } => (*line, *column),
            Statement::CreateDateStatement { line, column, .. } => (*line, *column),
            Statement::CreateTimeStatement { line, column, .. } => (*line, *column),
            Statement::AddToListStatement { line, column, .. } => (*line, *column),
            Statement::RemoveFromListStatement { line, column, .. } => (*line, *column),
            Statement::ClearListStatement { line, column, .. } => (*line, *column),
            // Container-related statements
            Statement::ContainerDefinition { line, column, .. } => (*line, *column),
            Statement::ContainerInstantiation { line, column, .. } => (*line, *column),
            Statement::InterfaceDefinition { line, column, .. } => (*line, *column),
            Statement::EventDefinition { line, column, .. } => (*line, *column),
            Statement::EventTrigger { line, column, .. } => (*line, *column),
            Statement::EventHandler { line, column, .. } => (*line, *column),
            Statement::ParentMethodCall { line, column, .. } => (*line, *column),
            Statement::PatternDefinition { line, column, .. } => (*line, *column),
            Statement::ListenStatement { line, column, .. } => (*line, *column),
            Statement::WaitForRequestStatement { line, column, .. } => (*line, *column),
            Statement::RespondStatement { line, column, .. } => (*line, *column),
            Statement::RegisterSignalHandlerStatement { line, column, .. } => (*line, *column),
            Statement::StopAcceptingConnectionsStatement { line, column, .. } => (*line, *column),
            Statement::CloseServerStatement { line, column, .. } => (*line, *column),
            Statement::ListenWebSocketStatement { line, column, .. } => (*line, *column),
            Statement::WebSocketHandlerStatement { line, column, .. } => (*line, *column),
            Statement::SendWebSocketMessageStatement { line, column, .. } => (*line, *column),
            Statement::BroadcastWebSocketMessageStatement { line, column, .. } => (*line, *column),
            Statement::ExecuteCommandStatement { line, column, .. } => (*line, *column),
            Statement::ExecuteFileStatement { line, column, .. } => (*line, *column),
            Statement::SpawnProcessStatement { line, column, .. } => (*line, *column),
            Statement::ReadProcessOutputStatement { line, column, .. } => (*line, *column),
            Statement::KillProcessStatement { line, column, .. } => (*line, *column),
            Statement::WaitForProcessStatement { line, column, .. } => (*line, *column),
            // Test framework statements
            Statement::DescribeBlock { line, column, .. } => (*line, *column),
            Statement::TestBlock { line, column, .. } => (*line, *column),
            Statement::ExpectStatement { line, column, .. } => (*line, *column),
            Statement::ConfigureSessionsStatement { line, column, .. } => (*line, *column),
            Statement::EnableCsrfProtectionStatement { line, column, .. } => (*line, *column),
            Statement::EnableSecureCookiesStatement { line, column, .. } => (*line, *column),
            Statement::SetSessionValueStatement { line, column, .. } => (*line, *column),
            Statement::DestroySessionStatement { line, column, .. } => (*line, *column),
            Statement::StoreSessionDataStatement { line, column, .. } => (*line, *column),
            Statement::DeleteSessionDataStatement { line, column, .. } => (*line, *column),
        };

        let result = match stmt {
            Statement::VariableDeclaration {
                name,
                value,
                is_constant,
                line: _line,
                column: _column,
            } => {
                let evaluated_value = self.evaluate_expression(value, Rc::clone(&env)).await?;

                #[cfg(debug_assertions)]
                exec_var_declare!(name, &evaluated_value);

                // OPTIMIZATION: declare_variable handles scope traversal, shadowing detection,
                // and definition/assignment in a single pass.
                match env
                    .borrow_mut()
                    .declare_variable(name, evaluated_value, *is_constant)
                {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, line, column)),
                }
            }

            Statement::Assignment {
                name,
                value,
                line,
                column,
            } => {
                let value = self.evaluate_expression(value, Rc::clone(&env)).await?;
                #[cfg(debug_assertions)]
                exec_var_assign!(name, &value);
                match env.borrow_mut().assign(name, value.clone()) {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                }
            }

            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line: _line,
                column: _column,
            } => {
                let condition_value = self.evaluate_expression(condition, Rc::clone(&env)).await?;
                #[cfg(debug_assertions)]
                exec_control_flow!("if condition", condition_value.is_truthy());

                if condition_value.is_truthy() {
                    #[cfg(debug_assertions)]
                    let _guard = IndentGuard::new();
                    #[cfg(debug_assertions)]
                    exec_block_enter!("if branch");
                    let result = self.execute_block(then_block, Rc::clone(&env)).await;
                    #[cfg(debug_assertions)]
                    exec_block_exit!("if branch");
                    result
                } else if let Some(else_stmts) = else_block {
                    #[cfg(debug_assertions)]
                    let _guard = IndentGuard::new();
                    #[cfg(debug_assertions)]
                    exec_block_enter!("else branch");
                    let result = self.execute_block(else_stmts, Rc::clone(&env)).await;
                    #[cfg(debug_assertions)]
                    exec_block_exit!("else branch");
                    result
                } else {
                    Ok((Value::Null, ControlFlow::None))
                }
            }

            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                line: _line,
                column: _column,
            } => {
                let condition_value = self.evaluate_expression(condition, Rc::clone(&env)).await?;

                if condition_value.is_truthy() {
                    self.execute_statement(then_stmt, Rc::clone(&env)).await
                } else if let Some(else_stmt) = else_stmt {
                    self.execute_statement(else_stmt, Rc::clone(&env)).await
                } else {
                    Ok((Value::Null, ControlFlow::None))
                }
            }

            Statement::DisplayStatement {
                value,
                line: _line,
                column: _column,
            } => {
                let value = self.evaluate_expression(value, Rc::clone(&env)).await?;
                io_capture::emit_line(&value.to_string());
                Ok((Value::Null, ControlFlow::None))
            }

            Statement::ActionDefinition {
                name,
                parameters,
                body,
                return_type: _return_type,
                line,
                column,
            } => {
                let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();
                let param_types: Vec<Option<Type>> =
                    parameters.iter().map(|p| p.param_type.clone()).collect();

                let function = FunctionValue {
                    name: Some(name.clone()),
                    params: param_names,
                    param_types,
                    body: body.clone(),
                    env: Rc::downgrade(&env),
                    line: *line,
                    column: *column,
                    // Pre-scanned: true when this name has several definitions
                    // in its block, so even the first member enforces its
                    // declared types during the window before the second
                    // definition executes ("the overloads defined so far").
                    enforce_param_types: std::cell::Cell::new(
                        self.current_block_overload_dups
                            .borrow()
                            .as_ref()
                            .is_some_and(|dups| dups.contains(name)),
                    ),
                    static_method_context: None,
                };

                // A same-scope redefinition of an action name merges into an
                // overload set instead of erroring. An explicit method-local
                // declaration may shadow a synthetic property binding, while
                // ordinary outer lexical bindings retain the historical
                // no-shadowing rule.
                let shadows_property = self.definition_shadows_container_property(&env, name);
                let result = if shadows_property {
                    env.borrow_mut()
                        .define_or_merge_action_direct(name, Rc::new(function))
                } else {
                    env.borrow_mut()
                        .define_or_merge_action(name, Rc::new(function))
                };
                match result {
                    Ok(defined_value) => Ok((defined_value, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                }
            }

            Statement::ReturnStatement {
                value,
                line: _line,
                column: _column,
            } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing return statement");

                if let Some(expr) = value {
                    let result = self.evaluate_expression(expr, Rc::clone(&env)).await?;
                    Ok((result.clone(), ControlFlow::Return(result)))
                } else {
                    Ok((Value::Null, ControlFlow::Return(Value::Null)))
                }
            }

            Statement::ExpressionStatement {
                expression,
                line: _line,
                column: _column,
            } => {
                // Check if this is a bare action call (just the action name without parentheses)
                if let Expression::Variable(name, var_line, var_column) = expression {
                    // Check if the variable refers to an action
                    // Extract lookup result so the Ref<Environment> is dropped before call_function
                    let lookup = env.borrow().get(name);
                    if let Some(Value::Function(func)) = lookup {
                        // It's an action, so execute it as a call with no arguments
                        #[cfg(debug_assertions)]
                        exec_trace!("Executing bare action call: {}", name);
                        return self
                            .call_function(&func, vec![], *var_line, *var_column)
                            .await
                            .map(|value| (value, ControlFlow::None));
                    } else if let Some(Value::Overloaded(overloaded)) = lookup {
                        // A bare overloaded name auto-calls its zero-argument
                        // overload, matching single-function behavior.
                        if let Some(func) = overloaded
                            .overloads
                            .iter()
                            .find(|func| func.params.is_empty())
                        {
                            #[cfg(debug_assertions)]
                            exec_trace!("Executing bare overloaded action call: {}", name);
                            return self
                                .call_function(func, vec![], *var_line, *var_column)
                                .await
                                .map(|value| (value, ControlFlow::None));
                        }
                    }
                }

                // Regular expression evaluation
                let value = self
                    .evaluate_expression(expression, Rc::clone(&env))
                    .await?;
                Ok((value, ControlFlow::None))
            }

            Statement::CountLoop {
                start,
                end,
                step,
                downward,
                variable_name,
                body,
                line,
                column,
            } => {
                // === CRITICAL FIX: Reset count loop state before starting ===
                let previous_count = *self.current_count.borrow();
                let was_in_count_loop = *self.in_count_loop.borrow();

                // Force reset state to prevent inheriting stale values
                *self.current_count.borrow_mut() = None;
                *self.in_count_loop.borrow_mut() = false;

                crate::exec_trace!("Count loop: resetting state before evaluation");

                let start_val = self.evaluate_expression(start, Rc::clone(&env)).await?;
                let end_val = self.evaluate_expression(end, Rc::clone(&env)).await?;

                let (start_num, end_num) = match (start_val, end_val) {
                    (Value::Number(s), Value::Number(e)) => (s, e),
                    _ => {
                        return Err(RuntimeError::new(
                            "Count loop requires numeric start and end values".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                let step_num = if let Some(step_expr) = step {
                    match self.evaluate_expression(step_expr, Rc::clone(&env)).await? {
                        Value::Number(n) => n,
                        _ => {
                            return Err(RuntimeError::new(
                                "Count loop step must be a number".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    1.0
                };

                let mut count = start_num;

                let should_continue: Box<dyn Fn(f64, f64) -> bool> = if *downward {
                    Box::new(|count, end_num| count >= end_num)
                } else {
                    Box::new(|count, end_num| count <= end_num)
                };

                // A step that cannot move the counter toward the end value would
                // spin until the execution timeout, so it is refused up front
                // with a diagnostic that names the problem. `count` always moves
                // *toward* its end value (a downward loop subtracts the step), so
                // the step is a magnitude and must be a positive, finite number.
                //
                // Only a loop that actually runs is validated: `count from 5 to 1`
                // never enters its body, so its step never mattered and a program
                // that passed a nonsense one has always completed successfully.
                // Validating unconditionally would break those programs.
                if should_continue(count, end_num) && (!step_num.is_finite() || step_num <= 0.0) {
                    *self.current_count.borrow_mut() = previous_count;
                    *self.in_count_loop.borrow_mut() = was_in_count_loop;
                    return Err(RuntimeError::new(
                        format!(
                            "Count loop step must be a positive number, got {step_num}. \
                             A count loop always moves toward its end value, so \
                             `count from 10 down to 1 by 2` steps down by 2."
                        ),
                        *line,
                        *column,
                    ));
                }

                *self.in_count_loop.borrow_mut() = true;

                // Determine the variable name to use - custom name or default "count"
                let loop_var_name = variable_name.as_deref().unwrap_or("count");
                let mut loop_env_recycle = None;

                // No trip-count ceiling: `count` is bounded by the same
                // execution timeout as `repeat` and `for each` (checked below on
                // every trip), so an ordinary long loop runs and a runaway one
                // still stops. The previous guard keyed on the *end value*, which
                // capped `count from 1 to 20000` while letting
                // `count from 1 to 1000001` run uncapped (issue #699).
                while should_continue(count, end_num) {
                    self.check_time()?;

                    *self.current_count.borrow_mut() = Some(count);

                    // OPTIMIZATION: Recycle environment if possible
                    let loop_env = self.get_recycled_env(loop_env_recycle.take(), &env);

                    // Make the loop variable available in the loop environment,
                    // shadowing any same-named variable from an outer scope.
                    // Use custom variable name if provided, otherwise default to "count"
                    loop_env
                        .borrow_mut()
                        .define_or_replace(loop_var_name, Value::Number(count));

                    let result = self.execute_block(body, Rc::clone(&loop_env)).await;

                    // Save environment for potential recycling in next iteration
                    loop_env_recycle = Some(loop_env);

                    match result {
                        Ok((_, control_flow)) => match control_flow {
                            ControlFlow::Break => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Breaking out of count loop");
                                break;
                            }
                            ControlFlow::Continue => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Continuing count loop");
                            }
                            ControlFlow::Exit => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Exiting from count loop");
                                *self.current_count.borrow_mut() = previous_count;
                                *self.in_count_loop.borrow_mut() = was_in_count_loop;
                                return Ok((Value::Null, ControlFlow::Exit));
                            }
                            ControlFlow::Return(val) => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Returning from count loop with value: {:?}", val);
                                *self.current_count.borrow_mut() = previous_count;
                                *self.in_count_loop.borrow_mut() = was_in_count_loop;
                                return Ok((val.clone(), ControlFlow::Return(val)));
                            }
                            ControlFlow::None => {}
                        },
                        Err(e) => {
                            *self.current_count.borrow_mut() = previous_count;
                            *self.in_count_loop.borrow_mut() = was_in_count_loop;
                            return Err(e);
                        }
                    }

                    // A positive step is not enough: past 2^53 the counter's
                    // floating-point resolution exceeds the step, so `count`
                    // stops changing and the loop can never reach its end. That
                    // is an endless loop the execution timeout would catch only
                    // outside a `main loop`, where the deadline is suspended —
                    // so it is refused here, where the stall is provable.
                    let next = if *downward {
                        count - step_num
                    } else {
                        count + step_num
                    };
                    if next == count {
                        *self.current_count.borrow_mut() = previous_count;
                        *self.in_count_loop.borrow_mut() = was_in_count_loop;
                        return Err(RuntimeError::new(
                            format!(
                                "Count loop step {step_num} is too small to move the counter \
                                 past {count}, so the loop can never reach {end_num}. \
                                 Numbers this large lose precision — use a larger step."
                            ),
                            *line,
                            *column,
                        ));
                    }
                    count = next;
                }

                *self.current_count.borrow_mut() = previous_count;
                *self.in_count_loop.borrow_mut() = was_in_count_loop;

                Ok((Value::Null, ControlFlow::None))
            }

            Statement::ForEachLoop {
                item_name,
                collection,
                reversed,
                body,
                line,
                column,
            } => {
                let collection_val = self
                    .evaluate_expression(collection, Rc::clone(&env))
                    .await?;

                match collection_val {
                    Value::List(list_rc) => {
                        let items: Vec<Value> = {
                            let list = list_rc.borrow();
                            let indices: Vec<usize> = if *reversed {
                                (0..list.len()).rev().collect()
                            } else {
                                (0..list.len()).collect()
                            };
                            indices.iter().map(|&i| list[i].clone()).collect()
                        };

                        let mut loop_env_recycle = None;
                        for item in items {
                            // OPTIMIZATION: Recycle environment if possible
                            let loop_env = self.get_recycled_env(loop_env_recycle.take(), &env);

                            match loop_env.borrow_mut().define_direct(item_name, item) {
                                Ok(_) => {}
                                Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                            }
                            let result = self.execute_block(body, Rc::clone(&loop_env)).await?;

                            // Save environment for potential recycling in next iteration
                            loop_env_recycle = Some(loop_env);

                            match result.1 {
                                ControlFlow::Break => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Breaking out of foreach loop");
                                    break;
                                }
                                ControlFlow::Continue => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Continuing foreach loop");
                                    continue;
                                }
                                ControlFlow::Exit => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Exiting from foreach loop");
                                    return Ok((Value::Null, ControlFlow::Exit));
                                }
                                ControlFlow::Return(val) => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!(
                                        "Returning from foreach loop with value: {:?}",
                                        val
                                    );
                                    return Ok((val.clone(), ControlFlow::Return(val)));
                                }
                                ControlFlow::None => {}
                            }
                        }
                    }
                    Value::Object(obj_rc) => {
                        let items: Vec<(String, Value)> = {
                            let obj = obj_rc.borrow();
                            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                        };

                        let mut loop_env_recycle = None;
                        for (_, value) in items {
                            // OPTIMIZATION: Recycle environment if possible
                            let loop_env = self.get_recycled_env(loop_env_recycle.take(), &env);

                            match loop_env.borrow_mut().define_direct(item_name, value) {
                                Ok(_) => {}
                                Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                            }
                            let result = self.execute_block(body, Rc::clone(&loop_env)).await?;

                            // Save environment for potential recycling in next iteration
                            loop_env_recycle = Some(loop_env);

                            match result.1 {
                                ControlFlow::Break => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Breaking out of foreach loop (object)");
                                    break;
                                }
                                ControlFlow::Continue => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Continuing foreach loop (object)");
                                    continue;
                                }
                                ControlFlow::Exit => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Exiting from foreach loop (object)");
                                    return Ok((Value::Null, ControlFlow::Exit));
                                }
                                ControlFlow::Return(val) => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!(
                                        "Returning from foreach loop with value: {:?}",
                                        val
                                    );
                                    return Ok((val.clone(), ControlFlow::Return(val)));
                                }
                                ControlFlow::None => {}
                            }
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Cannot iterate over {}", collection_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                }

                Ok((Value::Null, ControlFlow::None))
            }

            Statement::WhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let mut _last_value = Value::Null;

                while self
                    .evaluate_expression(condition, Rc::clone(&env))
                    .await?
                    .is_truthy()
                {
                    self.check_time()?;
                    let result = self.execute_block(body, Rc::clone(&env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of while loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing while loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from while loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from while loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }

                Ok((_last_value, ControlFlow::None))
            }

            Statement::RepeatUntilLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let mut _last_value = Value::Null;

                loop {
                    self.check_time()?;
                    let result = self.execute_block(body, Rc::clone(&env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of repeat-until loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing repeat-until loop");
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from repeat-until loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from repeat-until loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }

                    if self
                        .evaluate_expression(condition, Rc::clone(&env))
                        .await?
                        .is_truthy()
                    {
                        break;
                    }
                }

                Ok((_last_value, ControlFlow::None))
            }

            Statement::ForeverLoop {
                body,
                line: _line,
                column: _column,
            } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing forever loop");

                let mut _last_value = Value::Null;
                let mut loop_env_recycle = None;

                loop {
                    self.check_time()?;

                    // OPTIMIZATION: Recycle environment if possible
                    let loop_env = self.get_recycled_env(loop_env_recycle.take(), &env);
                    let result = self.execute_block(body, Rc::clone(&loop_env)).await?;
                    _last_value = result.0;

                    // Save environment for potential recycling in next iteration
                    loop_env_recycle = Some(loop_env);

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of forever loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing forever loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from forever loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from forever loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }

                Ok((_last_value, ControlFlow::None))
            }

            Statement::MainLoop {
                body,
                concurrent,
                line: _line,
                column: _column,
            } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing main loop (timeout disabled)");

                // Enter the main loop's deadline exemption via an RAII guard on
                // the shared budget. The guard restores the depth on EVERY exit —
                // a normal end, an early `return` below, a caught error unwinding
                // through `?`, or a nested main loop — so the exemption is never
                // leaked or cleared while an outer loop is still active. A child
                // `execute file` sharing this budget inherits the exemption too.
                let _main_loop_guard = self.budget.enter_main_loop();

                // `main loop concurrently:` runs body iterations cooperatively
                // concurrently, each in its own isolated scope, so a slow handler
                // (e.g. one streaming a slow upstream) does not block its
                // siblings. Plain `main loop` stays strictly serial below.
                if *concurrent {
                    return self.execute_concurrent_main_loop(body, &env).await;
                }

                let mut _last_value = Value::Null;
                let mut loop_env_recycle = None;

                loop {
                    // check_time() skips the deadline while the main-loop depth > 0
                    self.check_time()?;

                    // OPTIMIZATION: Recycle environment if possible
                    let loop_env = self.get_recycled_env(loop_env_recycle.take(), &env);
                    let result = self.execute_block(body, Rc::clone(&loop_env)).await;
                    // On every path (including the error path below): close any
                    // server response streams this iteration left open, and 500
                    // any request it dequeued but never answered — so a handler
                    // that forgets `close out`/`respond` never leaves the client
                    // hanging or waiting out the request timeout. Awaits body
                    // finish so a `break` right after the last chunk still
                    // delivers a clean end-of-stream (#680).
                    self.close_open_response_streams().await;
                    self.fail_open_pending_requests();
                    self.close_open_http_streams();
                    let result = result?;
                    _last_value = result.0;

                    // Save environment for potential recycling in next iteration
                    loop_env_recycle = Some(loop_env);

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of main loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing main loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from main loop");
                            // `_main_loop_guard` drops here, restoring the depth.
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from main loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }

                // `_main_loop_guard` drops here on normal exit.
                Ok((_last_value, ControlFlow::None))
            }

            Statement::BreakStatement { .. } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing break statement");
                Ok((Value::Null, ControlFlow::Break))
            }

            Statement::ContinueStatement { .. } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing continue statement");
                Ok((Value::Null, ControlFlow::Continue))
            }

            Statement::ExitStatement {
                scope,
                line,
                column,
            } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing exit statement ({scope:?})");
                match scope {
                    // `exit` / `exit loop`: unwind the enclosing loop(s).
                    ExitScope::Loop => Ok((Value::Null, ControlFlow::Exit)),
                    // `exit program`: stop the run where it stands. Raised as
                    // the `ExitProgram` sentinel so it unwinds action calls and
                    // expression evaluation too — neither of which carries a
                    // control-flow channel — and is turned back into a
                    // successful finish at the top of the run.
                    ExitScope::Program => Err(RuntimeError::exit_program(*line, *column)),
                }
            }

            Statement::OpenFileStatement {
                path,
                variable_name,
                mode,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // Use the appropriate file open mode
                match self
                    .io_client
                    .open_file_with_mode(&path_str, mode.clone())
                    .await
                {
                    Ok(handle) => {
                        match env
                            .borrow_mut()
                            .define_direct(variable_name, Value::Text(handle.into()))
                        {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Statement::OpenDatabaseStatement {
                url,
                variable_name,
                line,
                column,
            } => {
                let url_value = self.evaluate_expression(url, Rc::clone(&env)).await?;
                let url_str = match &url_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected text for database URL, got {url_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.open_database(&url_str).await {
                    Ok(handle) => {
                        let define_result = env
                            .borrow_mut()
                            .define_direct(variable_name, Value::Text(handle.as_str().into()));
                        match define_result {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => {
                                // Don't leave an unreachable pool behind when
                                // the variable binding fails.
                                let _ = self.io_client.close_database(&handle).await;
                                Err(RuntimeError::new(msg, *line, *column))
                            }
                        }
                    }
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::DatabaseQueryStatement {
                db,
                sql,
                parameters,
                variable_name,
                kind,
                line,
                column,
            } => {
                let result = self
                    .evaluate_database_query(
                        db,
                        sql,
                        parameters.as_ref(),
                        *kind,
                        *line,
                        *column,
                        Rc::clone(&env),
                    )
                    .await?;

                match env.borrow_mut().define_direct(variable_name, result) {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                }
            }
            Statement::CloseDatabaseStatement { db, line, column } => {
                let db_value = self.evaluate_expression(db, Rc::clone(&env)).await?;
                let handle = match &db_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected a database handle, got {db_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                self.io_client
                    .close_database(&handle)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::TransactionStatement {
                db,
                body,
                line,
                column,
            } => {
                // Boxed: `execute_statement` is a plain `async fn`, so every
                // `.await` in every arm enlarges the single state machine that
                // each level of statement recursion keeps on the stack. Holding
                // this arm's work behind a pointer keeps deeply nested programs
                // (and concurrent handlers, which recurse per request) clear of
                // the stack ceiling.
                Box::pin(self.execute_transaction_statement(db, body, *line, *column, env)).await
            }
            Statement::ReadFileStatement {
                path,
                variable_name,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path or handle, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let is_file_path = matches!(path, Expression::Literal(Literal::String(_), _, _));

                if is_file_path {
                    match self.io_client.open_file(&path_str).await {
                        Ok(handle) => match self.io_client.read_file(&handle, &self.budget).await {
                            Ok(content) => {
                                // Capture the define result and drop the env
                                // borrow before the `close_file` await below.
                                let define_result = env
                                    .borrow_mut()
                                    .define_direct(variable_name, Value::Text(content.into()));
                                match define_result {
                                    Ok(_) => {
                                        let _ = self.io_client.close_file(&handle).await;
                                        Ok((Value::Null, ControlFlow::None))
                                    }
                                    Err(msg) => {
                                        let _ = self.io_client.close_file(&handle).await;
                                        Err(RuntimeError::new(msg, *line, *column))
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = self.io_client.close_file(&handle).await;
                                Err(self.file_read_error(e, *line, *column))
                            }
                        },
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    }
                } else {
                    match self.io_client.read_file(&path_str, &self.budget).await {
                        Ok(content) => {
                            match env
                                .borrow_mut()
                                .define_direct(variable_name, Value::Text(content.into()))
                            {
                                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                            }
                        }
                        Err(e) => Err(self.file_read_error(e, *line, *column)),
                    }
                }
            }
            Statement::WriteFileStatement {
                file,
                content,
                mode,
                line,
                column,
            } => {
                let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;

                let file_str = match &file_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {file_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = match &content_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file content, got {content_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match mode {
                    crate::parser::ast::WriteMode::Append => {
                        match self.io_client.append_file(&file_str, &content_str).await {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(e) => Err(RuntimeError::new(e, *line, *column)),
                        }
                    }
                    crate::parser::ast::WriteMode::Overwrite => {
                        match self.io_client.write_file(&file_str, &content_str).await {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(e) => Err(RuntimeError::new(e, *line, *column)),
                        }
                    }
                }
            }
            Statement::CloseFileStatement { file, line, column } => {
                let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;

                match &file_value {
                    // A bare text handle closes a file (existing behavior).
                    Value::Text(s) => match self.io_client.close_file(s).await {
                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    },
                    // A streaming-response object closes its underlying stream.
                    // For a client upstream (`_stream`) this cancels the in-flight
                    // request; for a server response stream (`_server_stream`)
                    // this drops the body sender, ending the response. Closing a
                    // stream is always safe, even after EOF / already closed.
                    Value::Object(obj) => {
                        let (client_id, server_id) = {
                            let obj_ref = obj.borrow();
                            let client = obj_ref.get("_stream").and_then(|v| match v {
                                Value::Text(s) => Some(s.to_string()),
                                _ => None,
                            });
                            let server = obj_ref.get("_server_stream").and_then(|v| match v {
                                Value::Text(s) => Some(s.to_string()),
                                _ => None,
                            });
                            (client, server)
                        };
                        if let Some(id) = client_id {
                            self.io_client.close_stream(&id).await;
                            // Drop it from the handler's ownership tracking so the
                            // exit cleanup does not try to re-close it.
                            self.untrack_http_stream(&id);
                            Ok((Value::Null, ControlFlow::None))
                        } else if let Some(id) = server_id {
                            // Only the owning handler may end a LIVE response
                            // body: a sibling holding this handle must not be
                            // able to truncate the owner's in-flight response
                            // (#642). An already-closed stream has no owner
                            // (and nothing left to protect), so `close` on a
                            // stale handle is an idempotent no-op for any
                            // handler.
                            if !self.owns_response_stream(&id)
                                && self.server_response_streams.borrow().contains_key(&id)
                            {
                                return Err(self.response_stream_ownership_error(
                                    "close", &id, *line, *column,
                                ));
                            }
                            // Drop it from the handler's auto-close tracking so
                            // the list stays bounded to actually-open streams
                            // *before* awaiting finish (so a concurrent drain
                            // cannot double-close).
                            self.open_response_streams.borrow_mut().retain(|s| s != &id);
                            // Drop the sender (ends the body) and await the
                            // transport finishing the body so the terminating
                            // chunk is on the wire before we return (#680).
                            self.close_response_streams(std::slice::from_ref(&id)).await;
                            Ok((Value::Null, ControlFlow::None))
                        } else {
                            Err(RuntimeError::new(
                                "Cannot close this value: it is not a file handle or a \
                                 streaming response"
                                    .to_string(),
                                *line,
                                *column,
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!(
                            "Expected a file handle or streaming response, got {}",
                            file_value.type_name()
                        ),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::CreateDirectoryStatement { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.create_directory(&path_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::CreateFileStatement {
                path,
                content,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;

                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = format!("{content_value}");

                match self.io_client.create_file(&path_str, &content_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::DeleteFileStatement { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.delete_file(&path_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::WriteToStatement {
                content,
                file,
                line,
                column,
            } => {
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;
                let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;

                let file_str = match &file_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {file_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = format!("{content_value}");

                match self.io_client.write_file(&file_str, &content_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::WriteContentStatement {
                content,
                target,
                line,
                column,
            } => {
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;
                let target_value = self.evaluate_expression(target, Rc::clone(&env)).await?;

                let target_str = match &target_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {target_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = format!("{content_value}");

                // Check if target is a file handle (starts with "file") or a file path
                if target_str.starts_with("file") {
                    // This is a file handle, use append_file to respect the file's open mode
                    match self.io_client.append_file(&target_str, &content_str).await {
                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    }
                } else {
                    // This is a file path, use write_file (overwrite mode)
                    match self.io_client.write_file(&target_str, &content_str).await {
                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    }
                }
            }
            Statement::WriteBinaryStatement {
                content,
                target,
                line,
                column,
            } => {
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;
                let target_value = self.evaluate_expression(target, Rc::clone(&env)).await?;

                let target_str = match &target_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected string for file handle, got {}",
                                target_value.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                // 50MB limit to prevent memory exhaustion
                const MAX_BINARY_WRITE: usize = 50 * 1024 * 1024;
                let bytes = match &content_value {
                    Value::Binary(b) => b.clone(),
                    Value::List(items) => {
                        let items = items.borrow();
                        if items.len() > MAX_BINARY_WRITE {
                            return Err(RuntimeError::new(
                                format!(
                                    "Byte list length {} exceeds maximum allowed ({})",
                                    items.len(),
                                    MAX_BINARY_WRITE
                                ),
                                *line,
                                *column,
                            ));
                        }
                        let mut bytes = Vec::with_capacity(items.len());
                        for (i, item) in items.iter().enumerate() {
                            match item {
                                Value::Number(n) => {
                                    if !n.is_finite() || n.fract() != 0.0 || *n < 0.0 || *n > 255.0
                                    {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Invalid byte value at index {i}: {n} — must be an integer 0-255"
                                            ),
                                            *line,
                                            *column,
                                        ));
                                    }
                                    bytes.push(*n as u8);
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        format!(
                                            "Expected number in byte list at index {}, got {}",
                                            i,
                                            item.type_name()
                                        ),
                                        *line,
                                        *column,
                                    ));
                                }
                            }
                        }
                        Arc::from(bytes)
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected Binary or List for write binary, got {}",
                                content_value.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.write_binary(&target_str, &bytes).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::DeleteDirectoryStatement { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.delete_directory(&path_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }

            Statement::LoadModuleStatement {
                path, line, column, ..
            } => {
                // 1. Evaluate path expression to string
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str: String = match &path_value {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Module path must be a string, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // 2. Resolve absolute path
                let resolved_path = self.resolve_module_path(&path_str, *line, *column).await?;

                // 3. Check circular dependencies and the shared import-depth
                // ceiling (loading_stack length is the depth already entered).
                self.check_circular_dependency(&resolved_path, *line, *column)?;
                if let Err(exceeded) = self
                    .budget
                    .check_import_depth(self.loading_stack.borrow().len())
                {
                    return Err(self.budget_error(exceeded, *line, *column));
                }

                // 4. Read file content under the shared source-size ceiling.
                let content = self
                    .read_source_bounded(&resolved_path, *line, *column)
                    .await?;

                // 6. Parse module
                use crate::lexer::lex_wfl_with_positions_checked;
                use crate::parser::Parser;

                // Lex under the shared run budget: a deadline / cancellation /
                // operation breach during nested source loading surfaces as a
                // typed, catchable runtime error instead of a truncated token
                // stream that could execute as if it were the whole file.
                let tokens = lex_wfl_with_positions_checked(&content)
                    .map_err(|exceeded| self.budget_error(exceeded, *line, *column))?;
                let mut parser = Parser::new(&tokens);
                let program = parser.parse().map_err(|errors| {
                    // Use the parse error's position from the module file, not the load site
                    let first_error = errors.first();
                    let (error_line, error_column) =
                        first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                    RuntimeError::new(
                        format!(
                            "Parse error in module '{}': {}",
                            resolved_path.display(),
                            first_error.map(|e| e.message.as_str()).unwrap_or("unknown")
                        ),
                        error_line,
                        error_column,
                    )
                })?;

                // 7. Analyze semantics
                use crate::analyzer::Analyzer;

                // Extract parent variables from current environment for module analyzer
                let parent_vars = Self::extract_parent_variables(&env);
                let mut analyzer = Analyzer::with_parent_variables(parent_vars);
                if let Err(errors) = analyzer.analyze(&program) {
                    // Use the semantic error's position from the module file, not the load site
                    let first_error = errors.first();
                    let (error_line, error_column) =
                        first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                    return Err(RuntimeError::new(
                        format!(
                            "Semantic error in module '{}': {}",
                            resolved_path.display(),
                            first_error.map(|e| e.to_string()).unwrap_or_default()
                        ),
                        error_line,
                        error_column,
                    ));
                }

                // 8. Type check
                use crate::typechecker::{TypeCheckError, TypeChecker};

                // Use the analyzer with parent scope for type checking
                let mut tc = TypeChecker::with_analyzer(analyzer);
                if let Err(failure) = tc.check_types(&program) {
                    match failure {
                        // A shared-budget breach while type-checking the module is
                        // fatal: surface it as the catchable resource/timeout
                        // error rather than a "type error in module".
                        TypeCheckError::Budget(exceeded) => {
                            return Err(self.budget_error(exceeded, 0, 0));
                        }
                        TypeCheckError::Types(type_errors) => {
                            // Use the type error's position from the module file, not the load site
                            let first_error = type_errors.first();
                            let (error_line, error_column) =
                                first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                            return Err(RuntimeError::new(
                                format!(
                                    "Type error in module '{}': {}",
                                    resolved_path.display(),
                                    first_error.map(|e| e.to_string()).unwrap_or_default()
                                ),
                                error_line,
                                error_column,
                            ));
                        }
                    }
                }

                // 8. Create isolated child environment
                // This prevents mutations of containers (lists/objects) from affecting parent scope
                use crate::interpreter::environment::Environment;
                let module_env = Environment::new_isolated_child_env(&env);

                // 9. Create guard to ensure context restoration on scope exit
                let previous_source = self.current_source_file.borrow().clone();
                let _guard = ModuleLoadGuard::new(self, resolved_path.clone(), previous_source);

                // 10. Execute module in child scope
                let result = self.execute_block(&program.statements, module_env).await;

                // Note: Context automatically restored when _guard drops at end of scope

                // 11. Handle result
                match result {
                    Ok((_, ControlFlow::None)) => Ok((Value::Null, ControlFlow::None)),
                    Ok((_, ControlFlow::Return(_))) => Err(RuntimeError::new(
                        "Cannot use 'return' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Break)) => Err(RuntimeError::new(
                        "Cannot use 'break' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Continue)) => Err(RuntimeError::new(
                        "Cannot use 'continue' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Exit)) => Err(RuntimeError::new(
                        "Cannot use 'exit' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    // `exit program` inside a module stops the whole run, so it
                    // travels untouched rather than being retitled as a module
                    // failure.
                    Err(e) if e.is_exit_program() => Err(e),
                    Err(e) => {
                        // Capture chain BEFORE guard drops (while current module is still on stack)
                        let chain = _guard.get_chain();
                        if chain.len() > 1 {
                            // Only show chain if there are multiple modules
                            // Preserve the original error kind and use the original error's coordinates
                            Err(RuntimeError::with_kind(
                                format!(
                                    "Error in module chain {}: {}",
                                    chain.join(" → "),
                                    e.message
                                ),
                                e.line,
                                e.column,
                                e.kind,
                            ))
                        } else {
                            Err(e)
                        }
                    }
                }
            }

            Statement::IncludeStatement {
                path, line, column, ..
            } => {
                // Include statement is like LoadModule but executes in parent scope instead of isolated child

                // 1. Evaluate path expression to string
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str: String = match &path_value {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Include path must be a string, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // 2. Resolve absolute path
                let resolved_path = self.resolve_module_path(&path_str, *line, *column).await?;

                // 3. Check circular dependencies and the shared import-depth
                // ceiling (loading_stack length is the depth already entered).
                self.check_circular_dependency(&resolved_path, *line, *column)?;
                if let Err(exceeded) = self
                    .budget
                    .check_import_depth(self.loading_stack.borrow().len())
                {
                    return Err(self.budget_error(exceeded, *line, *column));
                }

                // 4. Read file content under the shared source-size ceiling.
                let content = self
                    .read_source_bounded(&resolved_path, *line, *column)
                    .await?;

                // 5. Parse included file
                use crate::lexer::lex_wfl_with_positions_checked;
                use crate::parser::Parser;

                // Lex under the shared run budget: a deadline / cancellation /
                // operation breach during nested source loading surfaces as a
                // typed, catchable runtime error instead of a truncated token
                // stream that could execute as if it were the whole file.
                let tokens = lex_wfl_with_positions_checked(&content)
                    .map_err(|exceeded| self.budget_error(exceeded, *line, *column))?;
                let mut parser = Parser::new(&tokens);
                let program = parser.parse().map_err(|errors| {
                    let first_error = errors.first();
                    let (error_line, error_column) =
                        first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                    RuntimeError::new(
                        format!(
                            "Parse error in included file '{}': {}",
                            resolved_path.display(),
                            first_error.map(|e| e.message.as_str()).unwrap_or("unknown")
                        ),
                        error_line,
                        error_column,
                    )
                })?;

                // 6. Analyze semantics
                use crate::analyzer::Analyzer;

                let parent_vars = Self::extract_parent_variables(&env);
                let mut analyzer = Analyzer::with_parent_variables_mutable(parent_vars);
                if let Err(errors) = analyzer.analyze(&program) {
                    let first_error = errors.first();
                    let (error_line, error_column) =
                        first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                    return Err(RuntimeError::new(
                        format!(
                            "Semantic error in included file '{}': {}",
                            resolved_path.display(),
                            first_error.map(|e| e.to_string()).unwrap_or_default()
                        ),
                        error_line,
                        error_column,
                    ));
                }

                // 7. Type check. Type errors are reported as non-fatal
                // warnings, exactly like the main-file pipeline (main.rs
                // prints them and continues): `include from` executes in the
                // parent scope, so included code must never be checked more
                // strictly than the same code written in the main program
                // (issues #551/#553).
                use crate::diagnostics::DiagnosticReporter;
                use crate::typechecker::{TypeCheckError, TypeChecker};

                let mut tc = TypeChecker::with_analyzer(analyzer);
                if let Err(failure) = tc.check_types(&program) {
                    match failure {
                        // Ordinary type diagnostics stay non-fatal warnings here
                        // (included code must never be checked more strictly than
                        // the same code in the main file). A shared-budget breach
                        // is the exception: the deadline/cancellation/resource
                        // limit was hit while checking the included file, so the
                        // run must stop instead of executing it.
                        TypeCheckError::Budget(exceeded) => {
                            return Err(self.budget_error(exceeded, 0, 0));
                        }
                        TypeCheckError::Types(type_errors) => {
                            eprintln!(
                                "Type checking warnings in included file '{}':",
                                resolved_path.display()
                            );
                            let mut reporter = DiagnosticReporter::new();
                            let file_id = reporter
                                .add_file(resolved_path.display().to_string(), content.clone());
                            for error in &type_errors {
                                let diagnostic = reporter.convert_type_error(file_id, error);
                                if reporter.report_diagnostic(file_id, &diagnostic).is_err() {
                                    eprintln!("{error}");
                                }
                            }
                        }
                    }
                }

                // 8. Create guard for context tracking
                let previous_source = self.current_source_file.borrow().clone();
                let _guard = ModuleLoadGuard::new(self, resolved_path.clone(), previous_source);

                // 9. Execute included file in PARENT scope (key difference from load module)
                // This allows containers/variables to be exposed to parent
                let result = self
                    .execute_block(&program.statements, Rc::clone(&env))
                    .await;

                // 10. Handle result
                match result {
                    Ok((_, ControlFlow::None)) => Ok((Value::Null, ControlFlow::None)),
                    Ok((val, ControlFlow::Return(_))) => {
                        // Return statements in included files are allowed and simply return the value
                        // This enables utility functions in included files to use return statements
                        Ok((val, ControlFlow::None))
                    }
                    Ok((_, ControlFlow::Break)) => Err(RuntimeError::new(
                        "Cannot use 'break' in included file scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Continue)) => Err(RuntimeError::new(
                        "Cannot use 'continue' in included file scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Exit)) => Err(RuntimeError::new(
                        "Cannot use 'exit' in included file scope".to_string(),
                        *line,
                        *column,
                    )),
                    // As in module scope: `exit program` stops the whole run and
                    // must not be retitled as an include failure.
                    Err(e) if e.is_exit_program() => Err(e),
                    Err(e) => {
                        let chain = _guard.get_chain();
                        if chain.len() > 1 {
                            Err(RuntimeError::with_kind(
                                format!(
                                    "Error in include chain {}: {}",
                                    chain.join(" → "),
                                    e.message
                                ),
                                e.line,
                                e.column,
                                e.kind,
                            ))
                        } else {
                            Err(e)
                        }
                    }
                }
            }

            Statement::ExportStatement {
                export_type,
                name,
                line,
                column,
                ..
            } => {
                // Export statement validates that the named item exists in current scope
                // In V1, this is a foundation for future module namespace system

                use crate::parser::ast::ExportType;

                match export_type {
                    ExportType::Container => {
                        // Check if container definition exists in local scope only
                        if let Some(value) = env.borrow().get_local(name) {
                            if matches!(value, Value::ContainerDefinition(_)) {
                                // Container exists - export is valid
                                // In future versions, this would add to export registry
                                Ok((Value::Null, ControlFlow::None))
                            } else {
                                Err(RuntimeError::new(
                                    format!("'{}' is not a container definition", name),
                                    *line,
                                    *column,
                                ))
                            }
                        } else {
                            // Check if it exists in parent scope to provide better error message
                            if env.borrow().get(name).is_some() {
                                Err(RuntimeError::new(
                                    format!(
                                        "Container '{}' is only defined in parent scope and cannot be exported",
                                        name
                                    ),
                                    *line,
                                    *column,
                                ))
                            } else {
                                Err(RuntimeError::new(
                                    format!("Container '{}' not found in current scope", name),
                                    *line,
                                    *column,
                                ))
                            }
                        }
                    }
                    ExportType::Action => {
                        // Check if action definition exists in local scope only
                        if let Some(value) = env.borrow().get_local(name) {
                            if matches!(value, Value::Function(_) | Value::Overloaded(_)) {
                                Ok((Value::Null, ControlFlow::None))
                            } else {
                                Err(RuntimeError::new(
                                    format!("'{}' is not an action definition", name),
                                    *line,
                                    *column,
                                ))
                            }
                        } else {
                            // Check if it exists in parent scope to provide better error message
                            if env.borrow().get(name).is_some() {
                                Err(RuntimeError::new(
                                    format!(
                                        "Action '{}' is only defined in parent scope and cannot be exported",
                                        name
                                    ),
                                    *line,
                                    *column,
                                ))
                            } else {
                                Err(RuntimeError::new(
                                    format!("Action '{}' not found in current scope", name),
                                    *line,
                                    *column,
                                ))
                            }
                        }
                    }
                    ExportType::Constant => {
                        // Check if the variable exists in local scope and is actually a constant
                        if let Some(_value) = env.borrow().get_local(name) {
                            if env.borrow().is_constant(name) {
                                Ok((Value::Null, ControlFlow::None))
                            } else {
                                Err(RuntimeError::new(
                                    format!(
                                        "Variable '{}' is not a constant and cannot be exported as one",
                                        name
                                    ),
                                    *line,
                                    *column,
                                ))
                            }
                        } else {
                            // Check if it exists in parent scope to provide better error message
                            if env.borrow().get(name).is_some() {
                                Err(RuntimeError::new(
                                    format!(
                                        "Constant '{}' is only defined in parent scope and cannot be exported",
                                        name
                                    ),
                                    *line,
                                    *column,
                                ))
                            } else {
                                Err(RuntimeError::new(
                                    format!("Constant '{}' not found in current scope", name),
                                    *line,
                                    *column,
                                ))
                            }
                        }
                    }
                }
            }

            Statement::WaitForStatement {
                inner,
                line: _line,
                column: _column,
            } => {
                match inner.as_ref() {
                    Statement::ExpressionStatement {
                        expression: Expression::Variable(var_name, _, _),
                        line: _,
                        column: _,
                    } => {
                        let max_attempts = 1000; // Prevent infinite waiting
                        for _ in 0..max_attempts {
                            if let Some(value) = env.borrow().get(var_name)
                                && !matches!(value, Value::Null)
                            {
                                return Ok((Value::Null, ControlFlow::None));
                            }

                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                            self.check_time()?;
                        }

                        Err(RuntimeError::new(
                            format!("Timeout waiting for variable '{var_name}'"),
                            0,
                            0,
                        ))
                    }
                    Statement::WriteFileStatement {
                        file,
                        content,
                        mode,
                        line,
                        column,
                    } => {
                        let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;
                        let content_value =
                            self.evaluate_expression(content, Rc::clone(&env)).await?;

                        let file_str = match &file_value {
                            Value::Text(s) => s.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!("Expected string for file handle, got {file_value:?}"),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        let content_str = match &content_value {
                            Value::Text(s) => s.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected string for file content, got {content_value:?}"
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        exec_trace!("Writing to file: {}, content: {}", file_str, content_str);
                        match mode {
                            crate::parser::ast::WriteMode::Append => {
                                match self.io_client.append_file(&file_str, &content_str).await {
                                    Ok(_) => {
                                        exec_trace!("Successfully appended to file");
                                        Ok((Value::Null, ControlFlow::None))
                                    }
                                    Err(e) => {
                                        exec_trace!("Error appending to file: {}", e);
                                        Err(RuntimeError::new(e, *line, *column))
                                    }
                                }
                            }
                            crate::parser::ast::WriteMode::Overwrite => {
                                match self.io_client.write_file(&file_str, &content_str).await {
                                    Ok(_) => {
                                        exec_trace!("Successfully wrote to file");
                                        Ok((Value::Null, ControlFlow::None))
                                    }
                                    Err(e) => {
                                        exec_trace!("Error writing to file: {}", e);
                                        Err(RuntimeError::new(e, *line, *column))
                                    }
                                }
                            }
                        }
                    }
                    Statement::ReadFileStatement {
                        path,
                        variable_name,
                        line,
                        column,
                    } => {
                        exec_trace!("Executing wait for read file statement");
                        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                        let path_str = match &path_value {
                            Value::Text(s) => s.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected string for file path or handle, got {path_value:?}"
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        let is_file_path =
                            matches!(path, Expression::Literal(Literal::String(_), _, _));

                        if is_file_path {
                            match self.io_client.open_file(&path_str).await {
                                Ok(handle) => {
                                    match self.io_client.read_file(&handle, &self.budget).await {
                                        Ok(content) => {
                                            // Capture the define result and drop the
                                            // env borrow before the `close_file` await.
                                            let define_result = env.borrow_mut().define_direct(
                                                variable_name,
                                                Value::Text(content.into()),
                                            );
                                            match define_result {
                                                Ok(_) => {
                                                    let _ =
                                                        self.io_client.close_file(&handle).await;
                                                    Ok((Value::Null, ControlFlow::None))
                                                }
                                                Err(msg) => {
                                                    let _ =
                                                        self.io_client.close_file(&handle).await;
                                                    Err(RuntimeError::new(msg, *line, *column))
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = self.io_client.close_file(&handle).await;
                                            Err(self.file_read_error(e, *line, *column))
                                        }
                                    }
                                }
                                Err(e) => Err(RuntimeError::new(e, *line, *column)),
                            }
                        } else {
                            match self.io_client.read_file(&path_str, &self.budget).await {
                                Ok(content) => {
                                    match env
                                        .borrow_mut()
                                        .define_direct(variable_name, Value::Text(content.into()))
                                    {
                                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                        Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                                    }
                                }
                                Err(e) => Err(self.file_read_error(e, *line, *column)),
                            }
                        }
                    }
                    _ => self.execute_statement(inner, Rc::clone(&env)).await,
                }
            }
            Statement::WaitForDurationStatement {
                duration,
                unit,
                line,
                column,
            } => {
                let duration_value = self.evaluate_expression(duration, Rc::clone(&env)).await?;
                let duration_ms = match &duration_value {
                    Value::Number(n) => match unit.as_str() {
                        "milliseconds" => *n as u64,
                        "seconds" => (*n * 1000.0) as u64,
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Unsupported time unit: {}", unit),
                                *line,
                                *column,
                            ));
                        }
                    },
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected number for duration, got {duration_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // While WebSocket servers are running, spend the wait window
                // dispatching their events to the registered handler blocks.
                // With no WebSocket servers this is an ordinary sleep, so the
                // statement's timing semantics are unchanged.
                self.pump_websocket_events(std::time::Duration::from_millis(duration_ms))
                    .await?;
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                line: _line,
                column: _column,
            } => {
                let child_env = Environment::new_child_env(&env);

                let primary_result = match self.execute_block(body, Rc::clone(&child_env)).await {
                    Ok(val) => Ok(val), // Success path: just bubble result
                    // `exit program` is a request to stop, not a failure: no
                    // `when`/`otherwise` clause may swallow it. (`finally:`
                    // below still runs, so cleanup is not skipped.)
                    Err(err) if err.is_exit_program() => Err(err),
                    Err(err) => {
                        // Find matching when clause based on error kind
                        let mut executed = false;
                        let mut result = Err(err.clone());

                        for when_clause in when_clauses {
                            let matches = match &when_clause.error_type {
                                crate::parser::ast::ErrorType::General => true, // General catches all errors
                                crate::parser::ast::ErrorType::FileNotFound => {
                                    err.kind == ErrorKind::FileNotFound
                                }
                                crate::parser::ast::ErrorType::PermissionDenied => {
                                    err.kind == ErrorKind::PermissionDenied
                                }
                                crate::parser::ast::ErrorType::ProcessNotFound => {
                                    err.kind == ErrorKind::ProcessNotFound
                                }
                                crate::parser::ast::ErrorType::ProcessSpawnFailed => {
                                    err.kind == ErrorKind::ProcessSpawnFailed
                                }
                                crate::parser::ast::ErrorType::ProcessKillFailed => {
                                    err.kind == ErrorKind::ProcessKillFailed
                                }
                                crate::parser::ast::ErrorType::CommandNotFound => {
                                    err.kind == ErrorKind::CommandNotFound
                                }
                            };

                            if matches {
                                // Bind the error under clause-local aliases.
                                // Ordinary handler-created bindings remain in
                                // the shared try environment for `finally`, but
                                // these aliases must reveal any previous local
                                // or outer bindings again when the clause ends.
                                let error_text = Value::Text(err.message.into());
                                let (saved_error_name, saved_error_message) = {
                                    let mut env_mut = child_env.borrow_mut();
                                    let saved_error_name =
                                        env_mut.take_local_binding(&when_clause.error_name);
                                    let saved_error_message =
                                        if when_clause.error_name == "error_message" {
                                            None
                                        } else {
                                            env_mut.take_local_binding("error_message")
                                        };
                                    env_mut.define_or_replace(
                                        &when_clause.error_name,
                                        error_text.clone(),
                                    );
                                    env_mut.define_or_replace("error_message", error_text);
                                    (saved_error_name, saved_error_message)
                                };

                                result = self
                                    .execute_block(&when_clause.body, Rc::clone(&child_env))
                                    .await;
                                {
                                    let mut env_mut = child_env.borrow_mut();
                                    if when_clause.error_name != "error_message" {
                                        env_mut.restore_local_binding(
                                            "error_message",
                                            saved_error_message,
                                        );
                                    }
                                    env_mut.restore_local_binding(
                                        &when_clause.error_name,
                                        saved_error_name,
                                    );
                                }
                                executed = true;
                                break;
                            }
                        }

                        // If no when clause matched and there's an otherwise block
                        if !executed && otherwise_block.is_some() {
                            result = self
                                .execute_block(
                                    otherwise_block.as_ref().unwrap(),
                                    Rc::clone(&child_env),
                                )
                                .await;
                        }

                        result
                    }
                };

                // A `finally:` block runs on both the success and error paths,
                // after any matching when/otherwise clause. If it raises its own
                // error, that error wins. Abrupt control flow from finally
                // (`return`, `break`, `continue`, `exit`) also wins; an ordinary
                // fallthrough preserves the primary result.
                if let Some(finally_stmts) = finally_block {
                    match self.execute_block(finally_stmts, child_env).await {
                        Ok((_, ControlFlow::None)) => primary_result,
                        Ok(finally_result) => Ok(finally_result),
                        Err(finally_err) => Err(finally_err),
                    }
                } else {
                    primary_result
                }
            }
            Statement::HttpGetStatement {
                url,
                variable_name,
                line,
                column,
            } => {
                let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
                let url_str = match &url_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for URL, got {url_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self
                    .io_client
                    .http_get(&url_str, Arc::clone(&self.budget))
                    .await
                {
                    Ok(body) => {
                        match env
                            .borrow_mut()
                            .define_direct(variable_name, Value::Text(body.into()))
                        {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(error) => Err(self.http_client_error(error, *line, *column)),
                }
            }
            Statement::HttpPostStatement {
                url,
                data,
                variable_name,
                line,
                column,
            } => {
                let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
                let data_val = self.evaluate_expression(data, Rc::clone(&env)).await?;

                let url_str = match &url_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for URL, got {url_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let data_str = match &data_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for data, got {data_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self
                    .io_client
                    .http_post(&url_str, &data_str, Arc::clone(&self.budget))
                    .await
                {
                    Ok(body) => {
                        match env
                            .borrow_mut()
                            .define_direct(variable_name, Value::Text(body.into()))
                        {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(error) => Err(self.http_client_error(error, *line, *column)),
                }
            }
            Statement::HttpRequestStatement {
                url,
                method,
                headers,
                body,
                variable_name,
                full_response,
                line,
                column,
            } => {
                let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
                let url_str = match &url_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for URL, got {url_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let method_str = match method {
                    Some(method_expr) => {
                        let method_val = self
                            .evaluate_expression(method_expr, Rc::clone(&env))
                            .await?;
                        match &method_val {
                            Value::Text(s) => s.trim().to_ascii_uppercase(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!("Expected text for HTTP method, got {method_val:?}"),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    None => "GET".to_string(),
                };

                let mut header_list: Vec<(String, String)> = Vec::new();
                if let Some(headers_expr) = headers {
                    let headers_val = self
                        .evaluate_expression(headers_expr, Rc::clone(&env))
                        .await?;
                    match &headers_val {
                        Value::Object(obj) => {
                            for (name, value) in obj.borrow().iter() {
                                let value_str = match value {
                                    Value::Text(s) => s.to_string(),
                                    Value::Number(_) | Value::Bool(_) => value.to_string(),
                                    _ => {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Header '{name}' must be text, got {}",
                                                value.type_name()
                                            ),
                                            *line,
                                            *column,
                                        ));
                                    }
                                };
                                header_list.push((name.clone(), value_str));
                            }
                            // HashMap iteration order is random; sort for
                            // deterministic requests and error messages
                            header_list.sort();
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Expected a map for headers, got {}",
                                    headers_val.type_name()
                                ),
                                *line,
                                *column,
                            ));
                        }
                    }
                }

                let body_str = match body {
                    Some(body_expr) => {
                        let body_val = self.evaluate_expression(body_expr, Rc::clone(&env)).await?;
                        match &body_val {
                            Value::Text(s) => Some(s.to_string()),
                            Value::Number(_) | Value::Bool(_) => Some(body_val.to_string()),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected text for request body, got {}",
                                        body_val.type_name()
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    None => None,
                };

                match self
                    .io_client
                    .http_request(
                        &method_str,
                        &url_str,
                        &header_list,
                        body_str,
                        Arc::clone(&self.budget),
                    )
                    .await
                {
                    Ok((status, response_headers, response_body)) => {
                        let value = if *full_response {
                            let mut headers_map = HashMap::new();
                            for (name, value) in response_headers {
                                headers_map.insert(name, Value::Text(value.into()));
                            }

                            let mut response_map = HashMap::new();
                            response_map.insert("status".to_string(), Value::Number(status as f64));
                            response_map.insert(
                                "ok".to_string(),
                                Value::Bool((200..300).contains(&status)),
                            );
                            response_map
                                .insert("body".to_string(), Value::Text(response_body.into()));
                            response_map.insert(
                                "headers".to_string(),
                                Value::Object(Rc::new(RefCell::new(headers_map))),
                            );
                            Value::Object(Rc::new(RefCell::new(response_map)))
                        } else {
                            Value::Text(response_body.into())
                        };

                        match env.borrow_mut().define_direct(variable_name, value) {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(error) => Err(self.http_client_error(error, *line, *column)),
                }
            }
            Statement::HttpStreamStatement {
                url,
                method,
                headers,
                body,
                variable_name,
                line,
                column,
            } => {
                let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
                let url_str = match &url_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for URL, got {url_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let method_str = match method {
                    Some(method_expr) => {
                        let method_val = self
                            .evaluate_expression(method_expr, Rc::clone(&env))
                            .await?;
                        match &method_val {
                            Value::Text(s) => s.trim().to_ascii_uppercase(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!("Expected text for HTTP method, got {method_val:?}"),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    None => "GET".to_string(),
                };

                let mut header_list: Vec<(String, String)> = Vec::new();
                if let Some(headers_expr) = headers {
                    let headers_val = self
                        .evaluate_expression(headers_expr, Rc::clone(&env))
                        .await?;
                    match &headers_val {
                        Value::Object(obj) => {
                            for (name, value) in obj.borrow().iter() {
                                let value_str = match value {
                                    Value::Text(s) => s.to_string(),
                                    Value::Number(_) | Value::Bool(_) => value.to_string(),
                                    _ => {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Header '{name}' must be text, got {}",
                                                value.type_name()
                                            ),
                                            *line,
                                            *column,
                                        ));
                                    }
                                };
                                header_list.push((name.clone(), value_str));
                            }
                            header_list.sort();
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Expected a map for headers, got {}",
                                    headers_val.type_name()
                                ),
                                *line,
                                *column,
                            ));
                        }
                    }
                }

                let body_str = match body {
                    Some(body_expr) => {
                        let body_val = self.evaluate_expression(body_expr, Rc::clone(&env)).await?;
                        match &body_val {
                            Value::Text(s) => Some(s.to_string()),
                            Value::Number(_) | Value::Bool(_) => Some(body_val.to_string()),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected text for request body, got {}",
                                        body_val.type_name()
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    None => None,
                };

                // Race the head open (connect + await response head) against a
                // client disconnect, so a browser that goes away while the upstream
                // withholds its head cancels the open promptly (dropping `open_fut`
                // aborts the upstream connection) instead of waiting out the head
                // timeout. Valid before `start streaming response` via the pending
                // request's oneshot (see `any_pending_request_disconnected`).
                let open_fut = self.io_client.open_http_stream(
                    &method_str,
                    &url_str,
                    &header_list,
                    body_str,
                    Arc::clone(&self.budget),
                );
                let disconnect = self.any_client_disconnected(self.downstream_disconnect_senders());
                let opened = {
                    tokio::pin!(open_fut);
                    tokio::pin!(disconnect);
                    tokio::select! {
                        r = &mut open_fut => r,
                        _ = &mut disconnect => Err(HttpClientError::Disconnected),
                    }
                };
                match opened {
                    Ok((status, response_headers, handle_id)) => {
                        // Track the outbound handle as handler-owned so it is
                        // dropped (cancelling the upstream) if the handler ends
                        // without closing/exhausting it.
                        let owner = Arc::clone(&self.open_http_streams.borrow());
                        self.io_client
                            .claim_stream_owner(&handle_id, &owner)
                            .map_err(|error| self.http_client_error(error, *line, *column))?;
                        let mut headers_map = HashMap::new();
                        for (name, value) in response_headers {
                            headers_map.insert(name, Value::Text(value.into()));
                        }

                        let mut stream_map = HashMap::new();
                        stream_map.insert("status".to_string(), Value::Number(status as f64));
                        stream_map
                            .insert("ok".to_string(), Value::Bool((200..300).contains(&status)));
                        stream_map.insert(
                            "headers".to_string(),
                            Value::Object(Rc::new(RefCell::new(headers_map))),
                        );
                        // Internal id used by `wait for next chunk|line` and
                        // `close`. Underscore-prefixed to signal "not for
                        // direct program use", mirroring request objects.
                        stream_map.insert("_stream".to_string(), Value::Text(handle_id.into()));

                        let value = Value::Object(Rc::new(RefCell::new(stream_map)));
                        // define_or_replace so a `main loop` handler that rebinds
                        // `upstream` on each request works.
                        env.borrow_mut().define_or_replace(variable_name, value);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    Err(error) => Err(self.http_client_error(error, *line, *column)),
                }
            }
            Statement::WaitForNextChunkStatement {
                source,
                variable_name,
                line,
                column,
            } => {
                let handle_id = self
                    .resolve_stream_handle(source, &env, *line, *column)
                    .await?;
                // Race the upstream read against a client disconnect (either an
                // open downstream response stream, or — if the handler has not
                // called `start streaming response` yet — the pending request's
                // oneshot) so a blocked proxy read is cancelled promptly when the
                // browser goes away.
                let disconnect = self.any_client_disconnected(self.downstream_disconnect_senders());
                let read = self
                    .io_client
                    .next_chunk(&handle_id, Arc::clone(&self.budget));
                let outcome = {
                    tokio::pin!(read);
                    tokio::pin!(disconnect);
                    tokio::select! {
                        r = &mut read => r,
                        _ = &mut disconnect => {
                            // Client gone: dropping `read` above already cancels
                            // the upstream; close the handle too in case the read
                            // had not yet taken it, and report the disconnect as a
                            // cooperative cancellation (not a handler failure).
                            self.io_client.close_stream(&handle_id).await;
                            Err(HttpClientError::Disconnected)
                        }
                    }
                };
                match outcome {
                    // Raw bytes as Binary so callers can handle any payload.
                    // define_or_replace (not define) so re-reading into the same
                    // variable across a loop refreshes it, matching
                    // `wait for request ... as req`.
                    Ok(Some(bytes)) => {
                        let value = Value::Binary(Arc::from(bytes.as_slice()));
                        env.borrow_mut().define_or_replace(variable_name, value);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    // Clean EOF binds `nothing` so `check if chunk is nothing` ends the loop.
                    Ok(None) => {
                        // The handle left the map at EOF — stop owning it.
                        self.untrack_http_stream(&handle_id);
                        env.borrow_mut()
                            .define_or_replace(variable_name, Value::Null);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    Err(error) => {
                        // The read dropped the handle (timeout/cancel/error).
                        self.untrack_http_stream(&handle_id);
                        Err(self.http_client_error(error, *line, *column))
                    }
                }
            }
            Statement::WaitForNextLineStatement {
                source,
                variable_name,
                line,
                column,
            } => {
                let handle_id = self
                    .resolve_stream_handle(source, &env, *line, *column)
                    .await?;
                // Race the upstream read against a client disconnect by EITHER
                // signal (open downstream stream OR the pre-response pending
                // request's oneshot), exactly like the `wait for next chunk`
                // handler — a line read blocked before `start streaming response`
                // must also be cancelled the moment the browser goes away.
                let disconnect = self.any_client_disconnected(self.downstream_disconnect_senders());
                let read = self
                    .io_client
                    .next_line(&handle_id, Arc::clone(&self.budget));
                let outcome = {
                    tokio::pin!(read);
                    tokio::pin!(disconnect);
                    tokio::select! {
                        r = &mut read => r,
                        _ = &mut disconnect => {
                            self.io_client.close_stream(&handle_id).await;
                            Err(HttpClientError::Disconnected)
                        }
                    }
                };
                match outcome {
                    Ok(Some(line_text)) => {
                        let value = Value::Text(line_text.into());
                        env.borrow_mut().define_or_replace(variable_name, value);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    Ok(None) => {
                        self.untrack_http_stream(&handle_id);
                        env.borrow_mut()
                            .define_or_replace(variable_name, Value::Null);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    Err(error) => {
                        self.untrack_http_stream(&handle_id);
                        Err(self.http_client_error(error, *line, *column))
                    }
                }
            }
            Statement::RepeatWhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let loop_env = Environment::new_child_env(&env);
                let mut _last_value = Value::Null;

                loop {
                    self.check_time()?;

                    let condition_value = self
                        .evaluate_expression(condition, Rc::clone(&loop_env))
                        .await?;

                    if !condition_value.is_truthy() {
                        break;
                    }

                    let result = self.execute_block(body, Rc::clone(&loop_env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of repeat-while loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing repeat-while loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from repeat-while loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from repeat-while loop");
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }

                Ok((_last_value, ControlFlow::None))
            }
            Statement::PushStatement {
                list,
                value,
                line,
                column,
            } => {
                let list_val = self.evaluate_expression(list, Rc::clone(&env)).await?;
                let value_val = self.evaluate_expression(value, Rc::clone(&env)).await?;

                match list_val {
                    Value::List(list_rc) => {
                        list_rc.borrow_mut().push(value_val);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot push to non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::CreateListStatement {
                name,
                initial_values,
                line,
                column,
            } => {
                // Create a new list with initial values
                let mut list_items = Vec::new();
                for value_expr in initial_values {
                    let value = self
                        .evaluate_expression(value_expr, Rc::clone(&env))
                        .await?;
                    list_items.push(value);
                }

                let list_value = Value::List(Rc::new(RefCell::new(list_items)));
                match env.borrow_mut().define_direct(name, list_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::MapCreation {
                name,
                entries,
                line,
                column,
            } => {
                // Create a new map/object with initial entries
                let mut map = std::collections::HashMap::new();
                for (key, value_expr) in entries {
                    let value = self
                        .evaluate_expression(value_expr, Rc::clone(&env))
                        .await?;
                    map.insert(key.clone(), value);
                }

                let map_value = Value::Object(Rc::new(RefCell::new(map)));
                match env.borrow_mut().define_direct(name, map_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::CreateDateStatement {
                name,
                value,
                line,
                column,
            } => {
                let date_value = if let Some(expr) = value {
                    // Evaluate the expression to get the date
                    self.evaluate_expression(expr, Rc::clone(&env)).await?
                } else {
                    // Default to today's date
                    let today = chrono::Local::now().date_naive();
                    Value::Date(Rc::new(today))
                };

                match env.borrow_mut().define_direct(name, date_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::CreateTimeStatement {
                name,
                value,
                line,
                column,
            } => {
                let time_value = if let Some(expr) = value {
                    // Evaluate the expression to get the time
                    self.evaluate_expression(expr, Rc::clone(&env)).await?
                } else {
                    // Default to current time
                    let now = chrono::Local::now().time();
                    Value::Time(Rc::new(now))
                };

                match env.borrow_mut().define_direct(name, time_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::AddToListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                // Evaluate the value to add
                let value_to_add = self.evaluate_expression(value, Rc::clone(&env)).await?;

                // Get the list from the environment
                let list_val = env.borrow().get(list_name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined variable: {list_name}"), *line, *column)
                })?;

                match list_val {
                    Value::List(list_rc) => {
                        list_rc.borrow_mut().push(value_to_add);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    Value::Number(_) => {
                        // This is actually arithmetic add
                        // Convert to arithmetic operation
                        let current = list_val;
                        if let (Value::Number(n1), Value::Number(n2)) = (&current, &value_to_add) {
                            let result = Value::Number(n1 + n2);
                            env.borrow_mut()
                                .assign(list_name, result)
                                .map_err(|e| RuntimeError::new(e, *line, *column))?;
                            Ok((Value::Null, ControlFlow::None))
                        } else {
                            Err(RuntimeError::new(
                                "Cannot add non-numeric value to number".to_string(),
                                *line,
                                *column,
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot add to non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::RemoveFromListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                // Evaluate the value to remove
                let value_to_remove = self.evaluate_expression(value, Rc::clone(&env)).await?;

                // Get the list from the environment
                let list_val = env.borrow().get(list_name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined variable: {list_name}"), *line, *column)
                })?;

                match list_val {
                    Value::List(list_rc) => {
                        let mut list = list_rc.borrow_mut();
                        // Remove the first occurrence of the value
                        if let Some(pos) = list.iter().position(|v| v == &value_to_remove) {
                            list.remove(pos);
                        }
                        Ok((Value::Null, ControlFlow::None))
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot remove from non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::ClearListStatement {
                list_name,
                line,
                column,
            } => {
                // Get the list from the environment
                let list_val = env.borrow().get(list_name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined variable: {list_name}"), *line, *column)
                })?;

                match list_val {
                    Value::List(list_rc) => {
                        list_rc.borrow_mut().clear();
                        Ok((Value::Null, ControlFlow::None))
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot clear non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            // Container-related statements
            Statement::ContainerDefinition {
                name,
                extends,
                implements,
                properties,
                methods,
                events,
                static_properties,
                static_methods,
                line,
                column,
            } => {
                // Create a new container definition
                let mut container_properties = HashMap::new();
                let mut container_methods = HashMap::new();
                let mut container_static_properties = HashMap::new();
                let mut container_static_methods = HashMap::new();

                for prop in properties {
                    let property_type_str = prop
                        .property_type
                        .as_ref()
                        .map(|ast_type| format!("{ast_type:?}"));

                    let default_val = match &prop.default_value {
                        Some(expr) => {
                            // Evaluate the default expression to get a Value
                            (self._evaluate_expression(expr, env.clone()).await).ok()
                        }
                        None => None,
                    };

                    let value_prop = value::PropertyDefinition {
                        name: prop.name.clone(),
                        property_type: property_type_str,
                        default_value: default_val,
                        validation_rules: Vec::new(),
                        is_static: false,
                        is_public: true,
                        line: prop.line,
                        column: prop.column,
                    };
                    container_properties.insert(prop.name.clone(), value_prop);
                }

                for method in methods {
                    if let Statement::ActionDefinition {
                        name,
                        parameters,
                        body,
                        return_type,
                        line,
                        column,
                        ..
                    } = method
                    {
                        let container_method = ContainerMethodValue {
                            name: name.clone(),
                            params: parameters.iter().map(|p| p.name.clone()).collect(),
                            return_type: return_type.clone(),
                            body: body.clone(),
                            is_static: false,
                            is_public: true,
                            env: Rc::downgrade(&env),
                            line: *line,
                            column: *column,
                        };
                        // TODO(#638): container methods do not support
                        // overloading — a repeated method name silently keeps
                        // the last definition here. Route same-name methods
                        // through an overload set (see
                        // `Environment::define_or_merge_action` /
                        // `select_overload` for the action equivalent).
                        container_methods.insert(name.clone(), container_method);
                    }
                }

                for prop in static_properties {
                    let value = match &prop.default_value {
                        Some(expression) => {
                            self._evaluate_expression(expression, env.clone()).await?
                        }
                        None => Value::Null,
                    };
                    container_static_properties.insert(prop.name.clone(), value);
                }

                for method in static_methods {
                    if let Statement::ActionDefinition {
                        name,
                        parameters,
                        body,
                        return_type,
                        line,
                        column,
                        ..
                    } = method
                    {
                        container_static_methods.insert(
                            name.clone(),
                            ContainerMethodValue {
                                name: name.clone(),
                                params: parameters.iter().map(|p| p.name.clone()).collect(),
                                return_type: return_type.clone(),
                                body: body.clone(),
                                is_static: true,
                                is_public: true,
                                env: Rc::downgrade(&env),
                                line: *line,
                                column: *column,
                            },
                        );
                    }
                }

                // Process events
                let mut container_events = HashMap::new();
                for event in events {
                    let container_event = ContainerEventValue {
                        name: event.name.clone(),
                        params: event.parameters.iter().map(|p| p.name.clone()).collect(),
                        handlers: Vec::new(),
                        line: event.line,
                        column: event.column,
                    };
                    container_events.insert(event.name.clone(), container_event);
                }

                // Enforce interface contracts before the definition becomes
                // visible: a container that claims 'implements X' must provide
                // every action X (and anything X extends) requires.
                Self::validate_interface_conformance(
                    &env,
                    name,
                    extends.as_ref(),
                    &container_methods,
                    implements,
                    *line,
                    *column,
                )?;

                let container_def = ContainerDefinitionValue {
                    name: name.clone(),
                    extends: extends.clone(),
                    implements: implements.clone(),
                    properties: container_properties,
                    methods: container_methods,
                    events: container_events,
                    static_properties: Rc::new(RefCell::new(container_static_properties)),
                    static_methods: container_static_methods,
                    line: *line,
                    column: *column,
                };

                // Create the container definition value
                let container_value = Value::ContainerDefinition(Rc::new(container_def));

                // A nested type declaration follows the same method-local
                // property-shadowing rule as action declarations.
                let shadows_property = self.definition_shadows_container_property(&env, name);
                let result = if shadows_property {
                    env.borrow_mut()
                        .define_direct(name, container_value.clone())
                } else {
                    env.borrow_mut().define(name, container_value.clone())
                };
                match result {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                Ok((container_value, ControlFlow::None))
            }
            Statement::ContainerInstantiation {
                container_type,
                instance_name,
                arguments,
                property_initializers,
                line,
                column,
            } => {
                // Create container instance with inheritance support
                let mut instance = self.create_container_instance_with_inheritance(
                    container_type,
                    &env,
                    *line,
                    *column,
                )?;

                // Process property initializers (override inherited properties)
                for initializer in property_initializers {
                    let init_value = self
                        ._evaluate_expression(&initializer.value, env.clone())
                        .await?;
                    instance
                        .properties
                        .insert(initializer.name.clone(), init_value);
                }

                let instance_value = Value::ContainerInstance(Rc::new(RefCell::new(instance)));

                // Store the instance in the environment
                match env
                    .borrow_mut()
                    .define_direct(instance_name, instance_value.clone())
                {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                // Call constructor method if arguments are provided
                if !arguments.is_empty() {
                    // Look up the container definition to find the initialize method
                    let container_def = match env.borrow().get(container_type) {
                        Some(Value::ContainerDefinition(def)) => def.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Container '{container_type}' not found"),
                                *line,
                                *column,
                            ));
                        }
                    };

                    // Check if the container has an "initialize" method
                    if let Some(init_method) = container_def.methods.get("initialize") {
                        // Create a function value from the initialize method
                        let init_function = FunctionValue {
                            name: Some("initialize".to_string()),
                            params: init_method.params.clone(),
                            param_types: vec![None; init_method.params.len()],
                            body: init_method.body.clone(),
                            env: init_method.env.clone(),
                            line: init_method.line,
                            column: init_method.column,
                            enforce_param_types: std::cell::Cell::new(false),
                            static_method_context: None,
                        };

                        // Create a new environment for the constructor execution
                        let init_env = Environment::new_child_env(&env);

                        // Add 'this' to the environment (the instance being constructed)
                        let _ = init_env.borrow_mut().define("this", instance_value.clone());

                        // Evaluate the arguments
                        let mut arg_values = Vec::with_capacity(arguments.len());
                        for arg in arguments {
                            let arg_val = self.evaluate_expression(&arg.value, env.clone()).await?;
                            arg_values.push(arg_val);
                        }

                        // Call the initialize method
                        self.call_function(&init_function, arg_values, *line, *column)
                            .await?;
                    } else if !arguments.is_empty() {
                        return Err(RuntimeError::new(
                            format!(
                                "Container '{container_type}' does not have an initialize method but arguments were provided"
                            ),
                            *line,
                            *column,
                        ));
                    }
                }

                Ok((instance_value, ControlFlow::None))
            }
            Statement::InterfaceDefinition {
                name,
                extends,
                required_actions,
                line: _line,
                column: _column,
            } => {
                // Create a new interface definition
                let mut interface_required_actions = HashMap::new();

                for action in required_actions {
                    let value_action = value::ActionSignature {
                        name: action.name.clone(),
                        params: action.parameters.iter().map(|p| p.name.clone()).collect(),
                        return_type: action.return_type.clone(),
                        line: action.line,
                        column: action.column,
                    };
                    interface_required_actions.insert(action.name.clone(), value_action);
                }

                let interface_def = InterfaceDefinitionValue {
                    name: name.clone(),
                    extends: extends.clone(),
                    required_actions: interface_required_actions,
                    line: *_line,
                    column: *_column,
                };

                let interface_value = Value::InterfaceDefinition(Rc::new(interface_def));

                // Match the analyzer's lexical binding model without relaxing
                // ordinary outer-scope shadowing.
                let shadows_property = self.definition_shadows_container_property(&env, name);
                let result = if shadows_property {
                    env.borrow_mut()
                        .define_direct(name, interface_value.clone())
                } else {
                    env.borrow_mut().define(name, interface_value.clone())
                };
                match result {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *_line, *_column)),
                }

                Ok((interface_value, ControlFlow::None))
            }
            Statement::EventDefinition {
                name,
                parameters,
                line: _line,
                column: _column,
            } => {
                // Create a new event definition
                let event_def = ContainerEventValue {
                    name: name.clone(),
                    params: parameters.iter().map(|p| p.name.clone()).collect(),
                    handlers: Vec::new(),
                    line: *_line,
                    column: *_column,
                };

                let event_value = Value::ContainerEvent(Rc::new(event_def));

                // Store the event definition in the environment
                match env.borrow_mut().define(name, event_value.clone()) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *_line, *_column)),
                }

                Ok((event_value, ControlFlow::None))
            }
            Statement::EventTrigger {
                name,
                arguments,
                line: _line,
                column: _column,
            } => {
                // Look up the event
                let event = match env.borrow().get(name) {
                    Some(Value::ContainerEvent(event)) => event.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Event '{name}' not found"),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Evaluate the arguments
                let mut arg_values = Vec::with_capacity(arguments.len());
                for arg in arguments {
                    let arg_val = self
                        .evaluate_expression(&arg.value, Rc::clone(&env))
                        .await?;
                    arg_values.push(arg_val);
                }

                // Execute all event handlers
                for handler in &event.handlers {
                    // Create a new environment for the handler
                    let handler_env = Environment::new_child_env(&env);

                    // Bind arguments to parameters. Use define_direct so a
                    // parameter shadows any same-named global rather than being
                    // rejected as already-defined-in-outer-scope (#582).
                    for (i, param_name) in event.params.iter().enumerate() {
                        if i < arg_values.len() {
                            let _ = handler_env
                                .borrow_mut()
                                .define_direct(param_name, arg_values[i].clone());
                        } else {
                            let _ = handler_env
                                .borrow_mut()
                                .define_direct(param_name, Value::Null);
                        }
                    }

                    // Execute the handler
                    self.execute_block(&handler.body, handler_env).await?;
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::EventHandler {
                event_source,
                event_name,
                handler_body,
                line: _line,
                column: _column,
            } => {
                // Evaluate the event source
                let source_val = self
                    .evaluate_expression(event_source, Rc::clone(&env))
                    .await?;

                // Check if the source is a container instance
                if let Value::ContainerInstance(instance_rc) = &source_val {
                    let instance = instance_rc.borrow();
                    let container_type = instance.container_type.clone();

                    // Look up the container definition
                    let container_def = match env.borrow().get(&container_type) {
                        Some(Value::ContainerDefinition(def)) => def.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Container '{container_type}' not found"),
                                *_line,
                                *_column,
                            ));
                        }
                    };

                    // Look up the event
                    if let Some(event) = container_def.events.get(event_name) {
                        // Create a new event handler
                        let handler = EventHandler {
                            body: handler_body.clone(),
                            env: Rc::downgrade(&env),
                            line: *_line,
                            column: *_column,
                        };

                        // Create a new event with the handler added
                        let mut handlers = event.handlers.clone();
                        handlers.push(handler);

                        // Create a new event value
                        let new_event = ContainerEventValue {
                            name: event.name.clone(),
                            params: event.params.clone(),
                            handlers,
                            line: event.line,
                            column: event.column,
                        };

                        // Store the updated event in the environment
                        let event_value = Value::ContainerEvent(Rc::new(new_event));
                        let _ = env.borrow_mut().define(event_name, event_value.clone());

                        Ok((Value::Null, ControlFlow::None))
                    } else {
                        Err(RuntimeError::new(
                            format!(
                                "Event '{event_name}' not found in container '{container_type}'"
                            ),
                            *_line,
                            *_column,
                        ))
                    }
                } else {
                    Err(RuntimeError::new(
                        "Cannot add event handler to non-container value".to_string(),
                        *_line,
                        *_column,
                    ))
                }
            }
            Statement::ParentMethodCall {
                method_name,
                arguments,
                line,
                column,
            } => {
                // Get the current container instance (this)
                let this_val = match env.borrow().get("this") {
                    Some(val) => val.clone(),
                    None => {
                        return Err(RuntimeError::new(
                            "Parent method call can only be used inside a container method"
                                .to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Check if this is a container instance
                if let Value::ContainerInstance(instance_rc) = &this_val {
                    // Clone the parent Rc out so the instance's RefCell borrow does
                    // not span the awaited method call below (a sibling handler
                    // could otherwise re-borrow the same instance across the yield).
                    let parent_opt = instance_rc.borrow().parent.clone();

                    // Check if the instance has a parent
                    if let Some(parent_rc) = parent_opt {
                        // Read the parent's type, then release its borrow too — no
                        // container RefCell borrow is held across the `.await`s.
                        let parent_type = parent_rc.borrow().container_type.clone();

                        // Look up the parent container definition
                        let parent_def = match env.borrow().get(&parent_type) {
                            Some(Value::ContainerDefinition(def)) => def.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!("Parent container '{parent_type}' not found"),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        // Look up the method in the parent
                        if let Some(method_val) = parent_def.methods.get(method_name) {
                            // Create a function value from the method
                            let function = FunctionValue {
                                name: Some(method_val.name.clone()),
                                params: method_val.params.clone(),
                                param_types: vec![None; method_val.params.len()],
                                body: method_val.body.clone(),
                                env: method_val.env.clone(),
                                line: method_val.line,
                                column: method_val.column,
                                enforce_param_types: std::cell::Cell::new(false),
                                static_method_context: None,
                            };

                            // Create a new environment for the method execution
                            let method_env = Environment::new_child_env(&env);

                            // Add 'this' to the environment (the current instance, not the parent)
                            let _ = method_env.borrow_mut().define("this", this_val.clone());

                            // Evaluate the arguments
                            let mut arg_values = Vec::with_capacity(arguments.len());
                            for arg in arguments {
                                let arg_val = self
                                    .evaluate_expression(&arg.value, Rc::clone(&env))
                                    .await?;
                                arg_values.push(arg_val);
                            }

                            // Call the function
                            let result = self
                                .call_function(&function, arg_values, *line, *column)
                                .await?;

                            Ok((result, ControlFlow::None))
                        } else {
                            Err(RuntimeError::new(
                                format!(
                                    "Method '{method_name}' not found in parent container '{parent_type}'"
                                ),
                                *line,
                                *column,
                            ))
                        }
                    } else {
                        Err(RuntimeError::new(
                            "Cannot call parent method: no parent container".to_string(),
                            *line,
                            *column,
                        ))
                    }
                } else {
                    Err(RuntimeError::new(
                        "Parent method call can only be used inside a container method".to_string(),
                        *line,
                        *column,
                    ))
                }
            }
            Statement::PatternDefinition {
                name,
                pattern,
                line,
                column,
                ..
            } => {
                // Compile the pattern AST into bytecode with environment access for list references
                let compiled_pattern = {
                    let env_borrow = env.borrow();
                    CompiledPattern::compile_with_env(pattern, &env_borrow)
                };
                match compiled_pattern {
                    Ok(compiled_pattern) => {
                        // Store the compiled pattern in the environment
                        let pattern_value = Value::Pattern(Rc::new(compiled_pattern));
                        match env.borrow_mut().define_direct(name, pattern_value.clone()) {
                            Ok(_) => {}
                            Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                        }
                        Ok((pattern_value, ControlFlow::None))
                    }
                    Err(compile_error) => Err(RuntimeError {
                        kind: ErrorKind::General,
                        message: format!("Failed to compile pattern '{name}': {compile_error}"),
                        line: *line,
                        column: *column,
                    }),
                }
            }
            Statement::ListenStatement {
                port,
                server_name,
                tls,
                redirect_to_port,
                sessions_enabled,
                line,
                column,
            } => {
                let port_val = self.evaluate_expression(port, Rc::clone(&env)).await?;
                let port_num = match &port_val {
                    Value::Number(n) => *n as u16,
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected number for port, got {port_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // Create request/response channels. The request queue is bounded
                // (Phase 0, PR-0c) so a flood of accepted-but-unhandled requests
                // sheds with 503 instead of growing memory without bound.
                let queue_bound = self.budget.max_pending_requests();
                let (request_sender, request_receiver) =
                    mpsc::channel::<WflHttpRequest>(queue_bound);
                let request_receiver = Arc::new(tokio::sync::Mutex::new(request_receiver));

                // Create warp routes that handle all HTTP methods and paths.
                // In-flight admission uses the shared ExecutionBudget's global
                // request cap (RequestGuard), so a flood is bounded across every
                // listener — not per-server. The guard is acquired *before* the
                // body is read and held until the handler answers, the request
                // times out, or the client disconnects, so a dequeued request
                // can no longer pin memory indefinitely. Body size is enforced
                // *while streaming* (below), which bounds chunked bodies that
                // carry no Content-Length.
                let session_manager = if *sessions_enabled {
                    Some(Rc::new(
                        sessions::SessionManager::new(sessions::SessionConfig::from_wfl_config(
                            &self.config,
                        ))
                        .await
                        .map_err(|e| RuntimeError::new(e, *line, *column))?,
                    ))
                } else {
                    None
                };

                let request_sender_clone = request_sender.clone();
                let max_body_size = self.budget.max_request_body_bytes();
                let max_body_size_u64 = max_body_size as u64;
                let request_timeout = self.budget.max_request_duration();
                let admit_budget = Arc::clone(&self.budget);
                let routes = warp::any()
                    .and(warp::method())
                    .and(warp::path::full())
                    .and(warp::query::raw().or(warp::any().map(String::new)).unify())
                    .and(warp::header::headers_cloned())
                    // Fast path: reject when the client advertises an oversized
                    // body, before we admit it or read a byte. Optional header so
                    // GETs (and chunked bodies) without Content-Length are still
                    // admitted and then bounded by the streaming check below.
                    .and(
                        warp::header::optional::<u64>("content-length").and_then(
                            move |len: Option<u64>| async move {
                                if let Some(len) = len
                                    && len > max_body_size_u64
                                {
                                    return Err(warp::reject::custom(PayloadTooLarge));
                                }
                                Ok::<(), warp::Rejection>(())
                            },
                        ),
                    )
                    // Admission control: reserve a global in-flight slot *before*
                    // the body is read. At the ceiling, shed with 503 (via the
                    // `Overloaded` rejection) without reading a body.
                    .and({
                        let admit_budget = Arc::clone(&admit_budget);
                        warp::any().and_then(move || {
                            let admit_budget = Arc::clone(&admit_budget);
                            async move {
                                admit_budget
                                    .try_acquire_request()
                                    .ok_or_else(|| warp::reject::custom(Overloaded))
                            }
                        })
                    })
                    .and(warp::body::stream())
                    .and(request_remote_addr())
                    .and_then(
                        move |method: warp::http::Method,
                              path: warp::path::FullPath,
                              query: String,
                              headers: warp::http::HeaderMap,
                              (),
                              guard: crate::exec::budget::RequestGuard,
                              body_stream,
                              remote_addr: Option<std::net::SocketAddr>| {
                            let sender = request_sender_clone.clone();
                            async move {
                                // Hold the admission slot for this request's WHOLE
                                // transport lifetime by binding the guard into this
                                // future: it drops when the future completes — on a
                                // delivered response, a response/body timeout, or a
                                // client disconnect (warp cancels the future) —
                                // releasing the in-flight slot INDEPENDENTLY of any
                                // later admitted request. This future stays alive
                                // awaiting the response (below) even after the
                                // interpreter dequeues the request, so the slot is
                                // still held during handling — it is not released at
                                // dequeue. Parking the guard in the interpreter's
                                // pending map instead pinned it until a *future*
                                // dequeued request pruned it, which could never
                                // happen once the cap was full — permanently wedging
                                // admission. The bounded request mpsc separately
                                // caps still-queued bodies, so releasing here does
                                // not let queued-body memory grow unbounded.
                                let _admission_guard = guard;

                                // One deadline for the whole accepted-request
                                // lifetime (body read + handler response), set at
                                // admission. Applying it to the *body read* is
                                // what stops a slow "trickle" upload (a chunked
                                // body dribbled under the size cap forever) from
                                // pinning its global in-flight slot: without this,
                                // only the response wait was bounded.
                                let overall_deadline = request_timeout
                                    .map(|dur| tokio::time::Instant::now() + dur);

                                // Enforce the body limit while streaming so a
                                // chunked body (no Content-Length) is bounded too,
                                // and bound the read by the shared deadline.
                                let read_fut = read_body_capped(body_stream, max_body_size);
                                let body_read = match overall_deadline {
                                    Some(dl) => match tokio::time::timeout_at(dl, read_fut).await {
                                        Ok(r) => r,
                                        Err(_) => {
                                            log::warn!(
                                                "web server request from {} did not finish its body in time; shedding 408",
                                                remote_addr
                                                    .map(|a| a.ip().to_string())
                                                    .unwrap_or_else(|| "unknown".to_string()),
                                            );
                                            return Ok(request_timeout_response()
                                                .map(warp::hyper::Body::from));
                                        }
                                    },
                                    None => read_fut.await,
                                };
                                let body_bytes = match body_read {
                                    Ok(bytes) => bytes,
                                    Err(BodyReadError::TooLarge) => {
                                        return Ok(
                                            payload_too_large_response().map(warp::hyper::Body::from)
                                        );
                                    }
                                    Err(BodyReadError::Io) => {
                                        return Err(warp::reject::custom(ServerError(
                                            "Failed to read request body".to_string(),
                                        )));
                                    }
                                };

                                // Generate unique request ID
                                let request_id = uuid::Uuid::new_v4().to_string();

                                // Extract client IP
                                let client_ip = remote_addr
                                    .map(|addr| addr.ip().to_string())
                                    .unwrap_or_else(|| "unknown".to_string());

                                // Convert headers to HashMap
                                let mut header_map = HashMap::new();
                                for (name, value) in headers.iter() {
                                    if let Ok(value_str) = value.to_str() {
                                        header_map.insert(name.to_string(), value_str.to_string());
                                    }
                                }

                                // Create response channel
                                let (response_sender, response_receiver) =
                                    oneshot::channel::<HandlerReply>();

                                // Create the WFL request. The admission guard is
                                // NOT moved in — it stays bound to this transport
                                // future (see `_admission_guard` above), which
                                // outlives the enqueue and awaits the response, so
                                // the slot is held through handling and released
                                // when this future ends (respond/timeout/disconnect).
                                let wfl_request = WflHttpRequest {
                                    id: request_id,
                                    method: method.to_string(),
                                    path: path.as_str().to_string(),
                                    query,
                                    client_ip,
                                    body: body_bytes,
                                    headers: header_map,
                                    response_sender: Arc::new(tokio::sync::Mutex::new(Some(
                                        response_sender,
                                    ))),
                                };

                                // Send request to WFL interpreter. The queue is
                                // bounded: a full queue means the interpreter is
                                // saturated, so shed with 503 rather than
                                // buffering unbounded work. `try_send` never
                                // blocks the transport task.
                                match sender.try_send(wfl_request) {
                                    Ok(()) => {}
                                    Err(mpsc::error::TrySendError::Full(shed)) => {
                                        log::warn!(
                                            "web server request queue full (capacity {}); shedding {} {} from {} with 503",
                                            sender.max_capacity(),
                                            shed.method,
                                            shed.path,
                                            shed.client_ip
                                        );
                                        return Ok(
                                            overloaded_response().map(warp::hyper::Body::from)
                                        );
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        return Err(warp::reject::custom(ServerError(
                                            "Request channel closed".to_string(),
                                        )));
                                    }
                                }

                                // Wait for the handler's response, bounded by the
                                // *same* deadline as the body read so a handler
                                // that never answers frees its in-flight slot with
                                // a 504. Dropping `response_receiver` here closes
                                // its oneshot sender, which the interpreter
                                // observes (`is_closed`) to skip/prune the
                                // abandoned request rather than run zombie work.
                                let received = match overall_deadline {
                                    Some(dl) => {
                                        match tokio::time::timeout_at(dl, response_receiver).await {
                                            Ok(r) => r,
                                            Err(_) => {
                                                log::warn!(
                                                    "web server request from {} timed out awaiting handler; shedding 504",
                                                    remote_addr
                                                        .map(|a| a.ip().to_string())
                                                        .unwrap_or_else(|| "unknown".to_string()),
                                                );
                                                return Ok(gateway_timeout_response()
                                                    .map(warp::hyper::Body::from));
                                            }
                                        }
                                    }
                                    None => response_receiver.await,
                                };

                                match received {
                                    Ok(HandlerReply::Buffered(response)) => {
                                        let status_code =
                                            warp::http::StatusCode::from_u16(response.status)
                                                .unwrap_or(warp::http::StatusCode::OK);

                                        // Content is already raw bytes (text responses
                                        // stored their UTF-8 encoding, binary responses
                                        // their verbatim bytes), so Content-Length is the
                                        // exact byte count of the body.
                                        let content_bytes = response.content;
                                        let content_length = content_bytes.len();

                                        let mut reply_builder = warp::http::Response::builder()
                                            .status(status_code)
                                            .header("Content-Type", response.content_type)
                                            .header("Content-Length", content_length);

                                        // Add additional headers
                                        for (name, value) in response.headers {
                                            reply_builder = reply_builder.header(name, value);
                                        }

                                        match reply_builder.body(warp::hyper::Body::from(content_bytes)) {
                                            Ok(response) => Ok(response),
                                            Err(_) => Err(warp::reject::custom(ServerError(
                                                "Failed to build response".to_string(),
                                            ))),
                                        }
                                    }
                                    Ok(HandlerReply::Streaming {
                                        status,
                                        content_type,
                                        headers,
                                        body,
                                    }) => {
                                        let status_code = warp::http::StatusCode::from_u16(status)
                                            .unwrap_or(warp::http::StatusCode::OK);

                                        // No Content-Length: the body length is unknown
                                        // up front. hyper frames it with chunked
                                        // transfer-encoding and writes each chunk as it
                                        // arrives off the bounded channel.
                                        let mut reply_builder = warp::http::Response::builder()
                                            .status(status_code)
                                            .header("Content-Type", content_type);
                                        for (name, value) in headers {
                                            reply_builder = reply_builder.header(name, value);
                                        }

                                        // `body` is a `NotifyingChunkStream`: when the
                                        // client disconnects, hyper drops it (firing
                                        // the finish oneshot and making the handler's
                                        // next `write` fail); when the sender is
                                        // closed cleanly, the stream ends and Drop
                                        // still fires so `close out` can await
                                        // delivery of the terminating chunk (#680).
                                        match reply_builder
                                            .body(warp::hyper::Body::wrap_stream(body))
                                        {
                                            Ok(response) => Ok(response),
                                            Err(_) => Err(warp::reject::custom(ServerError(
                                                "Failed to build streaming response".to_string(),
                                            ))),
                                        }
                                    }
                                    Err(_) => Err(warp::reject::custom(ServerError(
                                        "Response channel closed".to_string(),
                                    ))),
                                }
                            }
                        },
                    )
                    .recover(handle_overloaded);

                // Parse the bind address from config
                let bind_addr: IpAddr = match self.config.web_server_bind_address.parse() {
                    Ok(addr) => addr,
                    Err(_) => {
                        return Err(RuntimeError::new(
                            format!(
                                "Invalid web_server_bind_address in config: '{}'. Expected a valid IP address (e.g., '127.0.0.1' or '0.0.0.0')",
                                self.config.web_server_bind_address
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                if let Some(target_port_expr) = redirect_to_port {
                    // Redirect server: answers every request natively with a
                    // 301 to the HTTPS port. Requests never reach the WFL
                    // request loop, so `wait for request` on this server
                    // never fires.
                    let target_val = self
                        .evaluate_expression(target_port_expr, Rc::clone(&env))
                        .await?;
                    // Reject non-integer and out-of-range values instead of
                    // letting the float->u16 cast saturate to a wrong port
                    let target_port = match &target_val {
                        Value::Number(n) if n.fract() == 0.0 && *n >= 1.0 && *n <= 65535.0 => {
                            *n as u16
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Expected a whole number between 1 and 65535 for redirect target port, got {target_val:?}"
                                ),
                                *line,
                                *column,
                            ));
                        }
                    };

                    let fallback_host = match bind_addr {
                        IpAddr::V6(v6) => format!("[{v6}]"),
                        IpAddr::V4(v4) => v4.to_string(),
                    };
                    let redirect_routes = warp::any()
                        .and(warp::header::optional::<String>("host"))
                        .and(warp::path::full())
                        .and(warp::query::raw().or(warp::any().map(String::new)).unify())
                        .map(
                            move |host: Option<String>,
                                  path: warp::path::FullPath,
                                  query: String| {
                                let host_value = host.unwrap_or_else(|| fallback_host.clone());
                                let mut location =
                                    format!("https://{}", strip_host_port(&host_value));
                                if target_port != 443 {
                                    location.push_str(&format!(":{target_port}"));
                                }
                                location.push_str(path.as_str());
                                if !query.is_empty() {
                                    location.push('?');
                                    location.push_str(&query);
                                }
                                warp::http::Response::builder()
                                    .status(warp::http::StatusCode::MOVED_PERMANENTLY)
                                    .header("Location", location)
                                    .header("Content-Length", 0)
                                    .body(Vec::new())
                                    .unwrap_or_else(|_| {
                                        let mut resp = warp::http::Response::new(Vec::new());
                                        *resp.status_mut() =
                                            warp::http::StatusCode::INTERNAL_SERVER_ERROR;
                                        resp
                                    })
                            },
                        );

                    match warp::serve(redirect_routes).try_bind_ephemeral((bind_addr, port_num)) {
                        Ok((addr, server)) => {
                            let server_handle = tokio::spawn(server);

                            // Registered like any other server so `close server`
                            // works; its request channels are never fed.
                            let wfl_server = WflWebServer {
                                request_receiver: request_receiver.clone(),
                                request_sender: request_sender.clone(),
                                server_handle: Some(server_handle),
                                sessions: session_manager.clone(),
                            };
                            self.web_servers
                                .borrow_mut()
                                .insert(server_name.clone(), wfl_server);

                            let server_value = Value::Text(Arc::from(format!(
                                "WebServer::{}:{}",
                                addr.ip(),
                                addr.port()
                            )));

                            println!(
                                "Redirect server is listening on port {} (redirecting to HTTPS port {})",
                                addr.port(),
                                target_port
                            );

                            match env.borrow_mut().define_direct(server_name, server_value) {
                                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                            }
                        }
                        Err(e) => Err(RuntimeError::new(
                            format!("Failed to start web server on port {}: {}", port_num, e),
                            *line,
                            *column,
                        )),
                    }
                } else if let Some(tls_config) = tls {
                    // HTTPS server. Certificate/key paths come from the listen
                    // statement itself, falling back to .wflcfg for the bare
                    // `secured` form.
                    let cert_path = match &tls_config.cert_path {
                        Some(expr) => {
                            let v = self.evaluate_expression(expr, Rc::clone(&env)).await?;
                            match &v {
                                Value::Text(t) => t.to_string(),
                                _ => {
                                    return Err(RuntimeError::new(
                                        format!(
                                            "Expected text for TLS certificate path, got {v:?}"
                                        ),
                                        *line,
                                        *column,
                                    ));
                                }
                            }
                        }
                        None => match &self.config.web_server_tls_cert_file {
                            Some(path) => path.clone(),
                            None => {
                                return Err(RuntimeError::new(
                                    "This listen statement is marked 'secured' but no certificate is configured. Either write 'secured with certificate \"cert.pem\" and key \"key.pem\"' or set web_server_tls_cert_file and web_server_tls_key_file in .wflcfg".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        },
                    };
                    let key_path = match &tls_config.key_path {
                        Some(expr) => {
                            let v = self.evaluate_expression(expr, Rc::clone(&env)).await?;
                            match &v {
                                Value::Text(t) => t.to_string(),
                                _ => {
                                    return Err(RuntimeError::new(
                                        format!(
                                            "Expected text for TLS private key path, got {v:?}"
                                        ),
                                        *line,
                                        *column,
                                    ));
                                }
                            }
                        }
                        None => match &self.config.web_server_tls_key_file {
                            Some(path) => path.clone(),
                            None => {
                                return Err(RuntimeError::new(
                                    "This listen statement is marked 'secured' but no private key is configured. Either write 'secured with certificate \"cert.pem\" and key \"key.pem\"' or set web_server_tls_cert_file and web_server_tls_key_file in .wflcfg".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        },
                    };

                    let tls_config = match tls::load_server_config(&cert_path, &key_path) {
                        Ok(config) => config,
                        Err(message) => {
                            return Err(RuntimeError::new(message, *line, *column));
                        }
                    };

                    // Keep Warp's routing/filter contract while using the
                    // patched Rustls stack. Hyper owns the listener and
                    // connection lifecycle just as it did under Warp's legacy
                    // TLS adapter; each accepted stream drives its handshake
                    // independently, so a silent client cannot block accepts.
                    match tls::SecuredIncoming::bind(
                        std::net::SocketAddr::new(bind_addr, port_num),
                        tls_config,
                    ) {
                        Ok((addr, incoming)) => {
                            let filter_service = warp::service(routes);
                            let make_service = warp::hyper::service::make_service_fn(
                                move |connection: &tls::SecuredStream| {
                                    let filter_service = filter_service.clone();
                                    let remote_addr = connection.remote_addr();
                                    async move {
                                        Ok::<_, std::convert::Infallible>(
                                            warp::hyper::service::service_fn(
                                                move |mut request: warp::hyper::Request<
                                                    warp::hyper::Body,
                                                >| {
                                                    request.extensions_mut().insert(remote_addr);
                                                    let mut service = filter_service.clone();
                                                    warp::hyper::service::Service::call(
                                                        &mut service,
                                                        request,
                                                    )
                                                },
                                            ),
                                        )
                                    }
                                },
                            );
                            let tracked_connections = tls::TrackedConnections::new();
                            let server = warp::hyper::Server::builder(incoming)
                                .executor(tracked_connections.executor())
                                .serve(make_service)
                                .with_graceful_shutdown(std::future::pending::<()>());
                            let server_handle = tokio::spawn(async move {
                                let _cancel_connections = tracked_connections.cancel_on_drop();
                                if let Err(error) = server.await {
                                    log::error!("Secure web server stopped: {error}");
                                }
                            });

                            let wfl_server = WflWebServer {
                                request_receiver: request_receiver.clone(),
                                request_sender: request_sender.clone(),
                                server_handle: Some(server_handle),
                                sessions: session_manager.clone(),
                            };
                            self.web_servers
                                .borrow_mut()
                                .insert(server_name.clone(), wfl_server);

                            let server_value = Value::Text(Arc::from(format!(
                                "WebServer::{}:{}",
                                addr.ip(),
                                addr.port()
                            )));

                            println!("Secure server is listening on port {}", addr.port());

                            match env.borrow_mut().define_direct(server_name, server_value) {
                                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                            }
                        }
                        Err(e) => Err(RuntimeError::new(
                            format!(
                                "Failed to start secure web server on port {}: {}",
                                port_num, e
                            ),
                            *line,
                            *column,
                        )),
                    }
                } else {
                    // Plain HTTP server (unchanged behavior)
                    let server_task = warp::serve(routes).try_bind_ephemeral((bind_addr, port_num));

                    match server_task {
                        Ok((addr, server)) => {
                            // Spawn the server in the background
                            let server_handle = tokio::spawn(server);

                            // Create WFL web server object
                            let wfl_server = WflWebServer {
                                request_receiver: request_receiver.clone(),
                                request_sender: request_sender.clone(),
                                server_handle: Some(server_handle),
                                sessions: session_manager.clone(),
                            };

                            // Store the server in the interpreter
                            self.web_servers
                                .borrow_mut()
                                .insert(server_name.clone(), wfl_server);

                            // Create a server value with the actual address
                            let server_value = Value::Text(Arc::from(format!(
                                "WebServer::{}:{}",
                                addr.ip(),
                                addr.port()
                            )));

                            println!("Server is listening on port {}", addr.port());

                            match env.borrow_mut().define_direct(server_name, server_value) {
                                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                            }
                        }
                        Err(e) => Err(RuntimeError::new(
                            format!("Failed to start web server on port {}: {}", port_num, e),
                            *line,
                            *column,
                        )),
                    }
                }
            }
            Statement::WaitForRequestStatement {
                server,
                request_name,
                timeout,
                line,
                column,
            } => {
                // Look up the server by name
                let server_name = match self.evaluate_expression(server, Rc::clone(&env)).await? {
                    Value::Text(name) => {
                        // Extract server name from "WebServer::host:port" format
                        let name_str = name.as_ref();
                        if name_str.starts_with("WebServer::") {
                            // Find the server by matching the exact server value
                            let web_servers = self.web_servers.borrow();

                            // Search through all servers to find which one has this exact value
                            let mut found_server = None;
                            for server_name in web_servers.keys() {
                                // Get the stored value for this server name
                                if let Some(Value::Text(stored_text)) =
                                    env.borrow().get(server_name)
                                    && stored_text.as_ref() == name_str
                                {
                                    // Found the matching server
                                    found_server = Some(server_name.clone());
                                    break;
                                }
                            }

                            // Return the found server or use first server as fallback
                            if let Some(server_name) = found_server {
                                server_name
                            } else if let Some((found_name, _)) = web_servers.iter().next() {
                                found_name.clone()
                            } else {
                                return Err(RuntimeError::new(
                                    "No web servers found".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        } else {
                            name_str.to_string()
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected server name as text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Get the server's request receiver
                let request_receiver = {
                    let web_servers = self.web_servers.borrow();
                    if let Some(server) = web_servers.get(&server_name) {
                        server.request_receiver.clone()
                    } else {
                        return Err(RuntimeError::new(
                            format!("Web server '{}' not found", server_name),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate the timeout expression BEFORE locking the shared
                // receiver. The timeout clause is an arbitrary WFL expression
                // and evaluating it can await (a user action that sleeps, does
                // I/O, ...). The receiver mutex is shared by every concurrent
                // handler, so evaluating under it would let one handler's slow
                // timeout expression stall the whole server (#642).
                let timeout_duration = if let Some(timeout_expr) = timeout {
                    let timeout_val = self
                        .evaluate_expression(timeout_expr, Rc::clone(&env))
                        .await?;
                    match timeout_val {
                        // Reject fractional values that would truncate to 0 ms
                        // (e.g. 0.5) and hot-spin forever with zero-duration
                        // timeouts. Require at least 1 millisecond.
                        Value::Number(ms) if ms >= 1.0 => {
                            Some(std::time::Duration::from_millis(ms as u64))
                        }
                        Value::Number(ms) if ms > 0.0 => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Timeout must be at least 1 millisecond (got {ms} ms); \
                                     fractional values below 1 would truncate to zero and spin"
                                ),
                                *line,
                                *column,
                            ));
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                "Timeout must be a positive number (milliseconds)".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    None
                };

                // Wait for a request to come in (with optional timeout)
                let request = {
                    let mut receiver = request_receiver.lock().await;

                    // Wait for request with or without timeout. Loop so a request
                    // whose client already gave up (its oneshot receiver dropped
                    // on 408/504/disconnect, closing the sender) is skipped rather
                    // than handled — otherwise the interpreter would run a handler
                    // for a dead request and register a dead pending-response
                    // entry, letting repeated timeouts accumulate zombie work.
                    loop {
                        let req = if let Some(duration) = timeout_duration {
                            match tokio::time::timeout(duration, receiver.recv()).await {
                                Ok(Some(req)) => req,
                                Ok(None) => {
                                    return Err(RuntimeError::new(
                                        "Request channel closed".to_string(),
                                        *line,
                                        *column,
                                    ));
                                }
                                Err(_) => {
                                    // Finite wait expiry is an expected idle-server
                                    // outcome, not a structural fault. Classify as
                                    // `Timeout` so the concurrent loop's consecutive-
                                    // failure breaker does not tear a healthy server
                                    // down after enough empty poll intervals.
                                    return Err(RuntimeError::with_kind(
                                        format!(
                                            "{REQUEST_WAIT_TIMEOUT_PREFIX} ({} ms)",
                                            duration.as_millis()
                                        ),
                                        *line,
                                        *column,
                                        ErrorKind::Timeout,
                                    ));
                                }
                            }
                        } else {
                            // No timeout - wait indefinitely
                            match receiver.recv().await {
                                Some(req) => req,
                                None => {
                                    return Err(RuntimeError::new(
                                        "Request channel closed".to_string(),
                                        *line,
                                        *column,
                                    ));
                                }
                            }
                        };

                        let abandoned = {
                            let sender_opt = req.response_sender.lock().await;
                            sender_opt.as_ref().is_none_or(|s| s.is_closed())
                        };
                        if abandoned {
                            log::debug!(
                                "skipping abandoned request {} ({} {}) from {}",
                                req.id,
                                req.method,
                                req.path,
                                req.client_ip
                            );
                            continue;
                        }
                        break req;
                    }
                };

                // Define individual variables for request properties (more natural for WFL)
                let mut env_mut = env.borrow_mut();

                // Convert headers to a WFL object (shared by the request object and
                // the standalone headers variable defined below)
                let mut headers_map = HashMap::new();
                for (key, value) in request.headers.iter() {
                    headers_map.insert(key.clone(), Value::Text(Arc::from(value.clone())));
                }
                let headers_object = Value::Object(Rc::new(RefCell::new(headers_map)));

                // Define the main request variable (for use in respond statements and
                // as request context for `execute file ... with <request>`)
                let mut request_properties = HashMap::new();
                request_properties.insert(
                    "_response_sender".to_string(),
                    Value::Text(Arc::from(request.id.clone())),
                );
                request_properties.insert(
                    "method".to_string(),
                    Value::Text(Arc::from(request.method.clone())),
                );
                request_properties.insert(
                    "path".to_string(),
                    Value::Text(Arc::from(request.path.clone())),
                );
                request_properties.insert(
                    "query".to_string(),
                    Value::Text(Arc::from(request.query.clone())),
                );
                request_properties.insert(
                    "client_ip".to_string(),
                    Value::Text(Arc::from(request.client_ip.clone())),
                );
                // `body` is a lossy-UTF-8 text view (backward compatible);
                // `body_bytes` is the lossless binary view for binary uploads.
                let body_text = String::from_utf8_lossy(&request.body).into_owned();
                let body_binary = Value::Binary(Arc::from(request.body.as_slice()));
                request_properties.insert(
                    "body".to_string(),
                    Value::Text(Arc::from(body_text.as_str())),
                );
                request_properties.insert("body_bytes".to_string(), body_binary.clone());
                request_properties.insert("headers".to_string(), headers_object.clone());
                request_properties.insert(
                    "_session_server".to_string(),
                    Value::Text(Arc::from(server_name.as_str())),
                );
                let request_object = Value::Object(Rc::new(RefCell::new(request_properties)));

                // These bindings are refreshed on every wait, so overwrite any
                // previous request's values instead of failing on redefinition.
                env_mut.define_or_replace(request_name, request_object);

                // Define individual request property variables
                env_mut.define_or_replace("method", Value::Text(Arc::from(request.method.clone())));

                env_mut.define_or_replace("path", Value::Text(Arc::from(request.path.clone())));

                env_mut.define_or_replace("query", Value::Text(Arc::from(request.query.clone())));

                env_mut.define_or_replace(
                    "client_ip",
                    Value::Text(Arc::from(request.client_ip.clone())),
                );

                env_mut.define_or_replace("body", Value::Text(Arc::from(body_text.as_str())));
                env_mut.define_or_replace("body_bytes", body_binary);

                env_mut.define_or_replace("headers", headers_object);

                drop(env_mut); // Release the borrow

                // Store the request in a global map for RespondStatement to access.
                // Done only after every define above succeeded: registering earlier
                // would park the oneshot sender on an error path and leave the HTTP
                // client hanging instead of failing fast.
                {
                    let mut pending_responses = self.pending_responses.borrow_mut();
                    // Prune entries whose client already disconnected/timed out
                    // (oneshot sender closed) before inserting the new one, so a
                    // handler that never `respond`s to a since-abandoned request
                    // cannot let the map grow without bound across many timeouts.
                    // (The admission slot itself is released by the transport task,
                    // not this prune — see `PendingResponse`.)
                    pending_responses.retain(|_, pending| match pending.sender.try_lock() {
                        Ok(guard) => guard.as_ref().is_some_and(|s| !s.is_closed()),
                        // Locked right now (being responded to) — keep it.
                        Err(_) => true,
                    });
                    pending_responses.insert(
                        request.id.clone(),
                        PendingResponse {
                            sender: request.response_sender,
                        },
                    );
                }
                // Track it against the current handler so it is answered 500 if
                // the handler ends without responding (rather than the client
                // waiting out the request timeout).
                self.open_pending_requests
                    .borrow_mut()
                    .push(request.id.clone());
                // Sticky: this handler accepted work. Request-local failures from
                // here on must not trip the concurrent structural-failure breaker.
                self.accepted_request.set(true);

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::RespondStatement {
                request,
                content,
                status,
                content_type,
                headers,
                set_session,
                clear_session,
                line,
                column,
            } => {
                // The request operand can call actions or wait asynchronously.
                // Establish the response-attempt baseline before evaluating it
                // so a client disconnect cancels that work too.
                let (request_val, response_snapshot) = self
                    .evaluate_response_request(
                        *line,
                        *column,
                        "Client disconnected before the response request was resolved",
                        self.evaluate_expression(request, Rc::clone(&env)),
                    )
                    .await?;
                let request_id = match &request_val {
                    Value::Object(obj) => {
                        let obj_ref = obj.borrow();
                        match obj_ref.get("_response_sender") {
                            Some(Value::Text(id)) => id.as_ref().to_string(),
                            _ => {
                                return Err(RuntimeError::new(
                                    "Request object missing response sender ID".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected request object".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                let request_for_cookie = request_val.clone();

                // Keep the pending request parked until content/status/header
                // expressions are evaluated so a browser disconnect still cancels
                // any upstream open/read performed during that evaluation
                // (`any_pending_request_disconnected` watches the parked sender).
                // Only after evaluation do we take the sender into the completion
                // guard. Early eval errors leave the id in open_pending so the
                // handler-exit 500 path still resolves the client.
                let (response, response_snapshot) = self
                    .evaluate_response_precommit(
                        &request_id,
                        *line,
                        *column,
                        "Client disconnected before the response was sent",
                        response_snapshot,
                        async {
                            // Evaluate response content. Binary values are carried through
                            // as raw bytes so fonts/images/etc. serve losslessly; text and
                            // scalar values keep their existing UTF-8 rendering.
                            let content_val =
                                self.evaluate_expression(content, Rc::clone(&env)).await?;
                let is_binary = matches!(content_val, Value::Binary(_));

                // Enforce the response-body ceiling on the *borrowed* length
                // first, so an oversized Text/Binary body is refused before it is
                // duplicated into `content_bytes` (bounding peak allocation).
                if let Value::Text(text) = &content_val
                    && let Err(exceeded) = self.budget.check_response_bytes(text.len())
                {
                    return Err(self.budget_error(exceeded, *line, *column));
                }
                if let Value::Binary(bytes) = &content_val
                    && let Err(exceeded) = self.budget.check_response_bytes(bytes.len())
                {
                    return Err(self.budget_error(exceeded, *line, *column));
                }

                let content_bytes: Vec<u8> = match &content_val {
                    Value::Text(text) => text.as_bytes().to_vec(),
                    Value::Number(n) => n.to_string().into_bytes(),
                    Value::Bool(b) => b.to_string().into_bytes(),
                    Value::Binary(bytes) => bytes.to_vec(),
                    Value::Null => Vec::new(),
                    // Composite/opaque values (lists, objects, functions, …) have
                    // no meaningful HTTP body rendering, and their `{:?}` form is
                    // unbounded — materializing it would allocate past the
                    // response cap before it could be checked. Reject them with a
                    // clear error instead.
                    other => {
                        return Err(RuntimeError::new(
                            format!(
                                "Cannot use {} as a response body; respond with text, a number, a boolean, binary data, or nothing",
                                other.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                // Re-check the materialized length to cover the small formatted
                // variants (Number/Bool), which have no cheap borrowed length.
                if let Err(exceeded) = self.budget.check_response_bytes(content_bytes.len()) {
                    return Err(self.budget_error(exceeded, *line, *column));
                }

                // Evaluate status code (optional)
                let status_code = if let Some(status_expr) = status {
                    let status_val = self
                        .evaluate_expression(status_expr, Rc::clone(&env))
                        .await?;
                    match &status_val {
                        Value::Number(n) => *n as u16,
                        _ => {
                            return Err(RuntimeError::new(
                                "Status code must be a number".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    200 // Default to 200 OK
                };

                // Evaluate content type (optional)
                let content_type_str = if let Some(ct_expr) = content_type {
                    let ct_val = self.evaluate_expression(ct_expr, Rc::clone(&env)).await?;
                    match &ct_val {
                        Value::Text(text) => text.as_ref().to_string(),
                        _ => {
                            return Err(RuntimeError::new(
                                "Content type must be text".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else if is_binary {
                    // Binary responses default to a generic binary media type
                    // rather than text/plain so browsers don't misinterpret them.
                    "application/octet-stream".to_string()
                } else {
                    "text/plain".to_string() // Default content type
                };

                // Evaluate custom response headers (optional). Mirrors the
                // outbound client's headers map: a WFL Object of name -> value.
                // Enables RFC 10008 (HTTP QUERY) servers to advertise
                // `Accept-Query` and point at results with `Content-Location`
                // or `Location`.
                let mut custom_headers: HashMap<String, String> = HashMap::new();
                if let Some(headers_expr) = headers {
                    let headers_val = self
                        .evaluate_expression(headers_expr, Rc::clone(&env))
                        .await?;
                    match &headers_val {
                        Value::Object(obj) => {
                            for (name, value) in obj.borrow().iter() {
                                let value_str = match value {
                                    Value::Text(s) => s.to_string(),
                                    Value::Number(_) | Value::Bool(_) => value.to_string(),
                                    _ => {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Response header '{name}' must be text, a number, or a boolean, got {}",
                                                value.type_name()
                                            ),
                                            *line,
                                            *column,
                                        ));
                                    }
                                };
                                // Content-Type, Content-Length, and
                                // Transfer-Encoding are computed by the response
                                // pipeline (the `content_type` clause and warp's
                                // builder set them explicitly). Warp *appends*
                                // custom headers, so letting the map override
                                // these would emit duplicate/conflicting headers
                                // (RFC 7230 §3.3.2) and risk response splitting.
                                // Drop them so the pipeline stays authoritative.
                                if name.eq_ignore_ascii_case("content-type")
                                    || name.eq_ignore_ascii_case("content-length")
                                    || name.eq_ignore_ascii_case("transfer-encoding")
                                {
                                    continue;
                                }
                                custom_headers.insert(name.clone(), value_str);
                            }
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Expected a map for response headers, got {}",
                                    headers_val.type_name()
                                ),
                                *line,
                                *column,
                            ));
                        }
                    }
                }

                if let Some(session_expr) = set_session {
                    let session_val = self
                        .evaluate_expression(session_expr, Rc::clone(&env))
                        .await?;
                    let cookie = self
                        .session_set_cookie(&session_val, false, *line, *column)
                        .await?;
                    custom_headers
                        .entry("Set-Cookie".to_string())
                        .or_insert(cookie);
                } else if *clear_session {
                    let cookie = self
                        .session_set_cookie(&request_for_cookie, true, *line, *column)
                        .await?;
                    custom_headers
                        .entry("Set-Cookie".to_string())
                        .or_insert(cookie);
                }

                            Ok(WflHttpResponse {
                                content: content_bytes,
                                status: status_code,
                                content_type: content_type_str,
                                headers: custom_headers,
                            })
                        },
                    )
                    .await?;

                // Now commit: take the sender (disconnect signal ends) and deliver.
                let mut completion = match self
                    .take_pending_response_completion(&request_id, *line, *column)
                    .await
                {
                    Ok(completion) => completion,
                    Err(error) => {
                        if error.kind == ErrorKind::Cancelled {
                            self.cancel_response_precommit(&request_id, response_snapshot);
                        }
                        return Err(error);
                    }
                };
                match completion.take_sender() {
                    Some(sender) => {
                        if sender.send(HandlerReply::Buffered(response)).is_err() {
                            // Receiver dropped => the client hung up before the
                            // buffered reply landed. That is a cooperative
                            // cancellation, not a handler fault — mark it `Cancelled`
                            // so the concurrent loop's structural-failure breaker
                            // skips it (a burst of post-dequeue disconnects must not
                            // tear the server down).
                            self.cancel_response_precommit(&request_id, response_snapshot);
                            return Err(RuntimeError::with_kind(
                                "Client disconnected before the response was sent".to_string(),
                                *line,
                                *column,
                                ErrorKind::Cancelled,
                            ));
                        }
                    }
                    None => {
                        return Err(RuntimeError::new(
                            "Response already sent for this request".to_string(),
                            *line,
                            *column,
                        ));
                    }
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::StartStreamingResponseStatement {
                request,
                status,
                content_type,
                headers,
                variable_name,
                line,
                column,
            } => {
                // Resolve the request id under the same cancellation baseline as
                // the streaming-head fields and final transport commit.
                let (request_val, response_snapshot) = self
                    .evaluate_response_request(
                        *line,
                        *column,
                        "Client disconnected before the streaming response request was resolved",
                        self.evaluate_expression(request, Rc::clone(&env)),
                    )
                    .await?;
                let request_id = match &request_val {
                    Value::Object(obj) => match obj.borrow().get("_response_sender") {
                        Some(Value::Text(id)) => id.as_ref().to_string(),
                        _ => {
                            return Err(RuntimeError::new(
                                "Request object missing response sender ID".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    },
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected request object".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Keep pending parked through status/content-type/header
                // evaluation so disconnect still cancels any upstream work those
                // expressions perform. Handler-exit 500 covers early eval errors.
                let ((status_code, content_type_str, custom_headers), response_snapshot) = self
                    .evaluate_response_precommit(
                        &request_id,
                        *line,
                        *column,
                        "Client disconnected before the streaming response started",
                        response_snapshot,
                        async {
                            let status_code = match status {
                    Some(expr) => {
                        let v = self.evaluate_expression(expr, Rc::clone(&env)).await?;
                        match &v {
                            // Require a whole number in the HTTP status range
                            // rather than silently wrapping a fractional or
                            // out-of-range value through `as u16`.
                            Value::Number(n) if n.fract() == 0.0 && *n >= 100.0 && *n <= 599.0 => {
                                *n as u16
                            }
                            Value::Number(n) => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected a whole HTTP status code between 100 and 599, got {n}"
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                            _ => {
                                return Err(RuntimeError::new(
                                    format!("Expected number for status, got {}", v.type_name()),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    None => 200,
                };

                let content_type_str = match content_type {
                    Some(expr) => {
                        let v = self.evaluate_expression(expr, Rc::clone(&env)).await?;
                        match &v {
                            Value::Text(s) => s.to_string(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected text for content type, got {}",
                                        v.type_name()
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    None => "application/octet-stream".to_string(),
                };

                let mut custom_headers: HashMap<String, String> = HashMap::new();
                if let Some(expr) = headers {
                    let v = self.evaluate_expression(expr, Rc::clone(&env)).await?;
                    match &v {
                        Value::Object(obj) => {
                            for (name, value) in obj.borrow().iter() {
                                let value_str = match value {
                                    Value::Text(s) => s.to_string(),
                                    Value::Number(_) | Value::Bool(_) => value.to_string(),
                                    _ => {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Header '{name}' must be text, got {}",
                                                value.type_name()
                                            ),
                                            *line,
                                            *column,
                                        ));
                                    }
                                };
                                // The pipeline owns framing headers.
                                if name.eq_ignore_ascii_case("content-type")
                                    || name.eq_ignore_ascii_case("content-length")
                                    || name.eq_ignore_ascii_case("transfer-encoding")
                                {
                                    continue;
                                }
                                custom_headers.insert(name.clone(), value_str);
                            }
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Expected a map for response headers, got {}",
                                    v.type_name()
                                ),
                                *line,
                                *column,
                            ));
                        }
                    }
                }

                            Ok((status_code, content_type_str, custom_headers))
                        },
                    )
                    .await?;

                // Commit: take the sender and hand the streaming head to the transport.
                // The oneshot pairs `close out` with the transport finishing the
                // body so the program does not return mid-flush (#680).
                let (tx, rx) = mpsc::channel::<Vec<u8>>(RESPONSE_STREAM_BUFFER);
                let (done_tx, done_rx) = oneshot::channel::<()>();
                let body = NotifyingChunkStream {
                    rx,
                    done: Some(done_tx),
                };
                let mut completion = match self
                    .take_pending_response_completion(&request_id, *line, *column)
                    .await
                {
                    Ok(completion) => completion,
                    Err(error) => {
                        if error.kind == ErrorKind::Cancelled {
                            self.cancel_response_precommit(&request_id, response_snapshot);
                        }
                        return Err(error);
                    }
                };
                match completion.take_sender() {
                    Some(sender) => {
                        if sender
                            .send(HandlerReply::Streaming {
                                status: status_code,
                                content_type: content_type_str,
                                headers: custom_headers,
                                body,
                            })
                            .is_err()
                        {
                            // Client hung up before the streaming head committed —
                            // cooperative cancellation, not a fault (see the buffered
                            // `respond` path). `Cancelled` keeps the concurrent
                            // breaker from counting a disconnect as a failure.
                            self.cancel_response_precommit(&request_id, response_snapshot);
                            return Err(RuntimeError::with_kind(
                                "Client disconnected before the streaming response started"
                                    .to_string(),
                                *line,
                                *column,
                                ErrorKind::Cancelled,
                            ));
                        }
                    }
                    None => {
                        return Err(RuntimeError::new(
                            "Response already sent for this request".to_string(),
                            *line,
                            *column,
                        ));
                    }
                }

                let handle_id = {
                    let n = self.next_response_stream_id.get();
                    self.next_response_stream_id.set(n + 1);
                    format!("respstream{n}")
                };
                self.server_response_streams.borrow_mut().insert(
                    handle_id.clone(),
                    ServerResponseStream {
                        tx,
                        bytes_written: 0,
                        finished: Some(done_rx),
                    },
                );
                // Track it against the current handler so it is auto-closed if
                // the handler ends without an explicit `close out`.
                self.open_response_streams
                    .borrow_mut()
                    .push(handle_id.clone());

                let mut stream_map = HashMap::new();
                stream_map.insert("_server_stream".to_string(), Value::Text(handle_id.into()));
                stream_map.insert("status".to_string(), Value::Number(status_code as f64));
                let value = Value::Object(Rc::new(RefCell::new(stream_map)));
                // define_or_replace so a `main loop` handler that rebinds `out`
                // each request works — and so binding never fails after the
                // stream head was already committed (which would otherwise leak
                // the sender and hang the client).
                env.borrow_mut().define_or_replace(variable_name, value);
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::StreamWriteStatement {
                value,
                target,
                is_line,
                fallback_content,
                line,
                column,
            } => {
                // The target decides the interpretation. Evaluate it once; if it
                // is a server response stream, this is a stream write. If it is
                // not and this parsed from the ambiguous merged form
                // (`write line <ident> to <target>`), fall back to the classic
                // file write `write <fallback_content> to <target>` so a
                // pre-existing file write is never reinterpreted.
                let target_val = self.evaluate_expression(target, Rc::clone(&env)).await?;
                let handle_id = match &target_val {
                    Value::Object(obj) => match obj.borrow().get("_server_stream") {
                        Some(Value::Text(id)) => Some(id.to_string()),
                        _ => None,
                    },
                    _ => None,
                };
                let handle_id = match handle_id {
                    Some(id) => id,
                    None => {
                        if let Some(fallback) = fallback_content {
                            // Classic file write: `write <fallback> to <target>`.
                            let content_value =
                                self.evaluate_expression(fallback, Rc::clone(&env)).await?;
                            let file_str = match &target_val {
                                Value::Text(s) => s.clone(),
                                _ => {
                                    return Err(RuntimeError::new(
                                        format!(
                                            "Expected a server response stream or a file handle, got {}",
                                            target_val.type_name()
                                        ),
                                        *line,
                                        *column,
                                    ));
                                }
                            };
                            let content_str = format!("{content_value}");
                            return match self.io_client.write_file(&file_str, &content_str).await {
                                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                Err(e) => Err(RuntimeError::new(e, *line, *column)),
                            };
                        }
                        return Err(RuntimeError::new(
                            format!(
                                "Expected a server response stream (from `start streaming response as ...`), got {}",
                                target_val.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };
                if !self.owns_response_stream(&handle_id) {
                    return Err(self
                        .response_stream_ownership_error("write to", &handle_id, *line, *column));
                }
                let val = self.evaluate_expression(value, Rc::clone(&env)).await?;
                let newline = usize::from(*is_line);
                // Compute the outgoing byte length WITHOUT cloning a large
                // text/binary value, so an over-budget write is rejected *before*
                // any big allocation/copy (a huge write must not be materialized
                // just to be refused).
                let incoming_len = match &val {
                    Value::Text(s) => s.len() + newline,
                    Value::Binary(b) => b.len() + newline,
                    // Numbers/booleans render to a short string; the tiny
                    // allocation to measure them is not a DoS concern.
                    Value::Number(_) | Value::Bool(_) => val.to_string().len() + newline,
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Can only write text or binary to a response stream, got {}",
                                val.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                // Reserve the response-byte budget on the running total FIRST (so a
                // stream cannot bypass `web_server_max_response_size` via one huge
                // chunk or many chunks), then clone the sender out so the map borrow
                // is not held across the (possibly backpressured) send await.
                let max_response_bytes = self.budget.limits().max_response_bytes;
                let sender = {
                    let mut map = self.server_response_streams.borrow_mut();
                    match map.get_mut(&handle_id) {
                        Some(stream) => {
                            let new_total = stream.bytes_written.saturating_add(incoming_len);
                            if new_total > max_response_bytes {
                                let actual = new_total;
                                // Drop the stream so the body ends rather than
                                // silently truncating, and untrack it so a handler
                                // that catches this error keeps no stale id.
                                map.remove(&handle_id);
                                self.open_response_streams
                                    .borrow_mut()
                                    .retain(|s| s != &handle_id);
                                return Err(self.budget_error(
                                    BudgetExceeded::ResponseBytes {
                                        limit: max_response_bytes,
                                        actual,
                                    },
                                    *line,
                                    *column,
                                ));
                            }
                            stream.bytes_written = new_total;
                            Some(stream.tx.clone())
                        }
                        None => None,
                    }
                };
                match sender {
                    Some(tx) => {
                        // Within budget and the stream is open: materialize the
                        // bytes now (after the ceiling check) and send them.
                        let mut bytes = match &val {
                            Value::Text(s) => s.as_bytes().to_vec(),
                            Value::Binary(b) => b.to_vec(),
                            _ => val.to_string().into_bytes(),
                        };
                        if *is_line {
                            bytes.push(b'\n');
                        }
                        // Bound the (possibly backpressured) send. Once the 64-slot
                        // channel fills, `tx.send(..).await` parks until the client
                        // reads — a client that stays connected but stops reading
                        // would otherwise pin this handler forever (`main loop` is
                        // deadline-exempt). Cap the wait at
                        // `web_server_response_timeout_seconds` (0 = disabled, the
                        // documented sentinel — keep the unbounded behavior). A
                        // dropped receiver (disconnect) still returns immediately.
                        let write_timeout = self.config.web_server_response_timeout_seconds;
                        // `Ok(())` sent; `Err(false)` receiver dropped (disconnect);
                        // `Err(true)` timed out with the client still connected but
                        // not reading (a stall).
                        let outcome: Result<(), bool> = if write_timeout == 0 {
                            tx.send(bytes).await.map_err(|_| false)
                        } else {
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(write_timeout),
                                tx.send(bytes),
                            )
                            .await
                            {
                                Ok(Ok(())) => Ok(()),
                                Ok(Err(_)) => Err(false),
                                Err(_) => Err(true),
                            }
                        };
                        match outcome {
                            Ok(()) => Ok((Value::Null, ControlFlow::None)),
                            Err(stalled) => {
                                // Disconnect → Cancelled (cooperative). Stall
                                // (client still connected, not reading) → Timeout
                                // (not Cancelled). Post-accept either way so the
                                // concurrent breaker does not tear the server down.
                                self.server_response_streams.borrow_mut().remove(&handle_id);
                                self.open_response_streams
                                    .borrow_mut()
                                    .retain(|s| s != &handle_id);
                                if stalled {
                                    Err(RuntimeError::with_kind(
                                        "Cannot write to response stream: the client stopped reading \
                                         (write timed out)"
                                            .to_string(),
                                        *line,
                                        *column,
                                        ErrorKind::Timeout,
                                    ))
                                } else {
                                    Err(RuntimeError::with_kind(
                                        "Cannot write to response stream: the client has disconnected"
                                            .to_string(),
                                        *line,
                                        *column,
                                        ErrorKind::Cancelled,
                                    ))
                                }
                            }
                        }
                    }
                    None => Err(RuntimeError::new(
                        "Cannot write to a closed response stream".to_string(),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::FlushStreamStatement {
                target,
                legacy_binding,
                action_fallback,
                line,
                column,
            } => {
                // Backward compatibility: before `flush` was a streaming command,
                // the full merged form (including postfix) was an expression
                // statement. When the root binding of the legacy AST exists,
                // evaluate it with the same ExpressionStatement semantics
                // (zero-arg auto-call; parameterized bare call → arity error).
                if let Some(fallback_expr) = action_fallback {
                    let root_bound = legacy_binding
                        .as_deref()
                        .is_some_and(|name| env.borrow().get(name).is_some());
                    if root_bound {
                        // Reuse ExpressionStatement semantics by dispatching a
                        // synthetic statement.
                        let flush_result = self
                            .execute_statement(
                                &Statement::ExpressionStatement {
                                    expression: fallback_expr.clone(),
                                    line: *line,
                                    column: *column,
                                },
                                Rc::clone(&env),
                            )
                            .await?;
                        // Exact `flush (…)` / `flush call …` (legacy binding is
                        // the bare word `flush`; merged forms always carry the
                        // operand in the phrase): pre-streaming these were TWO
                        // statements — bare `flush`, then the operand as its own
                        // expression statement. Keep the operand's side effects
                        // by evaluating it too, in the original order (#642).
                        if legacy_binding.as_deref() == Some("flush") {
                            return self
                                .execute_statement(
                                    &Statement::ExpressionStatement {
                                        expression: target.clone(),
                                        line: *line,
                                        column: *column,
                                    },
                                    Rc::clone(&env),
                                )
                                .await;
                        }
                        return Ok(flush_result);
                    }
                }
                let handle_id = self
                    .resolve_server_stream_handle(target, &env, *line, *column)
                    .await?;
                if !self.owns_response_stream(&handle_id) {
                    return Err(
                        self.response_stream_ownership_error("flush", &handle_id, *line, *column)
                    );
                }
                let exists = self
                    .server_response_streams
                    .borrow()
                    .contains_key(&handle_id);
                if !exists {
                    return Err(RuntimeError::new(
                        "Cannot flush a closed response stream".to_string(),
                        *line,
                        *column,
                    ));
                }
                // Advisory: chunks are already handed to the transport as they
                // are written; yield so the transport task is scheduled to push
                // them to the socket.
                tokio::task::yield_now().await;
                Ok((Value::Null, ControlFlow::None))
            }
            // Graceful shutdown and signal handling statements
            Statement::RegisterSignalHandlerStatement {
                signal_type,
                handler_name,
                line,
                column,
            } => {
                // For now, just store the signal handler registration
                // In a full implementation, this would set up actual signal handlers
                let signal_handler_key = format!("signal_handler_{}", signal_type);

                env.borrow_mut()
                    .define(
                        &signal_handler_key,
                        Value::Text(Arc::from(handler_name.clone())),
                    )
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                // TODO: Implement actual signal handling with tokio::signal
                // For now, we'll simulate this in the graceful shutdown test

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::StopAcceptingConnectionsStatement {
                server,
                line,
                column,
            } => {
                let server_val = self.evaluate_expression(server, Rc::clone(&env)).await?;
                let server_name = match &server_val {
                    Value::Text(name) => {
                        let name_str = name.as_ref();
                        if name_str.starts_with("WebServer::") {
                            // Find the original server name in our web_servers map
                            let web_servers = self.web_servers.borrow();
                            if let Some((found_name, _)) = web_servers.iter().next() {
                                found_name.clone()
                            } else {
                                return Err(RuntimeError::new(
                                    "No web servers found".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        } else {
                            name_str.to_string()
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected server name as text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Mark server as no longer accepting connections
                // In a full implementation, this would stop the warp server from accepting new connections
                // For now, we'll just set a flag
                env.borrow_mut()
                    .define(
                        &format!("{}_accepting_connections", server_name),
                        Value::Bool(false),
                    )
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::CloseServerStatement {
                server,
                line,
                column,
            } => {
                let server_val = self.evaluate_expression(server, Rc::clone(&env)).await?;

                // A WebSocket server closes by asking each connection's writer to
                // send a close frame, then aborting the accept task.
                if let Value::Text(name) = &server_val
                    && name.starts_with("WebSocketServer::")
                {
                    let key = name.to_string();
                    // Remove and drop the borrow before any `.await` below.
                    let removed_ws = self.web_socket_servers.borrow_mut().remove(&key);
                    if let Some(mut ws_server) = removed_ws {
                        // Wake every live connection's reader so it stops waiting
                        // on the peer and tears down (releasing its slot), even if
                        // the peer never answers the close handshake.
                        let _ = ws_server.close_tx.send(true);
                        let ids = ws_server
                            .connection_ids
                            .lock()
                            .map(|g| g.clone())
                            .unwrap_or_default();
                        if let Ok(mut map) = self.ws_connections.lock() {
                            for id in ids {
                                if let Some(tx) = map.remove(&id) {
                                    let _ = tx.try_send(WsOutbound::Close);
                                }
                            }
                        }
                        if let Some(handle) = ws_server.server_handle.take() {
                            // Give queued close frames a moment to flush before
                            // the accept task is torn down.
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            handle.abort();
                        }
                        return Ok((Value::Null, ControlFlow::None));
                    }
                    return Err(RuntimeError::new(
                        format!("WebSocket server '{key}' not found"),
                        *line,
                        *column,
                    ));
                }

                let server_name = match &server_val {
                    Value::Text(name) => {
                        let name_str = name.as_ref();
                        if name_str.starts_with("WebServer::") {
                            // Find the server name that corresponds to this WebServer value
                            let web_servers = self.web_servers.borrow();

                            // Search through all servers to find which one has this exact value
                            let mut found_server = None;
                            for server_name in web_servers.keys() {
                                // Check if this server name's variable has the matching value
                                if let Some(Value::Text(stored_text)) =
                                    env.borrow().get(server_name)
                                    && stored_text.as_ref() == name_str
                                {
                                    found_server = Some(server_name.clone());
                                    break;
                                }
                            }

                            // Return the found server or use first server as fallback
                            if let Some(server_name) = found_server {
                                server_name
                            } else if let Some((found_name, _)) = web_servers.iter().next() {
                                found_name.clone()
                            } else {
                                return Err(RuntimeError::new(
                                    "No web servers found".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        } else {
                            name_str.to_string()
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected server name as text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Close the server. Remove it from the map and DROP the borrow
                // before the graceful-shutdown await — holding `web_servers`
                // borrowed across `.await` would panic a concurrent sibling that
                // touches the map during the yield (the reason this module can
                // drop its `await_holding_refcell_ref` allow — see lib.rs).
                let removed = self.web_servers.borrow_mut().remove(&server_name);
                match removed {
                    Some(mut wfl_server) => {
                        // Graceful shutdown: give in-flight responses time to
                        // complete transmission before forcefully aborting the
                        // server task. The map borrow is already released, so
                        // this await cannot conflict with a sibling handler.
                        if let Some(handle) = wfl_server.server_handle.take() {
                            // Allow 50ms for pending HTTP responses to reach the
                            // client before abort() closes the TCP connection
                            // (otherwise IncompleteMessage on the client).
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            handle.abort();
                        }
                    }
                    None => {
                        return Err(RuntimeError::new(
                            format!("Server '{}' not found", server_name),
                            *line,
                            *column,
                        ));
                    }
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::ListenWebSocketStatement {
                port,
                server_name,
                line,
                column,
            } => {
                let port_val = self.evaluate_expression(port, Rc::clone(&env)).await?;
                let port_num = match &port_val {
                    Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 && *n <= 65535.0 => *n as u16,
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected a whole number between 0 and 65535 for the websocket port, got {port_val:?}"
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                // Bounded lifecycle-event channel (sized from the shared budget):
                // a flood of connect/message/disconnect events sheds on `Full`
                // rather than growing memory without bound.
                let (event_sender, event_receiver) =
                    mpsc::channel::<WflWsEvent>(self.budget.ws_queue_bound());
                let event_receiver = Arc::new(tokio::sync::Mutex::new(event_receiver));
                let connection_ids = Arc::new(std::sync::Mutex::new(Vec::new()));
                // Per-server cancellation channel. Each connection clones the
                // receiver; `close server` flips/drops the sender to wake them.
                let (close_tx, close_rx) = tokio::sync::watch::channel(false);

                // Clones handed to warp's per-connection tasks.
                let ws_connections = Arc::clone(&self.ws_connections);
                let connection_ids_task = Arc::clone(&connection_ids);
                let event_sender_task = event_sender.clone();
                let budget_task = Arc::clone(&self.budget);
                let close_rx_task = close_rx.clone();

                // Cap the transport's own message/frame assembly at the budget's
                // per-message limit *before* upgrade, so a fragmented text frame
                // or an ignored binary frame cannot allocate up to Tungstenite's
                // independent defaults on the receive side. The queued-byte permit
                // in the reader loop is then the second (global) layer.
                let max_ws_message = self.budget.max_ws_message_bytes();
                let route = warp::ws().and(warp::addr::remote()).map(
                    move |ws: warp::ws::Ws, remote: Option<std::net::SocketAddr>| {
                        let events = event_sender_task.clone();
                        let connections = Arc::clone(&ws_connections);
                        let ids = Arc::clone(&connection_ids_task);
                        let budget = Arc::clone(&budget_task);
                        let cancel = close_rx_task.clone();
                        ws.max_message_size(max_ws_message)
                            .max_frame_size(max_ws_message)
                            .on_upgrade(move |socket| {
                                handle_ws_connection(
                                    socket,
                                    remote,
                                    events,
                                    connections,
                                    ids,
                                    budget,
                                    cancel,
                                )
                            })
                    },
                );

                let bind_addr: IpAddr = match self.config.web_server_bind_address.parse() {
                    Ok(addr) => addr,
                    Err(_) => {
                        return Err(RuntimeError::new(
                            format!(
                                "Invalid web_server_bind_address in config: '{}'. Expected a valid IP address (e.g., '127.0.0.1' or '0.0.0.0')",
                                self.config.web_server_bind_address
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                match warp::serve(route).try_bind_ephemeral((bind_addr, port_num)) {
                    Ok((addr, server)) => {
                        let server_handle = tokio::spawn(server);
                        let key = format!("WebSocketServer::{}:{}", addr.ip(), addr.port());

                        let wfl_ws = WflWebSocketServer {
                            event_receiver,
                            connection_ids,
                            handlers: RefCell::new(WsHandlerSet::default()),
                            server_handle: Some(server_handle),
                            close_tx,
                        };
                        self.web_socket_servers
                            .borrow_mut()
                            .insert(key.clone(), wfl_ws);

                        let server_value = Value::Text(Arc::from(key));
                        println!("WebSocket server is listening on port {}", addr.port());

                        match env.borrow_mut().define_direct(server_name, server_value) {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to start websocket server on port {port_num}: {e}"),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::WebSocketHandlerStatement {
                event,
                server,
                binding,
                body,
                line,
                column,
            } => {
                let server_key = self
                    .resolve_ws_server_key(server, Rc::clone(&env), *line, *column)
                    .await?;

                let handler = WsRegisteredHandler {
                    binding: binding.clone(),
                    body: body.clone(),
                    env: Rc::clone(&env),
                };

                let servers = self.web_socket_servers.borrow();
                let ws_server = servers.get(&server_key).ok_or_else(|| {
                    RuntimeError::new(
                        format!(
                            "WebSocket server '{server_key}' is not running. Start it with 'listen for websockets ...' first."
                        ),
                        *line,
                        *column,
                    )
                })?;

                let mut handlers = ws_server.handlers.borrow_mut();
                match event {
                    WsHandlerEvent::Connect => handlers.connect = Some(handler),
                    WsHandlerEvent::Message => handlers.message = Some(handler),
                    WsHandlerEvent::Disconnect => handlers.disconnect = Some(handler),
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::SendWebSocketMessageStatement {
                message,
                target,
                line,
                column,
            } => {
                let message_val = self.evaluate_expression(message, Rc::clone(&env)).await?;
                // Measure the payload from the borrowed value first (no clone).
                let msg_len = Self::ws_message_byte_len(&message_val, *line, *column)?;

                let target_val = self.evaluate_expression(target, Rc::clone(&env)).await?;
                let conn_id = Self::ws_connection_id(&target_val, *line, *column)?;

                let sender = self
                    .ws_connections
                    .lock()
                    .ok()
                    .and_then(|map| map.get(&conn_id).cloned());

                match sender {
                    Some(tx) => {
                        // Reserve the frame's bytes against the per-message and
                        // global queued-byte budget *before* materializing the
                        // payload, so an oversized value is never cloned first.
                        match self.budget.try_reserve_ws_bytes(msg_len) {
                            Some(permit) => {
                                let text = Self::ws_message_text(&message_val, *line, *column)?;
                                // A closed writer task is indistinguishable from a
                                // live one here; a dropped frame simply means the
                                // peer left (or the bounded queue is saturated).
                                if let Err(err) = tx.try_send(WsOutbound::Text {
                                    text,
                                    _permit: permit,
                                }) {
                                    log::warn!(
                                        "WebSocket outbound queue full/closed for {conn_id}; dropping frame: {err}"
                                    );
                                }
                            }
                            None => {
                                log::warn!(
                                    "WebSocket outbound frame for {conn_id} ({msg_len} bytes) exceeds the per-message or global queued-byte limit; dropping frame"
                                );
                            }
                        }
                        Ok((Value::Null, ControlFlow::None))
                    }
                    None => Err(RuntimeError::new(
                        "That websocket connection is closed or unknown".to_string(),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::BroadcastWebSocketMessageStatement {
                message,
                server,
                line,
                column,
            } => {
                let message_val = self.evaluate_expression(message, Rc::clone(&env)).await?;
                // Measure first; reject an oversized broadcast payload before it
                // is materialized (and then cloned per recipient).
                let msg_len = Self::ws_message_byte_len(&message_val, *line, *column)?;
                if msg_len > self.budget.max_ws_message_bytes() {
                    log::warn!(
                        "WebSocket broadcast payload ({msg_len} bytes) exceeds the per-message limit; dropping broadcast"
                    );
                    return Ok((Value::Null, ControlFlow::None));
                }
                let text = Self::ws_message_text(&message_val, *line, *column)?;

                let server_key = self
                    .resolve_ws_server_key(server, Rc::clone(&env), *line, *column)
                    .await?;

                let ids: Vec<String> = {
                    let servers = self.web_socket_servers.borrow();
                    match servers.get(&server_key) {
                        Some(ws_server) => ws_server
                            .connection_ids
                            .lock()
                            .map(|g| g.clone())
                            .unwrap_or_default(),
                        None => Vec::new(),
                    }
                };

                if let Ok(map) = self.ws_connections.lock() {
                    for id in ids {
                        if let Some(tx) = map.get(&id) {
                            // Reserve each recipient's copy against the global
                            // queued-byte budget; shed over-budget frames rather
                            // than buffering them without bound.
                            match self.budget.try_reserve_ws_bytes(msg_len) {
                                Some(permit) => {
                                    if let Err(err) = tx.try_send(WsOutbound::Text {
                                        text: text.clone(),
                                        _permit: permit,
                                    }) {
                                        log::warn!(
                                            "WebSocket broadcast: outbound queue full/closed for {id}; dropping frame: {err}"
                                        );
                                    }
                                }
                                None => {
                                    log::warn!(
                                        "WebSocket broadcast frame for {id} ({msg_len} bytes) exceeds the global queued-byte limit; dropping frame"
                                    );
                                }
                            }
                        }
                    }
                }

                Ok((Value::Null, ControlFlow::None))
            }
            // Subprocess statements
            Statement::ExecuteCommandStatement {
                command,
                arguments,
                variable_name,
                use_shell,
                line,
                column,
            } => {
                // Evaluate command expression
                let cmd_val = self.evaluate_expression(command, Rc::clone(&env)).await?;
                let cmd_str = match &cmd_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Command must be text, got {}", cmd_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate arguments if provided
                let args_vec: Vec<String> = if let Some(args_expr) = arguments {
                    let args_val = self.evaluate_expression(args_expr, Rc::clone(&env)).await?;
                    match &args_val {
                        Value::List(list) => {
                            let list_ref = list.borrow();
                            list_ref
                                .iter()
                                .map(|v| match v {
                                    Value::Text(t) => Ok(t.as_ref().to_string()),
                                    _ => Ok(v.to_string()),
                                })
                                .collect::<Result<Vec<_>, RuntimeError>>()?
                        }
                        Value::Text(text) => vec![text.as_ref().to_string()],
                        _ => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Arguments must be a list or text, got {}",
                                    args_val.type_name()
                                ),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    Vec::new()
                };

                // Execute command
                let args_refs: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
                let (stdout, stderr, exit_code) = self
                    .io_client
                    .execute_command(cmd_str, &args_refs, *use_shell, *line, *column)
                    .await
                    .map_err(|e| match e {
                        ExecuteCommandError::Budget(exceeded) => {
                            self.budget_error(exceeded, *line, *column)
                        }
                        ExecuteCommandError::Timeout { seconds } => RuntimeError::with_kind(
                            format!("Subprocess execution exceeded timeout ({seconds}s)"),
                            *line,
                            *column,
                            ErrorKind::Timeout,
                        ),
                        ExecuteCommandError::Other(message) => {
                            // Preserve the existing subprocess error
                            // classification for non-budget failures.
                            let kind = if message.contains("program not found")
                                || message.contains("cannot find")
                                || message.contains("not recognized")
                            {
                                ErrorKind::CommandNotFound
                            } else if message.contains("spawn") {
                                ErrorKind::ProcessSpawnFailed
                            } else {
                                ErrorKind::General
                            };
                            RuntimeError::with_kind(message, *line, *column, kind)
                        }
                    })?;

                // Build result object
                let mut result_map = HashMap::new();
                result_map.insert(
                    "output".to_string(),
                    Value::Text(Arc::from(stdout.as_str())),
                );
                result_map.insert("error".to_string(), Value::Text(Arc::from(stderr.as_str())));
                result_map.insert("exit_code".to_string(), Value::Number(exit_code as f64));
                result_map.insert("success".to_string(), Value::Bool(exit_code == 0));

                let result_obj = Value::Object(Rc::new(RefCell::new(result_map)));

                // Store result if variable name provided
                if let Some(var_name) = variable_name {
                    env.borrow_mut()
                        .define_direct(var_name, result_obj)
                        .map_err(|e| RuntimeError::new(e, *line, *column))?;
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::ExecuteFileStatement {
                path,
                request,
                variable_name,
                line,
                column,
            } => {
                // Boxed out of `_execute_statement`'s shared state machine so the
                // full lex→parse→analyze→interpret pipeline does not inflate every
                // other statement's future (and so nested `execute file` depth is
                // not multiplied by that giant enum on the native stack — #681).
                Box::pin(self.execute_wfl_file(
                    path,
                    request.as_ref(),
                    variable_name.as_deref(),
                    *line,
                    *column,
                    env,
                ))
                .await
            }
            Statement::SpawnProcessStatement {
                command,
                arguments,
                variable_name,
                use_shell,
                line,
                column,
            } => {
                // Evaluate command expression
                let cmd_val = self.evaluate_expression(command, Rc::clone(&env)).await?;
                let cmd_str = match &cmd_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Command must be text, got {}", cmd_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate arguments if provided
                let args_vec: Vec<String> = if let Some(args_expr) = arguments {
                    let args_val = self.evaluate_expression(args_expr, Rc::clone(&env)).await?;
                    match &args_val {
                        Value::List(list) => {
                            let list_ref = list.borrow();
                            list_ref
                                .iter()
                                .map(|v| match v {
                                    Value::Text(t) => Ok(t.as_ref().to_string()),
                                    _ => Ok(v.to_string()),
                                })
                                .collect::<Result<Vec<_>, RuntimeError>>()?
                        }
                        Value::Text(text) => vec![text.as_ref().to_string()],
                        _ => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Arguments must be a list or text, got {}",
                                    args_val.type_name()
                                ),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    Vec::new()
                };

                // Spawn process
                let args_refs: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
                let process_id = self
                    .io_client
                    .spawn_process(cmd_str, &args_refs, *use_shell, *line, *column)
                    .await
                    .map_err(|e| {
                        let kind = if e.contains("program not found")
                            || e.contains("cannot find")
                            || e.contains("not recognized")
                        {
                            ErrorKind::CommandNotFound
                        } else {
                            ErrorKind::ProcessSpawnFailed
                        };
                        RuntimeError::with_kind(e, *line, *column, kind)
                    })?;

                // Store process ID in variable
                env.borrow_mut()
                    .define_direct(variable_name, Value::Text(Arc::from(process_id.as_str())))
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::ReadProcessOutputStatement {
                process_id,
                variable_name,
                line,
                column,
            } => {
                // Evaluate process ID expression
                let proc_val = self
                    .evaluate_expression(process_id, Rc::clone(&env))
                    .await?;
                let proc_id = match &proc_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Process ID must be text, got {}", proc_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                };

                // Read process output
                let output = self
                    .io_client
                    .read_process_output(proc_id)
                    .await
                    .map_err(|e| {
                        let kind = if e.contains("Invalid process ID") {
                            ErrorKind::ProcessNotFound
                        } else {
                            ErrorKind::General
                        };
                        RuntimeError::with_kind(e, *line, *column, kind)
                    })?;

                // Store output in variable
                env.borrow_mut()
                    .define_direct(variable_name, Value::Text(Arc::from(output.as_str())))
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::KillProcessStatement {
                process_id,
                line,
                column,
            } => {
                // Evaluate process ID expression
                let proc_val = self
                    .evaluate_expression(process_id, Rc::clone(&env))
                    .await?;
                let proc_id = match &proc_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Process ID must be text, got {}", proc_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                };

                // Kill process
                self.io_client.kill_process(proc_id).await.map_err(|e| {
                    let kind = if e.contains("Invalid process ID") {
                        ErrorKind::ProcessNotFound
                    } else {
                        ErrorKind::ProcessKillFailed
                    };
                    RuntimeError::with_kind(e, *line, *column, kind)
                })?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::WaitForProcessStatement {
                process_id,
                variable_name,
                line,
                column,
            } => {
                // Evaluate process ID expression
                let proc_val = self
                    .evaluate_expression(process_id, Rc::clone(&env))
                    .await?;
                let proc_id = match &proc_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Process ID must be text, got {}", proc_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                };

                // Wait for process to complete
                let exit_code = self
                    .io_client
                    .wait_for_process(proc_id)
                    .await
                    .map_err(|e| {
                        let kind = if e.contains("Invalid process ID") {
                            ErrorKind::ProcessNotFound
                        } else {
                            ErrorKind::General
                        };
                        RuntimeError::with_kind(e, *line, *column, kind)
                    })?;

                // Store exit code in variable if provided
                if let Some(var_name) = variable_name {
                    env.borrow_mut()
                        .define_direct(var_name, Value::Number(exit_code as f64))
                        .map_err(|e| RuntimeError::new(e, *line, *column))?;
                }

                Ok((Value::Null, ControlFlow::None))
            }
            // Test framework statements
            Statement::DescribeBlock {
                description,
                setup,
                teardown,
                tests,
                line,
                column,
            } => {
                if !*self.test_mode.borrow() {
                    return Err(RuntimeError::new(
                        "describe blocks can only be used in test mode (run with --test flag)"
                            .to_string(),
                        *line,
                        *column,
                    ));
                }

                // Push describe context
                self.current_describe_stack
                    .borrow_mut()
                    .push(description.clone());

                // Create describe-level environment for setup/teardown sharing
                // This allows tests to access setup variables while remaining isolated from each other
                let describe_env = Environment::new_child_env(&env);

                // Run setup if present (runs in describe environment).
                // Setup/tests/teardown are three separate statement blocks,
                // each with its own overload-duplicate scope.
                if let Some(setup_stmts) = setup {
                    let (_overload_dups, _armed_members) =
                        self.enter_block_overloads(setup_stmts, &describe_env);
                    for stmt in setup_stmts {
                        Box::pin(self._execute_statement(stmt, describe_env.clone())).await?;
                    }
                }

                // Execute all tests (each gets a child of describe_env for isolation)
                {
                    let (_overload_dups, _armed_members) =
                        self.enter_block_overloads(tests, &describe_env);
                    for test in tests {
                        Box::pin(self._execute_statement(test, describe_env.clone())).await?;
                    }
                }

                // Run teardown if present (runs in describe environment)
                if let Some(teardown_stmts) = teardown {
                    let (_overload_dups, _armed_members) =
                        self.enter_block_overloads(teardown_stmts, &describe_env);
                    for stmt in teardown_stmts {
                        Box::pin(self._execute_statement(stmt, describe_env.clone())).await?;
                    }
                }

                // Pop describe context
                self.current_describe_stack.borrow_mut().pop();

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::TestBlock {
                description,
                body,
                line,
                column,
            } => {
                if !*self.test_mode.borrow() {
                    return Err(RuntimeError::new(
                        "test blocks can only be used in test mode (run with --test flag)"
                            .to_string(),
                        *line,
                        *column,
                    ));
                }

                // Set current test name for failure tracking
                *self.current_test_name.borrow_mut() = Some(description.clone());

                // Increment test count
                self.test_results.borrow_mut().total_tests += 1;

                // Create isolated environment for test (child of describe env)
                // Using isolated mode prevents tests from mutating setup variables,
                // ensuring each test gets a fresh copy for true isolation
                let test_env = Environment::new_isolated_child_env(&env);

                // Execute test body and catch assertion failures. The body
                // is its own statement block for overload enforcement.
                let (_overload_dups, _armed_members) = self.enter_block_overloads(body, &test_env);
                let mut test_passed = true;

                for stmt in body {
                    match Box::pin(self._execute_statement(stmt, test_env.clone())).await {
                        Ok(_) => {}
                        Err(e) if e.is_exit_program() => {
                            // Stopping the program is not a failing test. Pass the
                            // sentinel through so the run ends cleanly instead of
                            // recording a bogus failure and continuing.
                            return Err(e);
                        }
                        Err(e) => {
                            test_passed = false;

                            // Assertion failures are already recorded (failure entry +
                            // failed_tests increment) by the ExpectStatement handler; its
                            // raw message begins with "Assertion failed:". We must inspect
                            // the raw `message` field here rather than the Display string,
                            // which is prefixed with "Runtime error at line ...:" and would
                            // otherwise never match the guard, causing the assertion to be
                            // recorded twice. Any error that is NOT an assertion failure is a
                            // runtime error in the test body that we record and count here so
                            // it is reflected in the failure count and the process exit code.
                            if !e.message.starts_with("Assertion failed:") {
                                let context = self.current_describe_stack.borrow().clone();
                                let failure = TestFailure {
                                    describe_context: context,
                                    test_name: description.clone(),
                                    assertion_message: e.to_string(),
                                    line: *line,
                                    column: *column,
                                };
                                let mut results = self.test_results.borrow_mut();
                                results.failures.push(failure);
                                results.failed_tests += 1;
                            }

                            // Don't propagate the error - continue running other tests
                            break;
                        }
                    }
                }

                if test_passed {
                    self.test_results.borrow_mut().passed_tests += 1;
                }

                // Clear current test name
                *self.current_test_name.borrow_mut() = None;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::ExpectStatement {
                subject,
                assertion,
                line,
                column,
            } => {
                if !*self.test_mode.borrow() {
                    return Err(RuntimeError::new(
                        "expect statements can only be used in test mode (run with --test flag)"
                            .to_string(),
                        *line,
                        *column,
                    ));
                }

                // Evaluate subject expression
                let subject_value = self.evaluate_expression(subject, env.clone()).await?;

                // Check assertion
                let (passed, expected_value) = self
                    .check_assertion(&subject_value, assertion, env.clone())
                    .await?;

                if !passed {
                    // Record failure with proper test name tracking
                    let message = self.create_assertion_message_with_values(
                        assertion,
                        &subject_value,
                        expected_value.as_ref(),
                    );
                    let context = self.current_describe_stack.borrow().clone();
                    let test_name = self
                        .current_test_name
                        .borrow()
                        .clone()
                        .unwrap_or_else(|| "unknown test".to_string());

                    let failure = TestFailure {
                        describe_context: context,
                        test_name,
                        assertion_message: message.clone(),
                        line: *line,
                        column: *column,
                    };

                    self.test_results.borrow_mut().failures.push(failure);
                    self.test_results.borrow_mut().failed_tests += 1;

                    return Err(RuntimeError::new(
                        format!("Assertion failed: {message}"),
                        *line,
                        *column,
                    ));
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::ConfigureSessionsStatement {
                server,
                timeout,
                storage,
                line,
                column,
            } => {
                let manager = self
                    .session_manager_from_server_expr(server, Rc::clone(&env), *line, *column)
                    .await?;
                let timeout_val = self.evaluate_expression(timeout, Rc::clone(&env)).await?;
                let timeout_ms = match timeout_val {
                    Value::Number(n) if n >= 1.0 => n as u64,
                    _ => {
                        return Err(RuntimeError::new(
                            "Session timeout must be a number of milliseconds >= 1".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };
                let storage_val = self.evaluate_expression(storage, Rc::clone(&env)).await?;
                let storage_kind = match &storage_val {
                    Value::Text(s) => sessions::SessionStorageKind::parse(s)
                        .map_err(|e| RuntimeError::new(e, *line, *column))?,
                    _ => {
                        return Err(RuntimeError::new(
                            "Session storage must be text: memory, file, or database".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };
                manager
                    .configure(timeout_ms, storage_kind)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::EnableCsrfProtectionStatement {
                server,
                line,
                column,
            } => {
                let manager = self
                    .session_manager_from_server_expr(server, Rc::clone(&env), *line, *column)
                    .await?;
                manager.enable_csrf().await;
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::EnableSecureCookiesStatement {
                server,
                line,
                column,
            } => {
                let manager = self
                    .session_manager_from_server_expr(server, Rc::clone(&env), *line, *column)
                    .await?;
                manager.enable_secure_cookies().await;
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::SetSessionValueStatement {
                key,
                value,
                session,
                line,
                column,
            } => {
                let session_val = self.evaluate_expression(session, Rc::clone(&env)).await?;
                let key_val = self.evaluate_expression(key, Rc::clone(&env)).await?;
                let value_val = self.evaluate_expression(value, Rc::clone(&env)).await?;
                let key_text = match &key_val {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Session value key must be text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };
                let (manager, id) = self
                    .session_manager_from_session_object(&session_val, *line, *column)
                    .await?;
                manager
                    .set_value(&id, &key_text, value_val)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::DestroySessionStatement {
                session,
                line,
                column,
            } => {
                let session_val = self.evaluate_expression(session, Rc::clone(&env)).await?;
                let (manager, id) = self
                    .session_manager_from_session_object(&session_val, *line, *column)
                    .await?;
                manager
                    .destroy(&id)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::StoreSessionDataStatement {
                key,
                data,
                line,
                column,
            } => {
                let manager = self.sole_or_named_session_manager(*line, *column).await?;
                let key_val = self.evaluate_expression(key, Rc::clone(&env)).await?;
                let data_val = self.evaluate_expression(data, Rc::clone(&env)).await?;
                let key_text = match &key_val {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Session storage key must be text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };
                manager
                    .put_kv(&key_text, data_val)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::DeleteSessionDataStatement { key, line, column } => {
                let manager = self.sole_or_named_session_manager(*line, *column).await?;
                let key_val = self.evaluate_expression(key, Rc::clone(&env)).await?;
                let key_text = match &key_val {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Session storage key must be text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };
                manager
                    .delete_kv(&key_text)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                Ok((Value::Null, ControlFlow::None))
            }
        };

        if self.step_mode {
            self.dump_state(stmt, line, column, &env_before);
            if !self.prompt_continue() {
                std::process::exit(0);
            }
        }

        result
    }

    async fn session_server_name_from_expr(
        &self,
        server: &Expression,
        env: Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<String, RuntimeError> {
        if let Expression::Variable(name, ..) = server
            && self.web_servers.borrow().contains_key(name)
        {
            return Ok(name.clone());
        }
        let value = self.evaluate_expression(server, env).await?;
        match value {
            Value::Text(name) => {
                let name_str = name.as_ref();
                if self.web_servers.borrow().contains_key(name_str) {
                    return Ok(name_str.to_string());
                }
                if name_str.starts_with("WebServer::") {
                    let web_servers = self.web_servers.borrow();
                    if let Some(server_name) = web_servers.keys().next() {
                        return Ok(server_name.clone());
                    }
                }
                Err(RuntimeError::new(
                    format!("Unknown web server '{name_str}'"),
                    line,
                    column,
                ))
            }
            _ => Err(RuntimeError::new(
                "Expected a web server".to_string(),
                line,
                column,
            )),
        }
    }

    async fn session_manager_from_server_expr(
        &self,
        server: &Expression,
        env: Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<Rc<sessions::SessionManager>, RuntimeError> {
        let name = self
            .session_server_name_from_expr(server, env, line, column)
            .await?;
        self.session_manager_by_name(&name, line, column)
    }

    fn session_manager_by_name(
        &self,
        name: &str,
        line: usize,
        column: usize,
    ) -> Result<Rc<sessions::SessionManager>, RuntimeError> {
        let servers = self.web_servers.borrow();
        match servers.get(name).and_then(|s| s.sessions.clone()) {
            Some(manager) => Ok(manager),
            None => Err(RuntimeError::new(
                format!(
                    "Server '{name}' does not have sessions enabled. Use `listen ... with sessions enabled`."
                ),
                line,
                column,
            )),
        }
    }

    async fn session_manager_from_request(
        &self,
        request: &Value,
        line: usize,
        column: usize,
    ) -> Result<(Rc<sessions::SessionManager>, String), RuntimeError> {
        let server = match request {
            Value::Object(obj) => match obj.borrow().get("_session_server") {
                Some(Value::Text(name)) => name.to_string(),
                _ => {
                    return Err(RuntimeError::new(
                        "Request is missing its session server. Wait for the request on a session-enabled listener.".to_string(),
                        line,
                        column,
                    ));
                }
            },
            _ => {
                return Err(RuntimeError::new(
                    "Expected a request object".to_string(),
                    line,
                    column,
                ));
            }
        };
        let manager = self.session_manager_by_name(&server, line, column)?;
        Ok((manager, server))
    }

    async fn session_manager_from_session_object(
        &self,
        session: &Value,
        line: usize,
        column: usize,
    ) -> Result<(Rc<sessions::SessionManager>, String), RuntimeError> {
        let (server, id) = match session {
            Value::Object(obj) => {
                let obj = obj.borrow();
                let server = match obj.get("_server") {
                    Some(Value::Text(name)) => name.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected a session object from create session or get session"
                                .to_string(),
                            line,
                            column,
                        ));
                    }
                };
                let id = match obj.get("id") {
                    Some(Value::Text(id)) => id.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Session object is missing its id".to_string(),
                            line,
                            column,
                        ));
                    }
                };
                (server, id)
            }
            _ => {
                return Err(RuntimeError::new(
                    "Expected a session object".to_string(),
                    line,
                    column,
                ));
            }
        };
        let manager = self.session_manager_by_name(&server, line, column)?;
        Ok((manager, id))
    }

    async fn sole_or_named_session_manager(
        &self,
        line: usize,
        column: usize,
    ) -> Result<Rc<sessions::SessionManager>, RuntimeError> {
        let managers: Vec<_> = self
            .web_servers
            .borrow()
            .iter()
            .filter_map(|(name, server)| server.sessions.clone().map(|m| (name.clone(), m)))
            .collect();
        match managers.len() {
            0 => Err(RuntimeError::new(
                "No session-enabled server is running. Use `listen ... with sessions enabled`."
                    .to_string(),
                line,
                column,
            )),
            1 => Ok(managers.into_iter().next().unwrap().1),
            _ => Err(RuntimeError::new(
                "More than one session-enabled server is running; name the server in configure sessions."
                    .to_string(),
                line,
                column,
            )),
        }
    }

    async fn session_set_cookie(
        &self,
        source: &Value,
        clear: bool,
        line: usize,
        column: usize,
    ) -> Result<String, RuntimeError> {
        let (manager, id) = if matches!(source, Value::Object(obj) if obj.borrow().contains_key("id"))
        {
            self.session_manager_from_session_object(source, line, column)
                .await?
        } else {
            let (manager, _) = self
                .session_manager_from_request(source, line, column)
                .await?;
            (manager, String::new())
        };
        let cfg = manager.config_snapshot().await;
        Ok(sessions::SessionManager::format_set_cookie(
            &cfg,
            if clear { "" } else { &id },
            clear,
        ))
    }

    /// Resolves a server expression to the key of a running WebSocket server.
    async fn resolve_ws_server_key(
        &self,
        server: &Expression,
        env: Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<String, RuntimeError> {
        let value = self.evaluate_expression(server, env).await?;
        match &value {
            Value::Text(t) => {
                let key = t.to_string();
                if self.web_socket_servers.borrow().contains_key(&key) {
                    Ok(key)
                } else {
                    Err(RuntimeError::new(
                        format!("'{key}' is not a running websocket server"),
                        line,
                        column,
                    ))
                }
            }
            _ => Err(RuntimeError::new(
                "Expected a websocket server (the name bound by 'listen for websockets')"
                    .to_string(),
                line,
                column,
            )),
        }
    }

    /// Coerces a WFL value into the text payload of a websocket frame.
    fn ws_message_text(value: &Value, line: usize, column: usize) -> Result<String, RuntimeError> {
        match value {
            Value::Text(t) => Ok(t.to_string()),
            Value::Number(n) => Ok(format!("{n}")),
            Value::Bool(b) => Ok(if *b { "yes" } else { "no" }.to_string()),
            Value::Null | Value::Nothing => Err(RuntimeError::new(
                "Cannot send an empty websocket message".to_string(),
                line,
                column,
            )),
            other => Err(RuntimeError::new(
                format!(
                    "Cannot send a {} as a websocket message; expected text",
                    other.type_name()
                ),
                line,
                column,
            )),
        }
    }

    /// The byte length a `send`/`broadcast` value would serialize to, computed
    /// from the *borrowed* value — for `Value::Text` (`Arc<str>`) this is
    /// `t.len()` with no allocation. Lets the queued-byte permit be reserved
    /// (and an oversized message rejected) *before* the payload is cloned into a
    /// `String`, so an oversized runtime value is never fully duplicated first.
    fn ws_message_byte_len(
        value: &Value,
        line: usize,
        column: usize,
    ) -> Result<usize, RuntimeError> {
        match value {
            Value::Text(t) => Ok(t.len()),
            // Small, bounded scalars — measuring == materializing cost.
            Value::Number(n) => Ok(format!("{n}").len()),
            Value::Bool(b) => Ok(if *b { 3 } else { 2 }),
            // Reuse `ws_message_text`'s errors for the unsupported cases.
            _ => Self::ws_message_text(value, line, column).map(|s| s.len()),
        }
    }

    /// Extracts a connection id from a `send ... to <target>` target. Accepts the
    /// connection object bound by a connect/message handler (reads its `id`).
    fn ws_connection_id(value: &Value, line: usize, column: usize) -> Result<String, RuntimeError> {
        if let Value::Object(map) = value
            && let Some(Value::Text(id)) = map.borrow().get("id")
        {
            return Ok(id.to_string());
        }
        Err(RuntimeError::new(
            "Expected a websocket connection (the value bound by 'on websocket connect/message')"
                .to_string(),
            line,
            column,
        ))
    }

    /// Drains and dispatches queued websocket events for up to `budget`, running
    /// the matching handler block for each. With no websocket servers active it
    /// is a plain sleep, preserving `wait for <duration>` semantics.
    async fn pump_websocket_events(&self, budget: Duration) -> Result<(), RuntimeError> {
        let deadline = tokio::time::Instant::now() + budget;
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            let remaining = deadline - now;

            // Snapshot the receivers with a short borrow; dispatch below must not
            // hold a borrow of web_socket_servers across handler execution.
            let receivers: Vec<(String, Arc<tokio::sync::Mutex<mpsc::Receiver<WflWsEvent>>>)> =
                self.web_socket_servers
                    .borrow()
                    .iter()
                    .map(|(key, srv)| (key.clone(), Arc::clone(&srv.event_receiver)))
                    .collect();

            if receivers.is_empty() {
                tokio::time::sleep(remaining).await;
                break;
            }

            let mut recv_futs = Vec::with_capacity(receivers.len());
            for (key, rx) in receivers {
                recv_futs.push(Box::pin(async move {
                    let mut guard = rx.lock().await;
                    let event = guard.recv().await;
                    (key, event)
                }));
            }

            let sleep_fut = tokio::time::sleep(remaining);
            tokio::pin!(sleep_fut);

            tokio::select! {
                _ = &mut sleep_fut => break,
                ((key, event), _idx, _rest) = futures_util::future::select_all(recv_futs) => {
                    if let Some(event) = event {
                        self.dispatch_ws_event(&key, event).await?;
                    }
                    // A `None` means that server's channel closed; the next loop
                    // iteration rebuilds the receiver set.
                }
            }
        }
        Ok(())
    }

    /// Runs the registered handler block for one websocket event. Handler errors
    /// are reported but do not tear down the server, matching event-driven norms.
    async fn dispatch_ws_event(
        &self,
        server_key: &str,
        event: WflWsEvent,
    ) -> Result<(), RuntimeError> {
        // Clone the selected handler out before executing: the body may itself
        // register handlers or send frames, which re-borrow web_socket_servers.
        let handler = {
            let servers = self.web_socket_servers.borrow();
            let Some(server) = servers.get(server_key) else {
                return Ok(());
            };
            let handlers = server.handlers.borrow();
            let selected = match event.kind {
                WsEventKind::Connect => handlers.connect.as_ref(),
                WsEventKind::Message => handlers.message.as_ref(),
                WsEventKind::Disconnect => handlers.disconnect.as_ref(),
            };
            selected.map(|h| (h.binding.clone(), h.body.clone(), Rc::clone(&h.env)))
        };

        let Some((binding, body, handler_env)) = handler else {
            return Ok(());
        };

        let event_obj = build_ws_event_object(&event);
        let child_env = Environment::new_child_env(&handler_env);
        // `define_direct` shadows in the fresh handler scope without consulting
        // parents, so a same-named outer variable (`store conn as ...` before an
        // `on websocket connect ... as conn`) does not abort the handler.
        if let Err(msg) = child_env.borrow_mut().define_direct(&binding, event_obj) {
            return Err(RuntimeError::new(msg, 0, 0));
        }

        if let Err(err) = self.execute_block(&body, child_env).await {
            // `exit program` is a stop request, not a handler failure: it must
            // leave the pump and unwind to the top of the run rather than be
            // reported and swallowed like an ordinary handler error.
            if err.is_exit_program() {
                return Err(err);
            }
            eprintln!(
                "WebSocket {} handler error: {}",
                event.kind.as_str(),
                err.message
            );
        }
        Ok(())
    }

    async fn execute_block(
        &self,
        statements: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        Box::pin(self._execute_block(statements, env)).await
    }

    async fn _execute_block(
        &self,
        statements: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        self.assert_invariants();
        // Each block carries its own overload-duplicate set for temporal
        // enforcement (restored on drop, including `?` early returns), and
        // arms same-scope members its definitions will merge with.
        let (_overload_dups, _armed_members) = self.enter_block_overloads(statements, &env);
        let mut last_value = Value::Null;

        #[cfg(debug_assertions)]
        exec_trace!("Executing block of {} statements", statements.len());

        #[cfg(debug_assertions)]
        let _guard = IndentGuard::new();

        let mut control_flow = ControlFlow::None;

        for statement in statements {
            let result = self.execute_statement(statement, Rc::clone(&env)).await?;
            last_value = result.0;
            control_flow = result.1;

            if !matches!(control_flow, ControlFlow::None) {
                #[cfg(debug_assertions)]
                exec_trace!(
                    "Block execution interrupted by control flow: {:?}",
                    control_flow
                );
                break;
            }
        }

        self.assert_invariants();
        Ok((last_value, control_flow))
    }

    /// Attempts to recycle a loop iteration environment to avoid heap allocation.
    ///
    /// Recycling is only safe when all three conditions are met:
    /// 1. We are the sole owner (`strong_count == 1`) — no other code holds a reference.
    /// 2. No weak references exist (`weak_count == 0`) — closures capture environments via
    ///    `Weak` refs, so any weak ref means a closure may still need the environment's state.
    /// 3. The recycled environment's parent matches the expected `parent` — prevents scoping
    ///    bugs where an environment from a different scope is reused with the wrong parent chain.
    ///
    /// If any condition fails, a fresh child environment is allocated instead.
    fn get_recycled_env(
        &self,
        reusable_env: Option<Rc<RefCell<Environment>>>,
        parent: &Rc<RefCell<Environment>>,
    ) -> Rc<RefCell<Environment>> {
        if let Some(env) = reusable_env
            && Rc::strong_count(&env) == 1
            && Rc::weak_count(&env) == 0
        {
            // Validate parent matches to prevent scoping bugs
            let parent_matches = env
                .borrow()
                .parent
                .as_ref()
                .is_some_and(|p| p.ptr_eq(&Rc::downgrade(parent)));
            if parent_matches {
                env.borrow_mut().clear();
                return env;
            }
        }
        Environment::new_child_env(parent)
    }

    // Helper to evaluate literals directly without Box::pin allocation
    fn evaluate_literal_direct(
        &self,
        literal: &Literal,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        match literal {
            Literal::String(s) => Ok(Some(Value::Text(s.clone()))),
            Literal::Integer(i) => Ok(Some(Value::Number(*i as f64))),
            Literal::Float(f) => Ok(Some(Value::Number(*f))),
            Literal::Boolean(b) => Ok(Some(Value::Bool(*b))),
            Literal::Nothing => Ok(Some(Value::Null)),
            // Pattern literals might error, so we can handle them here
            Literal::Pattern(ir_string) => self
                .compile_pattern_literal(ir_string, env, line, column)
                .map(Some),
            Literal::List(elements) => {
                // First, pre-scan all elements to detect if any require async evaluation
                // This prevents double execution of side effects
                for element in elements {
                    if self.requires_async_evaluation(element, env) {
                        // At least one element requires async, abort sync optimization for the whole list
                        return Ok(None);
                    }
                }

                // All elements can be evaluated synchronously, proceed safely
                let mut list_values = Vec::with_capacity(elements.len());
                for element in elements {
                    // Since we've already verified all elements are sync-compatible,
                    // this should never return None, but handle it gracefully just in case
                    if let Some(value) = self.try_evaluate_simple_expr_sync(element, env)? {
                        list_values.push(value);
                    } else {
                        // This shouldn't happen after our pre-scan, but fall back to async
                        return Ok(None);
                    }
                }
                Ok(Some(Value::List(Rc::new(RefCell::new(list_values)))))
            }
        }
    }

    /// Compiles a pattern literal string into a Value::Pattern
    fn compile_pattern_literal(
        &self,
        ir_string: &str,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        let pattern_expr = crate::parser::ast::PatternExpression::Literal(ir_string.to_string());
        let compiled_pattern = {
            let env_borrow = env.borrow();
            CompiledPattern::compile_with_env(&pattern_expr, &env_borrow)
        };
        match compiled_pattern {
            Ok(compiled) => Ok(Value::Pattern(Rc::new(compiled))),
            Err(e) => Err(RuntimeError::new(
                format!("Failed to compile pattern literal: {}", e),
                line,
                column,
            )),
        }
    }

    /// Probes whether an expression requires async evaluation without executing it.
    ///
    /// Returns `true` if the expression must be evaluated asynchronously (e.g., it contains
    /// a zero-argument user-defined function that would trigger auto-call), or `false` if
    /// it can be safely evaluated on the synchronous fast-path. Used to pre-scan list
    /// elements so we can avoid double-execution of side effects.
    fn requires_async_evaluation(&self, expr: &Expression, env: &Rc<RefCell<Environment>>) -> bool {
        match expr {
            Expression::Literal(literal, _line, _column) => {
                match literal {
                    Literal::Pattern(_) => false, // Patterns don't require async
                    Literal::List(elements) => {
                        // Recursively check all elements
                        elements
                            .iter()
                            .any(|element| self.requires_async_evaluation(element, env))
                    }
                    _ => false, // Other literals are synchronous
                }
            }
            Expression::Variable(name, _line, _column) => {
                // Check if variable exists and if it would require async auto-call
                if let Ok(env_borrowed) = env.try_borrow() {
                    if let Some(value) = env_borrowed.get(name) {
                        match &value {
                            Value::Function(func) => func.params.is_empty(), // Zero-arg user functions auto-call (async)
                            Value::Overloaded(overloaded) => overloaded
                                .overloads
                                .iter()
                                .any(|func| func.params.is_empty()),
                            Value::NativeFunction(_, _) => false, // Native functions evaluate sync
                            _ => false,
                        }
                    } else {
                        false // Variable doesn't exist, will be sync error
                    }
                } else {
                    true // Can't borrow environment, conservatively assume async
                }
            }
            Expression::UnaryOperation { expression, .. } => {
                self.requires_async_evaluation(expression, env)
            }
            Expression::BinaryOperation { left, right, .. } => {
                self.requires_async_evaluation(left, env)
                    || self.requires_async_evaluation(right, env)
            }
            Expression::Concatenation { left, right, .. } => {
                self.requires_async_evaluation(left, env)
                    || self.requires_async_evaluation(right, env)
            }
            _ => true, // All other expressions require async (function calls, etc.)
        }
    }

    /// Handles the WFL auto-call convention for variables that resolve to functions.
    ///
    /// When a variable holds a zero-argument native function, it is invoked immediately
    /// and the result is returned. For zero-argument user-defined functions (which require
    /// async execution), returns `Ok(None)` to signal fallback to the async path.
    /// Non-function values and functions with parameters are returned as-is.
    fn handle_variable_auto_call(
        &self,
        value: Value,
        line: usize,
        column: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        match &value {
            Value::NativeFunction(func_name, native_fn) => {
                if get_function_arity(func_name) == 0 {
                    // Native functions are synchronous
                    native_fn(vec![])
                        .map(Some)
                        .map_err(|e| RuntimeError::new(format!("{}", e), line, column))
                } else {
                    Ok(Some(value))
                }
            }
            Value::Function(func) => {
                if func.params.is_empty() {
                    // User functions are async -> return None to signal fallback to async
                    Ok(None)
                } else {
                    Ok(Some(value))
                }
            }
            Value::Overloaded(overloaded) => {
                if overloaded
                    .overloads
                    .iter()
                    .any(|func| func.params.is_empty())
                {
                    // The zero-arg overload auto-calls (async path)
                    Ok(None)
                } else {
                    Ok(Some(value))
                }
            }
            _ => Ok(Some(value)),
        }
    }

    /// Attempts to evaluate an expression synchronously to avoid `Box::pin` allocation overhead.
    ///
    /// Handles literals, variables, and simple binary/unary operations recursively.
    /// Returns `Ok(Some(value))` when the expression was fully evaluated on the sync path,
    /// or `Ok(None)` when async evaluation is required (e.g., function calls, complex expressions).
    fn try_evaluate_simple_expr_sync(
        &self,
        expr: &Expression,
        env: &Rc<RefCell<Environment>>,
    ) -> Result<Option<Value>, RuntimeError> {
        match expr {
            Expression::Literal(literal, line, column) => {
                self.evaluate_literal_direct(literal, env, *line, *column)
            }
            Expression::Variable(name, line, column) => {
                self.try_evaluate_variable_sync(name, env, *line, *column)
            }
            Expression::UnaryOperation {
                operator,
                expression,
                line,
                column,
            } => {
                if let Some(val) = self.try_evaluate_simple_expr_sync(expression, env)? {
                    self.perform_unary_op(operator, val, *line, *column)
                        .map(Some)
                } else {
                    Ok(None)
                }
            }
            Expression::BinaryOperation {
                left,
                operator,
                right,
                line,
                column,
            } => {
                // Evaluate left side
                let left_val = match self.try_evaluate_simple_expr_sync(left, env)? {
                    Some(v) => v,
                    None => return Ok(None),
                };

                // Evaluate right side (WFL evaluates both eagerly currently)
                let right_val = match self.try_evaluate_simple_expr_sync(right, env)? {
                    Some(v) => v,
                    None => return Ok(None),
                };

                // Perform binary op
                self.perform_binary_op(operator, left_val, right_val, *line, *column)
                    .map(Some)
            }
            Expression::Concatenation {
                left,
                right,
                line: _line,
                column: _column,
            } => {
                // Evaluate left side
                let left_val = match self.try_evaluate_simple_expr_sync(left, env)? {
                    Some(v) => v,
                    None => return Ok(None),
                };

                // Evaluate right side
                let right_val = match self.try_evaluate_simple_expr_sync(right, env)? {
                    Some(v) => v,
                    None => return Ok(None),
                };

                // Concatenate
                Ok(Some(self.perform_concatenation(left_val, right_val)))
            }
            _ => Ok(None),
        }
    }

    /// Synchronously looks up a variable and handles auto-call for native functions.
    ///
    /// Returns `Ok(Some(value))` for regular values or zero-arg native function auto-calls,
    /// `Ok(None)` when a zero-arg user-defined function requires async execution, or
    /// `Err(...)` for runtime errors (e.g., undefined variable).
    fn try_evaluate_variable_sync(
        &self,
        name: &str,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        // Handle special count variable inside count loops
        if name == "count" && *self.in_count_loop.borrow() {
            if let Some(count_value) = *self.current_count.borrow() {
                return Ok(Some(Value::Number(count_value)));
            }
            return Err(RuntimeError::new(
                "Internal error: count variable accessed in count loop but no current count set"
                    .to_string(),
                line,
                column,
            ));
        }

        // Try normal variable lookup first
        if let Some(value) = env.borrow().get(name) {
            self.handle_variable_auto_call(value, line, column)
        } else if name == "count" {
            Err(RuntimeError::new(
                "Variable 'count' can only be used inside count loops. Use 'count from X to Y:' to create a count loop.".to_string(),
                line,
                column,
            ))
        } else {
            Err(RuntimeError::new(
                format!("Undefined variable '{name}'"),
                line,
                column,
            ))
        }
    }

    /// Run a database query/execute and return the result value. Shared by
    /// `DatabaseQueryStatement` and the expression form (`return query ...`).
    #[allow(clippy::too_many_arguments)]
    async fn evaluate_database_query(
        &self,
        db: &Expression,
        sql: &Expression,
        parameters: Option<&Expression>,
        kind: crate::parser::ast::DatabaseQueryKind,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let db_value = self.evaluate_expression(db, Rc::clone(&env)).await?;
        let handle = match &db_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected a database handle, got {db_value:?}"),
                    line,
                    column,
                ));
            }
        };

        let sql_value = self.evaluate_expression(sql, Rc::clone(&env)).await?;
        let sql_str = match &sql_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected text for SQL statement, got {sql_value:?}"),
                    line,
                    column,
                ));
            }
        };

        let params = match parameters {
            Some(params_expr) => {
                let params_value = self
                    .evaluate_expression(params_expr, Rc::clone(&env))
                    .await?;
                match &params_value {
                    Value::List(list) => {
                        let mut sql_params = Vec::new();
                        for value in list.borrow().iter() {
                            sql_params.push(
                                database::value_to_sql_param(value)
                                    .map_err(|e| RuntimeError::new(e, line, column))?,
                            );
                        }
                        sql_params
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected a list of query parameters, got {params_value:?}"),
                            line,
                            column,
                        ));
                    }
                }
            }
            None => Vec::new(),
        };

        // Routed through the IoClient so an open transaction on this handle
        // gets its own pinned connection rather than an arbitrary pooled one.
        match kind {
            crate::parser::ast::DatabaseQueryKind::Query => {
                self.io_client
                    .db_query(self.tx_scope.get(), &handle, &sql_str, &params)
                    .await
            }
            crate::parser::ast::DatabaseQueryKind::Execute => {
                self.io_client
                    .db_execute(self.tx_scope.get(), &handle, &sql_str, &params)
                    .await
            }
        }
        .map_err(|e| RuntimeError::new(e, line, column))
    }

    async fn evaluate_expression(
        &self,
        expr: &Expression,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        #[cfg(debug_assertions)]
        exec_trace!("Evaluating expression: {}", expr_type(expr));

        // OPTIMIZATION: Handle simple expressions synchronously to avoid Box::pin allocation
        // This recursively handles literals, variables, and simple math operations.
        // It significantly improves performance for tight loops with arithmetic.
        if let Some(value) = self.try_evaluate_simple_expr_sync(expr, &env)? {
            return Ok(value);
        }

        Box::pin(self._evaluate_expression(expr, env)).await
    }

    async fn _evaluate_expression(
        &self,
        expr: &Expression,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        self.assert_invariants();
        self.check_time()?;

        let result = match expr {
            // Container-related expressions
            &Expression::StaticMemberAccess {
                ref container,
                ref member,
                line,
                column,
            } => {
                // Look up the container definition
                let container_def = match env.borrow().get(container) {
                    Some(Value::ContainerDefinition(def)) => def.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Container '{container}' not found"),
                            line,
                            column,
                        ));
                    }
                };

                self.persist_active_static_method_context();

                // Look up the static member, following the same inheritance
                // chain the static checker accepts.
                if let Some(value) =
                    Self::resolve_static_property(&env, Rc::clone(&container_def), member)
                {
                    Ok(value)
                } else if let Some((method_owner, method)) =
                    Self::resolve_static_method(&env, Rc::clone(&container_def), member)
                {
                    Ok(Self::static_method_reference(&env, method_owner, method))
                } else {
                    Err(RuntimeError::new(
                        format!("Static member '{member}' not found in container '{container}'"),
                        line,
                        column,
                    ))
                }
            }

            &Expression::MethodCall {
                ref object,
                ref method,
                ref arguments,
                line,
                column,
            } => {
                // Evaluate the object
                let object_val = self.evaluate_expression(object, Rc::clone(&env)).await?;

                // Clone the object value to avoid borrow issues
                let object_val_clone = object_val.clone();

                // Static methods are called on the container definition value.
                if let Value::ContainerDefinition(container_def) = &object_val_clone {
                    self.persist_active_static_method_context();

                    let (method_owner, method_val) =
                        Self::resolve_static_method(&env, Rc::clone(container_def), method)
                            .ok_or_else(|| {
                                RuntimeError::new(
                                    format!(
                                        "Static method '{method}' not found in container '{}'",
                                        container_def.name
                                    ),
                                    line,
                                    column,
                                )
                            })?;

                    let Value::Function(function) =
                        Self::static_method_reference(&env, method_owner, method_val)
                    else {
                        unreachable!("static method references are always functions");
                    };

                    let mut argument_values = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        argument_values.push(
                            self.evaluate_expression(&argument.value, Rc::clone(&env))
                                .await?,
                        );
                    }

                    self.call_function(&function, argument_values, line, column)
                        .await
                } else if let Value::ContainerInstance(instance_rc) = &object_val_clone {
                    // Clone instance_rc for later property write-back
                    let instance_rc_for_writeback = instance_rc.clone();

                    let (container_type, property_names) = {
                        let instance = instance_rc.borrow();
                        let container_type = instance.container_type.clone();
                        let prop_names: Vec<String> = instance.properties.keys().cloned().collect();
                        (container_type, prop_names)
                    };

                    // Look up the container definition
                    let container_def = match env.borrow().get(&container_type) {
                        Some(Value::ContainerDefinition(def)) => def.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Container '{container_type}' not found"),
                                line,
                                column,
                            ));
                        }
                    };

                    // Look up the method (with inheritance support)
                    let mut found_method = container_def.methods.get(method).cloned();
                    let mut current_container_name = container_type.clone();

                    // If method not found, check parent containers
                    while found_method.is_none() {
                        if let Some(Value::ContainerDefinition(def)) =
                            env.borrow().get(&current_container_name)
                        {
                            if let Some(parent_name) = &def.extends {
                                current_container_name = parent_name.clone();
                                if let Some(Value::ContainerDefinition(parent_def)) =
                                    env.borrow().get(parent_name)
                                {
                                    found_method = parent_def.methods.get(method).cloned();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if let Some(method_val) = found_method {
                        // Create a function value from the method
                        let function = FunctionValue {
                            name: Some(method_val.name.clone()),
                            params: method_val.params.clone(),
                            param_types: vec![None; method_val.params.len()],
                            body: method_val.body.clone(),
                            env: method_val.env.clone(),
                            line: method_val.line,
                            column: method_val.column,
                            enforce_param_types: std::cell::Cell::new(false),
                            static_method_context: None,
                        };

                        // Create a new environment for the method execution
                        let method_env = Environment::new_child_env(&env);

                        // Add 'this' to the environment
                        let _ = method_env.borrow_mut().define("this", object_val.clone());

                        // Add container properties and events as accessible variables
                        {
                            let instance = instance_rc.borrow();

                            // Add properties
                            for (prop_name, prop_value) in &instance.properties {
                                let _ = method_env
                                    .borrow_mut()
                                    .define(prop_name, prop_value.clone());
                            }

                            // Add events from the container definition
                            if let Some(Value::ContainerDefinition(container_def_rc)) =
                                env.borrow().get(&instance.container_type)
                            {
                                let container_def = container_def_rc.clone();
                                for (event_name, event_value) in &container_def.events {
                                    let _ = method_env.borrow_mut().define(
                                        event_name,
                                        Value::ContainerEvent(Rc::new(event_value.clone())),
                                    );
                                }
                            }
                        } // Drop instance borrow here

                        // Evaluate the arguments
                        let mut arg_values = Vec::with_capacity(arguments.len());
                        for arg in arguments {
                            let arg_val = self
                                .evaluate_expression(&arg.value, Rc::clone(&env))
                                .await?;
                            arg_values.push(arg_val);
                        }

                        // Create a modified function with the method environment
                        let method_function = FunctionValue {
                            name: function.name.clone(),
                            params: function.params.clone(),
                            param_types: function.param_types.clone(),
                            body: function.body.clone(),
                            env: Rc::downgrade(&method_env),
                            line: function.line,
                            column: function.column,
                            enforce_param_types: function.enforce_param_types.clone(),
                            static_method_context: None,
                        };

                        // Call the function with the method environment
                        let result = self
                            .call_function(&method_function, arg_values, line, column)
                            .await;

                        // WRITE BACK MODIFIED PROPERTIES TO CONTAINER
                        // This fixes the property mutation issue where properties modified
                        // in container actions weren't persisting
                        for prop_name in property_names {
                            if let Some(updated_value) = method_env.borrow().get(&prop_name) {
                                instance_rc_for_writeback
                                    .borrow_mut()
                                    .properties
                                    .insert(prop_name, updated_value);
                            }
                        }

                        result
                    } else {
                        Err(RuntimeError::new(
                            format!("Method '{method}' not found in container '{container_type}'"),
                            line,
                            column,
                        ))
                    }
                } else {
                    Err(RuntimeError::new(
                        format!("Cannot call method '{method}' on non-container value"),
                        line,
                        column,
                    ))
                }
            }
            &Expression::AwaitExpression {
                ref expression,
                line: _line,
                column: _column,
            } => {
                let value = self
                    .evaluate_expression(expression, Rc::clone(&env))
                    .await?;
                Ok(value)
            }
            Expression::Literal(literal, _line, _column) => match literal {
                Literal::String(s) => Ok(Value::Text(s.clone())),
                Literal::Integer(i) => Ok(Value::Number(*i as f64)),
                Literal::Float(f) => Ok(Value::Number(*f)),
                Literal::Boolean(b) => Ok(Value::Bool(*b)),
                Literal::Nothing => Ok(Value::Null),
                Literal::Pattern(ir_string) => {
                    self.compile_pattern_literal(ir_string, &env, *_line, *_column)
                }
                Literal::List(elements) => {
                    let mut list_values = Vec::new();
                    for element in elements {
                        // Use Box::pin to handle recursion in async fn
                        let future = Box::pin(self._evaluate_expression(element, Rc::clone(&env)));
                        let value = future.await?;
                        list_values.push(value);
                    }
                    Ok(Value::List(Rc::new(RefCell::new(list_values))))
                }
            },

            Expression::Variable(name, line, column) => {
                // Handle special count variable inside count loops
                if name == "count" && *self.in_count_loop.borrow() {
                    if let Some(count_value) = *self.current_count.borrow() {
                        return Ok(Value::Number(count_value));
                    }
                    // If we're in a count loop but don't have a current count, this is an error
                    return Err(RuntimeError::new(
                        "Internal error: count variable accessed in count loop but no current count set".to_string(),
                        *line,
                        *column,
                    ));
                }

                // Try normal variable lookup first (allows user-defined 'count' variables outside loops)
                // Extract lookup result so the Ref<Environment> is dropped before call_function
                let lookup = env.borrow().get(name);
                if let Some(value) = lookup {
                    // Check if this is a zero-argument native function that should be auto-called
                    match &value {
                        Value::NativeFunction(func_name, native_fn) => {
                            if get_function_arity(func_name) == 0 {
                                // Auto-call zero-argument functions when referenced as variables
                                native_fn(vec![]).map_err(|e| {
                                    RuntimeError::new(
                                        format!("Error in native function '{}': {}", func_name, e),
                                        *line,
                                        *column,
                                    )
                                })
                            } else {
                                // Return function object for functions with arguments
                                Ok(value)
                            }
                        }
                        Value::Function(func) => {
                            if func.params.is_empty() {
                                // Auto-call zero-argument user-defined functions
                                self.call_function(func, vec![], *line, *column).await
                            } else {
                                // Return function object for functions with arguments
                                Ok(value)
                            }
                        }
                        Value::Overloaded(overloaded) => {
                            if let Some(func) = overloaded
                                .overloads
                                .iter()
                                .find(|func| func.params.is_empty())
                            {
                                // Auto-call the zero-argument overload
                                self.call_function(func, vec![], *line, *column).await
                            } else {
                                // Return the overload set for calls with arguments
                                Ok(value)
                            }
                        }
                        _ => Ok(value),
                    }
                } else if name == "count" {
                    // If 'count' is not found and we're not in a count loop, provide helpful error
                    Err(RuntimeError::new(
                        "Variable 'count' can only be used inside count loops. Use 'count from X to Y:' to create a count loop.".to_string(),
                        *line,
                        *column,
                    ))
                } else {
                    Err(RuntimeError::new(
                        format!("Undefined variable '{name}'"),
                        *line,
                        *column,
                    ))
                }
            }

            Expression::BinaryOperation {
                left,
                operator,
                right,
                line,
                column,
            } => {
                // Use Box::pin to handle recursion in async fn
                let left_future = Box::pin(self.evaluate_expression(left, Rc::clone(&env)));
                let left_val = left_future.await?;

                let right_val = self.evaluate_expression(right, Rc::clone(&env)).await?;

                self.perform_binary_op(operator, left_val, right_val, *line, *column)
            }

            Expression::UnaryOperation {
                operator,
                expression,
                line,
                column,
            } => {
                let value = self
                    .evaluate_expression(expression, Rc::clone(&env))
                    .await?;

                self.perform_unary_op(operator, value, *line, *column)
            }

            Expression::FunctionCall {
                function,
                arguments,
                line,
                column,
            } => {
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(
                        self.evaluate_expression(&arg.value, Rc::clone(&env))
                            .await?,
                    );
                }

                // Natural-language member access: `property of object`. WFL parses
                // `body of msg` as a call `body(msg)`; when the callee is a bare
                // name that is not a real function and the single argument is an
                // object carrying that key, resolve it as a property read. This is
                // how websocket handlers read `body of msg` / `id of conn` and how
                // HTTP handlers read `method of request` / `body of request`.
                if let Expression::Variable(name, _, _) = function.as_ref()
                    && arg_values.len() == 1
                    && let Value::Object(obj) = &arg_values[0]
                {
                    let field = obj.borrow().get(name.as_str()).cloned();
                    if let Some(value) = field {
                        let callee_is_function = matches!(
                            env.borrow().get(name),
                            Some(Value::Function(_))
                                | Some(Value::Overloaded(_))
                                | Some(Value::NativeFunction(_, _))
                        ) || crate::builtins::is_builtin_function(name);
                        if !callee_is_function {
                            return Ok(value);
                        }
                    }
                }

                // A bare-Variable callee that names an overload set must not
                // be evaluated through the Variable arm: that would auto-call
                // a zero-argument overload and then try to call its result.
                // The set itself is the call target (PR #639 review).
                let overloaded_callee = if let Expression::Variable(name, _, _) = function.as_ref()
                {
                    match env.borrow().get(name) {
                        Some(value @ Value::Overloaded(_)) => Some(value),
                        _ => None,
                    }
                } else {
                    None
                };
                let function_val = match overloaded_callee {
                    Some(value) => value,
                    None => self.evaluate_expression(function, Rc::clone(&env)).await?,
                };

                #[cfg(debug_assertions)]
                let func_name = match &function_val {
                    Value::Function(f) => {
                        f.name.clone().unwrap_or_else(|| "<anonymous>".to_string())
                    }
                    _ => format!("{function_val:?}"),
                };

                #[cfg(debug_assertions)]
                exec_function_call!(&func_name, &arg_values);

                let result = match function_val {
                    Value::Function(func) => {
                        self.call_function(&func, arg_values, *line, *column).await
                    }
                    Value::Overloaded(overloaded) => {
                        let func = Self::select_overload(&overloaded, &arg_values, *line, *column)?;
                        self.call_function(&func, arg_values, *line, *column).await
                    }
                    Value::NativeFunction(native_name, native_fn) => {
                        // CPU-heavy crypto builtins hop onto the blocking pool so
                        // they don't monopolize the interpreter thread (Phase 0,
                        // PR-0b). Everything else runs synchronously as before.
                        if let Some(fut) =
                            crate::stdlib::crypto_async::route(native_name, &arg_values)
                        {
                            fut.await.map_err(|e| {
                                RuntimeError::new(
                                    format!("Error in native function: {e}"),
                                    *line,
                                    *column,
                                )
                            })
                        } else {
                            native_fn(arg_values.clone()).map_err(|e| {
                                RuntimeError::new(
                                    format!("Error in native function: {e}"),
                                    *line,
                                    *column,
                                )
                            })
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot call {}", function_val.type_name()),
                        *line,
                        *column,
                    )),
                };

                #[cfg(debug_assertions)]
                if let Ok(ref val) = result {
                    exec_function_return!(&func_name, val);
                }

                result
            }

            Expression::ActionCall {
                name,
                arguments,
                line,
                column,
            } => {
                let function_val = env.borrow().get(name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined action '{name}'"), *line, *column)
                })?;

                match function_val {
                    Value::Overloaded(overloaded) => {
                        let mut arg_values = Vec::new();
                        for arg in arguments.iter() {
                            arg_values.push(
                                self.evaluate_expression(&arg.value, Rc::clone(&env))
                                    .await?,
                            );
                        }
                        let func = Self::select_overload(&overloaded, &arg_values, *line, *column)?;
                        self.call_function(&func, arg_values, *line, *column).await
                    }
                    Value::Function(func) => {
                        let mut arg_values = Vec::new();
                        for arg in arguments.iter() {
                            arg_values.push(
                                self.evaluate_expression(&arg.value, Rc::clone(&env))
                                    .await?,
                            );
                        }

                        #[cfg(debug_assertions)]
                        let func_name = func
                            .name
                            .clone()
                            .unwrap_or_else(|| "<anonymous>".to_string());

                        #[cfg(debug_assertions)]
                        exec_function_call!(&func_name, &arg_values);

                        let result = self.call_function(&func, arg_values, *line, *column).await;

                        #[cfg(debug_assertions)]
                        if let Ok(ref val) = result {
                            exec_function_return!(&func_name, val);
                        }

                        result
                    }
                    Value::NativeFunction(_, native_fn) => {
                        let mut arg_values = Vec::new();
                        for arg in arguments.iter() {
                            arg_values.push(
                                self.evaluate_expression(&arg.value, Rc::clone(&env))
                                    .await?,
                            );
                        }

                        // Preserve the native error's message and kind; only
                        // point the location at the call site (natives report
                        // their position as 0,0). CPU-heavy crypto builtins are
                        // routed onto the blocking pool (Phase 0, PR-0b).
                        if let Some(fut) = crate::stdlib::crypto_async::route(name, &arg_values) {
                            fut.await.map_err(|mut e| {
                                e.line = *line;
                                e.column = *column;
                                e
                            })
                        } else {
                            native_fn(arg_values).map_err(|mut e| {
                                e.line = *line;
                                e.column = *column;
                                e
                            })
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("'{name}' is not callable"),
                        *line,
                        *column,
                    )),
                }
            }

            Expression::MemberAccess {
                object,
                property,
                line,
                column,
            } => {
                let object_val = self.evaluate_expression(object, Rc::clone(&env)).await?;

                match object_val {
                    Value::Object(obj_rc) => {
                        let obj = obj_rc.borrow();
                        if let Some(value) = obj.get(property) {
                            Ok(value.clone())
                        } else {
                            Err(RuntimeError::new(
                                format!("Object has no property '{property}'"),
                                *line,
                                *column,
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot access property of {}", object_val.type_name()),
                        *line,
                        *column,
                    )),
                }
            }

            Expression::IndexAccess {
                collection,
                index,
                line,
                column,
            } => {
                let collection_val = self
                    .evaluate_expression(collection, Rc::clone(&env))
                    .await?;
                let index_val = self.evaluate_expression(index, Rc::clone(&env)).await?;

                match (collection_val, index_val) {
                    (Value::List(list_rc), Value::Number(idx)) => {
                        let list = list_rc.borrow();
                        let idx = idx as usize;

                        if idx < list.len() {
                            Ok(list[idx].clone())
                        } else {
                            Err(RuntimeError::new(
                                format!(
                                    "Index {} out of bounds for list of length {}",
                                    idx,
                                    list.len()
                                ),
                                *line,
                                *column,
                            ))
                        }
                    }
                    (Value::Object(obj_rc), Value::Text(key)) => {
                        let obj = obj_rc.borrow();
                        let key_str = key.to_string();

                        if let Some(value) = obj.get(&key_str) {
                            Ok(value.clone())
                        } else {
                            Err(RuntimeError::new(
                                format!("Object has no key '{key_str}'"),
                                *line,
                                *column,
                            ))
                        }
                    }
                    (collection, index) => Err(RuntimeError::new(
                        format!(
                            "Cannot index {} with {}",
                            collection.type_name(),
                            index.type_name()
                        ),
                        *line,
                        *column,
                    )),
                }
            }

            Expression::Concatenation {
                left,
                right,
                line: _line,
                column: _column,
            } => {
                // Use Box::pin to handle recursion in async fn
                let left_future = Box::pin(self.evaluate_expression(left, Rc::clone(&env)));
                let left_val = left_future.await?;

                let right_val = self.evaluate_expression(right, Rc::clone(&env)).await?;

                Ok(self.perform_concatenation(left_val, right_val))
            }

            Expression::PatternMatch {
                text,
                pattern,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

                // Extract text string
                let text_str = match &text_val {
                    Value::Text(s) => s.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern match requires text as first argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Extract compiled pattern
                let compiled_pattern = match &pattern_val {
                    Value::Pattern(p) => p,
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern match requires pattern as second argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Perform the match under the shared budget so a pathological
                // pattern is bounded by the run's step/state ceilings; a breach
                // surfaces as a catchable error, not a silent non-match.
                let matches = compiled_pattern
                    .matches_with_budget(text_str, &self.budget)
                    .map_err(|e| self.pattern_error(e, *_line, *_column))?;
                Ok(Value::Bool(matches))
            }

            Expression::PatternFind {
                text,
                pattern,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

                // Extract text string
                let text_str = match &text_val {
                    Value::Text(s) => s.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern find requires text as first argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Extract compiled pattern
                let compiled_pattern = match &pattern_val {
                    Value::Pattern(p) => p,
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern find requires pattern as second argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Find the first match under the shared budget (a breach is a
                // catchable error rather than a silent non-match).
                let found = compiled_pattern
                    .find_with_budget(text_str, &self.budget)
                    .map_err(|e| self.pattern_error(e, *_line, *_column))?;
                match found {
                    Some(match_result) => {
                        // Return an object with match information
                        let mut result_map = std::collections::HashMap::new();
                        result_map.insert(
                            "matched_text".to_string(),
                            Value::Text(Arc::from(match_result.matched_text.as_str())),
                        );
                        result_map.insert(
                            "start".to_string(),
                            Value::Number(match_result.start as f64),
                        );
                        result_map
                            .insert("end".to_string(), Value::Number(match_result.end as f64));

                        // Add captures if any
                        if !match_result.captures.is_empty() {
                            let mut captures_map = std::collections::HashMap::new();
                            for (name, value) in match_result.captures {
                                captures_map.insert(name, Value::Text(Arc::from(value.as_str())));
                            }
                            result_map.insert(
                                "captures".to_string(),
                                Value::Object(Rc::new(RefCell::new(captures_map))),
                            );
                        }

                        Ok(Value::Object(Rc::new(RefCell::new(result_map))))
                    }
                    None => Ok(Value::Null),
                }
            }

            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;
                let replacement_val = self
                    .evaluate_expression(replacement, Rc::clone(&env))
                    .await?;

                let args = vec![text_val, pattern_val, replacement_val]; // Note: text, pattern, then replacement
                crate::stdlib::pattern::native_pattern_replace(args, *_line, *_column)
            }

            Expression::PatternSplit {
                text,
                pattern,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

                let args = vec![text_val, pattern_val];
                crate::stdlib::pattern::native_pattern_split(args, *_line, *_column)
            }
            Expression::StringSplit {
                text,
                delimiter,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let delimiter_val = self.evaluate_expression(delimiter, Rc::clone(&env)).await?;

                // Validate types
                if !matches!(text_val, Value::Text(_)) {
                    return Err(RuntimeError::new(
                        format!("Cannot split {} - expected text", text_val.type_name()),
                        *_line,
                        *_column,
                    ));
                }
                if !matches!(delimiter_val, Value::Text(_)) {
                    return Err(RuntimeError::new(
                        format!("Delimiter must be text - got {}", delimiter_val.type_name()),
                        *_line,
                        *_column,
                    ));
                }

                let args = vec![text_val, delimiter_val];
                crate::stdlib::text::native_string_split(args)
            }
            Expression::PropertyAccess {
                object,
                property,
                line,
                column,
            } => {
                let obj_value = self.evaluate_expression(object, Rc::clone(&env)).await?;
                match obj_value {
                    Value::ContainerInstance(instance) => {
                        let instance_ref = instance.borrow();
                        if let Some(prop_value) = instance_ref.properties.get(property) {
                            Ok(prop_value.clone())
                        } else {
                            Err(RuntimeError::new(
                                format!("Property '{property}' not found"),
                                *line,
                                *column,
                            ))
                        }
                    }
                    Value::ContainerDefinition(definition) => {
                        self.persist_active_static_method_context();

                        if let Some(value) =
                            Self::resolve_static_property(&env, Rc::clone(&definition), property)
                        {
                            Ok(value)
                        } else if let Some((method_owner, method)) =
                            Self::resolve_static_method(&env, Rc::clone(&definition), property)
                        {
                            Ok(Self::static_method_reference(&env, method_owner, method))
                        } else {
                            Err(RuntimeError::new(
                                format!(
                                    "Static member '{property}' not found in container '{}'",
                                    definition.name
                                ),
                                *line,
                                *column,
                            ))
                        }
                    }
                    Value::Object(obj_rc) => {
                        let obj = obj_rc.borrow();
                        if let Some(prop_value) = obj.get(property) {
                            Ok(prop_value.clone())
                        } else {
                            Err(RuntimeError::new(
                                format!("Object has no property '{property}'"),
                                *line,
                                *column,
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot access property '{property}' on non-container value"),
                        *line,
                        *column,
                    )),
                }
            }
            Expression::FileExists { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                Ok(Value::Bool(tokio::fs::metadata(&*path_str).await.is_ok()))
            }
            Expression::DirectoryExists { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match tokio::fs::metadata(&*path_str).await {
                    Ok(metadata) => Ok(Value::Bool(metadata.is_dir())),
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            Expression::ListFiles { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match tokio::fs::read_dir(&*path_str).await {
                    Ok(mut entries) => {
                        let mut files = Vec::new();
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            if let Ok(file_name) = entry.file_name().into_string() {
                                files.push(Value::Text(file_name.into()));
                            }
                        }
                        Ok(Value::List(Rc::new(RefCell::new(files))))
                    }
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to list files in directory: {e}"),
                        *line,
                        *column,
                    )),
                }
            }
            Expression::ReadContent {
                file_handle,
                line,
                column,
            } => {
                let handle_value = self
                    .evaluate_expression(file_handle, Rc::clone(&env))
                    .await?;
                let handle_str = match &handle_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {handle_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.read_file(&handle_str, &self.budget).await {
                    Ok(content) => Ok(Value::Text(content.into())),
                    Err(e) => Err(self.file_read_error(e, *line, *column)),
                }
            }
            Expression::ReadBinaryContent {
                file_handle,
                line,
                column,
            } => {
                let handle_value = self
                    .evaluate_expression(file_handle, Rc::clone(&env))
                    .await?;
                let handle_str = match &handle_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected string for file handle, got {}",
                                handle_value.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.read_binary(&handle_str, &self.budget).await {
                    Ok(bytes) => Ok(Value::Binary(Arc::from(bytes))),
                    Err(e) => Err(self.file_read_error(e, *line, *column)),
                }
            }
            Expression::ReadBinaryN {
                file_handle,
                count,
                line,
                column,
            } => {
                let handle_value = self
                    .evaluate_expression(file_handle, Rc::clone(&env))
                    .await?;
                let handle_str = match &handle_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected string for file handle, got {}",
                                handle_value.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                let count_value = self.evaluate_expression(count, Rc::clone(&env)).await?;
                let n = match &count_value {
                    Value::Number(n) => {
                        if !n.is_finite() || n.fract() != 0.0 || *n < 0.0 || *n > usize::MAX as f64
                        {
                            return Err(RuntimeError::new(
                                format!("Invalid byte count: {n} — must be a non-negative integer"),
                                *line,
                                *column,
                            ));
                        }
                        *n as usize
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected number for byte count, got {}",
                                count_value.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                match self
                    .io_client
                    .read_binary_n(&handle_str, n, &self.budget)
                    .await
                {
                    Ok(bytes) => Ok(Value::Binary(Arc::from(bytes))),
                    Err(e) => Err(self.file_read_error(e, *line, *column)),
                }
            }
            Expression::FileSizeOf {
                file_handle,
                line,
                column,
            } => {
                let handle_value = self
                    .evaluate_expression(file_handle, Rc::clone(&env))
                    .await?;
                let handle_str = match &handle_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected string for file handle, got {}",
                                handle_value.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.file_size(&handle_str).await {
                    Ok(size) => Ok(Value::Number(size as f64)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Expression::ListFilesRecursive {
                path,
                extensions,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate extensions if provided
                let ext_filters = if let Some(ext_exprs) = extensions {
                    let mut filters = Vec::new();
                    for ext_expr in ext_exprs {
                        let ext_value = self.evaluate_expression(ext_expr, Rc::clone(&env)).await?;
                        match &ext_value {
                            Value::Text(s) => filters.push(s.to_string()),
                            Value::List(list) => {
                                // If we get a list, extract all string values from it
                                let list_ref = list.borrow();
                                for item in list_ref.iter() {
                                    match item {
                                        Value::Text(s) => filters.push(s.to_string()),
                                        _ => {
                                            return Err(RuntimeError::new(
                                                format!(
                                                    "Expected string in extension list, got {item:?}"
                                                ),
                                                *line,
                                                *column,
                                            ));
                                        }
                                    }
                                }
                            }
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected string or list for extension, got {ext_value:?}"
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    Some(filters)
                } else {
                    None
                };

                // Perform recursive directory listing
                match self.list_files_recursive(&path_str, ext_filters).await {
                    Ok(files) => Ok(Value::List(Rc::new(RefCell::new(files)))),
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to list files recursively: {e}"),
                        *line,
                        *column,
                    )),
                }
            }
            Expression::ListFilesFiltered {
                path,
                extensions,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate extensions
                let mut ext_filters = Vec::new();
                for ext_expr in extensions {
                    let ext_value = self.evaluate_expression(ext_expr, Rc::clone(&env)).await?;
                    match &ext_value {
                        Value::Text(s) => ext_filters.push(s.to_string()),
                        Value::List(list) => {
                            // If we get a list, extract all string values from it
                            let list_ref = list.borrow();
                            for item in list_ref.iter() {
                                match item {
                                    Value::Text(s) => ext_filters.push(s.to_string()),
                                    _ => {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Expected string in extension list, got {item:?}"
                                            ),
                                            *line,
                                            *column,
                                        ));
                                    }
                                }
                            }
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Expected string or list for extension, got {ext_value:?}"),
                                *line,
                                *column,
                            ));
                        }
                    }
                }

                // List files with filtering
                match self.list_files_filtered(&path_str, ext_filters).await {
                    Ok(files) => Ok(Value::List(Rc::new(RefCell::new(files)))),
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to list files with filter: {e}"),
                        *line,
                        *column,
                    )),
                }
            }
            Expression::HeaderAccess {
                header_name,
                request,
                line,
                column,
            } => {
                // Resolve headers from the request object first so
                // `header "X" of req` works inside actions that receive
                // `req` as a parameter (issue #597). Fall back to the
                // loop-scoped `headers` binding for backward compatibility
                // when the request expression is not a request object.
                let headers_val = {
                    let request_val = self.evaluate_expression(request, Rc::clone(&env)).await?;
                    match &request_val {
                        Value::Object(obj) => {
                            let map = obj.borrow();
                            if let Some(headers) = map.get("headers") {
                                headers.clone()
                            } else {
                                // Not a request object — try env fallback
                                match env.borrow().get("headers") {
                                    Some(val) => val.clone(),
                                    None => {
                                        return Err(RuntimeError::new(
                                            "Cannot access headers: no request in scope. Use 'wait for request comes in' first, or pass a request object to the action.".to_string(),
                                            *line,
                                            *column,
                                        ));
                                    }
                                }
                            }
                        }
                        _ => match env.borrow().get("headers") {
                            Some(val) => val.clone(),
                            None => {
                                return Err(RuntimeError::new(
                                    "Cannot access headers: no request in scope. Use 'wait for request comes in' first, or pass a request object to the action.".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        },
                    }
                };

                // Get the specific header from the headers object.
                match &headers_val {
                    Value::Object(headers_map) => {
                        let map = headers_map.borrow();
                        match lookup_header_case_insensitive(&map, header_name) {
                            Some(header_value) => Ok(header_value),
                            // Value::Null is the runtime value of WFL's
                            // `nothing` literal
                            None => Ok(Value::Null),
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            format!(
                                "Expected headers to be an object, got {}",
                                headers_val.type_name()
                            ),
                            *line,
                            *column,
                        ));
                    }
                }
            }
            Expression::CurrentTimeMilliseconds { line: _, column: _ } => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
                    RuntimeError::new(format!("Failed to get current time: {}", e), 0, 0)
                })?;
                Ok(Value::Number(now.as_millis() as f64))
            }
            Expression::CurrentTimeFormatted {
                format,
                line: _,
                column: _,
            } => {
                use chrono::{DateTime, Local};
                let now: DateTime<Local> = Local::now();

                // Convert WFL format to chrono format
                // For now, support basic formats
                let chrono_format = format
                    .replace("yyyy", "%Y")
                    .replace("MM", "%m")
                    .replace("dd", "%d")
                    .replace("HH", "%H")
                    .replace("mm", "%M")
                    .replace("ss", "%S");

                let formatted = now.format(&chrono_format).to_string();
                Ok(Value::Text(Arc::from(formatted)))
            }
            Expression::ProcessRunning {
                process_id,
                line,
                column,
            } => {
                // Evaluate process ID expression
                let proc_val = self
                    .evaluate_expression(process_id, Rc::clone(&env))
                    .await?;
                let proc_id = match &proc_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Process ID must be text, got {}", proc_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                };

                // Check if process is running
                let is_running = self.io_client.is_process_running(proc_id).await;
                Ok(Value::Bool(is_running))
            }
            Expression::DatabaseQuery {
                db,
                sql,
                parameters,
                kind,
                line,
                column,
            } => {
                self.evaluate_database_query(
                    db,
                    sql,
                    parameters.as_deref(),
                    *kind,
                    *line,
                    *column,
                    Rc::clone(&env),
                )
                .await
            }
            Expression::CreateSession {
                request,
                line,
                column,
            } => {
                let request_val = self.evaluate_expression(request, Rc::clone(&env)).await?;
                let (manager, server_name) = self
                    .session_manager_from_request(&request_val, *line, *column)
                    .await?;
                let record = manager
                    .create()
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                Ok(sessions::SessionManager::session_object(
                    &record,
                    &server_name,
                ))
            }
            Expression::GetSession {
                request,
                line,
                column,
            } => {
                let request_val = self.evaluate_expression(request, Rc::clone(&env)).await?;
                let (manager, server_name) = self
                    .session_manager_from_request(&request_val, *line, *column)
                    .await?;
                let cfg = manager.config_snapshot().await;
                let sid = match cookie_value_from_request(&request_val, &cfg.cookie_name) {
                    Some(id) => id,
                    None => return Ok(Value::Nothing),
                };
                match manager
                    .get(&sid)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?
                {
                    Some(record) => Ok(sessions::SessionManager::session_object(
                        &record,
                        &server_name,
                    )),
                    None => Ok(Value::Nothing),
                }
            }
            Expression::GetSessionValue {
                key,
                session,
                line,
                column,
            } => {
                let session_val = self.evaluate_expression(session, Rc::clone(&env)).await?;
                let key_val = self.evaluate_expression(key, Rc::clone(&env)).await?;
                let key_text = match &key_val {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Session value key must be text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };
                let (manager, id) = self
                    .session_manager_from_session_object(&session_val, *line, *column)
                    .await?;
                match manager
                    .get(&id)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?
                {
                    Some(record) => Ok(record
                        .data
                        .get(&key_text)
                        .cloned()
                        .unwrap_or(Value::Nothing)),
                    None => Ok(Value::Nothing),
                }
            }
            Expression::GenerateCsrfTokenForSession {
                session,
                line,
                column,
            } => {
                let session_val = self.evaluate_expression(session, Rc::clone(&env)).await?;
                let (manager, id) = self
                    .session_manager_from_session_object(&session_val, *line, *column)
                    .await?;
                let token = sessions::SessionManager::generate_csrf_hex();
                manager
                    .set_value(&id, "csrf_token", Value::Text(Arc::from(token.as_str())))
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                Ok(Value::Text(Arc::from(token.as_str())))
            }
            Expression::FindExpiredSessions {
                server,
                line,
                column,
            } => {
                let manager = self
                    .session_manager_from_server_expr(server, Rc::clone(&env), *line, *column)
                    .await?;
                let server_name = self
                    .session_server_name_from_expr(server, Rc::clone(&env), *line, *column)
                    .await?;
                let expired = manager
                    .find_expired()
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;
                let values = expired
                    .iter()
                    .map(|record| sessions::SessionManager::session_object(record, &server_name))
                    .collect();
                Ok(Value::List(Rc::new(RefCell::new(values))))
            }
            Expression::GetSessionStatistics {
                server,
                line,
                column,
            } => {
                let manager = self
                    .session_manager_from_server_expr(server, Rc::clone(&env), *line, *column)
                    .await?;
                let stats = manager.stats().await;
                Ok(sessions::SessionManager::stats_object(&stats))
            }
            Expression::LoadSessionData { key, line, column } => {
                let manager = self.sole_or_named_session_manager(*line, *column).await?;
                let key_val = self.evaluate_expression(key, Rc::clone(&env)).await?;
                let key_text = match &key_val {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Session storage key must be text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };
                match manager
                    .load_kv(&key_text)
                    .await
                    .map_err(|e| RuntimeError::new(e, *line, *column))?
                {
                    Some(value) => Ok(value),
                    None => Ok(Value::Nothing),
                }
            }
        };
        self.assert_invariants();
        result
    }

    /// Picks the overload whose parameter count and declared parameter types
    /// match the actual argument values: filter by count, drop candidates
    /// whose concrete annotations reject an argument, then prefer the
    /// candidate with the most concretely-matched parameters. When that count
    /// ties, an exact branded temporal annotation (`date`, `time`, or
    /// `datetime`) outranks its broader historical custom-name annotation;
    /// remaining ties resolve to definition order.
    fn select_overload(
        overloaded: &OverloadedFunction,
        args: &[Value],
        line: usize,
        column: usize,
    ) -> Result<Rc<FunctionValue>, RuntimeError> {
        let arity_matches: Vec<&Rc<FunctionValue>> = overloaded
            .overloads
            .iter()
            .filter(|func| func.params.len() == args.len())
            .collect();

        if arity_matches.is_empty() {
            let mut arities: Vec<usize> = overloaded
                .overloads
                .iter()
                .map(|func| func.params.len())
                .collect();
            arities.sort_unstable();
            arities.dedup();
            let arities_str = arities
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(" or ");
            return Err(RuntimeError::new(
                format!(
                    "No version of '{}' takes {} argument(s). It is defined with {} parameter(s).",
                    overloaded.name,
                    args.len(),
                    arities_str
                ),
                line,
                column,
            ));
        }

        let mut best: Option<(&Rc<FunctionValue>, (usize, usize))> = None;
        for func in &arity_matches {
            let mut concrete_matches = 0usize;
            let mut exact_temporal_matches = 0usize;
            let mut accepts = true;
            for (param_type, arg) in func.param_types.iter().zip(args) {
                if let Some(expected) = param_type {
                    // `any`/`Unknown` annotations accept everything and earn
                    // no specificity credit, matching untyped parameters.
                    if matches!(expected, Type::Any | Type::Unknown) {
                        continue;
                    }
                    if Self::value_matches_type(arg, expected) {
                        // A `nothing` argument is accepted by every parameter
                        // type but earns specificity credit only for an
                        // explicit `as nothing` parameter (its exact match) —
                        // otherwise versions accepting `nothing` stay tied
                        // and definition order decides, as documented.
                        if !matches!(arg, Value::Null | Value::Nothing)
                            || matches!(expected, Type::Nothing)
                        {
                            concrete_matches += 1;
                            if matches!(
                                (expected, arg),
                                (Type::Date, Value::Date(_))
                                    | (Type::Time, Value::Time(_))
                                    | (Type::DateTime, Value::DateTime(_))
                            ) {
                                exact_temporal_matches += 1;
                            }
                        }
                    } else {
                        accepts = false;
                        break;
                    }
                }
            }
            let specificity = (concrete_matches, exact_temporal_matches);
            if accepts && best.is_none_or(|(_, score)| specificity > score) {
                best = Some((func, specificity));
            }
        }

        match best {
            Some((func, _)) => Ok(Rc::clone(func)),
            None => {
                let provided: Vec<&str> = args.iter().map(|arg| arg.type_name()).collect();
                let mut message = format!(
                    "No version of '{}' matches this call.\nYou provided ({}), but '{}' accepts:",
                    overloaded.name,
                    provided.join(", "),
                    overloaded.name
                );
                for func in &arity_matches {
                    message.push_str(&format!("\n  {}", Self::format_overload_signature(func)));
                }
                Err(RuntimeError::new(message, line, column))
            }
        }
    }

    fn definition_shadows_container_property(
        &self,
        env: &Rc<RefCell<Environment>>,
        name: &str,
    ) -> bool {
        let Some(defining_scope) = env.borrow().parent_scope_defining(name) else {
            return false;
        };

        let this_value = defining_scope.borrow().values.get("this").cloned();
        if let Some(Value::ContainerInstance(instance)) = this_value
            && instance.borrow().properties.contains_key(name)
        {
            return true;
        }

        self.active_static_method_contexts
            .borrow()
            .last()
            .is_some_and(|context| {
                Rc::ptr_eq(&context.env, &defining_scope)
                    && context.property_owners.contains_key(name)
            })
    }

    /// Validate that a container provides every action required by the
    /// interfaces it claims to implement. Requirements accumulate through
    /// interface `extends` chains, and a requirement may be satisfied by a
    /// method inherited through the container's own `extends` chain. An
    /// interface with no body (`create interface Name`) is an empty contract
    /// that every container satisfies.
    fn validate_interface_conformance(
        env: &Rc<RefCell<Environment>>,
        container_name: &str,
        container_extends: Option<&String>,
        container_methods: &HashMap<String, ContainerMethodValue>,
        implements: &[String],
        line: usize,
        column: usize,
    ) -> Result<(), RuntimeError> {
        // Resolve an instance method's parameter count: the container's own
        // methods first, then the parent chain (bounded against definition
        // cycles, which the environment cannot otherwise rule out).
        let find_method = |method_name: &str| -> Option<(usize, Option<Type>)> {
            if let Some(method) = container_methods.get(method_name) {
                return Some((method.params.len(), method.return_type.clone()));
            }
            let mut parent_name = container_extends.cloned();
            let mut visited_parents = HashSet::new();
            while let Some(name) = parent_name {
                if !visited_parents.insert(name.clone()) {
                    return None;
                }
                let parent = match env.borrow().get(&name) {
                    Some(Value::ContainerDefinition(definition)) => definition,
                    _ => return None,
                };
                if let Some(method) = parent.methods.get(method_name) {
                    return Some((method.params.len(), method.return_type.clone()));
                }
                parent_name = parent.extends.clone();
            }
            None
        };

        for interface_name in implements {
            // (name, reached-through-extends): parent interfaces are named in
            // errors together with the interface the container actually
            // implements, so the message doesn't claim a direct 'implements'.
            let mut pending = vec![(interface_name.clone(), false)];
            let mut visited = HashSet::new();
            while let Some((current_name, inherited)) = pending.pop() {
                if !visited.insert(current_name.clone()) {
                    continue;
                }
                let via = if inherited {
                    format!(" (required through interface '{interface_name}')")
                } else {
                    String::new()
                };
                let interface = match env.borrow().get(&current_name) {
                    Some(Value::InterfaceDefinition(definition)) => definition,
                    Some(_) => {
                        return Err(RuntimeError::new(
                            format!(
                                "Container '{container_name}' requires '{current_name}'{via}, but '{current_name}' is not an interface"
                            ),
                            line,
                            column,
                        ));
                    }
                    None => {
                        return Err(RuntimeError::new(
                            format!(
                                "Interface '{current_name}' not found for container '{container_name}'{via}"
                            ),
                            line,
                            column,
                        ));
                    }
                };

                for parent in &interface.extends {
                    pending.push((parent.clone(), true));
                }

                for (action_name, signature) in &interface.required_actions {
                    match find_method(action_name) {
                        None => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Container '{container_name}' does not satisfy interface '{}': missing required action '{action_name}'",
                                    interface.name
                                ),
                                line,
                                column,
                            ));
                        }
                        Some((count, _)) if count != signature.params.len() => {
                            return Err(RuntimeError::new(
                                format!(
                                    "Container '{container_name}' does not satisfy interface '{}': action '{action_name}' takes {count} parameter(s) but the interface requires {}",
                                    interface.name,
                                    signature.params.len()
                                ),
                                line,
                                column,
                            ));
                        }
                        Some((_, return_type)) => {
                            if let (Some(required_return), Some(actual_return)) =
                                (&signature.return_type, &return_type)
                                && !matches!(required_return, Type::Unknown | Type::Any)
                                && !matches!(actual_return, Type::Unknown | Type::Any)
                                && required_return != actual_return
                            {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Container '{container_name}' does not satisfy interface '{}': action '{action_name}' returns {actual_return} but the interface requires {required_return}",
                                        interface.name
                                    ),
                                    line,
                                    column,
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_static_property(
        env: &Rc<RefCell<Environment>>,
        mut definition: Rc<ContainerDefinitionValue>,
        member: &str,
    ) -> Option<Value> {
        loop {
            let value = definition.static_properties.borrow().get(member).cloned();
            if value.is_some() {
                return value;
            }
            let parent_name = definition.extends.as_ref()?.clone();
            definition = match env.borrow().get(&parent_name) {
                Some(Value::ContainerDefinition(parent)) => parent,
                _ => return None,
            };
        }
    }

    fn resolve_static_method(
        env: &Rc<RefCell<Environment>>,
        mut definition: Rc<ContainerDefinitionValue>,
        member: &str,
    ) -> Option<(Rc<ContainerDefinitionValue>, ContainerMethodValue)> {
        loop {
            if let Some(method) = definition.static_methods.get(member).cloned() {
                return Some((definition, method));
            }
            let parent_name = definition.extends.as_ref()?.clone();
            definition = match env.borrow().get(&parent_name) {
                Some(Value::ContainerDefinition(parent)) => parent,
                _ => return None,
            };
        }
    }

    fn static_definition_chain(
        env: &Rc<RefCell<Environment>>,
        mut definition: Rc<ContainerDefinitionValue>,
    ) -> Vec<Rc<ContainerDefinitionValue>> {
        let mut chain = vec![Rc::clone(&definition)];
        while let Some(parent_name) = definition.extends.as_ref() {
            let Some(Value::ContainerDefinition(parent)) = env.borrow().get(parent_name) else {
                break;
            };
            definition = parent;
            chain.push(Rc::clone(&definition));
        }
        chain.reverse();
        chain
    }

    fn static_method_reference(
        env: &Rc<RefCell<Environment>>,
        method_owner: Rc<ContainerDefinitionValue>,
        method: ContainerMethodValue,
    ) -> Value {
        let static_env = Environment::new_child_env(env);
        let mut property_owners = HashMap::new();
        for definition in Self::static_definition_chain(env, method_owner) {
            for (property_name, property_value) in definition.static_properties.borrow().iter() {
                static_env
                    .borrow_mut()
                    .define_or_replace(property_name, property_value.clone());
                property_owners.insert(property_name.clone(), Rc::clone(&definition));
            }
        }
        let static_method_context = Rc::new(StaticMethodContext {
            env: Rc::clone(&static_env),
            property_owners,
        });
        Value::Function(Rc::new(FunctionValue {
            name: Some(method.name.clone()),
            params: method.params.clone(),
            param_types: vec![None; method.params.len()],
            body: method.body,
            env: Rc::downgrade(&static_env),
            line: method.line,
            column: method.column,
            enforce_param_types: std::cell::Cell::new(false),
            static_method_context: Some(static_method_context),
        }))
    }

    fn refresh_static_method_context(context: &StaticMethodContext) {
        for (property_name, owner) in &context.property_owners {
            let current_value = owner.static_properties.borrow().get(property_name).cloned();
            if let Some(current_value) = current_value {
                context
                    .env
                    .borrow_mut()
                    .define_or_replace(property_name, current_value);
            }
        }
    }

    fn persist_static_method_context(context: &StaticMethodContext) {
        for (property_name, owner) in &context.property_owners {
            if let Some(updated_value) = context.env.borrow().get_local(property_name) {
                owner
                    .static_properties
                    .borrow_mut()
                    .insert(property_name.clone(), updated_value);
            }
        }
    }

    fn persist_active_static_method_context(&self) {
        let active_context = self.active_static_method_contexts.borrow().last().cloned();
        if let Some(active_context) = active_context {
            Self::persist_static_method_context(&active_context);
        }
    }

    fn refresh_active_static_method_context(&self) {
        let active_context = self.active_static_method_contexts.borrow().last().cloned();
        if let Some(active_context) = active_context {
            Self::refresh_static_method_context(&active_context);
        }
    }

    /// Whether a runtime value satisfies a declared parameter type. Untyped
    /// and unknown annotations accept everything; `Custom` types match a
    /// container instance of that type or of a descendant (via the parent
    /// instance chain).
    fn value_matches_type(value: &Value, expected: &Type) -> bool {
        // `nothing` is compatible with every parameter type, mirroring the
        // static checkers' `(_, Type::Nothing) => true` rule; ties among
        // overloads resolve by specificity then definition order.
        if matches!(value, Value::Null | Value::Nothing) {
            return true;
        }
        match expected {
            Type::Number => matches!(value, Value::Number(_)),
            Type::Text => matches!(value, Value::Text(_)),
            Type::Boolean => matches!(value, Value::Bool(_)),
            Type::Nothing => matches!(value, Value::Null | Value::Nothing),
            Type::Pattern => matches!(value, Value::Pattern(_)),
            Type::Date => matches!(value, Value::Date(_)),
            Type::Time => matches!(value, Value::Time(_)),
            Type::DateTime => matches!(value, Value::DateTime(_)),
            Type::List(_) => matches!(value, Value::List(_)),
            Type::Map(_, _) => matches!(value, Value::Object(_)),
            Type::Custom(name) => {
                if name.eq_ignore_ascii_case("any") {
                    return true;
                }
                if name.eq_ignore_ascii_case("date") && matches!(value, Value::Date(_)) {
                    return true;
                }
                if name.eq_ignore_ascii_case("time") && matches!(value, Value::Time(_)) {
                    return true;
                }
                if name.eq_ignore_ascii_case("datetime") && matches!(value, Value::DateTime(_)) {
                    return true;
                }
                if let Value::ContainerInstance(instance) = value {
                    let mut current = Some(Rc::clone(instance));
                    while let Some(inst) = current {
                        let inst_ref = inst.borrow();
                        if inst_ref.container_type == *name {
                            return true;
                        }
                        current = inst_ref.parent.clone();
                    }
                    false
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    /// Renders an overload's signature in WFL surface syntax for error
    /// messages, e.g. `depict with value as number`.
    fn format_overload_signature(func: &FunctionValue) -> String {
        let name = func.name.as_deref().unwrap_or("anonymous");
        if func.params.is_empty() {
            return format!("{name} (no parameters)");
        }
        let params = func
            .params
            .iter()
            .zip(&func.param_types)
            .map(|(param, param_type)| match param_type {
                Some(t) => format!("{param} as {}", crate::analyzer::format_param_type(t)),
                None => param.clone(),
            })
            .collect::<Vec<_>>()
            .join(" and ");
        format!("{name} with {params}")
    }

    async fn call_function(
        &self,
        func: &FunctionValue,
        args: Vec<Value>,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        #[cfg(debug_assertions)]
        let func_name = func
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_string());

        if args.len() != func.params.len() {
            return Err(RuntimeError::new(
                format!(
                    "Expected {} arguments but got {}",
                    func.params.len(),
                    args.len()
                ),
                line,
                column,
            ));
        }

        // Declared parameter types are runtime-enforced only for actions
        // participating in overload dispatch: a lone member of a
        // not-yet-complete overload set rejects non-matching arguments instead
        // of silently running the wrong body — calls dispatch on "the
        // overloads defined so far". Plain single actions keep their
        // historical dynamic behavior (annotations are static hints, not
        // runtime guards), preserving backward compatibility. `nothing` and
        // untyped/`any` parameters accept every value.
        if func.enforce_param_types.get() {
            for ((param_name, param_type), arg) in
                func.params.iter().zip(&func.param_types).zip(args.iter())
            {
                if let Some(expected) = param_type
                    && !matches!(expected, Type::Any | Type::Unknown)
                    && !Self::value_matches_type(arg, expected)
                {
                    let action_name = func.name.as_deref().unwrap_or("anonymous");
                    return Err(RuntimeError::new(
                        format!(
                            "Argument '{param_name}' of '{action_name}' expects {}, but got {}",
                            crate::analyzer::format_param_type(expected),
                            arg.type_name()
                        ),
                        line,
                        column,
                    ));
                }
            }
        }

        let func_env = match func.env.upgrade() {
            Some(env) => {
                exec_trace!("call_function - Successfully upgraded function environment");
                env
            }
            None => {
                exec_trace!("call_function - Failed to upgrade function environment");
                return Err(RuntimeError::new(
                    "Environment no longer exists".to_string(),
                    line,
                    column,
                ));
            }
        };

        let call_env = Environment::new_child_env(&func_env);
        exec_trace!("call_function - Created child environment for function call");

        for (_i, (param, arg)) in func.params.iter().zip(args.clone()).enumerate() {
            exec_trace!(
                "call_function - Binding parameter {} '{}' to argument {:?}",
                _i,
                param,
                arg
            );

            #[cfg(debug_assertions)]
            exec_var_declare!(param, &arg);
            // Bind parameters directly in the call scope so they shadow any
            // same-named global/outer binding. `define` (which rejects names
            // present in a parent scope) would otherwise leave the parameter
            // unbound and let the body resolve to the global instead (#582).
            let _ = call_env.borrow_mut().define_direct(param, arg.clone());
        }

        // Enforce the shared recursion ceiling before descending another level,
        // turning runaway recursion into a clean error instead of a native stack
        // overflow. The dedicated `call_depth` counter (not `call_stack.len()`)
        // is the enforcement source of truth: it is decremented by the RAII
        // guard below as the call unwinds — including when a `try`/`when`
        // catches a `ResourceLimit` — so catch-and-recurse cannot under-count
        // and pile onto still-live native frames.
        if let Err(exceeded) = self.budget.check_call_depth(self.call_depth.get()) {
            return Err(self.budget_error(exceeded, line, column));
        }
        let _depth_guard = CallDepthGuard::enter(&self.call_depth);
        let _static_method_scope = func
            .static_method_context
            .as_ref()
            .map(|context| StaticMethodCallScope::enter(self, Rc::clone(context)));

        let frame = CallFrame::new(
            func.name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string()),
            line,
            column,
        );
        self.call_stack.borrow_mut().push(frame);
        exec_trace!("call_function - Pushed frame to call stack");

        #[cfg(debug_assertions)]
        exec_block_enter!(format!("function {}", func_name));

        #[cfg(debug_assertions)]
        let _guard = IndentGuard::new();

        exec_trace!("call_function - Executing function body");
        let result = self.execute_block(&func.body, call_env.clone()).await;
        exec_trace!("call_function - Function execution result: {:?}", result);

        #[cfg(debug_assertions)]
        exec_block_exit!(format!("function {}", func_name));

        match result {
            Ok((value, control_flow)) => {
                self.call_stack.borrow_mut().pop();

                let return_value = match control_flow {
                    ControlFlow::Return(val) => {
                        exec_trace!(
                            "call_function - Function explicitly returned with value: {:?}",
                            val
                        );
                        val
                    }
                    _ => {
                        exec_trace!("call_function - Function completed with value: {:?}", value);
                        value
                    }
                };

                exec_trace!(
                    "call_function - Function returned successfully with value: {:?}",
                    return_value
                );
                Ok(return_value)
            }
            Err(err) => {
                exec_trace!(
                    "call_function - Function execution failed with error: {:?}",
                    err
                );
                if let Some(last_frame) = self.call_stack.borrow_mut().last_mut() {
                    last_frame.capture_locals(&call_env);
                }

                let error_with_stack = err.clone();

                self.call_stack.borrow_mut().pop();

                Err(error_with_stack)
            }
        }
    }

    fn evaluate_numeric_op<Op, ErrGen>(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
        op: Op,
        err_gen: ErrGen,
    ) -> Result<Value, RuntimeError>
    where
        Op: Fn(f64, f64) -> Result<f64, String>,
        ErrGen: Fn(&str, &str) -> String,
    {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => match op(a, b) {
                Ok(res) => Ok(Value::Number(res)),
                Err(msg) => Err(RuntimeError::new(msg, line, column)),
            },
            (a, b) => Err(RuntimeError::new(
                err_gen(a.type_name(), b.type_name()),
                line,
                column,
            )),
        }
    }

    fn evaluate_comparison_op<Comp>(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
        op_symbol: &str,
        comp: Comp,
    ) -> Result<Value, RuntimeError>
    where
        Comp: Fn(std::cmp::Ordering) -> bool,
    {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => match a.partial_cmp(&b) {
                Some(ord) => Ok(Value::Bool(comp(ord))),
                None => Ok(Value::Bool(false)),
            },
            (Value::Text(a), Value::Text(b)) => Ok(Value::Bool(comp(a.cmp(&b)))),
            (Value::Date(a), Value::Date(b)) => Ok(Value::Bool(comp(a.cmp(&b)))),
            (Value::Time(a), Value::Time(b)) => Ok(Value::Bool(comp(a.cmp(&b)))),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(Value::Bool(comp(a.cmp(&b)))),
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot compare {} and {} with {}",
                    a.type_name(),
                    b.type_name(),
                    op_symbol
                ),
                line,
                column,
            )),
        }
    }

    fn perform_binary_op(
        &self,
        operator: &Operator,
        left_val: Value,
        right_val: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match operator {
            Operator::Plus => self.add(left_val, right_val, line, column),
            Operator::Minus => self.evaluate_numeric_op(
                left_val,
                right_val,
                line,
                column,
                |a, b| Ok(a - b),
                |a_type, b_type| format!("Cannot subtract {b_type} from {a_type}"),
            ),
            Operator::Multiply => self.evaluate_numeric_op(
                left_val,
                right_val,
                line,
                column,
                |a, b| Ok(a * b),
                |a_type, b_type| format!("Cannot multiply {a_type} and {b_type}"),
            ),
            Operator::Divide => self.evaluate_numeric_op(
                left_val,
                right_val,
                line,
                column,
                |a, b| {
                    #[cfg(feature = "dhat-ad-hoc")]
                    dhat::ad_hoc_event(1); // Track division operations for memory profiling

                    if b == 0.0 {
                        Err("Division by zero".to_string())
                    } else {
                        let res = a / b;
                        if !res.is_finite() {
                            Err(format!("Division resulted in invalid number: {res}"))
                        } else {
                            Ok(res)
                        }
                    }
                },
                |a_type, b_type| format!("Cannot divide {a_type} by {b_type}"),
            ),
            Operator::Modulo => self.evaluate_numeric_op(
                left_val,
                right_val,
                line,
                column,
                |a, b| {
                    #[cfg(feature = "dhat-ad-hoc")]
                    dhat::ad_hoc_event(1); // Track modulo operations for memory profiling

                    if b == 0.0 {
                        Err("Modulo by zero".to_string())
                    } else {
                        let res = a % b;
                        if !res.is_finite() {
                            Err(format!("Modulo resulted in invalid number: {res}"))
                        } else {
                            Ok(res)
                        }
                    }
                },
                |a_type, b_type| format!("Cannot compute modulo of {a_type} by {b_type}"),
            ),
            Operator::Equals => Ok(Value::Bool(self.is_equal(&left_val, &right_val))),
            Operator::NotEquals => Ok(Value::Bool(!self.is_equal(&left_val, &right_val))),
            Operator::GreaterThan => {
                self.evaluate_comparison_op(left_val, right_val, line, column, ">", |ord| {
                    matches!(ord, std::cmp::Ordering::Greater)
                })
            }
            Operator::LessThan => {
                self.evaluate_comparison_op(left_val, right_val, line, column, "<", |ord| {
                    matches!(ord, std::cmp::Ordering::Less)
                })
            }
            Operator::GreaterThanOrEqual => {
                self.evaluate_comparison_op(left_val, right_val, line, column, ">=", |ord| {
                    matches!(ord, std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                })
            }
            Operator::LessThanOrEqual => {
                self.evaluate_comparison_op(left_val, right_val, line, column, "<=", |ord| {
                    matches!(ord, std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                })
            }
            Operator::And => Ok(Value::Bool(left_val.is_truthy() && right_val.is_truthy())),
            Operator::Or => Ok(Value::Bool(left_val.is_truthy() || right_val.is_truthy())),
            Operator::Contains => self.contains(left_val, right_val, line, column),
        }
    }

    fn perform_unary_op(
        &self,
        operator: &UnaryOperator,
        value: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match operator {
            UnaryOperator::Not => Ok(Value::Bool(!value.is_truthy())),
            UnaryOperator::Minus => match value {
                Value::Number(n) => Ok(Value::Number(-n)),
                _ => Err(RuntimeError::new(
                    format!("Cannot negate {}", value.type_name()),
                    line,
                    column,
                )),
            },
        }
    }

    fn perform_concatenation(&self, left_val: Value, right_val: Value) -> Value {
        // Optimization: Fast path for string concatenation to avoid format! machinery overhead
        if let (Value::Text(left), Value::Text(right)) = (&left_val, &right_val) {
            let mut s = String::with_capacity(left.len() + right.len());
            s.push_str(left);
            s.push_str(right);
            return Value::Text(Arc::from(s));
        }

        let result = format!("{left_val}{right_val}");
        Value::Text(Arc::from(result.as_str()))
    }

    fn add(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Text(a), Value::Text(b)) => {
                // Optimization: Fast path for string concatenation
                let mut s = String::with_capacity(a.len() + b.len());
                s.push_str(&a);
                s.push_str(&b);
                Ok(Value::Text(Arc::from(s)))
            }
            (Value::Text(a), b) => {
                let result = format!("{a}{b}");
                Ok(Value::Text(Arc::from(result.as_str())))
            }
            (a, Value::Text(b)) => {
                let result = format!("{a}{b}");
                Ok(Value::Text(Arc::from(result.as_str())))
            }
            (a, b) => Err(RuntimeError::new(
                format!("Cannot add {} and {}", a.type_name(), b.type_name()),
                line,
                column,
            )),
        }
    }

    fn is_equal(&self, left: &Value, right: &Value) -> bool {
        left == right
    }

    // Helper method to create container instance with inheritance
    #[allow(clippy::only_used_in_recursion)]
    fn create_container_instance_with_inheritance(
        &self,
        container_type: &str,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<ContainerInstanceValue, RuntimeError> {
        // Look up the container definition
        let container_def = match env.borrow().get(container_type) {
            Some(Value::ContainerDefinition(def)) => def.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Container '{container_type}' not found"),
                    line,
                    column,
                ));
            }
        };

        // Create parent instance if container extends another
        let parent_instance = if let Some(parent_type) = &container_def.extends {
            // Recursively create parent instance
            let parent =
                self.create_container_instance_with_inheritance(parent_type, env, line, column)?;
            Some(Rc::new(RefCell::new(parent)))
        } else {
            None
        };

        // Create instance with inherited properties
        let mut instance_properties = HashMap::new();

        // Copy properties from parent if exists
        if let Some(ref parent) = parent_instance {
            for (key, value) in &parent.borrow().properties {
                instance_properties.insert(key.clone(), value.clone());
            }
        }

        // Initialize properties with default values from container definition
        for (prop_name, prop_def) in &container_def.properties {
            if let Some(default_value) = &prop_def.default_value {
                instance_properties.insert(prop_name.clone(), default_value.clone());
            }
        }

        Ok(ContainerInstanceValue {
            container_type: container_type.to_string(),
            properties: instance_properties,
            parent: parent_instance,
            line,
            column,
        })
    }

    fn contains(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::List(list_rc), item) => {
                let list = list_rc.borrow();
                for value in list.iter() {
                    if self.is_equal(value, &item) {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            (Value::Object(obj_rc), Value::Text(key)) => {
                let obj = obj_rc.borrow();
                Ok(Value::Bool(obj.contains_key(&key.to_string())))
            }
            (Value::Text(text), Value::Text(substring)) => {
                Ok(Value::Bool(text.contains(&*substring)))
            }
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot check if {} contains {}",
                    a.type_name(),
                    b.type_name()
                ),
                line,
                column,
            )),
        }
    }

    async fn list_files_recursive(
        &self,
        path: &str,
        extensions: Option<Vec<String>>,
    ) -> Result<Vec<Value>, std::io::Error> {
        use tokio::fs;

        let mut files = Vec::new();
        let mut dirs_to_process = vec![path.to_string()];

        while let Some(current_dir) = dirs_to_process.pop() {
            let mut entries = fs::read_dir(&current_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let path_str = path.to_string_lossy().to_string();

                if path.is_dir() {
                    dirs_to_process.push(path_str);
                } else if path.is_file() {
                    // Check extension filter if provided
                    if let Some(ref exts) = extensions {
                        let file_ext = path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| format!(".{ext}"));

                        if let Some(ext) = file_ext
                            && exts.iter().any(|e| e == &ext)
                        {
                            files.push(Value::Text(path_str.into()));
                        }
                    } else {
                        files.push(Value::Text(path_str.into()));
                    }
                }
            }
        }

        Ok(files)
    }

    async fn list_files_filtered(
        &self,
        path: &str,
        extensions: Vec<String>,
    ) -> Result<Vec<Value>, std::io::Error> {
        use tokio::fs;

        let mut files = Vec::new();
        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                let path_str = path.to_string_lossy().to_string();

                // Check extension filter
                let file_ext = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| format!(".{ext}"));

                if let Some(ext) = file_ext
                    && extensions.iter().any(|e| e == &ext)
                {
                    files.push(Value::Text(path_str.into()));
                }
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod concurrent_handler_classification_tests {
    use super::*;
    use std::future::{Future, poll_fn};
    use std::pin::Pin;
    use std::task::{Context, Poll};

    fn error(kind: ErrorKind, message: &str) -> RuntimeError {
        RuntimeError::with_kind(message.to_string(), 1, 1, kind)
    }

    fn one_yield_static_scope<'a>(
        interpreter: &'a Interpreter,
        context: Rc<StaticMethodContext>,
    ) -> impl Future<Output = ()> + 'a {
        let yielded = Rc::new(Cell::new(false));
        async move {
            let _depth = CallDepthGuard::enter(&interpreter.call_depth);
            let _scope = StaticMethodCallScope::enter(interpreter, context);
            poll_fn(move |cx| {
                if yielded.replace(true) {
                    Poll::Ready(())
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            })
            .await;
        }
    }

    fn isolated_static_handler<'a>(
        interpreter: &'a Interpreter,
        context: Rc<StaticMethodContext>,
    ) -> Pin<Box<IsolatedHandler<'a, ()>>> {
        Box::pin(IsolatedHandler {
            interp: interpreter,
            state: RunState::fresh(interpreter.base_call_depth),
            inner: Some(Box::pin(one_yield_static_scope(interpreter, context))),
            drain: None,
            pending_output: None,
            pending_finish_ids: Vec::new(),
        })
    }

    fn shared_static_owner() -> Rc<ContainerDefinitionValue> {
        let mut static_properties = HashMap::new();
        static_properties.insert("x".to_string(), Value::Number(0.0));
        static_properties.insert("y".to_string(), Value::Number(0.0));
        Rc::new(ContainerDefinitionValue {
            name: "Shared".to_string(),
            extends: None,
            implements: Vec::new(),
            properties: HashMap::new(),
            methods: HashMap::new(),
            events: HashMap::new(),
            static_properties: Rc::new(RefCell::new(static_properties)),
            static_methods: HashMap::new(),
            line: 1,
            column: 1,
        })
    }

    fn static_context_for(owner: &Rc<ContainerDefinitionValue>) -> Rc<StaticMethodContext> {
        let env = Environment::new_global();
        let mut property_owners = HashMap::new();
        for (name, value) in owner.static_properties.borrow().iter() {
            env.borrow_mut().define_or_replace(name, value.clone());
            property_owners.insert(name.clone(), Rc::clone(owner));
        }
        Rc::new(StaticMethodContext {
            env,
            property_owners,
        })
    }

    #[test]
    fn more_than_the_breaker_threshold_of_request_wait_timeouts_stays_request_local() {
        let timeout = error(
            ErrorKind::Timeout,
            &format!("{REQUEST_WAIT_TIMEOUT_PREFIX} (1 ms)"),
        );
        for observed in 0..=MAX_CONSECUTIVE_HANDLER_FAILURES {
            assert_eq!(
                classify_concurrent_handler_error(&timeout, false),
                ConcurrentHandlerDisposition::RequestLocal,
                "finite request-wait timeout #{observed} must not feed the structural breaker"
            );
        }
    }

    #[test]
    fn only_the_expected_timeout_origin_is_exempt_before_request_acceptance() {
        let structural_timeout = error(
            ErrorKind::Timeout,
            "unrelated pre-request operation timed out",
        );
        assert_eq!(
            classify_concurrent_handler_error(&structural_timeout, false),
            ConcurrentHandlerDisposition::Structural
        );
        assert_eq!(
            classify_concurrent_handler_error(&error(ErrorKind::General, "request failed"), true),
            ConcurrentHandlerDisposition::RequestLocal,
            "any failure after request acceptance is isolated to that request"
        );
        assert_eq!(
            classify_concurrent_handler_error(
                &error(ErrorKind::Cancelled, "client disconnected"),
                false,
            ),
            ConcurrentHandlerDisposition::RequestLocal
        );
    }

    #[test]
    fn concurrent_handlers_park_and_cancel_static_contexts_independently() {
        let interpreter = Interpreter::new();
        let context_a = Rc::new(StaticMethodContext {
            env: Environment::new_global(),
            property_owners: HashMap::new(),
        });
        let context_b = Rc::new(StaticMethodContext {
            env: Environment::new_global(),
            property_owners: HashMap::new(),
        });
        let mut handler_a = isolated_static_handler(&interpreter, Rc::clone(&context_a));
        let mut handler_b = isolated_static_handler(&interpreter, Rc::clone(&context_b));
        let mut cx = Context::from_waker(std::task::Waker::noop());

        assert!(handler_a.as_mut().poll(&mut cx).is_pending());
        assert!(
            interpreter
                .active_static_method_contexts
                .borrow()
                .is_empty()
        );
        assert_eq!(interpreter.call_depth.get(), interpreter.base_call_depth);
        assert!(
            handler_a
                .as_ref()
                .get_ref()
                .state
                .active_static_method_contexts
                .first()
                .is_some_and(|context| Rc::ptr_eq(context, &context_a))
        );

        assert!(handler_b.as_mut().poll(&mut cx).is_pending());
        assert!(
            interpreter
                .active_static_method_contexts
                .borrow()
                .is_empty()
        );
        assert_eq!(interpreter.call_depth.get(), interpreter.base_call_depth);
        assert!(
            handler_b
                .as_ref()
                .get_ref()
                .state
                .active_static_method_contexts
                .first()
                .is_some_and(|context| Rc::ptr_eq(context, &context_b))
        );

        assert!(handler_a.as_mut().poll(&mut cx).is_ready());
        assert!(
            handler_a
                .as_ref()
                .get_ref()
                .state
                .active_static_method_contexts
                .is_empty()
        );
        assert!(
            interpreter
                .active_static_method_contexts
                .borrow()
                .is_empty()
        );

        // Handler B is still suspended with its own scope. Cancellation must
        // unwind B against B's parked state without leaking into interpreter
        // scratch state or reviving A's completed context.
        drop(handler_b);
        assert!(
            interpreter
                .active_static_method_contexts
                .borrow()
                .is_empty()
        );
        assert_eq!(interpreter.call_depth.get(), interpreter.base_call_depth);
    }

    #[test]
    fn poll_boundaries_merge_interleaved_static_property_updates() {
        let interpreter = Interpreter::new();
        let owner = shared_static_owner();
        let context_a = static_context_for(&owner);
        let context_b = static_context_for(&owner);
        let mut state_a = RunState::fresh(interpreter.base_call_depth);
        let mut state_b = RunState::fresh(interpreter.base_call_depth);
        state_a
            .active_static_method_contexts
            .push(Rc::clone(&context_a));
        state_b
            .active_static_method_contexts
            .push(Rc::clone(&context_b));

        {
            let _installed = InstalledRunState::new(&interpreter, &mut state_a);
            context_a
                .env
                .borrow_mut()
                .define_or_replace("x", Value::Number(1.0));
        }
        {
            let _installed = InstalledRunState::new(&interpreter, &mut state_b);
            assert!(matches!(
                context_b.env.borrow().get_local("x"),
                Some(Value::Number(1.0))
            ));
            context_b
                .env
                .borrow_mut()
                .define_or_replace("y", Value::Number(2.0));
        }
        {
            let _installed = InstalledRunState::new(&interpreter, &mut state_a);
            assert!(matches!(
                context_a.env.borrow().get_local("y"),
                Some(Value::Number(2.0))
            ));
        }

        let properties = owner.static_properties.borrow();
        assert!(matches!(properties.get("x"), Some(Value::Number(1.0))));
        assert!(matches!(properties.get("y"), Some(Value::Number(2.0))));
        assert!(
            interpreter
                .active_static_method_contexts
                .borrow()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn missing_pending_entry_is_cancelled_only_while_the_handler_owns_it() {
        let interpreter = Interpreter::new();
        interpreter
            .open_pending_requests
            .borrow_mut()
            .push("request-1".to_string());

        let disconnected = interpreter
            .ensure_pending_response_owned("request-1", 1, 1)
            .await
            .expect_err("owned-but-pruned pending entry must be cancellation");
        assert_eq!(disconnected.kind, ErrorKind::Cancelled);

        interpreter.open_pending_requests.borrow_mut().clear();
        let duplicate = interpreter
            .ensure_pending_response_owned("request-1", 1, 1)
            .await
            .expect_err("non-owned missing entry must remain a duplicate-response error");
        assert_eq!(duplicate.kind, ErrorKind::General);
        assert!(
            duplicate.message.contains("already been sent"),
            "duplicate response diagnostic changed unexpectedly: {duplicate}"
        );
    }
}

#[cfg(test)]
mod response_expression_disconnect_tests {
    use super::*;
    use crate::lexer::lex_wfl_with_positions;
    use crate::parser::Parser;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct StalledUpstream {
        port: u16,
        head_sent: oneshot::Receiver<()>,
        peer_closed: oneshot::Receiver<()>,
        release: Option<oneshot::Sender<()>>,
    }

    async fn spawn_stalled_upstream() -> StalledUpstream {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stalled upstream");
        let port = listener.local_addr().expect("upstream address").port();
        let (head_sent_tx, head_sent) = oneshot::channel();
        let (peer_closed_tx, peer_closed) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept upstream request");
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = socket
                    .read(&mut chunk)
                    .await
                    .expect("read upstream request");
                assert!(read > 0, "client closed before sending request head");
                request.extend_from_slice(&chunk[..read]);
            }
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\n\
                      Transfer-Encoding: chunked\r\n\
                      Connection: close\r\n\r\n",
                )
                .await
                .expect("send stalled response head");
            let _ = head_sent_tx.send(());

            let mut probe = [0u8; 1];
            tokio::select! {
                result = socket.read(&mut probe) => {
                    assert!(
                        matches!(result, Ok(0) | Err(_)),
                        "client unexpectedly sent bytes while response body was stalled: {result:?}"
                    );
                    let _ = peer_closed_tx.send(());
                }
                _ = release_rx => {
                    let _ = socket.shutdown().await;
                }
            }
        });

        StalledUpstream {
            port,
            head_sent,
            peer_closed,
            release: Some(release_tx),
        }
    }

    fn parse_statements(source: &str) -> Vec<Statement> {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        parser
            .parse()
            .unwrap_or_else(|errors| {
                panic!("response disconnect fixture did not parse: {errors:?}")
            })
            .statements
    }

    fn install_pending_request(
        interpreter: &Interpreter,
        env: &Rc<RefCell<Environment>>,
        request_id: &str,
    ) -> oneshot::Receiver<HandlerReply> {
        let (sender, receiver) = oneshot::channel();
        interpreter.pending_responses.borrow_mut().insert(
            request_id.to_string(),
            PendingResponse {
                sender: Arc::new(tokio::sync::Mutex::new(Some(sender))),
            },
        );
        interpreter
            .open_pending_requests
            .borrow_mut()
            .push(request_id.to_string());

        let mut request = HashMap::new();
        request.insert(
            "_response_sender".to_string(),
            Value::Text(Arc::from(request_id)),
        );
        env.borrow_mut()
            .define_or_replace("req", Value::Object(Rc::new(RefCell::new(request))));
        receiver
    }

    fn response_eval_is_stalled(interpreter: &Interpreter) -> bool {
        let owned_streams = {
            let owner = Arc::clone(&interpreter.open_http_streams.borrow());
            owner
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .len()
        };
        owned_streams == 1
            && interpreter.get_call_stack().len() == 1
            && *interpreter.current_count.borrow() == Some(1.0)
            && *interpreter.in_count_loop.borrow()
    }

    async fn assert_precommit_disconnect_cancels(response_statement: &str, action_return: &str) {
        let mut upstream = spawn_stalled_upstream().await;
        let config = Arc::new(WflConfig {
            timeout_seconds: 120,
            outbound_stream_max_seconds: 120,
            ..WflConfig::default()
        });
        let interpreter = Interpreter::with_config(config);
        let source = format!(
            "define action called stalled_value:\n\
             \x20\x20\x20\x20count from 1 to 1:\n\
             \x20\x20\x20\x20\x20\x20\x20\x20open url at \"http://127.0.0.1:{}/stall\" and stream response as upstream\n\
             \x20\x20\x20\x20\x20\x20\x20\x20wait for 60000 milliseconds\n\
             \x20\x20\x20\x20end count\n\
             \x20\x20\x20\x20return {action_return}\n\
             end action\n\
             {response_statement}\n",
            upstream.port
        );
        let statements = parse_statements(&source);
        assert_eq!(
            statements.len(),
            2,
            "unexpected fixture AST: {statements:#?}"
        );
        let env = Rc::clone(interpreter.global_env());
        interpreter
            .execute_statement(&statements[0], Rc::clone(&env))
            .await
            .expect("define stalled action");

        // Simulate an already-active outer count loop. Dropping the response
        // evaluation must restore this exact run-state snapshot.
        *interpreter.current_count.borrow_mut() = Some(41.0);
        *interpreter.in_count_loop.borrow_mut() = true;
        let receiver = install_pending_request(&interpreter, &env, "request-1");

        let mut response = Box::pin(interpreter.execute_statement(&statements[1], env));
        let mut stalled = Box::pin(async {
            loop {
                if response_eval_is_stalled(&interpreter) {
                    return;
                }
                tokio::task::yield_now().await;
            }
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            tokio::select! {
                result = response.as_mut() => {
                    panic!("response evaluation finished before the disconnect latch: {result:?}")
                }
                _ = stalled.as_mut() => {}
            }
        })
        .await
        .expect("response expression never reached its stalled action");
        upstream
            .head_sent
            .await
            .expect("stalled upstream did not send its response head");

        // Causal trigger: only after the action owns a live upstream stream and
        // is sleeping inside a count loop do we drop the client receiver.
        drop(receiver);
        let result = match tokio::time::timeout(Duration::from_secs(2), response.as_mut()).await {
            Ok(result) => result,
            Err(_) => {
                drop(response);
                interpreter.close_open_http_streams();
                if let Some(release) = upstream.release.take() {
                    let _ = release.send(());
                }
                panic!(
                    "response expression evaluation did not cancel after its request disconnected"
                );
            }
        };
        drop(response);

        let error = result.expect_err("disconnect must cancel response precommit evaluation");
        assert_eq!(error.kind, ErrorKind::Cancelled, "wrong error: {error:?}");
        assert!(
            interpreter.get_call_stack().is_empty(),
            "dropped response evaluation leaked call frames"
        );
        assert_eq!(interpreter.call_depth.get(), 0, "call depth leaked");
        assert_eq!(
            *interpreter.current_count.borrow(),
            Some(41.0),
            "outer count binding was not restored"
        );
        assert!(
            *interpreter.in_count_loop.borrow(),
            "outer count-loop state was not restored"
        );
        assert!(
            !interpreter
                .pending_responses
                .borrow()
                .contains_key("request-1"),
            "cancelled request remained pending"
        );
        assert!(
            !interpreter
                .open_pending_requests
                .borrow()
                .iter()
                .any(|id| id == "request-1"),
            "cancelled request remained handler-owned"
        );

        tokio::time::timeout(Duration::from_secs(2), &mut upstream.peer_closed)
            .await
            .expect("cancelled response evaluation did not drop its upstream socket")
            .expect("upstream close observation task ended early");
        assert!(
            interpreter
                .open_http_streams
                .borrow()
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .is_empty(),
            "cancelled response evaluation retained upstream ownership"
        );
    }

    #[tokio::test]
    async fn buffered_content_evaluation_cancels_on_request_disconnect() {
        assert_precommit_disconnect_cancels("respond to req with call stalled_value", "\"late\"")
            .await;
    }

    #[tokio::test]
    async fn streaming_head_evaluation_cancels_on_request_disconnect() {
        assert_precommit_disconnect_cancels(
            "start streaming response to req with status call stalled_value and content type \"text/plain\" as out",
            "201",
        )
        .await;
    }

    #[tokio::test]
    async fn buffered_request_operand_cancels_on_request_disconnect() {
        assert_precommit_disconnect_cancels("respond to (call stalled_value) with \"ok\"", "req")
            .await;
    }

    #[tokio::test]
    async fn streaming_request_operand_cancels_on_request_disconnect() {
        assert_precommit_disconnect_cancels(
            "start streaming response to (call stalled_value) with status 200 as out",
            "req",
        )
        .await;
    }
}

#[cfg(test)]
mod response_disconnect_result_tests {
    use super::*;
    use crate::lexer::lex_wfl_with_positions;
    use crate::parser::Parser;
    use std::future::Future;
    use std::task::Poll;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn parse_statement(source: &str) -> Statement {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser
            .parse()
            .unwrap_or_else(|errors| panic!("disconnect fixture did not parse: {errors:?}"));
        assert_eq!(program.statements.len(), 1);
        program.statements.into_iter().next().expect("statement")
    }

    fn parse_statements(source: &str) -> Vec<Statement> {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        parser
            .parse()
            .unwrap_or_else(|errors| panic!("disconnect fixture did not parse: {errors:?}"))
            .statements
    }

    fn request_value(request_id: &str) -> Value {
        let mut request = HashMap::new();
        request.insert(
            "_response_sender".to_string(),
            Value::Text(Arc::from(request_id)),
        );
        Value::Object(Rc::new(RefCell::new(request)))
    }

    async fn assert_response_commit_disconnect_is_cancelled(statement_source: &str) {
        let interpreter = Interpreter::new();
        let env = Rc::clone(interpreter.global_env());
        env.borrow_mut()
            .define_or_replace("req", request_value("request-commit"));
        let (sender, receiver) = oneshot::channel();
        let shared_sender = Arc::new(tokio::sync::Mutex::new(Some(sender)));
        interpreter.pending_responses.borrow_mut().insert(
            "request-commit".to_string(),
            PendingResponse {
                sender: Arc::clone(&shared_sender),
            },
        );
        interpreter
            .open_pending_requests
            .borrow_mut()
            .push("request-commit".to_string());

        // Hold the sender lock so the statement can finish evaluation and reach
        // the exact commit await without taking the sender yet.
        let guard = shared_sender.lock().await;
        let statement = parse_statement(statement_source);
        let mut execution = Box::pin(interpreter.execute_statement(&statement, env));
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if !interpreter
                    .pending_responses
                    .borrow()
                    .contains_key("request-commit")
                {
                    break;
                }
                tokio::select! {
                    result = execution.as_mut() => {
                        panic!("response finished before reaching the commit latch: {result:?}")
                    }
                    _ = tokio::task::yield_now() => {}
                }
            }
        })
        .await
        .expect("response did not reach its commit latch");

        drop(receiver);
        drop(guard);
        let error = tokio::time::timeout(Duration::from_secs(2), execution.as_mut())
            .await
            .expect("commit did not observe the closed receiver")
            .expect_err("closed receiver must cancel the response commit");
        assert_eq!(error.kind, ErrorKind::Cancelled, "wrong error: {error:?}");
    }

    async fn spawn_commit_upstream() -> (u16, oneshot::Receiver<()>, oneshot::Receiver<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind commit upstream");
        let port = listener
            .local_addr()
            .expect("commit upstream address")
            .port();
        let (head_tx, head_sent) = oneshot::channel();
        let (closed_tx, peer_closed) = oneshot::channel();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept commit upstream");
            let mut request = Vec::new();
            let mut buffer = [0u8; 512];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = socket.read(&mut buffer).await.expect("read request head");
                assert!(read > 0, "client closed before commit request");
                request.extend_from_slice(&buffer[..read]);
            }
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\n\
                      Connection: close\r\n\r\n",
                )
                .await
                .expect("write commit response head");
            socket.flush().await.expect("flush commit response head");
            let _ = head_tx.send(());
            loop {
                match socket.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
            let _ = closed_tx.send(());
        });
        (port, head_sent, peer_closed)
    }

    async fn assert_commit_disconnect_closes_evaluation_stream(response_statement: &str) {
        let (port, head_sent, mut peer_closed) = spawn_commit_upstream().await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 120,
            timeout_seconds: 120,
            ..WflConfig::default()
        });
        let interpreter = Interpreter::with_config(config);
        let source = format!(
            "define action called open_then_return:\n\
             \x20\x20\x20\x20open url at \"http://127.0.0.1:{port}/commit\" and stream response as upstream\n\
             \x20\x20\x20\x20return 201\n\
             end action\n\
             {response_statement}\n"
        );
        let statements = parse_statements(&source);
        assert_eq!(statements.len(), 2);
        let env = Rc::clone(interpreter.global_env());
        interpreter
            .execute_statement(&statements[0], Rc::clone(&env))
            .await
            .expect("define commit action");

        env.borrow_mut()
            .define_or_replace("req", request_value("request-resource"));
        let (sender, receiver) = oneshot::channel();
        let shared_sender = Arc::new(tokio::sync::Mutex::new(Some(sender)));
        interpreter.pending_responses.borrow_mut().insert(
            "request-resource".to_string(),
            PendingResponse {
                sender: Arc::clone(&shared_sender),
            },
        );
        interpreter
            .open_pending_requests
            .borrow_mut()
            .push("request-resource".to_string());
        let guard = shared_sender.lock().await;

        let mut execution =
            Box::pin(interpreter.execute_statement(&statements[1], Rc::clone(&env)));
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let at_commit = !interpreter
                    .pending_responses
                    .borrow()
                    .contains_key("request-resource");
                let owns_stream = interpreter
                    .open_http_streams
                    .borrow()
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .len()
                    == 1;
                if at_commit && owns_stream {
                    break;
                }
                tokio::select! {
                    result = execution.as_mut() => {
                        panic!("response finished before resource commit latch: {result:?}")
                    }
                    _ = tokio::task::yield_now() => {}
                }
            }
        })
        .await
        .expect("response did not reach resource commit latch");
        head_sent.await.expect("commit upstream did not send head");

        drop(receiver);
        drop(guard);
        let error = tokio::time::timeout(Duration::from_secs(2), execution.as_mut())
            .await
            .expect("commit did not observe receiver disconnect")
            .expect_err("commit disconnect must cancel");
        assert_eq!(error.kind, ErrorKind::Cancelled, "wrong error: {error:?}");

        if tokio::time::timeout(Duration::from_secs(1), &mut peer_closed)
            .await
            .is_err()
        {
            interpreter.close_open_http_streams();
            let _ = tokio::time::timeout(Duration::from_secs(2), &mut peer_closed).await;
            panic!("commit-time cancellation retained a stream opened during evaluation");
        }
        assert!(
            interpreter
                .open_http_streams
                .borrow()
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .is_empty(),
            "commit cancellation retained stream ownership"
        );
    }

    #[tokio::test]
    async fn buffered_commit_disconnect_is_exactly_cancelled() {
        assert_response_commit_disconnect_is_cancelled("respond to req with \"ok\"").await;
    }

    #[tokio::test]
    async fn streaming_head_commit_disconnect_is_exactly_cancelled() {
        assert_response_commit_disconnect_is_cancelled(
            "start streaming response to req with status 200 as out",
        )
        .await;
    }

    #[tokio::test]
    async fn already_disconnected_precheck_removes_stale_pending_state() {
        let interpreter = Interpreter::new();
        let env = Rc::clone(interpreter.global_env());
        env.borrow_mut()
            .define_or_replace("req", request_value("request-closed"));
        let (sender, receiver) = oneshot::channel();
        interpreter.pending_responses.borrow_mut().insert(
            "request-closed".to_string(),
            PendingResponse {
                sender: Arc::new(tokio::sync::Mutex::new(Some(sender))),
            },
        );
        interpreter
            .open_pending_requests
            .borrow_mut()
            .push("request-closed".to_string());
        drop(receiver);

        let statement = parse_statement("respond to req with \"late\"");
        let error = interpreter
            .execute_statement(&statement, env)
            .await
            .expect_err("closed request must cancel before evaluation");
        assert_eq!(error.kind, ErrorKind::Cancelled);
        assert!(
            !interpreter
                .pending_responses
                .borrow()
                .contains_key("request-closed"),
            "early cancellation retained the pending sender"
        );
        assert!(
            !interpreter
                .open_pending_requests
                .borrow()
                .iter()
                .any(|id| id == "request-closed"),
            "early cancellation retained handler ownership"
        );
    }

    #[tokio::test]
    async fn ordinary_response_expression_errors_remain_general_and_pending() {
        let interpreter = Interpreter::new();
        let env = Rc::clone(interpreter.global_env());
        env.borrow_mut()
            .define_or_replace("req", request_value("request-error"));
        let (sender, _receiver) = oneshot::channel();
        interpreter.pending_responses.borrow_mut().insert(
            "request-error".to_string(),
            PendingResponse {
                sender: Arc::new(tokio::sync::Mutex::new(Some(sender))),
            },
        );
        interpreter
            .open_pending_requests
            .borrow_mut()
            .push("request-error".to_string());

        let statement = parse_statement("respond to req with missing_value");
        let error = interpreter
            .execute_statement(&statement, env)
            .await
            .expect_err("undefined content must remain an ordinary expression error");
        assert_eq!(error.kind, ErrorKind::General, "wrong error: {error:?}");
        assert!(
            error.message.contains("missing_value"),
            "ordinary expression diagnostic changed: {error}"
        );
        assert!(
            interpreter
                .pending_responses
                .borrow()
                .contains_key("request-error"),
            "ordinary expression error consumed the pending response"
        );
        assert!(
            interpreter
                .open_pending_requests
                .borrow()
                .iter()
                .any(|id| id == "request-error"),
            "ordinary expression error removed handler ownership"
        );
    }

    #[tokio::test]
    async fn successful_buffered_response_behavior_is_preserved() {
        let interpreter = Interpreter::new();
        let env = Rc::clone(interpreter.global_env());
        env.borrow_mut()
            .define_or_replace("req", request_value("request-success"));
        let (sender, receiver) = oneshot::channel();
        interpreter.pending_responses.borrow_mut().insert(
            "request-success".to_string(),
            PendingResponse {
                sender: Arc::new(tokio::sync::Mutex::new(Some(sender))),
            },
        );
        interpreter
            .open_pending_requests
            .borrow_mut()
            .push("request-success".to_string());

        let statement = parse_statement(
            "respond to req with \"ok\" and content_type \"text/plain\" and status 201",
        );
        interpreter
            .execute_statement(&statement, env)
            .await
            .expect("connected buffered response must still succeed");
        let reply = receiver.await.expect("buffered response was not delivered");
        match reply {
            HandlerReply::Buffered(response) => {
                assert_eq!(response.status, 201);
                assert_eq!(response.content_type, "text/plain");
                assert_eq!(response.content, b"ok");
            }
            HandlerReply::Streaming { .. } => panic!("buffered respond produced streaming reply"),
        }
        assert!(
            !interpreter
                .pending_responses
                .borrow()
                .contains_key("request-success"),
            "successful response remained pending"
        );
        assert!(
            !interpreter
                .open_pending_requests
                .borrow()
                .iter()
                .any(|id| id == "request-success"),
            "successful response retained handler ownership"
        );
    }

    #[tokio::test]
    async fn buffered_commit_disconnect_closes_evaluation_streams() {
        assert_commit_disconnect_closes_evaluation_stream(
            "respond to req with call open_then_return",
        )
        .await;
    }

    #[tokio::test]
    async fn streaming_commit_disconnect_closes_evaluation_streams() {
        assert_commit_disconnect_closes_evaluation_stream(
            "start streaming response to req with status call open_then_return as out",
        )
        .await;
    }

    #[tokio::test]
    async fn backpressured_stream_write_disconnect_is_exactly_cancelled() {
        let config = Arc::new(WflConfig {
            web_server_response_timeout_seconds: 0,
            ..WflConfig::default()
        });
        let interpreter = Interpreter::with_config(config);
        let env = Rc::clone(interpreter.global_env());
        let handle_id = "respstream-test";
        let (sender, receiver) = mpsc::channel(RESPONSE_STREAM_BUFFER);
        for _ in 0..RESPONSE_STREAM_BUFFER {
            sender
                .try_send(vec![0])
                .expect("fill response stream buffer");
        }
        let (_done_tx, done_rx) = oneshot::channel();
        interpreter.server_response_streams.borrow_mut().insert(
            handle_id.to_string(),
            ServerResponseStream {
                tx: sender,
                bytes_written: 0,
                finished: Some(done_rx),
            },
        );
        interpreter
            .open_response_streams
            .borrow_mut()
            .push(handle_id.to_string());
        let mut stream = HashMap::new();
        stream.insert(
            "_server_stream".to_string(),
            Value::Text(Arc::from(handle_id)),
        );
        env.borrow_mut()
            .define_or_replace("out", Value::Object(Rc::new(RefCell::new(stream))));

        let statement = parse_statement("write chunk \"next\" to out");
        let mut execution = Box::pin(interpreter.execute_statement(&statement, env));
        futures_util::future::poll_fn(|cx| match execution.as_mut().poll(cx) {
            Poll::Pending => Poll::Ready(()),
            Poll::Ready(result) => {
                panic!("full response stream write did not backpressure: {result:?}")
            }
        })
        .await;
        drop(receiver);

        let error = tokio::time::timeout(Duration::from_secs(2), execution.as_mut())
            .await
            .expect("write did not wake after receiver disconnect")
            .expect_err("closed stream receiver must cancel the write");
        assert_eq!(error.kind, ErrorKind::Cancelled, "wrong error: {error:?}");
        assert!(
            !interpreter
                .server_response_streams
                .borrow()
                .contains_key(handle_id),
            "cancelled write retained its response stream"
        );
    }
}

#[cfg(test)]
mod request_wait_timeout_tests {
    use super::*;
    use crate::lexer::lex_wfl_with_positions;
    use crate::parser::Parser;

    fn parse_statement(source: &str) -> Statement {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        parser
            .parse()
            .unwrap_or_else(|errors| panic!("timeout fixture did not parse: {errors:?}"))
            .statements
            .into_iter()
            .next()
            .expect("statement")
    }

    async fn timeout_error(value: &str) -> RuntimeError {
        let interpreter = Interpreter::new();
        let env = Rc::clone(interpreter.global_env());
        let (request_sender, request_receiver) = mpsc::channel(1);
        interpreter.web_servers.borrow_mut().insert(
            "srv".to_string(),
            WflWebServer {
                request_receiver: Arc::new(tokio::sync::Mutex::new(request_receiver)),
                request_sender,
                server_handle: None,
                sessions: None,
            },
        );
        env.borrow_mut()
            .define_or_replace("srv", Value::Text(Arc::from("WebServer::127.0.0.1:1")));
        let statement = parse_statement(&format!(
            "wait for request comes in on srv as req with timeout {value}"
        ));
        interpreter
            .execute_statement(&statement, env)
            .await
            .expect_err("invalid sub-millisecond timeout must be rejected")
    }

    #[tokio::test]
    async fn zero_request_timeout_has_the_established_positive_number_error() {
        let error = timeout_error("0").await;
        assert_eq!(error.kind, ErrorKind::General);
        assert_eq!(
            error.message,
            "Timeout must be a positive number (milliseconds)"
        );
    }

    #[tokio::test]
    async fn fractional_request_timeout_below_one_millisecond_is_rejected() {
        let error = timeout_error("0.5").await;
        assert_eq!(error.kind, ErrorKind::General);
        assert_eq!(
            error.message,
            "Timeout must be at least 1 millisecond (got 0.5 ms); fractional values below 1 would truncate to zero and spin"
        );
    }
}

#[cfg(test)]
mod outbound_stream_deadline_tests {
    use super::*;
    use futures_util::task::AtomicWaker;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context, Poll};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn spawn_stream_cleanup_upstream(expected_requests: usize) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stream cleanup upstream");
        let port = listener.local_addr().expect("upstream address").port();
        tokio::spawn(async move {
            for _ in 0..expected_requests {
                let (mut socket, _) = listener.accept().await.expect("accept cleanup request");
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buf = [0u8; 512];
                    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                        let read = socket.read(&mut buf).await.expect("read request head");
                        if read == 0 {
                            return;
                        }
                        request.extend_from_slice(&buf[..read]);
                    }
                    let request = String::from_utf8_lossy(&request);
                    let truncated = request.starts_with("GET /truncated ");
                    let unterminated = request.starts_with("GET /unterminated ");
                    let response = if truncated {
                        "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nx"
                    } else if unterminated {
                        "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc"
                    } else {
                        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    };
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("write response");
                    socket.flush().await.expect("flush response");
                });
            }
        });
        port
    }

    async fn spawn_stalled_streams(
        expected_requests: usize,
    ) -> (u16, tokio::sync::mpsc::UnboundedReceiver<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind terminal-retention upstream");
        let port = listener.local_addr().expect("upstream address").port();
        let (peer_closed_tx, peer_closed_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            for _ in 0..expected_requests {
                let (mut socket, _) = listener
                    .accept()
                    .await
                    .expect("accept terminal-retention request");
                let peer_closed_tx = peer_closed_tx.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0u8; 512];
                    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                        let read = socket.read(&mut buffer).await.expect("read request head");
                        if read == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..read]);
                    }
                    socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\n\
                              Connection: close\r\n\r\n",
                        )
                        .await
                        .expect("write stalled response head");
                    socket.flush().await.expect("flush stalled response head");

                    loop {
                        match socket.read(&mut buffer).await {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {}
                        }
                    }
                    let _ = peer_closed_tx.send(());
                });
            }
        });
        (port, peer_closed_rx)
    }

    async fn assert_reapers_drained(client: &IoClient) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if client
                    .active_stream_reapers
                    .load(std::sync::atomic::Ordering::SeqCst)
                    == 0
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("finished streams left hard-lifetime reaper tasks sleeping");
    }

    fn install_owned_empty_stream(client: &IoClient, handle_id: &str) -> StreamOwner {
        let owner: StreamOwner =
            Arc::new(std::sync::Mutex::new(HashSet::from(
                [handle_id.to_string()],
            )));
        client
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .live
            .insert(
                handle_id.to_string(),
                StreamSlot {
                    handle: Some(HttpStreamHandle {
                        stream: Box::pin(futures_util::stream::empty::<reqwest::Result<Vec<u8>>>()),
                        buffer: Vec::new(),
                        done: false,
                        bytes_read: 0,
                        total_deadline: None,
                    }),
                    deadline: None,
                    cancel: StreamCancel::new(),
                    reaper_abort: None,
                    owner: Some(Arc::clone(&owner)),
                },
            );
        owner
    }

    async fn take_and_observe_clean_eof(
        client: &IoClient,
        handle_id: &str,
        budget: &Arc<ExecutionBudget>,
    ) -> TakenStream {
        let Some(mut taken) = client
            .take_stream(handle_id)
            .expect("take synthetic empty stream")
        else {
            panic!("live synthetic stream unexpectedly resolved as clean EOF");
        };
        assert!(
            !client
                .stream_pull(&mut taken.handle, budget, &taken.cancel)
                .await
                .expect("observe upstream clean EOF"),
            "an empty upstream must return clean EOF"
        );
        assert!(taken.handle.done, "observing EOF must mark the body done");
        assert_eq!(
            taken.cancel.terminal(),
            Some(StreamTerminal::CleanEof),
            "observing upstream None must latch CleanEof before slot removal"
        );
        taken
    }

    async fn assert_one_shot_clean_eof_after_put(
        client: &IoClient,
        handle_id: &str,
        owner: &StreamOwner,
        budget: Arc<ExecutionBudget>,
    ) {
        {
            let registry = client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            assert!(
                !registry.live.contains_key(handle_id),
                "put_stream must not recreate a heavy live slot after clean EOF"
            );
            assert_eq!(
                registry
                    .recent
                    .iter()
                    .filter(|entry| {
                        entry.id == handle_id && entry.reason == StreamTerminal::CleanEof
                    })
                    .count(),
                1,
                "put_stream must leave exactly one CleanEof tombstone"
            );
        }
        assert!(
            owner
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .is_empty(),
            "slot removal before put_stream must clear handler ownership"
        );
        assert_eq!(
            client
                .next_line(handle_id, Arc::clone(&budget))
                .await
                .expect("consume restored CleanEof tombstone"),
            None,
            "the first read after terminalization must observe clean EOF"
        );
        let later = client
            .next_line(handle_id, budget)
            .await
            .expect_err("the CleanEof tombstone must be one-shot");
        assert!(
            matches!(&later, HttpClientError::Request(message)
                if message.contains("Unknown or already-closed")),
            "the read after consuming CleanEof must report a closed/unknown handle, got {later:?}"
        );
    }

    struct DelayedHeadUpstream {
        port: u16,
        request_received: oneshot::Receiver<()>,
        release_head: Option<oneshot::Sender<()>>,
        peer_closed: oneshot::Receiver<()>,
    }

    async fn spawn_delayed_head_upstream() -> DelayedHeadUpstream {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind delayed-head upstream");
        let port = listener.local_addr().expect("upstream address").port();
        let (request_tx, request_received) = oneshot::channel();
        let (release_head, release_rx) = oneshot::channel();
        let (peer_closed_tx, peer_closed) = oneshot::channel();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept delayed head");
            let mut request = Vec::new();
            let mut buffer = [0u8; 512];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = socket.read(&mut buffer).await.expect("read request");
                assert!(read > 0, "client closed before request head");
                request.extend_from_slice(&buffer[..read]);
            }
            let _ = request_tx.send(());
            release_rx.await.expect("release delayed response head");
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\n\
                      Connection: close\r\n\r\n",
                )
                .await
                .expect("write delayed response head");
            socket.flush().await.expect("flush delayed response head");
            loop {
                match socket.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
            let _ = peer_closed_tx.send(());
        });
        DelayedHeadUpstream {
            port,
            request_received,
            release_head: Some(release_head),
            peer_closed,
        }
    }

    struct GatedChunkState {
        polled: AtomicBool,
        ready: AtomicBool,
        dropped: AtomicBool,
        waker: AtomicWaker,
    }

    impl GatedChunkState {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                polled: AtomicBool::new(false),
                ready: AtomicBool::new(false),
                dropped: AtomicBool::new(false),
                waker: AtomicWaker::new(),
            })
        }

        fn make_ready(&self) {
            self.ready.store(true, Ordering::SeqCst);
            self.waker.wake();
        }
    }

    struct GatedChunkStream {
        state: Arc<GatedChunkState>,
        yielded: bool,
    }

    impl futures_util::Stream for GatedChunkStream {
        type Item = reqwest::Result<Vec<u8>>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.yielded {
                return Poll::Ready(None);
            }
            self.state.polled.store(true, Ordering::SeqCst);
            self.state.waker.register(cx.waker());
            if self.state.ready.load(Ordering::SeqCst) {
                self.yielded = true;
                Poll::Ready(Some(Ok(vec![7])))
            } else {
                Poll::Pending
            }
        }
    }

    impl Drop for GatedChunkStream {
        fn drop(&mut self) {
            self.state.dropped.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn close_during_active_read_returns_closed_and_drops_upstream() {
        let (port, mut peer_closed) = spawn_stalled_streams(1).await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 60,
            timeout_seconds: 30,
            ..WflConfig::default()
        });
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let (_, _, handle_id) = client
            .open_http_stream(
                "GET",
                &format!("http://127.0.0.1:{port}/active-close"),
                &[],
                None,
                Arc::clone(&budget),
            )
            .await
            .expect("open stalled stream");

        let mut read = Box::pin(client.next_chunk(&handle_id, budget));
        futures_util::future::poll_fn(|cx| match read.as_mut().poll(cx) {
            Poll::Pending => Poll::Ready(()),
            Poll::Ready(result) => panic!("stalled read unexpectedly completed: {result:?}"),
        })
        .await;
        assert!(
            client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .live
                .get(&handle_id)
                .is_some_and(|slot| slot.handle.is_none()),
            "the close latch must observe a body read actively owning the handle"
        );

        assert!(
            client.finish_stream_slot_sync(&handle_id, StreamTerminal::Closed),
            "close must claim the active stream slot"
        );
        let error = tokio::time::timeout(Duration::from_secs(2), read.as_mut())
            .await
            .expect("active read did not wake after close")
            .expect_err("active read must return Closed");
        assert!(
            matches!(error, HttpClientError::Closed),
            "active close returned the wrong error: {error:?}"
        );
        tokio::time::timeout(Duration::from_secs(2), peer_closed.recv())
            .await
            .expect("active close did not drop the upstream socket")
            .expect("upstream close notifier ended early");
        assert_reapers_drained(&client).await;
        assert!(
            !client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .live
                .contains_key(&handle_id),
            "active close retained its stream slot"
        );
    }

    #[tokio::test]
    async fn delayed_head_keeps_the_request_start_as_the_total_deadline_origin() {
        let mut upstream = spawn_delayed_head_upstream().await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 2,
            timeout_seconds: 30,
            ..WflConfig::default()
        });
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let started = Instant::now();
        let url = format!("http://127.0.0.1:{}/delayed-head", upstream.port);
        let mut opening =
            Box::pin(client.open_http_stream("GET", &url, &[], None, Arc::clone(&budget)));
        tokio::select! {
            result = opening.as_mut() => {
                panic!("stream opened before the response-head latch: {result:?}")
            }
            received = &mut upstream.request_received => {
                received.expect("upstream did not receive request");
            }
        }
        tokio::time::sleep(Duration::from_millis(1_200)).await;
        upstream
            .release_head
            .take()
            .expect("head release")
            .send(())
            .expect("release response head");
        let (_, _, handle_id) = tokio::time::timeout(Duration::from_secs(2), opening.as_mut())
            .await
            .expect("stream did not open after response-head release")
            .expect("delayed-head stream open");

        let deadline = client
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .live
            .get(&handle_id)
            .and_then(|slot| slot.deadline)
            .expect("positive cap must register a slot deadline");
        assert!(
            deadline.saturating_duration_since(started) <= Duration::from_millis(2_100),
            "the total deadline was restarted after the delayed head"
        );
        assert!(
            deadline.saturating_duration_since(Instant::now()) < Duration::from_secs(1),
            "the delayed head must leave less than one second of the original cap"
        );

        tokio::time::timeout(Duration::from_secs(2), &mut upstream.peer_closed)
            .await
            .expect("spawned reaper did not drop delayed-head upstream")
            .expect("upstream close notifier ended early");
        let error = client
            .next_chunk(&handle_id, budget)
            .await
            .expect_err("read after delayed-head expiry must fail");
        assert!(
            matches!(error, HttpClientError::Timeout { seconds: 2 }),
            "delayed-head expiry lost its typed timeout: {error:?}"
        );
        assert_reapers_drained(&client).await;
    }

    #[tokio::test]
    async fn spawned_reaper_wins_over_a_simultaneously_ready_chunk() {
        let (port, mut peer_closed) = spawn_stalled_streams(1).await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 1,
            timeout_seconds: 30,
            ..WflConfig::default()
        });
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let (_, _, handle_id) = client
            .open_http_stream(
                "GET",
                &format!("http://127.0.0.1:{port}/ready-expiry-race"),
                &[],
                None,
                Arc::clone(&budget),
            )
            .await
            .expect("open stalled stream");
        let state = GatedChunkState::new();
        let cancel = {
            let mut registry = client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let slot = registry.live.get_mut(&handle_id).expect("live stream slot");
            let handle = slot.handle.as_mut().expect("parked stream body");
            handle.stream = Box::pin(GatedChunkStream {
                state: Arc::clone(&state),
                yielded: false,
            });
            // Leave the slot deadline and real spawned reaper intact, but keep
            // the inner read timeout from independently deciding this race.
            handle.total_deadline = None;
            Arc::clone(&slot.cancel)
        };
        tokio::time::timeout(Duration::from_secs(2), peer_closed.recv())
            .await
            .expect("replacing the real body did not drop its upstream socket")
            .expect("upstream close notifier ended early");

        let mut read = Box::pin(client.next_chunk(&handle_id, budget));
        futures_util::future::poll_fn(|cx| match read.as_mut().poll(cx) {
            Poll::Pending => Poll::Ready(()),
            Poll::Ready(result) => panic!("gated read unexpectedly completed: {result:?}"),
        })
        .await;
        assert!(
            state.polled.load(Ordering::SeqCst),
            "gated body was not polled"
        );
        assert!(
            client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .live
                .get(&handle_id)
                .is_some_and(|slot| slot.handle.is_none()),
            "active read did not take ownership before expiry"
        );

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if cancel.terminal() == Some(StreamTerminal::Timeout)
                    && client.active_stream_reapers.load(Ordering::SeqCst) == 0
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the production reaper did not establish Timeout");
        state.make_ready();

        let error = tokio::time::timeout(Duration::from_secs(2), read.as_mut())
            .await
            .expect("simultaneously-ready race did not resolve")
            .expect_err("expiry must win over the ready body chunk");
        assert!(
            matches!(error, HttpClientError::Timeout { seconds: 1 }),
            "ready chunk beat the spawned reaper: {error:?}"
        );
        assert!(
            state.dropped.load(Ordering::SeqCst),
            "expired active body was not dropped"
        );
        let registry = client
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert!(
            !registry.live.contains_key(&handle_id),
            "expired body was reinserted after the reaper"
        );
        assert_eq!(
            client.active_stream_reapers.load(Ordering::SeqCst),
            0,
            "spawned reaper survived the race"
        );
    }

    #[tokio::test]
    async fn observed_clean_eof_wins_over_a_later_deadline_claim() {
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 0,
            timeout_seconds: 10,
            ..WflConfig::default()
        });
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let handle_id = "observed-clean-eof".to_string();
        let cancel = StreamCancel::new();
        let owner: StreamOwner =
            Arc::new(std::sync::Mutex::new(HashSet::from([handle_id.clone()])));

        client
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .live
            .insert(
                handle_id.clone(),
                StreamSlot {
                    handle: Some(HttpStreamHandle {
                        stream: Box::pin(futures_util::stream::empty::<reqwest::Result<Vec<u8>>>()),
                        buffer: b"abc".to_vec(),
                        done: false,
                        bytes_read: 3,
                        total_deadline: None,
                    }),
                    deadline: None,
                    cancel,
                    reaper_abort: None,
                    owner: Some(Arc::clone(&owner)),
                },
            );

        let Some(TakenStream { mut handle, cancel }) = client
            .take_stream(&handle_id)
            .expect("take synthetic stream")
        else {
            panic!("synthetic stream unexpectedly resolved as clean EOF");
        };

        assert!(
            !client
                .stream_pull(&mut handle, &budget, &cancel)
                .await
                .expect("observe clean EOF"),
            "the empty body must report clean EOF"
        );
        assert!(handle.done, "observing EOF must mark the body complete");

        let winner = cancel.terminate(StreamTerminal::Timeout);
        assert_eq!(
            winner,
            StreamTerminal::CleanEof,
            "a deadline claim after the upstream yielded EOF must not overwrite clean EOF"
        );

        let final_line = std::mem::take(&mut handle.buffer);
        client
            .put_stream(&handle_id, handle, &cancel)
            .expect("terminalize the EOF-latched body");
        assert_eq!(final_line, b"abc");
        assert!(
            owner
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .is_empty(),
            "terminalization must remove the stream owner"
        );

        {
            let registry = client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            assert!(
                !registry.live.contains_key(&handle_id),
                "terminalization must remove the live stream slot"
            );
            assert_eq!(
                registry
                    .recent
                    .iter()
                    .filter(|entry| {
                        entry.id == handle_id && entry.reason == StreamTerminal::CleanEof
                    })
                    .count(),
                1,
                "terminalization must retain exactly one one-shot clean-EOF record"
            );
        }

        assert_eq!(
            client
                .next_line(&handle_id, budget)
                .await
                .expect("consume one-shot clean EOF"),
            None
        );
    }

    #[tokio::test]
    async fn put_stream_restores_clean_eof_after_close_removed_the_live_slot() {
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 0,
            timeout_seconds: 10,
            ..WflConfig::default()
        });
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let handle_id = "clean-eof-after-close";
        let owner = install_owned_empty_stream(&client, handle_id);
        let TakenStream { handle, cancel } =
            take_and_observe_clean_eof(&client, handle_id, &budget).await;

        assert!(
            client.finish_stream_slot_sync(handle_id, StreamTerminal::Closed),
            "close must remove the live slot while the EOF-observing read owns its body"
        );
        assert_eq!(
            cancel.terminal(),
            Some(StreamTerminal::CleanEof),
            "the later close claim must not overwrite the observed CleanEof"
        );

        client
            .put_stream(handle_id, handle, &cancel)
            .expect("put_stream must restore CleanEof after close removed the slot");
        assert_one_shot_clean_eof_after_put(&client, handle_id, &owner, budget).await;
    }

    #[tokio::test]
    async fn put_stream_deduplicates_clean_eof_after_reaper_removed_the_live_slot() {
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 0,
            timeout_seconds: 10,
            ..WflConfig::default()
        });
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let handle_id = "clean-eof-after-reaper";
        let owner = install_owned_empty_stream(&client, handle_id);
        let TakenStream { handle, cancel } =
            take_and_observe_clean_eof(&client, handle_id, &budget).await;

        // Mirror the production reaper's critical section without a wall-clock
        // sleep: remove the slot, attempt Timeout, clear ownership, and retain
        // the first-wins terminal outcome before the active read calls put.
        {
            let now = Instant::now();
            let mut registry = client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            registry.prune_recent(now);
            let mut slot = registry
                .live
                .remove(handle_id)
                .expect("reaper-style terminalization must claim the live slot");
            let terminal = slot.cancel.terminate(StreamTerminal::Timeout);
            assert_eq!(
                terminal,
                StreamTerminal::CleanEof,
                "a reaper Timeout after observed EOF must retain CleanEof"
            );
            slot.reaper_abort = None;
            drop(slot.handle.take());
            remove_stream_owner(&mut slot, handle_id);
            registry.remember_recent(handle_id.to_string(), terminal, now);
        }

        client
            .put_stream(handle_id, handle, &cancel)
            .expect("put_stream must preserve CleanEof after the reaper removed the slot");
        assert_one_shot_clean_eof_after_put(&client, handle_id, &owner, budget).await;
    }

    #[test]
    fn extreme_outbound_stream_max_seconds_does_not_panic() {
        // u64::MAX must remain a finite cap rather than panicking or silently
        // disabling the hard lifetime.
        let before = Instant::now();
        let extreme = outbound_stream_deadline(u64::MAX)
            .expect("an extreme positive cap must still produce a finite deadline");
        let effective = extreme.saturating_duration_since(before);
        assert!(
            effective <= Duration::from_secs(MAX_OUTBOUND_STREAM_DEADLINE_SECS + 1),
            "extreme values must be clamped to the documented implementation ceiling; \
             got {effective:?}"
        );
        assert_eq!(
            outbound_stream_effective_seconds(u64::MAX),
            Some(MAX_OUTBOUND_STREAM_DEADLINE_SECS),
            "timeout diagnostics must report the effective clamp, not u64::MAX"
        );
        assert!(
            outbound_stream_deadline(0).is_none(),
            "0 is the documented sentinel for no absolute total cap"
        );
        assert!(
            outbound_stream_deadline(1).is_some(),
            "a normal positive cap must produce a deadline"
        );
        assert!(
            outbound_stream_deadline(MAX_OUTBOUND_STREAM_DEADLINE_SECS).is_some(),
            "the clamp ceiling itself must still produce a deadline"
        );
    }

    #[tokio::test]
    async fn final_unterminated_line_survives_deadline_after_clean_eof() {
        let port = spawn_stream_cleanup_upstream(1).await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 1,
            timeout_seconds: 10,
            ..WflConfig::default()
        });
        let interpreter = Interpreter::with_config(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let (_, _, handle) = tokio::time::timeout(
            Duration::from_secs(3),
            interpreter.io_client.open_http_stream(
                "GET",
                &format!("http://127.0.0.1:{port}/unterminated"),
                &[],
                None,
                Arc::clone(&budget),
            ),
        )
        .await
        .expect("open stream hung")
        .expect("open stream");
        interpreter
            .io_client
            .claim_stream_owner(
                &handle,
                &Arc::clone(&interpreter.open_http_streams.borrow()),
            )
            .expect("claim unterminated stream ownership");

        let first = tokio::time::timeout(
            Duration::from_secs(3),
            interpreter
                .io_client
                .next_line(&handle, Arc::clone(&budget)),
        )
        .await
        .expect("first line read hung")
        .expect("first line read");
        assert_eq!(
            first.as_deref(),
            Some("abc"),
            "the final unterminated line is returned only after clean EOF was observed"
        );

        let (live_slots, clean_eof_records) = {
            let registry = interpreter
                .io_client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            (
                registry.live.len(),
                registry
                    .recent
                    .iter()
                    .filter(|entry| entry.reason == StreamTerminal::CleanEof)
                    .count(),
            )
        };
        assert_eq!(
            live_slots, 0,
            "returning the final unterminated line must remove its live stream slot"
        );
        assert_eq!(
            clean_eof_records, 1,
            "the final line must leave exactly one lightweight clean-EOF record"
        );
        assert_eq!(
            interpreter
                .open_http_streams
                .borrow()
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .len(),
            0,
            "returning the final line must remove handler ownership immediately"
        );

        tokio::time::sleep(Duration::from_millis(1_100)).await;
        let eof = tokio::time::timeout(
            Duration::from_secs(2),
            interpreter
                .io_client
                .next_line(&handle, Arc::clone(&budget)),
        )
        .await
        .expect("clean EOF read hung")
        .expect("clean EOF observed before the cap must not become Timeout");
        assert_eq!(eof, None);

        let later = interpreter
            .io_client
            .next_line(&handle, budget)
            .await
            .expect_err("the single clean-EOF result must consume the handle");
        assert!(
            matches!(&later, HttpClientError::Request(message) if message.contains("already-closed")),
            "a later read must retain the established closed-handle error, got {later:?}"
        );
    }

    #[tokio::test]
    async fn unconsumed_clean_eof_records_are_bounded() {
        const STREAM_COUNT: usize = MAX_RECENT_STREAM_TERMINALS + 8;

        let port = spawn_stream_cleanup_upstream(STREAM_COUNT).await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 60 * 60,
            timeout_seconds: 10,
            ..WflConfig::default()
        });
        let interpreter = Interpreter::with_config(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));

        for sequence in 0..STREAM_COUNT {
            let (_, _, handle) = interpreter
                .io_client
                .open_http_stream(
                    "GET",
                    &format!("http://127.0.0.1:{port}/unterminated"),
                    &[],
                    None,
                    Arc::clone(&budget),
                )
                .await
                .expect("open unterminated stream");
            interpreter
                .io_client
                .claim_stream_owner(
                    &handle,
                    &Arc::clone(&interpreter.open_http_streams.borrow()),
                )
                .expect("claim unterminated stream ownership");

            let final_line = interpreter
                .io_client
                .next_line(&handle, Arc::clone(&budget))
                .await
                .expect("read final unterminated line");
            assert_eq!(
                final_line.as_deref(),
                Some("abc"),
                "stream {sequence} did not yield its final unterminated line"
            );

            let (live_slots, recent_records, all_clean_eof) = {
                let registry = interpreter
                    .io_client
                    .stream_handles
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                (
                    registry.live.len(),
                    registry.recent.len(),
                    registry
                        .recent
                        .iter()
                        .all(|entry| entry.reason == StreamTerminal::CleanEof),
                )
            };
            assert_eq!(
                live_slots, 0,
                "stream {sequence} retained a live slot after its final line"
            );
            assert_eq!(
                recent_records,
                (sequence + 1).min(MAX_RECENT_STREAM_TERMINALS),
                "clean-EOF records must fill only the bounded recent queue"
            );
            assert!(
                all_clean_eof,
                "the no-follow-up wave retained a non-clean-EOF terminal"
            );
            assert!(
                interpreter
                    .open_http_streams
                    .borrow()
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .is_empty(),
                "stream {sequence} retained handler ownership after its final line"
            );
        }
    }

    #[tokio::test]
    async fn unread_expired_stream_metadata_and_ownership_are_bounded() {
        const EXPECTED_RECENT_TIMEOUT_CAPACITY: usize = 64;
        const STREAM_COUNT: usize = EXPECTED_RECENT_TIMEOUT_CAPACITY + 8;

        let (port, mut peer_closed) = spawn_stalled_streams(STREAM_COUNT).await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 2,
            timeout_seconds: 10,
            ..WflConfig::default()
        });
        let interpreter = Interpreter::with_config(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let mut handles = Vec::with_capacity(STREAM_COUNT);

        for sequence in 0..STREAM_COUNT {
            let (_, _, handle) = interpreter
                .io_client
                .open_http_stream(
                    "GET",
                    &format!("http://127.0.0.1:{port}/stall/{sequence}"),
                    &[],
                    None,
                    Arc::clone(&budget),
                )
                .await
                .expect("open stalled stream");
            interpreter
                .io_client
                .claim_stream_owner(
                    &handle,
                    &Arc::clone(&interpreter.open_http_streams.borrow()),
                )
                .expect("claim stalled stream ownership");
            handles.push(handle);
        }

        tokio::time::timeout(Duration::from_secs(5), async {
            for _ in 0..STREAM_COUNT {
                peer_closed
                    .recv()
                    .await
                    .expect("upstream close notification");
            }
        })
        .await
        .expect("expired stream bodies were not dropped promptly");
        assert_reapers_drained(&interpreter.io_client).await;

        let (live_slots, retained_terminals) = {
            let registry = interpreter
                .io_client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            (registry.live.len(), registry.recent.len())
        };
        let retained_ownership = interpreter
            .open_http_streams
            .borrow()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len();
        assert_eq!(
            live_slots, 0,
            "expired stream bodies must leave no live registry slots"
        );
        assert!(
            retained_terminals <= EXPECTED_RECENT_TIMEOUT_CAPACITY,
            "recent terminal records must have a hard ceiling of \
             {EXPECTED_RECENT_TIMEOUT_CAPACITY}, got {retained_terminals}"
        );
        assert_eq!(
            retained_ownership, 0,
            "the reaper must remove expired ids from handler ownership immediately"
        );

        let newest = handles.last().expect("newest stream");
        let recent = interpreter
            .io_client
            .next_chunk(newest, budget)
            .await
            .expect_err("a recent expired stream must retain its typed terminal");
        assert!(
            matches!(recent, HttpClientError::Timeout { seconds: 2 }),
            "a recent read-after-expiry must report Timeout, got {recent:?}"
        );
    }

    #[tokio::test]
    async fn eof_error_and_rapid_close_cancel_reaper_tasks_on_a_retained_runtime() {
        const RAPID_CLOSES: usize = 40;
        let port = spawn_stream_cleanup_upstream(RAPID_CLOSES + 2).await;
        let config = Arc::new(WflConfig {
            outbound_stream_max_seconds: 60 * 60,
            timeout_seconds: 10,
            ..WflConfig::default()
        });
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));

        for sequence in 0..RAPID_CLOSES {
            let (_, _, handle) = client
                .open_http_stream(
                    "GET",
                    &format!("http://127.0.0.1:{port}/close/{sequence}"),
                    &[],
                    None,
                    Arc::clone(&budget),
                )
                .await
                .expect("open stream for explicit close");
            assert!(
                client.finish_stream_slot(&handle).await,
                "explicit close should remove its live stream"
            );
        }

        let (_, _, eof_handle) = client
            .open_http_stream(
                "GET",
                &format!("http://127.0.0.1:{port}/empty"),
                &[],
                None,
                Arc::clone(&budget),
            )
            .await
            .expect("open empty stream");
        assert_eq!(
            client
                .next_chunk(&eof_handle, Arc::clone(&budget))
                .await
                .expect("clean EOF"),
            None
        );

        let (_, _, error_handle) = client
            .open_http_stream(
                "GET",
                &format!("http://127.0.0.1:{port}/truncated"),
                &[],
                None,
                Arc::clone(&budget),
            )
            .await
            .expect("open truncated stream");
        let first = client.next_chunk(&error_handle, Arc::clone(&budget)).await;
        let terminal = match first {
            Ok(Some(_)) => client.next_chunk(&error_handle, budget).await,
            other => other,
        };
        assert!(
            matches!(terminal, Err(HttpClientError::Request(_))),
            "a truncated body should end as a network read error, got {terminal:?}"
        );

        // Keep this Tokio runtime alive while observing the task count. Runtime
        // shutdown would cancel leaked timers and make this regression false-green.
        assert_reapers_drained(&client).await;
        assert!(
            client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .live
                .is_empty(),
            "EOF, error, and explicit close must remove every stream slot"
        );
    }

    #[test]
    fn a_ready_read_result_cannot_reinsert_after_expiry_claims_the_slot() {
        let client = IoClient::new(Arc::new(WflConfig {
            outbound_stream_max_seconds: 1,
            ..WflConfig::default()
        }));
        let handle_id = "httpstream-race";
        let cancel = StreamCancel::new();
        client
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .live
            .insert(
                handle_id.to_string(),
                StreamSlot {
                    handle: Some(HttpStreamHandle {
                        // Model a network chunk that became ready while the read
                        // owned the handle.
                        stream: Box::pin(futures_util::stream::iter([Ok(vec![1u8])])),
                        buffer: vec![1],
                        done: false,
                        bytes_read: 1,
                        total_deadline: outbound_stream_deadline(1),
                    }),
                    deadline: outbound_stream_deadline(1),
                    cancel: Arc::clone(&cancel),
                    reaper_abort: None,
                    owner: None,
                },
            );

        let Some(TakenStream { handle, cancel }) = client
            .take_stream(handle_id)
            .expect("active read takes body")
        else {
            panic!("live stream unexpectedly reported clean EOF");
        };
        cancel.terminate(StreamTerminal::Timeout);
        let result = client.put_stream(handle_id, handle, &cancel);

        assert!(
            matches!(result, Err(HttpClientError::Timeout { .. })),
            "expiry must win over a ready body result, got {result:?}"
        );
        assert!(
            !client
                .stream_handles
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .live
                .contains_key(handle_id),
            "an expired handle must never be reinserted after its active read"
        );
    }
}

#[cfg(test)]
mod header_lookup_tests {
    use super::*;

    fn text(s: &str) -> Value {
        Value::Text(Arc::from(s))
    }

    fn headers_with(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), text(v)))
            .collect()
    }

    #[test]
    fn exact_key_match_returns_value() {
        let headers = headers_with(&[("user-agent", "exact")]);
        let got = lookup_header_case_insensitive(&headers, "user-agent");
        assert!(
            matches!(got, Some(Value::Text(t)) if t.as_ref() == "exact"),
            "exact key should hit without scanning"
        );
    }

    #[test]
    fn canonical_name_finds_lowercase_warp_key() {
        // Regression: warp normalizes to lowercase; programs use "User-Agent".
        let headers = headers_with(&[("user-agent", "wfl-header-test")]);
        let got = lookup_header_case_insensitive(&headers, "User-Agent");
        assert!(
            matches!(got, Some(Value::Text(t)) if t.as_ref() == "wfl-header-test"),
            "header \"User-Agent\" must resolve the lowercase 'user-agent' entry"
        );
    }

    #[test]
    fn mixed_case_lookup_against_mixed_case_key() {
        let headers = headers_with(&[("Content-Type", "application/json")]);
        let got = lookup_header_case_insensitive(&headers, "content-type");
        assert!(
            matches!(got, Some(Value::Text(t)) if t.as_ref() == "application/json"),
            "lookup should be case-insensitive in both directions"
        );
    }

    #[test]
    fn uppercase_lookup_against_lowercase_key() {
        let headers = headers_with(&[("x-custom-header", "present")]);
        let got = lookup_header_case_insensitive(&headers, "X-CUSTOM-HEADER");
        assert!(
            matches!(got, Some(Value::Text(t)) if t.as_ref() == "present"),
            "full uppercase name should still match"
        );
    }

    #[test]
    fn missing_header_returns_none() {
        let headers = headers_with(&[("user-agent", "bot")]);
        let got = lookup_header_case_insensitive(&headers, "Accept");
        assert!(
            got.is_none(),
            "absent header should return None (maps to nothing)"
        );
    }

    #[test]
    fn empty_headers_map_returns_none() {
        let headers = HashMap::new();
        assert!(lookup_header_case_insensitive(&headers, "User-Agent").is_none());
    }
}

#[cfg(test)]
mod file_read_tests {
    use super::*;
    use crate::exec::budget::BudgetLimits;

    fn budget_with_file_limit(limit: usize) -> ExecutionBudget {
        let limits = BudgetLimits {
            max_file_read_bytes: limit,
            ..BudgetLimits::default()
        };
        ExecutionBudget::new(limits)
    }

    #[tokio::test]
    async fn capped_read_stops_an_infinite_source_at_limit_plus_one() {
        let budget = budget_with_file_limit(8);
        let mut source = tokio::io::repeat(0xA5);
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            read_to_end_capped(&mut source, &budget, "test read"),
        )
        .await
        .expect("bounded read must not wait for EOF from an infinite source");

        assert!(matches!(
            result,
            Err(FileReadError::Budget(BudgetExceeded::FileReadBytes {
                limit: 8,
                actual: 9
            }))
        ));
    }

    /// Regression (nightly 2026-07-29): the interpreter must start on a machine
    /// whose CA trust store is empty.
    ///
    /// `reqwest::Client::new()` unwraps the TLS builder, and that builder fails
    /// with "No CA certificates were loaded from the system" on an image that
    /// ships no `ca-certificates` — so building the client in `IoClient::new`
    /// aborted the process before the first statement of *every* program, even
    /// one that never makes a request. The static-musl portability gate caught
    /// it on `debian:12-slim` the first night it ran. Keep construction lazy.
    #[test]
    fn io_client_new_does_not_build_the_http_client() {
        let client = IoClient::new(Arc::new(WflConfig::default()));
        assert!(
            client.http_client.get().is_none(),
            "IoClient::new must not construct the HTTP client: doing so makes \
             interpreter start-up depend on the system CA trust store"
        );
    }

    /// The lazily-built client is still a *shared* client: one connection pool
    /// per interpreter, as before. Callers must not get a fresh client (and a
    /// fresh pool) per request.
    ///
    /// Construction is a *checked* failure here, not an `expect`: this test has
    /// to keep passing on exactly the host the laziness exists for — one whose
    /// trust store is empty and where no client can be built at all. There is
    /// no client to memoize on such a host, so the reuse property is vacuous;
    /// what must hold instead is that the failure is an ordinary
    /// `HttpClientError` and that nothing was cached, so a later call on a
    /// repaired host can still succeed.
    #[test]
    fn http_client_is_built_once_and_reused() {
        let client = IoClient::new(Arc::new(WflConfig::default()));
        let first = match client.http_client() {
            Ok(first) => first,
            Err(error) => {
                assert!(
                    matches!(&error, HttpClientError::Request(message)
                        if message.contains("Could not create HTTP client")),
                    "a TLS stack that cannot produce a client must surface as a \
                     checked request error, got {error:?}"
                );
                assert!(
                    client.http_client.get().is_none(),
                    "a failed build must not memoize anything, or a host whose \
                     trust store is fixed mid-run could never build a client"
                );
                return;
            }
        };
        let second = client
            .http_client()
            .expect("a client that built once must build again");
        assert!(
            std::ptr::eq(first, second),
            "the HTTP client must be memoized, not rebuilt per call"
        );
    }

    /// The other half of the same regression: deferring construction must not
    /// turn a broken TLS stack into a *different* abort. A request is the first
    /// thing that forces the client to be built, so that build's failure has to
    /// travel the ordinary outbound-HTTP error path — a `RuntimeError` that
    /// `try`/`when` can catch — and never a panic that takes the process down.
    ///
    /// The program below deliberately has no `try`/`when`: the interpreter must
    /// still start, execute, and *return* the error. The request targets a port
    /// nothing is listening on, so it always fails; which failure it is depends
    /// on the host (connection refused, or "Could not create HTTP client" where
    /// the trust store is empty), and either way it must arrive as a returned
    /// `RuntimeError`.
    #[tokio::test]
    async fn a_request_reports_client_failure_as_a_catchable_runtime_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("reserve a port");
        let port = listener.local_addr().expect("reserved address").port();
        drop(listener);

        let source =
            format!("open url at \"http://127.0.0.1:{port}/\" and read content as reply\n");
        let tokens = crate::lexer::lex_wfl_with_positions(&source);
        let program = crate::parser::Parser::new(&tokens)
            .parse()
            .unwrap_or_else(|errors| panic!("fixture did not parse: {errors:?}"));

        let mut interpreter = Interpreter::new();
        let errors = interpreter
            .interpret(&program)
            .await
            .expect_err("a request to a dead port cannot succeed");

        let first = errors.first().expect("a returned runtime error");
        assert!(
            first.message.contains(&format!("127.0.0.1:{port}"))
                || first.message.contains("Could not create HTTP client"),
            "the returned error must come from the request that forced client \
             construction: {first:?}"
        );
        assert!(
            matches!(first.kind, ErrorKind::General),
            "an outbound request failure must stay an ordinary catchable error: {first:?}"
        );
        assert!(
            first.line > 0,
            "the failure must be attributed to the requesting statement: {first:?}"
        );
    }

    #[tokio::test]
    async fn capped_read_allows_a_payload_exactly_at_the_limit() {
        let budget = budget_with_file_limit(8);
        let mut source = std::io::Cursor::new(b"12345678".to_vec());
        let bytes = read_to_end_capped(&mut source, &budget, "test read")
            .await
            .expect("an exact-limit payload is valid");
        assert_eq!(bytes, b"12345678");
    }

    #[tokio::test]
    async fn text_and_binary_read_all_share_the_budget_ceiling() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("oversized.dat");
        std::fs::write(&path, b"123456789").expect("fixture");
        let path = path.to_string_lossy();
        let client = IoClient::new(Arc::new(WflConfig::default()));
        let budget = budget_with_file_limit(8);

        let text_handle = client
            .open_file_with_mode(&path, FileOpenMode::Read)
            .await
            .expect("open text fixture");
        assert!(matches!(
            client.read_file(&text_handle, &budget).await,
            Err(FileReadError::Budget(BudgetExceeded::FileReadBytes { .. }))
        ));
        client.close_file(&text_handle).await.expect("close text");

        let binary_handle = client
            .open_file_with_mode(&path, FileOpenMode::ReadBinary)
            .await
            .expect("open binary fixture");
        assert!(matches!(
            client.read_binary(&binary_handle, &budget).await,
            Err(FileReadError::Budget(BudgetExceeded::FileReadBytes { .. }))
        ));
        client
            .close_file(&binary_handle)
            .await
            .expect("close binary");
    }

    #[tokio::test]
    async fn explicit_binary_count_is_checked_before_allocation() {
        let client = IoClient::new(Arc::new(WflConfig::default()));
        let budget = budget_with_file_limit(8);
        let result = client.read_binary_n("not-open", 9, &budget).await;
        assert!(matches!(
            result,
            Err(FileReadError::Budget(BudgetExceeded::FileReadBytes {
                limit: 8,
                actual: 9
            }))
        ));
    }
}

#[cfg(test)]
mod process_tests {
    use super::*;
    use crate::config::ShellExecutionMode;
    use crate::exec::budget::BudgetLimits;

    /// Config that permits subprocesses for lifecycle tests (not the secure default).
    fn permissive_process_config() -> Arc<WflConfig> {
        Arc::new(WflConfig {
            allow_shell_execution: true,
            shell_execution_mode: ShellExecutionMode::Sanitized,
            warn_on_shell_execution: false,
            ..Default::default()
        })
    }

    /// Invoke one ignored helper test in a fresh copy of this test binary. This
    /// is cross-platform and avoids depending on optional shell utilities in CI.
    fn test_helper_command(filter: &str) -> (String, Vec<String>) {
        let executable = std::env::current_exe()
            .expect("current test executable")
            .to_string_lossy()
            .into_owned();
        let arguments = vec![
            filter.to_string(),
            "--ignored".to_string(),
            "--nocapture".to_string(),
            "--test-threads=1".to_string(),
        ];
        (executable, arguments)
    }

    #[test]
    #[ignore = "subprocess fixture; invoked by foreground execution tests"]
    fn subprocess_test_helper_floods_stdout_and_stderr() {
        use std::io::Write as _;

        let stdout_writer = std::thread::spawn(|| {
            let chunk = [b'O'; 8192];
            let mut stdout = std::io::stdout().lock();
            for _ in 0..64 {
                stdout.write_all(&chunk).expect("write helper stdout");
            }
            stdout.flush().expect("flush helper stdout");
        });
        let stderr_writer = std::thread::spawn(|| {
            let chunk = [b'E'; 8192];
            let mut stderr = std::io::stderr().lock();
            for _ in 0..64 {
                stderr.write_all(&chunk).expect("write helper stderr");
            }
            stderr.flush().expect("flush helper stderr");
        });

        stdout_writer.join().expect("stdout helper thread");
        stderr_writer.join().expect("stderr helper thread");
    }

    #[test]
    #[ignore = "subprocess fixture; invoked by foreground execution tests"]
    fn subprocess_test_helper_stalls() {
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    #[ignore = "subprocess fixture; invoked by foreground execution tests"]
    fn subprocess_test_helper_short_stall() {
        std::thread::sleep(Duration::from_secs(2));
    }

    #[test]
    #[ignore = "subprocess fixture; invoked by foreground execution tests"]
    // This fixture must drop the descendant handle: waiting would close the
    // inherited-pipe window that the foreground cleanup regression exercises.
    #[allow(clippy::zombie_processes)]
    fn subprocess_test_helper_leaves_inherited_pipes_open() {
        let (command, arguments) = test_helper_command("subprocess_test_helper_short_stall");
        let _descendant = std::process::Command::new(command)
            .args(arguments)
            .spawn()
            .expect("spawn descendant that inherits stdout/stderr");
        // Drop the handle and return. The descendant deliberately outlives this
        // direct child while retaining its inherited stdout/stderr handles.
    }

    #[tokio::test]
    async fn test_default_config_blocks_direct_exec() {
        let client = IoClient::new(Arc::new(WflConfig::default()));
        let result = client
            .execute_command("echo", &["hello"], false, 0, 0)
            .await;
        assert!(
            result.is_err(),
            "Default config must deny direct-exec; got {:?}",
            result
        );
        let err = result.unwrap_err();
        let err = err.to_string();
        assert!(
            err.contains("security policy") || err.contains("blocked") || err.contains("disabled"),
            "Error should mention policy: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_default_config_blocks_shell_interpreter_args() {
        let client = IoClient::new(Arc::new(WflConfig::default()));
        #[cfg(windows)]
        let result = client
            .execute_command("cmd.exe", &["/C", "echo pwned"], false, 0, 0)
            .await;
        #[cfg(not(windows))]
        let result = client
            .execute_command("sh", &["-c", "echo pwned"], false, 0, 0)
            .await;
        assert!(
            result.is_err(),
            "Default config must deny shell-with-args bypass; got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        let client = IoClient::new(permissive_process_config());

        // Use a real platform binary (Windows has no standalone `echo.exe`)
        #[cfg(windows)]
        let result = client
            .execute_command("cmd.exe", &["/C", "echo hello"], false, 0, 0)
            .await;
        #[cfg(not(windows))]
        let result = client
            .execute_command("echo", &["hello"], false, 0, 0)
            .await;

        assert!(result.is_ok(), "Failed to execute command: {:?}", result);
        let (stdout, stderr, exit_code) = result.unwrap();
        assert!(stdout.contains("hello"), "Output should contain 'hello'");
        assert_eq!(exit_code, 0, "Exit code should be 0 for successful command");
        assert!(
            stderr.is_empty() || stderr.trim().is_empty(),
            "Stderr should be empty"
        );
    }

    #[tokio::test]
    async fn test_execute_command_bounds_stdout_and_stderr() {
        const STREAM_LIMIT: usize = 1024;

        let mut config = (*permissive_process_config()).clone();
        config.subprocess_config.max_buffer_size_bytes = STREAM_LIMIT;
        let client = IoClient::new(Arc::new(config));
        let (command, arguments) =
            test_helper_command("subprocess_test_helper_floods_stdout_and_stderr");
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();

        let (stdout, stderr, exit_code) = tokio::time::timeout(
            Duration::from_secs(10),
            client.execute_command(&command, &argument_refs, false, 0, 0),
        )
        .await
        .expect("chatty helper must not deadlock")
        .expect("chatty helper should execute");

        assert_eq!(exit_code, 0);
        assert!(
            stdout.len() <= STREAM_LIMIT,
            "stdout retained {} bytes, limit is {STREAM_LIMIT}",
            stdout.len()
        );
        assert!(
            stderr.len() <= STREAM_LIMIT,
            "stderr retained {} bytes, limit is {STREAM_LIMIT}",
            stderr.len()
        );
        assert!(
            !stdout.is_empty(),
            "the bounded stdout tail should be retained"
        );
        assert!(
            !stderr.is_empty(),
            "the bounded stderr tail should be retained"
        );
    }

    #[tokio::test]
    async fn test_execute_command_kills_stalled_child_on_cancellation() {
        let client = IoClient::new(permissive_process_config());
        let (command, arguments) = test_helper_command("subprocess_test_helper_stalls");
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let budget = Arc::new(ExecutionBudget::unlimited());
        let cancellation_budget = Arc::clone(&budget);

        let execution = ExecutionBudget::scope(
            Arc::clone(&budget),
            client.execute_command(&command, &argument_refs, false, 0, 0),
        );
        let cancel = async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancellation_budget.cancel();
        };

        let (result, ()) = tokio::time::timeout(Duration::from_secs(3), async {
            tokio::join!(execution, cancel)
        })
        .await
        .expect("cancellation must terminate and reap the stalled child");

        assert!(matches!(
            result,
            Err(ExecuteCommandError::Budget(BudgetExceeded::Cancelled))
        ));
    }

    #[tokio::test]
    async fn test_execute_command_kills_stalled_child_at_deadline() {
        let client = IoClient::new(permissive_process_config());
        let (command, arguments) = test_helper_command("subprocess_test_helper_stalls");
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let mut limits = BudgetLimits::unlimited();
        limits.max_duration = Some(Duration::from_millis(100));
        let budget = Arc::new(ExecutionBudget::new(limits));

        let result = tokio::time::timeout(
            Duration::from_secs(3),
            ExecutionBudget::scope(
                Arc::clone(&budget),
                client.execute_command(&command, &argument_refs, false, 0, 0),
            ),
        )
        .await
        .expect("deadline must terminate and reap the stalled child");

        assert!(matches!(
            result,
            Err(ExecuteCommandError::Budget(BudgetExceeded::Deadline { .. }))
        ));
    }

    #[tokio::test]
    async fn test_execute_command_has_finite_timeout_inside_main_loop() {
        let mut config = (*permissive_process_config()).clone();
        config.timeout_seconds = 1;
        let config = Arc::new(config);
        let client = IoClient::new(Arc::clone(&config));
        let budget = Arc::new(ExecutionBudget::from_config(&config));
        let _main_loop = budget.enter_main_loop();
        let (command, arguments) = test_helper_command("subprocess_test_helper_stalls");
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();

        let result = tokio::time::timeout(
            Duration::from_secs(3),
            ExecutionBudget::scope(
                Arc::clone(&budget),
                client.execute_command(&command, &argument_refs, false, 0, 0),
            ),
        )
        .await
        .expect("a main-loop command must retain a finite per-operation timeout");

        assert!(matches!(
            result,
            Err(ExecuteCommandError::Timeout { seconds: 1 })
        ));
    }

    #[tokio::test]
    async fn test_execute_command_deadline_covers_inherited_pipe_drain() {
        let client = IoClient::new(permissive_process_config());
        let (command, arguments) =
            test_helper_command("subprocess_test_helper_leaves_inherited_pipes_open");
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let mut limits = BudgetLimits::unlimited();
        limits.max_duration = Some(Duration::from_millis(300));
        let budget = Arc::new(ExecutionBudget::new(limits));

        let result = tokio::time::timeout(
            Duration::from_secs(3),
            ExecutionBudget::scope(
                Arc::clone(&budget),
                client.execute_command(&command, &argument_refs, false, 0, 0),
            ),
        )
        .await
        .expect("inherited pipes must not outlive the execution deadline");

        assert!(matches!(
            result,
            Err(ExecuteCommandError::Budget(BudgetExceeded::Deadline { .. }))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_spawn_and_kill_process() {
        let client = IoClient::new(permissive_process_config());

        // Unix-specific test using sleep command
        let proc_id = client
            .spawn_process("sleep", &["10"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        // Check that process is running
        assert!(
            client.is_process_running(&proc_id).await,
            "Process should be running"
        );

        // Kill the process
        client
            .kill_process(&proc_id)
            .await
            .expect("Failed to kill process");

        // Give it time to terminate
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Process should no longer be running
        assert!(
            !client.is_process_running(&proc_id).await,
            "Process should not be running after kill"
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_spawn_and_kill_process() {
        let client = IoClient::new(permissive_process_config());

        // Windows-specific test using timeout command
        let proc_id = client
            .spawn_process("timeout", &["10"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        // Check that process is running
        assert!(
            client.is_process_running(&proc_id).await,
            "Process should be running"
        );

        // Kill the process
        client
            .kill_process(&proc_id)
            .await
            .expect("Failed to kill process");

        // Give it time to terminate
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Process should no longer be running
        assert!(
            !client.is_process_running(&proc_id).await,
            "Process should not be running after kill"
        );
    }

    #[tokio::test]
    async fn test_capture_process_output() {
        let client = IoClient::new(permissive_process_config());

        #[cfg(windows)]
        let proc_id = client
            .spawn_process("cmd.exe", &["/C", "echo test output"], false, 0, 0)
            .await
            .expect("Failed to spawn process");
        #[cfg(not(windows))]
        let proc_id = client
            .spawn_process("echo", &["test output"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        // Give process time to complete and output to be captured
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let output = client
            .read_process_output(&proc_id)
            .await
            .expect("Failed to read process output");

        assert!(
            output.contains("test output"),
            "Output should contain 'test output'"
        );
    }

    #[tokio::test]
    async fn test_wait_for_process_completion() {
        let client = IoClient::new(permissive_process_config());

        #[cfg(windows)]
        let proc_id = client
            .spawn_process("cmd.exe", &["/C", "echo done"], false, 0, 0)
            .await
            .expect("Failed to spawn process");
        #[cfg(not(windows))]
        let proc_id = client
            .spawn_process("echo", &["done"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        let exit_code = client
            .wait_for_process(&proc_id)
            .await
            .expect("Failed to wait for process");

        assert_eq!(exit_code, 0, "Process should exit with code 0");
    }

    #[tokio::test]
    async fn test_command_not_found() {
        let client = IoClient::new(permissive_process_config());

        // With shell execution, the shell runs successfully but reports command not found
        // So we check for non-zero exit code or error in stderr
        let result = client
            .execute_command("nonexistent_command_xyz_123", &[], false, 0, 0)
            .await;

        // Shell execution succeeds, but command fails
        if let Ok((_stdout, stderr, exit_code)) = result {
            // Either non-zero exit code or error message in stderr
            assert!(
                exit_code != 0 || stderr.contains("not found") || stderr.contains("not recognized"),
                "Should indicate command failure - exit_code: {}, stderr: {}",
                exit_code,
                stderr
            );
        } else {
            // Or direct execution might fail
            assert!(result.is_err(), "Should fail when command doesn't exist");
        }
    }

    #[tokio::test]
    async fn test_invalid_process_id() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // Test invalid process ID handling
        let result = client.read_process_output("invalid_proc_id").await;

        assert!(result.is_err(), "Should fail for invalid process ID");
        let err = result.unwrap_err();
        assert!(
            err.contains("Invalid process ID"),
            "Error should indicate invalid process ID: {}",
            err
        );
    }
}
