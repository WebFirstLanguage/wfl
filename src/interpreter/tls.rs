use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::future::Future;
use std::io::{self, BufReader};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, ready};

use futures_util::future::{AbortHandle, Abortable};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::server::TlsStream as EstablishedTlsStream;
use tokio_rustls::{Accept as PendingTlsHandshake, TlsAcceptor};
use warp::hyper::rt::Executor;
use warp::hyper::server::accept::Accept;
use warp::hyper::server::conn::{AddrIncoming, AddrStream};

enum SecuredStreamState {
    Handshaking(PendingTlsHandshake<AddrStream>),
    Established(EstablishedTlsStream<AddrStream>),
}

/// A TCP connection whose TLS handshake is driven by Hyper's per-connection
/// task. Keeping the handshake out of `poll_accept` prevents an incomplete
/// ClientHello from blocking unrelated clients.
pub(super) struct SecuredStream {
    state: SecuredStreamState,
    remote_addr: SocketAddr,
}

impl SecuredStream {
    fn new(stream: AddrStream, config: Arc<rustls::ServerConfig>) -> Self {
        let remote_addr = stream.remote_addr();
        let handshake = TlsAcceptor::from(config).accept(stream);
        Self {
            state: SecuredStreamState::Handshaking(handshake),
            remote_addr,
        }
    }

    pub(super) fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl AsyncRead for SecuredStream {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                SecuredStreamState::Handshaking(handshake) => {
                    let stream = ready!(Pin::new(handshake).poll(context))?;
                    this.state = SecuredStreamState::Established(stream);
                }
                SecuredStreamState::Established(stream) => {
                    return Pin::new(stream).poll_read(context, buffer);
                }
            }
        }
    }
}

impl AsyncWrite for SecuredStream {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                SecuredStreamState::Handshaking(handshake) => {
                    let stream = ready!(Pin::new(handshake).poll(context))?;
                    this.state = SecuredStreamState::Established(stream);
                }
                SecuredStreamState::Established(stream) => {
                    return Pin::new(stream).poll_write(context, bytes);
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().state {
            SecuredStreamState::Handshaking(_) => Poll::Ready(Ok(())),
            SecuredStreamState::Established(stream) => Pin::new(stream).poll_flush(context),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().state {
            SecuredStreamState::Handshaking(_) => Poll::Ready(Ok(())),
            SecuredStreamState::Established(stream) => Pin::new(stream).poll_shutdown(context),
        }
    }
}

struct TrackedConnectionInner {
    cancelled: AtomicBool,
    next_id: AtomicU64,
    tasks: Mutex<HashMap<u64, AbortHandle>>,
}

/// Tracks every connection task that Hyper launches for a secured listener.
///
/// Hyper normally detaches these tasks from its outer `Server` future. WFL
/// controls a listener through that outer task, so retaining abort handles is
/// necessary for `close server` and interpreter teardown to cancel accepted
/// connections, including clients parked in a partial TLS handshake.
#[derive(Clone)]
pub(super) struct TrackedConnections {
    inner: Arc<TrackedConnectionInner>,
}

impl TrackedConnections {
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(TrackedConnectionInner {
                cancelled: AtomicBool::new(false),
                next_id: AtomicU64::new(0),
                tasks: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub(super) fn executor(&self) -> TrackedConnectionExecutor {
        TrackedConnectionExecutor {
            connections: self.clone(),
        }
    }

    pub(super) fn cancel_on_drop(self) -> CancelConnectionsOnDrop {
        CancelConnectionsOnDrop(self)
    }

    fn cancel_all(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        let handles = {
            let mut tasks = self
                .inner
                .tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            tasks.drain().map(|(_, handle)| handle).collect::<Vec<_>>()
        };
        for handle in handles {
            handle.abort();
        }
    }

    #[cfg(test)]
    fn active_task_count(&self) -> usize {
        self.inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

impl fmt::Debug for TrackedConnections {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let active_tasks = self
            .inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();
        formatter
            .debug_struct("TrackedConnections")
            .field("cancelled", &self.inner.cancelled.load(Ordering::Acquire))
            .field("active_tasks", &active_tasks)
            .finish()
    }
}

pub(super) struct CancelConnectionsOnDrop(TrackedConnections);

impl Drop for CancelConnectionsOnDrop {
    fn drop(&mut self) {
        self.0.cancel_all();
    }
}

#[derive(Clone, Debug)]
pub(super) struct TrackedConnectionExecutor {
    connections: TrackedConnections,
}

struct TrackedTaskRegistration {
    id: u64,
    inner: Arc<TrackedConnectionInner>,
}

impl Drop for TrackedTaskRegistration {
    fn drop(&mut self) {
        self.inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.id);
    }
}

impl<F> Executor<F> for TrackedConnectionExecutor
where
    F: Future<Output = ()> + Send + 'static,
{
    fn execute(&self, future: F) {
        if self.connections.inner.cancelled.load(Ordering::Acquire) {
            return;
        }

        let id = self
            .connections
            .inner
            .next_id
            .fetch_add(1, Ordering::Relaxed);
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        {
            let mut tasks = self
                .connections
                .inner
                .tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            // Recheck while holding the task-map lock so `cancel_all` cannot
            // drain the map between this check and insertion.
            if self.connections.inner.cancelled.load(Ordering::Acquire) {
                return;
            }
            tasks.insert(id, abort_handle);
        }

        let inner = Arc::clone(&self.connections.inner);
        tokio::spawn(async move {
            let _registration = TrackedTaskRegistration { id, inner };
            let _ = Abortable::new(future, abort_registration).await;
        });
    }
}

/// Hyper incoming transport backed by the patched Rustls stack.
pub(super) struct SecuredIncoming {
    config: Arc<rustls::ServerConfig>,
    incoming: AddrIncoming,
}

impl SecuredIncoming {
    pub(super) fn bind(
        addr: SocketAddr,
        config: rustls::ServerConfig,
    ) -> Result<(SocketAddr, Self), warp::hyper::Error> {
        let mut incoming = AddrIncoming::bind(&addr)?;
        incoming.set_nodelay(true);
        let bound_addr = incoming.local_addr();
        Ok((
            bound_addr,
            Self {
                config: Arc::new(config),
                incoming,
            },
        ))
    }
}

impl Accept for SecuredIncoming {
    type Conn = SecuredStream;
    type Error = io::Error;

    fn poll_accept(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let this = self.get_mut();
        match ready!(Pin::new(&mut this.incoming).poll_accept(context)) {
            Some(Ok(stream)) => Poll::Ready(Some(Ok(SecuredStream::new(
                stream,
                Arc::clone(&this.config),
            )))),
            Some(Err(error)) => Poll::Ready(Some(Err(error))),
            None => Poll::Ready(None),
        }
    }
}

pub(super) fn load_server_config(
    cert_path: &str,
    key_path: &str,
) -> Result<rustls::ServerConfig, String> {
    let cert_file = File::open(cert_path).map_err(|error| {
        format!(
            "Cannot open TLS certificate file '{cert_path}': {error}. For local development you can create a self-signed certificate with: openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 365 -subj \"/CN=localhost\""
        )
    })?;
    let certs = rustls_pemfile::certs(&mut BufReader::new(cert_file))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("TLS certificate file '{cert_path}' is not valid PEM: {error}"))?;
    if certs.is_empty() {
        return Err(format!(
            "TLS certificate file '{cert_path}' contains no certificates. Expected at least one PEM 'CERTIFICATE' block"
        ));
    }

    let key_file = File::open(key_path)
        .map_err(|error| format!("Cannot open TLS private key file '{key_path}': {error}"))?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))
        .map_err(|error| {
            format!("TLS private key file '{key_path}' is not valid PEM: {error}")
        })?
        .ok_or_else(|| {
            format!(
                "TLS private key file '{key_path}' contains no private key. Expected a PEM 'PRIVATE KEY', 'RSA PRIVATE KEY', or 'EC PRIVATE KEY' block"
            )
        })?;

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| format!("Failed to configure safe TLS protocol versions: {error}"))?;
    let mut config = builder
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|error| {
            format!(
                "TLS certificate '{cert_path}' and private key '{key_path}' are not a valid pair: {error}"
            )
        })?;

    // Match Warp's prior TLS behavior and ordering: prefer HTTP/2, with a
    // standards-compatible HTTP/1.1 fallback.
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn panicking_tracked_task_is_unregistered() {
        let connections = TrackedConnections::new();
        connections.executor().execute(async {
            panic!("intentional tracked-task panic");
        });

        for _ in 0..100 {
            if connections.active_task_count() == 0 {
                return;
            }
            tokio::task::yield_now().await;
        }

        assert_eq!(
            connections.active_task_count(),
            0,
            "A panicking task must not leave a dead abort handle registered"
        );
    }
}
