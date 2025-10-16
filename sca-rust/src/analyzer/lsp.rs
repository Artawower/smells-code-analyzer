use crate::config::AppConfig;
use anyhow::{anyhow, bail, Context, Result};
use lsp_types::{
    ClientCapabilities, ClientInfo, ConfigurationParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, Location, ReferenceContext, ReferenceParams,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url,
    WorkDoneProgressParams, WorkspaceFolder,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::task::JoinHandle;
use tree_sitter::Point;

const JSONRPC_VERSION: &str = "2.0";

pub struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr_task: Option<JoinHandle<()>>,
    request_id: u64,
    pending_requests: HashMap<u64, String>,
    workspace_folders: Vec<WorkspaceFolder>,
}

impl LspClient {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let mut command = Command::new(&config.lsp_executable);
        command.args(&config.lsp_args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.current_dir(&config.project_root_path);

        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn {}", config.lsp_executable))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("LSP process stdin not available"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("LSP process stdout not available"))?;

        let stderr = child.stderr.take();
        let stderr_task = stderr.map(spawn_stderr_logger);

        let workspace_uri = Url::from_directory_path(&config.project_root_path).map_err(|_| {
            anyhow!(
                "Failed to convert project root {} to URI",
                config.project_root_path.display()
            )
        })?;
        let workspace_folders = vec![WorkspaceFolder {
            uri: workspace_uri.clone(),
            name: "workspace".to_string(),
        }];

        let mut client = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            stderr_task,
            request_id: 1,
            pending_requests: HashMap::new(),
            workspace_folders,
        };

        client.initialize(config).await?;
        Ok(client)
    }

    async fn initialize(&mut self, config: &AppConfig) -> Result<()> {
        let capabilities = config
            .lsp_capabilities
            .clone()
            .map(|value| serde_json::from_value::<ClientCapabilities>(value))
            .transpose()
            .context("Failed to parse lspCapabilities")?
            .unwrap_or_default();

        let params = InitializeParams {
            process_id: Some(std::process::id()),
            client_info: Some(ClientInfo {
                name: config.lsp_name.clone(),
                version: Some(config.lsp_version.clone()),
            }),
            locale: None,
            root_uri: None,
            initialization_options: Some(config.initialization_options.clone()),
            capabilities,
            trace: None,
            workspace_folders: Some(self.workspace_folders.clone()),
            ..Default::default()
        };

        let response = self
            .send_request("initialize", serde_json::to_value(params)?)
            .await?;
        let _: Value = response;

        self.send_notification("initialized", Value::Null).await?;

        let config_change = json!({
            "settings": Value::Null
        });
        self.send_notification("workspace/didChangeConfiguration", config_change)
            .await?;

        // Some servers expect synthetic didOpen/didClose handshake to settle; small delay helps.
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    pub async fn did_open(
        &mut self,
        uri: &Url,
        language_id: &str,
        text: String,
        version: i32,
    ) -> Result<()> {
        let item = TextDocumentItem {
            uri: uri.clone(),
            language_id: language_id.to_string(),
            version,
            text,
        };
        let params = DidOpenTextDocumentParams {
            text_document: item,
        };
        self.send_notification("textDocument/didOpen", serde_json::to_value(params)?)
            .await
    }

    pub async fn did_close(&mut self, uri: &Url) -> Result<()> {
        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        };
        self.send_notification("textDocument/didClose", serde_json::to_value(params)?)
            .await
    }

    pub async fn references(&mut self, uri: &Url, position: Point) -> Result<usize> {
        const MAX_RETRIES: u32 = 3;
        const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: lsp_types::Position {
                    line: position.row as u32,
                    character: position.column as u32,
                },
            },
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: Default::default(),
        };

        let mut last_error = None;

        for attempt in 1..=MAX_RETRIES {
            let params_value = serde_json::to_value(&params)?;
            
            match tokio::time::timeout(
                REQUEST_TIMEOUT,
                self.send_request("textDocument/references", params_value),
            )
            .await
            {
                Ok(Ok(response)) => {
                    let locations: Option<Vec<Location>> =
                        serde_json::from_value(response).context("Invalid references response")?;
                    return Ok(locations.map(|v| v.len()).unwrap_or(0));
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "LSP references request failed (attempt {}/{}): {}",
                        attempt,
                        MAX_RETRIES,
                        e
                    );
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
                Err(_) => {
                    let timeout_err = anyhow!("LSP request timeout after {:?}", REQUEST_TIMEOUT);
                    tracing::warn!(
                        "LSP references request timed out (attempt {}/{})",
                        attempt,
                        MAX_RETRIES
                    );
                    last_error = Some(timeout_err);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.send_request("shutdown", Value::Null).await?;
        self.send_notification("exit", Value::Null).await?;
        if let Some(handle) = self.stderr_task.take() {
            handle.abort();
        }
        let status = self.child.wait().await?;
        if !status.success() {
            tracing::warn!("LSP process exited with {:?}", status);
        }
        Ok(())
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();
        self.pending_requests.insert(id, method.to_string());
        let payload = json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(&payload).await?;

        loop {
            let message = self.read_message().await?;
            match message {
                IncomingMessage::Response(resp) => {
                    let response_id = id_from(&resp.id)?;
                    if response_id != id {
                        tracing::debug!("Received out of order response for id {}", response_id);
                        continue;
                    }
                    self.pending_requests.remove(&id);
                    if let Some(error) = resp.error {
                        bail!("LSP error {}: {}", method, error.message);
                    }
                    return Ok(resp.result.unwrap_or(Value::Null));
                }
                IncomingMessage::Notification(notif) => {
                    self.handle_notification(notif).await?;
                }
                IncomingMessage::Request(req) => {
                    self.handle_server_request(req).await?;
                }
            }
        }
    }

    async fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let payload = json!({
            "jsonrpc": JSONRPC_VERSION,
            "method": method,
            "params": params,
        });
        self.write_message(&payload).await
    }

    async fn write_message(&mut self, value: &Value) -> Result<()> {
        let body = serde_json::to_vec(value)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(&body).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<IncomingMessage> {
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let read = self.stdout.read_line(&mut line).await?;
            if read == 0 {
                bail!("LSP server closed the stream");
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                let len = value
                    .trim()
                    .parse::<usize>()
                    .context("Invalid Content-Length")?;
                content_length = Some(len);
            }
        }

        let length = content_length.ok_or_else(|| anyhow!("Missing Content-Length header"))?;
        let mut buffer = vec![0u8; length];
        self.stdout.read_exact(&mut buffer).await?;
        let value: Value = serde_json::from_slice(&buffer).context("Invalid JSON payload")?;

        parse_message(value)
    }

    async fn handle_notification(&self, notif: NotificationMessage) -> Result<()> {
        let NotificationMessage { method, params, .. } = notif;
        match method.as_str() {
            "window/logMessage" => {
                if let Some(payload) = params {
                    tracing::debug!("LSP log: {}", payload);
                }
            }
            "textDocument/publishDiagnostics" => {
                // ignore diagnostics for now
            }
            "$/progress" => {}
            "telemetry/event" => {}
            other => {
                tracing::debug!("Unhandled LSP notification {other}");
            }
        }
        Ok(())
    }

    async fn handle_server_request(&mut self, req: RequestMessage) -> Result<()> {
        let RequestMessage {
            id, method, params, ..
        } = req;
        let result = match method.as_str() {
            "workspace/configuration" => {
                let params: ConfigurationParams = params
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or(ConfigurationParams { items: vec![] });
                Value::Array(vec![Value::Null; params.items.len()])
            }
            "workspace/workspaceFolders" => serde_json::to_value(&self.workspace_folders)?,
            "window/workDoneProgress/create" => Value::Null,
            other => {
                tracing::debug!("Unhandled LSP server request {other}");
                Value::Null
            }
        };
        self.send_response(id, result).await
    }

    async fn send_response(&mut self, id: Value, result: Value) -> Result<()> {
        let payload = json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": id,
            "result": result
        });
        self.write_message(&payload).await
    }

    fn next_id(&mut self) -> u64 {
        let id = self.request_id;
        self.request_id += 1;
        id
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        if let Some(handle) = self.stderr_task.take() {
            handle.abort();
        }
        let _ = self.child.start_kill();
    }
}

fn spawn_stderr_logger(stderr: ChildStderr) -> JoinHandle<()> {
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!("LSP stderr: {}", line);
        }
    })
}

#[derive(Debug)]
enum IncomingMessage {
    Response(ResponseMessage),
    Notification(NotificationMessage),
    Request(RequestMessage),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ResponseMessage {
    jsonrpc: String,
    id: Value,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<ResponseError>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RequestMessage {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ResponseError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct NotificationMessage {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

fn parse_message(value: Value) -> Result<IncomingMessage> {
    if value.get("method").is_some() {
        if value.get("id").is_some() {
            let msg: RequestMessage = serde_json::from_value(value)?;
            Ok(IncomingMessage::Request(msg))
        } else {
            let msg: NotificationMessage = serde_json::from_value(value)?;
            Ok(IncomingMessage::Notification(msg))
        }
    } else {
        let msg: ResponseMessage = serde_json::from_value(value)?;
        Ok(IncomingMessage::Response(msg))
    }
}

fn id_from(id: &Value) -> Result<u64> {
    match id {
        Value::Number(n) => n
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid numeric id value")),
        Value::String(s) => s
            .parse::<u64>()
            .map_err(|_| anyhow!("Expected numeric id, got {s}")),
        Value::Null => bail!("Null id is not supported"),
        _ => bail!("Unsupported id type {:?}", id),
    }
}
