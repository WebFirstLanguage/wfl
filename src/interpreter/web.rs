use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

// Type alias for complex pending response type
pub type PendingResponseSender = Arc<tokio::sync::Mutex<Option<oneshot::Sender<WflHttpResponse>>>>;

// Web server data structures
#[derive(Debug, Clone)]
pub struct WflHttpRequest {
    pub id: String,
    pub method: String,
    pub path: String,
    pub client_ip: String,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub response_sender: Arc<tokio::sync::Mutex<Option<oneshot::Sender<WflHttpResponse>>>>,
}

#[derive(Debug, Clone)]
pub struct WflHttpResponse {
    pub content: String,
    pub status: u16,
    pub content_type: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct WflWebServer {
    pub request_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<WflHttpRequest>>>,
    pub request_sender: mpsc::UnboundedSender<WflHttpRequest>,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
}

// Custom error type for warp rejections
#[derive(Debug)]
#[allow(dead_code)]
pub struct ServerError(pub String);

impl warp::reject::Reject for ServerError {}
