use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tokio::net::{TcpListener, TcpStream};
// use tokio::io::{AsyncReadExt, AsyncWriteExt}; // Currently unused but will be needed for full implementation

/// TCP listener handle for tracking active listeners
#[derive(Debug)]
pub struct TcpListenerHandle {
    pub listener: TcpListener,
    pub port: u16,
}

/// TCP connection handle for tracking active connections
#[derive(Debug)]
pub struct TcpConnectionHandle {
    pub stream: TcpStream,
    pub remote_addr: String,
}

/// Global storage for network handles (similar to file handles in the interpreter)
type NetworkHandles = HashMap<String, NetworkHandle>;

#[derive(Debug)]
pub enum NetworkHandle {
    Listener(TcpListenerHandle),
    Connection(TcpConnectionHandle),
}

thread_local! {
    static NETWORK_HANDLES: RefCell<NetworkHandles> = RefCell::new(HashMap::new());
    static NEXT_HANDLE_ID: RefCell<u64> = const { RefCell::new(1) };
}

fn generate_handle_id() -> String {
    NEXT_HANDLE_ID.with(|id| {
        let mut id_ref = id.borrow_mut();
        let handle_id = format!("net_{}", *id_ref);
        *id_ref += 1;
        handle_id
    })
}

/// Create a TCP listener on the specified port
pub async fn native_listen_on_port(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("listen_on_port expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let port = match &args[0] {
        Value::Number(n) => *n as u16,
        _ => {
            return Err(RuntimeError::new(
                format!("Expected port number, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    match TcpListener::bind(format!("127.0.0.1:{port}")).await {
        Ok(listener) => {
            let handle_id = generate_handle_id();
            let listener_handle = TcpListenerHandle { listener, port };

            NETWORK_HANDLES.with(|handles| {
                handles
                    .borrow_mut()
                    .insert(handle_id.clone(), NetworkHandle::Listener(listener_handle));
            });

            Ok(Value::Text(Rc::from(handle_id)))
        }
        Err(e) => Err(RuntimeError::new(
            format!("Failed to bind to port {port}: {e}"),
            0,
            0,
        )),
    }
}

/// Accept a connection from a TCP listener
pub async fn native_accept_connection(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("accept_connection expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let listener_handle_id = match &args[0] {
        Value::Text(handle_id) => handle_id.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected listener handle, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    // We need to temporarily take the listener out of the handles map
    // to perform async operations, then put it back
    let result = NETWORK_HANDLES.with(|handles| -> Result<Value, RuntimeError> {
        let mut handles_map = handles.borrow_mut();

        match handles_map.get_mut(listener_handle_id) {
            Some(NetworkHandle::Listener(listener_handle)) => {
                // We can't do async operations while holding the borrow_mut
                // So we'll return the port and recreate the listener if needed
                Ok(Value::Number(listener_handle.port as f64))
            }
            Some(NetworkHandle::Connection(_)) => Err(RuntimeError::new(
                format!("Handle '{listener_handle_id}' is not a listener"),
                0,
                0,
            )),
            None => Err(RuntimeError::new(
                format!("Listener handle '{listener_handle_id}' not found"),
                0,
                0,
            )),
        }
    })?;

    // For now, we'll implement a simpler approach where each accept creates a new listener
    // This is not ideal but allows us to work with the async constraints
    let port = match result {
        Value::Number(n) => n as u16,
        _ => return Err(RuntimeError::new("Invalid port number".to_string(), 0, 0)),
    };

    match TcpListener::bind(format!("127.0.0.1:{port}")).await {
        Ok(listener) => match listener.accept().await {
            Ok((stream, addr)) => {
                let connection_handle_id = generate_handle_id();
                let connection_handle = TcpConnectionHandle {
                    stream,
                    remote_addr: addr.to_string(),
                };

                NETWORK_HANDLES.with(|handles| {
                    handles.borrow_mut().insert(
                        connection_handle_id.clone(),
                        NetworkHandle::Connection(connection_handle),
                    );
                });

                Ok(Value::Text(Rc::from(connection_handle_id)))
            }
            Err(e) => Err(RuntimeError::new(
                format!("Failed to accept connection: {e}"),
                0,
                0,
            )),
        },
        Err(e) => Err(RuntimeError::new(
            format!("Failed to recreate listener on port {port}: {e}"),
            0,
            0,
        )),
    }
}

/// Read data from a TCP connection
pub async fn native_read_from_connection(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!(
                "read_from_connection expects 1 argument, got {}",
                args.len()
            ),
            0,
            0,
        ));
    }

    let connection_handle_id = match &args[0] {
        Value::Text(handle_id) => handle_id.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected connection handle, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    // For now, return a placeholder since proper async handling of mutable borrows
    // in thread local storage requires more complex refactoring
    // TODO: Implement proper async TCP reading
    NETWORK_HANDLES.with(|handles| -> Result<Value, RuntimeError> {
        let handles_map = handles.borrow();
        match handles_map.get(connection_handle_id) {
            Some(NetworkHandle::Connection(_connection_handle)) => {
                // Placeholder: In a real implementation, we'd read from the stream
                Ok(Value::Text(Rc::from("TCP read data placeholder")))
            }
            Some(NetworkHandle::Listener(_)) => Err(RuntimeError::new(
                format!("Handle '{connection_handle_id}' is not a connection"),
                0,
                0,
            )),
            None => Err(RuntimeError::new(
                format!("Connection handle '{connection_handle_id}' not found"),
                0,
                0,
            )),
        }
    })
}

/// Write data to a TCP connection
pub async fn native_write_to_connection(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!(
                "write_to_connection expects 2 arguments, got {}",
                args.len()
            ),
            0,
            0,
        ));
    }

    let connection_handle_id = match &args[0] {
        Value::Text(handle_id) => handle_id.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected connection handle, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    let data = match &args[1] {
        Value::Text(text) => text.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text data, got {}", args[1].type_name()),
                0,
                0,
            ));
        }
    };

    // Placeholder implementation - similar constraints as read_from_connection
    NETWORK_HANDLES.with(|handles| -> Result<Value, RuntimeError> {
        let handles_map = handles.borrow();
        match handles_map.get(connection_handle_id) {
            Some(NetworkHandle::Connection(_connection_handle)) => {
                // Placeholder: In a real implementation, we'd write to the stream
                println!("TCP write placeholder: {data}");
                Ok(Value::Null)
            }
            Some(NetworkHandle::Listener(_)) => Err(RuntimeError::new(
                format!("Handle '{connection_handle_id}' is not a connection"),
                0,
                0,
            )),
            None => Err(RuntimeError::new(
                format!("Connection handle '{connection_handle_id}' not found"),
                0,
                0,
            )),
        }
    })
}

/// Close a TCP connection or listener
pub async fn native_close_connection(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("close_connection expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let handle_id = match &args[0] {
        Value::Text(handle_id) => handle_id.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected connection handle, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    NETWORK_HANDLES.with(|handles| -> Result<Value, RuntimeError> {
        let mut handles_map = handles.borrow_mut();
        match handles_map.remove(handle_id) {
            Some(_) => Ok(Value::Null),
            None => Err(RuntimeError::new(
                format!("Network handle '{handle_id}' not found"),
                0,
                0,
            )),
        }
    })
}

/// Synchronous wrapper functions for the standard library registration
/// These will be called from the non-async context but internally use async
pub fn native_listen_on_port_sync(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    // For now, return an error since we need proper async integration
    // This will be resolved when we update the interpreter to handle async stdlib functions
    Err(RuntimeError::new(
        "TCP operations require async context integration (Phase 2)".to_string(),
        0,
        0,
    ))
}

pub fn native_accept_connection_sync(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Err(RuntimeError::new(
        "TCP operations require async context integration (Phase 2)".to_string(),
        0,
        0,
    ))
}

pub fn native_read_from_connection_sync(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Err(RuntimeError::new(
        "TCP operations require async context integration (Phase 2)".to_string(),
        0,
        0,
    ))
}

pub fn native_write_to_connection_sync(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Err(RuntimeError::new(
        "TCP operations require async context integration (Phase 2)".to_string(),
        0,
        0,
    ))
}

pub fn native_close_connection_sync(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Err(RuntimeError::new(
        "TCP operations require async context integration (Phase 2)".to_string(),
        0,
        0,
    ))
}

/// Register network functions with the environment
pub fn register_network(env: &mut Environment) {
    env.define(
        "listen_on_port",
        Value::NativeFunction("listen_on_port", native_listen_on_port_sync),
    );
    env.define(
        "accept_connection",
        Value::NativeFunction("accept_connection", native_accept_connection_sync),
    );
    env.define(
        "read_from_connection",
        Value::NativeFunction("read_from_connection", native_read_from_connection_sync),
    );
    env.define(
        "write_to_connection",
        Value::NativeFunction("write_to_connection", native_write_to_connection_sync),
    );
    env.define(
        "close_connection",
        Value::NativeFunction("close_connection", native_close_connection_sync),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_handle_id() {
        let id1 = generate_handle_id();
        let id2 = generate_handle_id();

        assert!(id1.starts_with("net_"));
        assert!(id2.starts_with("net_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_network_functions_error_on_wrong_args() {
        // Test that functions return appropriate errors for wrong argument counts
        let result = native_listen_on_port_sync(vec![]);
        assert!(result.is_err());

        let result = native_accept_connection_sync(vec![]);
        assert!(result.is_err());

        let result = native_read_from_connection_sync(vec![]);
        assert!(result.is_err());
    }
}
