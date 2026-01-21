use std::cell::RefCell;
use std::collections::HashMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use warp::Filter;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::interpreter::value::Value;
use crate::interpreter::web::{ServerError, WflHttpRequest, WflHttpResponse, WflWebServer};
use crate::parser::ast::Expression;

pub trait WebExecutor {
    async fn execute_listen(
        &self,
        port: &Expression,
        server_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_wait_for_request(
        &self,
        server: &Expression,
        request_name: &str,
        timeout: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_respond(
        &self,
        request: &Expression,
        content: &Expression,
        status: Option<&Expression>,
        content_type: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_stop_accepting_connections(
        &self,
        server: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_close_server(
        &self,
        server: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_http_get(
        &self,
        url: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_http_post(
        &self,
        url: &Expression,
        data: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_register_signal_handler(
        &self,
        signal_type: &str,
        handler_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl WebExecutor for Interpreter {
    async fn execute_listen(
        &self,
        port: &Expression,
        server_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let port_val = self.evaluate_expression(port, Rc::clone(&env)).await?;
        let port_num = match &port_val {
            Value::Number(n) => *n as u16,
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected number for port, got {port_val:?}"),
                    line,
                    column,
                ));
            }
        };

        // Create request/response channels
        let (request_sender, request_receiver) = mpsc::unbounded_channel::<WflHttpRequest>();
        let request_receiver = Arc::new(tokio::sync::Mutex::new(request_receiver));

        // Create warp routes that handle all HTTP methods and paths
        // Note: Body size validation is performed manually in the handler below
        // to allow GET requests without Content-Length headers
        let request_sender_clone = request_sender.clone();
        let routes = warp::any()
            .and(warp::method())
            .and(warp::path::full())
            .and(warp::header::headers_cloned())
            .and(warp::body::bytes())
            .and(warp::addr::remote())
            .and_then(
                move |method: warp::http::Method,
                      path: warp::path::FullPath,
                      headers: warp::http::HeaderMap,
                      body: bytes::Bytes,
                      remote_addr: Option<std::net::SocketAddr>| {
                    let sender = request_sender_clone.clone();
                    async move {
                        // DoS PROTECTION: Enforce 1MB body size limit
                        // This maintains the security requirement from web_server_body_limit_test.wfl
                        const MAX_BODY_SIZE: usize = 1_048_576; // 1MB
                        if body.len() > MAX_BODY_SIZE {
                            return Err(warp::reject::custom(ServerError(format!(
                                "Request body too large: {} bytes (limit: {} bytes)",
                                body.len(),
                                MAX_BODY_SIZE
                            ))));
                        }

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

                        // Convert body to string
                        let body_str = String::from_utf8_lossy(&body).to_string();

                        // Create response channel
                        let (response_sender, response_receiver) =
                            oneshot::channel::<WflHttpResponse>();

                        // Create WFL request
                        let wfl_request = WflHttpRequest {
                            id: request_id,
                            method: method.to_string(),
                            path: path.as_str().to_string(),
                            client_ip,
                            body: body_str,
                            headers: header_map,
                            response_sender: Arc::new(tokio::sync::Mutex::new(Some(
                                response_sender,
                            ))),
                        };

                        // Send request to WFL interpreter
                        if sender.send(wfl_request).is_err() {
                            return Err(warp::reject::custom(ServerError(
                                "Request channel closed".to_string(),
                            )));
                        }

                        // Wait for response
                        match response_receiver.await {
                            Ok(response) => {
                                let status_code = warp::http::StatusCode::from_u16(response.status)
                                    .unwrap_or(warp::http::StatusCode::OK);

                                // Convert content to bytes for accurate Content-Length calculation
                                // HTTP Content-Length must match exact byte count of body
                                let content_bytes = response.content.into_bytes();
                                let content_length = content_bytes.len();

                                let mut reply_builder = warp::http::Response::builder()
                                    .status(status_code)
                                    .header("Content-Type", response.content_type)
                                    .header("Content-Length", content_length);

                                // Add additional headers
                                for (name, value) in response.headers {
                                    reply_builder = reply_builder.header(name, value);
                                }

                                match reply_builder.body(content_bytes) {
                                    Ok(response) => Ok(response),
                                    Err(_) => Err(warp::reject::custom(ServerError(
                                        "Failed to build response".to_string(),
                                    ))),
                                }
                            }
                            Err(_) => Err(warp::reject::custom(ServerError(
                                "Response channel closed".to_string(),
                            ))),
                        }
                    }
                },
            );

        // Parse the bind address from config
        let bind_addr: IpAddr = match self.config.web_server_bind_address.parse() {
            Ok(addr) => addr,
            Err(_) => {
                return Err(RuntimeError::new(
                    format!(
                        "Invalid web_server_bind_address in config: '{}'. Expected a valid IP address (e.g., '127.0.0.1' or '0.0.0.0')",
                        self.config.web_server_bind_address
                    ),
                    line,
                    column,
                ));
            }
        };

        // Start the server
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
                };

                // Store the server in the interpreter
                self.web_servers
                    .borrow_mut()
                    .insert(server_name.to_string(), wfl_server);

                // Create a server value with the actual address
                let server_value = Value::Text(Rc::from(format!(
                    "WebServer::{}:{}",
                    addr.ip(),
                    addr.port()
                )));

                println!("Server is listening on port {}", addr.port());

                match env.borrow_mut().define(server_name, server_value) {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, line, column)),
                }
            }
            Err(e) => Err(RuntimeError::new(
                format!("Failed to start web server on port {}: {}", port_num, e),
                line,
                column,
            )),
        }
    }

    async fn execute_wait_for_request(
        &self,
        server: &Expression,
        request_name: &str,
        timeout: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
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
                    for (server_name, _) in web_servers.iter() {
                        // Get the stored value for this server name
                        if let Some(Value::Text(stored_text)) = env.borrow().get(server_name)
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
                            line,
                            column,
                        ));
                    }
                } else {
                    name_str.to_string()
                }
            }
            _ => {
                return Err(RuntimeError::new(
                    "Expected server name as text".to_string(),
                    line,
                    column,
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
                    line,
                    column,
                ));
            }
        };

        // Wait for a request to come in (with optional timeout)
        let request = {
            let mut receiver = request_receiver.lock().await;

            // Evaluate timeout if provided
            let timeout_duration = if let Some(timeout_expr) = timeout {
                let timeout_val = self
                    .evaluate_expression(timeout_expr, Rc::clone(&env))
                    .await?;
                match timeout_val {
                    Value::Number(ms) if ms > 0.0 => {
                        Some(std::time::Duration::from_millis(ms as u64))
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Timeout must be a positive number (milliseconds)".to_string(),
                            line,
                            column,
                        ));
                    }
                }
            } else {
                None
            };

            // Wait for request with or without timeout
            if let Some(duration) = timeout_duration {
                match tokio::time::timeout(duration, receiver.recv()).await {
                    Ok(Some(req)) => req,
                    Ok(None) => {
                        return Err(RuntimeError::new(
                            "Request channel closed".to_string(),
                            line,
                            column,
                        ));
                    }
                    Err(_) => {
                        return Err(RuntimeError::new(
                            format!("Timeout waiting for request ({} ms)", duration.as_millis()),
                            line,
                            column,
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
                            line,
                            column,
                        ));
                    }
                }
            }
        };

        // Store the request in a global map for RespondStatement to access
        {
            let mut pending_responses = self.pending_responses.borrow_mut();
            pending_responses.insert(request.id.clone(), request.response_sender);
        }

        // Define individual variables for request properties (more natural for WFL)
        let mut env_mut = env.borrow_mut();

        // Define the main request variable (for use in respond statements)
        let mut request_properties = HashMap::new();
        request_properties.insert(
            "_response_sender".to_string(),
            Value::Text(Rc::from(request.id.clone())),
        );
        let request_object = Value::Object(Rc::new(RefCell::new(request_properties)));

        match env_mut.define(request_name, request_object) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        // Define individual request property variables
        match env_mut.define("method", Value::Text(Rc::from(request.method.clone()))) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        match env_mut.define("path", Value::Text(Rc::from(request.path.clone()))) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        match env_mut.define(
            "client_ip",
            Value::Text(Rc::from(request.client_ip.clone())),
        ) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        match env_mut.define("body", Value::Text(Rc::from(request.body.clone()))) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        // Convert headers to WFL object and define as headers variable
        let mut headers_map = HashMap::new();
        for (key, value) in request.headers.iter() {
            headers_map.insert(key.clone(), Value::Text(Rc::from(value.clone())));
        }
        let headers_object = Value::Object(Rc::new(RefCell::new(headers_map)));

        match env_mut.define("headers", headers_object) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        drop(env_mut); // Release the borrow

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_respond(
        &self,
        request: &Expression,
        content: &Expression,
        status: Option<&Expression>,
        content_type: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Get the request object
        let request_val = self.evaluate_expression(request, Rc::clone(&env)).await?;
        let request_id = match &request_val {
            Value::Object(obj) => {
                let obj_ref = obj.borrow();
                match obj_ref.get("_response_sender") {
                    Some(Value::Text(id)) => id.as_ref().to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Request object missing response sender ID".to_string(),
                            line,
                            column,
                        ));
                    }
                }
            }
            _ => {
                return Err(RuntimeError::new(
                    "Expected request object".to_string(),
                    line,
                    column,
                ));
            }
        };

        // Evaluate response content
        let content_val = self.evaluate_expression(content, Rc::clone(&env)).await?;
        let content_str = match &content_val {
            Value::Text(text) => text.as_ref().to_string(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => format!("{:?}", content_val),
        };

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
                        line,
                        column,
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
                        line,
                        column,
                    ));
                }
            }
        } else {
            "text/plain".to_string() // Default content type
        };

        // Create response
        let response = WflHttpResponse {
            content: content_str,
            status: status_code,
            content_type: content_type_str,
            headers: HashMap::new(), // TODO: Add support for custom headers
        };

        // Send response
        let response_sender = {
            let mut pending = self.pending_responses.borrow_mut();
            pending.remove(&request_id)
        };

        if let Some(sender_arc) = response_sender {
            let mut sender_opt = sender_arc.lock().await;
            if let Some(sender) = sender_opt.take() {
                if sender.send(response).is_err() {
                    return Err(RuntimeError::new(
                        "Failed to send response - client may have disconnected".to_string(),
                        line,
                        column,
                    ));
                }
            } else {
                return Err(RuntimeError::new(
                    "Response already sent for this request".to_string(),
                    line,
                    column,
                ));
            }
        } else {
            return Err(RuntimeError::new(
                "Request ID not found - response may have already been sent".to_string(),
                line,
                column,
            ));
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_stop_accepting_connections(
        &self,
        server: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
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
                            line,
                            column,
                        ));
                    }
                } else {
                    name_str.to_string()
                }
            }
            _ => {
                return Err(RuntimeError::new(
                    "Expected server name as text".to_string(),
                    line,
                    column,
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
            .map_err(|e| RuntimeError::new(e, line, column))?;

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_close_server(
        &self,
        server: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let server_val = self.evaluate_expression(server, Rc::clone(&env)).await?;
        let server_name = match &server_val {
            Value::Text(name) => {
                let name_str = name.as_ref();
                if name_str.starts_with("WebServer::") {
                    // Find the server name that corresponds to this WebServer value
                    let web_servers = self.web_servers.borrow();

                    // Search through all servers to find which one has this exact value
                    let mut found_server = None;
                    for (server_name, _) in web_servers.iter() {
                        // Check if this server name's variable has the matching value
                        if let Some(Value::Text(stored_text)) = env.borrow().get(server_name)
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
                            line,
                            column,
                        ));
                    }
                } else {
                    name_str.to_string()
                }
            }
            _ => {
                return Err(RuntimeError::new(
                    "Expected server name as text".to_string(),
                    line,
                    column,
                ));
            }
        };

        // Close the server
        let mut web_servers = self.web_servers.borrow_mut();
        if let Some(wfl_server) = web_servers.remove(&server_name) {
            // Graceful shutdown: Give in-flight responses time to complete transmission
            // before forcefully aborting the server task
            if let Some(handle) = wfl_server.server_handle {
                // Allow 50ms for pending HTTP responses to be transmitted
                // This prevents race condition where abort() closes the TCP connection
                // before response bytes reach the client, causing IncompleteMessage errors
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                handle.abort();
            }
        } else {
            return Err(RuntimeError::new(
                format!("Server '{}' not found", server_name),
                line,
                column,
            ));
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_http_get(
        &self,
        url: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
        let url_str = match &url_val {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for URL, got {url_val:?}"),
                    line,
                    column,
                ));
            }
        };

        match self.io_client.http_get(&url_str).await {
            Ok(body) => {
                match env
                    .borrow_mut()
                    .define(variable_name, Value::Text(body.into()))
                {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, line, column)),
                }
            }
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }

    async fn execute_http_post(
        &self,
        url: &Expression,
        data: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
        let data_val = self.evaluate_expression(data, Rc::clone(&env)).await?;

        let url_str = match &url_val {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for URL, got {url_val:?}"),
                    line,
                    column,
                ));
            }
        };

        let data_str = match &data_val {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for data, got {data_val:?}"),
                    line,
                    column,
                ));
            }
        };

        match self.io_client.http_post(&url_str, &data_str).await {
            Ok(body) => {
                match env
                    .borrow_mut()
                    .define(variable_name, Value::Text(body.into()))
                {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, line, column)),
                }
            }
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }

    async fn execute_register_signal_handler(
        &self,
        signal_type: &str,
        handler_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // For now, just store the signal handler registration
        // In a full implementation, this would set up actual signal handlers
        let signal_handler_key = format!("signal_handler_{}", signal_type);

        env.borrow_mut()
            .define(
                &signal_handler_key,
                Value::Text(Rc::from(handler_name.to_string())),
            )
            .map_err(|e| RuntimeError::new(e, line, column))?;

        // TODO: Implement actual signal handling with tokio::signal
        // For now, we'll simulate this in the graceful shutdown test

        Ok((Value::Null, ControlFlow::None))
    }
}
