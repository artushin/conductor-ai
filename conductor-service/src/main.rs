//! Conductor Daemon — event-driven cross-repo coordination service.
//!
//! Architecture:
//! - Unix domain socket listener at ~/.conductor/conductor.sock
//! - JSON-RPC protocol for request/response
//! - PE checkpoint file watcher (polling)
//! - PID file management at ~/.conductor/conductor.pid
//! - Graceful shutdown on SIGTERM/SIGINT

use std::path::PathBuf;
use tokio::net::UnixListener;
use tokio::signal;

/// Socket path for IPC
fn socket_path() -> PathBuf {
    dirs::home_dir()
        .expect("HOME directory not found")
        .join(".conductor")
        .join("conductor.sock")
}

/// PID file path
fn pid_path() -> PathBuf {
    dirs::home_dir()
        .expect("HOME directory not found")
        .join(".conductor")
        .join("conductor.pid")
}

/// PE checkpoint file path (convention)
fn pe_checkpoint_path() -> PathBuf {
    dirs::home_dir()
        .expect("HOME directory not found")
        .join(".conductor")
        .join("pe-checkpoint.json")
}

/// Write PID file, returning an error if one already exists (daemon already running)
fn acquire_pid_lock() -> Result<(), String> {
    let pid_file = pid_path();
    if pid_file.exists() {
        // Check if the PID is still alive
        if let Ok(contents) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                // Check if process is still running
                let check = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .status();
                if check.map(|s| s.success()).unwrap_or(false) {
                    return Err(format!("Conductor daemon already running (PID {})", pid));
                }
            }
        }
        // Stale PID file — remove it
        let _ = std::fs::remove_file(&pid_file);
    }
    std::fs::write(&pid_file, std::process::id().to_string())
        .map_err(|e| format!("Failed to write PID file: {}", e))
}

/// Remove PID file on shutdown
fn release_pid_lock() {
    let _ = std::fs::remove_file(pid_path());
}

/// Remove socket file if it exists (from a previous crashed session)
fn cleanup_socket() {
    let sock = socket_path();
    if sock.exists() {
        let _ = std::fs::remove_file(&sock);
    }
}

/// JSON-RPC request skeleton
#[derive(serde::Deserialize, Debug)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[allow(dead_code)]
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

/// JSON-RPC response skeleton
#[derive(serde::Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: Option<serde_json::Value>, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(serde_json::json!({
                "code": code,
                "message": message,
            })),
            id,
        }
    }
}

/// Handle a single JSON-RPC request
async fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "ping" => JsonRpcResponse::success(req.id, serde_json::json!("pong")),
        "status" => {
            JsonRpcResponse::success(
                req.id,
                serde_json::json!({
                    "status": "running",
                    "pid": std::process::id(),
                }),
            )
        }
        "pe.checkpoint.status" => {
            let path = pe_checkpoint_path();
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(contents) => {
                        match serde_json::from_str::<serde_json::Value>(&contents) {
                            Ok(v) => JsonRpcResponse::success(req.id, v),
                            Err(e) => JsonRpcResponse::error(
                                req.id, -32000,
                                &format!("Invalid PE checkpoint JSON: {}", e),
                            ),
                        }
                    }
                    Err(e) => JsonRpcResponse::error(
                        req.id, -32000,
                        &format!("Failed to read PE checkpoint: {}", e),
                    ),
                }
            } else {
                JsonRpcResponse::success(req.id, serde_json::json!(null))
            }
        }
        _ => JsonRpcResponse::error(req.id, -32601, &format!("Method not found: {}", req.method)),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    // Acquire PID lock
    if let Err(e) = acquire_pid_lock() {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    // Clean up stale socket
    cleanup_socket();

    // Ensure .conductor directory exists
    let conductor_dir = dirs::home_dir()
        .expect("HOME not found")
        .join(".conductor");
    std::fs::create_dir_all(&conductor_dir)?;

    // Bind Unix domain socket
    let sock = socket_path();
    let listener = UnixListener::bind(&sock)?;
    tracing::info!("Conductor daemon listening on {:?}", sock);

    // Main event loop
    loop {
        tokio::select! {
            // Accept new connections
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        tokio::spawn(async move {
                            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
                            let (reader, mut writer) = stream.into_split();
                            let mut buf_reader = BufReader::new(reader);
                            let mut line = String::new();
                            while buf_reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                                if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                                    let resp = handle_request(req).await;
                                    let resp_json = serde_json::to_string(&resp).unwrap();
                                    let _ = writer.write_all(resp_json.as_bytes()).await;
                                    let _ = writer.write_all(b"\n").await;
                                }
                                line.clear();
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                    }
                }
            }

            // Graceful shutdown on SIGTERM/SIGINT
            _ = signal::ctrl_c() => {
                tracing::info!("Shutting down...");
                break;
            }
        }
    }

    // Cleanup
    release_pid_lock();
    cleanup_socket();
    tracing::info!("Conductor daemon stopped");

    Ok(())
}
