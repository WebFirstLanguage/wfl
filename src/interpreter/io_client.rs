use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::config::WflConfig;
use crate::interpreter::bounded_buffer;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::parser::ast::FileOpenMode;

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

#[allow(dead_code)]
pub struct IoClient {
    http_client: reqwest::Client,
    file_handles: Mutex<HashMap<String, (PathBuf, tokio::fs::File)>>,
    next_file_id: Mutex<usize>,
    process_handles: Mutex<HashMap<String, ProcessHandle>>,
    next_process_id: Mutex<usize>,
    config: Arc<WflConfig>,
}

impl IoClient {
    pub fn new(config: Arc<WflConfig>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            file_handles: Mutex::new(HashMap::new()),
            next_file_id: Mutex::new(1),
            process_handles: Mutex::new(HashMap::new()),
            next_process_id: Mutex::new(1),
            config,
        }
    }

    #[allow(dead_code)]
    pub async fn http_get(&self, url: &str) -> Result<String, String> {
        match self.http_client.get(url).send().await {
            Ok(response) => match response.text().await {
                Ok(text) => Ok(text),
                Err(e) => Err(format!("Failed to read response body: {e}")),
            },
            Err(e) => Err(format!("Failed to send HTTP GET request: {e}")),
        }
    }

    #[allow(dead_code)]
    pub async fn http_post(&self, url: &str, data: &str) -> Result<String, String> {
        match self
            .http_client
            .post(url)
            .body(data.to_string())
            .send()
            .await
        {
            Ok(response) => match response.text().await {
                Ok(text) => Ok(text),
                Err(e) => Err(format!("Failed to read response body: {e}")),
            },
            Err(e) => Err(format!("Failed to send HTTP POST request: {e}")),
        }
    }

    #[allow(dead_code)]
    pub async fn open_file(&self, path: &str) -> Result<String, String> {
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
    pub async fn open_file_with_mode(
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
    pub async fn read_file(&self, handle_id: &str) -> Result<String, String> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            drop(file_handles);

            match self.open_file(handle_id).await {
                Ok(new_handle) => {
                    // Now read from the new handle - use Box::pin to handle recursion in async fn
                    let future = Box::pin(self.read_file(&new_handle));
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

        let mut contents = String::new();
        match AsyncReadExt::read_to_string(&mut file_clone, &mut contents).await {
            Ok(_) => Ok(contents),
            Err(e) => Err(format!("Failed to read file: {e}")),
        }
    }

    /// Syncs file to disk with Windows-specific error handling.
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
    pub async fn write_file(&self, handle_id: &str, content: &str) -> Result<(), String> {
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

    #[allow(dead_code)]
    pub async fn close_file(&self, handle_id: &str) -> Result<(), String> {
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
    pub async fn append_file(&self, handle_id: &str, content: &str) -> Result<(), String> {
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
    pub async fn create_directory(&self, path: &str) -> Result<(), String> {
        match tokio::fs::create_dir_all(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to create directory: {e}")),
        }
    }

    #[allow(dead_code)]
    pub async fn create_file(&self, path: &str, content: &str) -> Result<(), String> {
        match tokio::fs::write(path, content).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to create file: {e}")),
        }
    }

    #[allow(dead_code)]
    pub async fn delete_file(&self, path: &str) -> Result<(), String> {
        match tokio::fs::remove_file(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete file: {e}")),
        }
    }

    #[allow(dead_code)]
    pub async fn delete_directory(&self, path: &str) -> Result<(), String> {
        match tokio::fs::remove_dir_all(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete directory: {e}")),
        }
    }

    // Subprocess management methods

    /// Execute a command and wait for it to complete, returning (stdout, stderr, exit_code)
    #[allow(dead_code)]
    pub async fn execute_command(
        &self,
        command: &str,
        args: &[&str],
        use_shell: bool,
        line: usize,
        column: usize,
    ) -> Result<(String, String, i32), String> {
        use crate::interpreter::command_sanitizer::{CommandSanitizer, ValidationResult};
        use tokio::process::Command;

        // Determine if shell execution is needed
        let needs_shell = use_shell
            || (args.is_empty() && CommandSanitizer::contains_shell_metacharacters(command));

        // Security validation if shell is needed
        if needs_shell {
            let sanitizer = CommandSanitizer::new(Arc::clone(&self.config));
            match sanitizer.validate_command(command)? {
                ValidationResult::Safe => {
                    // No shell needed after all
                }
                ValidationResult::RequiresShell { warnings, .. } => {
                    // Shell is needed, show warnings if configured
                    if self.config.warn_on_shell_execution {
                        eprintln!("⚠️  Security Warning (line {}, column {}):", line, column);
                        eprintln!("   Shell execution enabled for command: {}", command);
                        for warning in warnings {
                            eprintln!("   - {}", warning);
                        }
                        eprintln!("   Consider using 'with arguments' syntax for safer execution.");
                    }
                }
                ValidationResult::Blocked { reason } => {
                    return Err(format!(
                        "Command blocked by security policy: {}\n\
                         To allow shell execution, update the configuration in .wflcfg:\n\
                         shell_execution_mode = \"sanitized\"  # or \"unrestricted\"\n\
                         Or use safe execution: execute command \"program\" with arguments [\"arg1\", \"arg2\"]",
                        reason
                    ));
                }
            }
        }

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
            // Safe path: parse and execute directly
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

        // Execute the command
        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute command '{}': {}", command, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok((stdout, stderr, exit_code))
    }

    /// Spawn a background process and return a process ID
    #[allow(dead_code)]
    pub async fn spawn_process(
        &self,
        command: &str,
        args: &[&str],
        use_shell: bool,
        line: usize,
        column: usize,
    ) -> Result<String, String> {
        use crate::interpreter::command_sanitizer::{CommandSanitizer, ValidationResult};
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

        // Determine if shell execution is needed
        let needs_shell = use_shell
            || (args.is_empty() && CommandSanitizer::contains_shell_metacharacters(command));

        // Security validation if shell is needed
        if needs_shell {
            let sanitizer = CommandSanitizer::new(Arc::clone(&self.config));
            match sanitizer.validate_command(command)? {
                ValidationResult::Safe => {
                    // No shell needed after all
                }
                ValidationResult::RequiresShell { warnings, .. } => {
                    // Shell is needed, show warnings if configured
                    if self.config.warn_on_shell_execution {
                        eprintln!("⚠️  Security Warning (line {}, column {}):", line, column);
                        eprintln!("   Shell execution enabled for command: {}", command);
                        for warning in warnings {
                            eprintln!("   - {}", warning);
                        }
                        eprintln!("   Consider using 'with arguments' syntax for safer execution.");
                    }
                }
                ValidationResult::Blocked { reason } => {
                    return Err(format!(
                        "Command blocked by security policy: {}\n\
                         To allow shell execution, update the configuration in .wflcfg:\n\
                         shell_execution_mode = \"sanitized\"  # or \"unrestricted\"\n\
                         Or use safe execution: spawn command \"program\" with arguments [\"arg1\", \"arg2\"] as proc_id",
                        reason
                    ));
                }
            }
        }

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
            // Safe path: parse and execute directly
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
    pub async fn cleanup_completed_processes(&self) {
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
    pub async fn read_process_output(&self, process_id: &str) -> Result<String, String> {
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
    pub async fn kill_process(&self, process_id: &str) -> Result<(), String> {
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
    pub async fn wait_for_process(&self, process_id: &str) -> Result<i32, String> {
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
    pub async fn is_process_running(&self, process_id: &str) -> bool {
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
                for (_id, handle) in handles.iter_mut() {
                    let _ = handle.child.start_kill();
                }
            }

            handles.clear();
        }
    }
}
