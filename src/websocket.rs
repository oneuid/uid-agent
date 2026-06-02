use std::sync::Arc;
use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::json;
use crate::crypto::AgentKeys;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct AgentWebSocketClient {
    keys: Arc<AgentKeys>,
    core_url: String,
}

impl AgentWebSocketClient {
    pub fn new(keys: Arc<AgentKeys>, core_url: String) -> Self {
        Self { keys, core_url }
    }

    pub async fn listen_and_sign(&self, challenge_token: &str, auth_token: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        // Construct WebSocket URL
        let ws_host = self.core_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        let ws_url = format!("{}/ws/challenges/{}/", ws_host, challenge_token);
        
        println!("[uid-agent] Connecting to WebSocket: {}", ws_url);
        
        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (_, mut read) = ws_stream.split();
        
        println!("[uid-agent] Connected to challenge channel. Waiting for requests...");

        let prompt_msg = format!("Do you approve the authorization challenge login request: {}?", challenge_token);
        if !prompt_gui_approval(&prompt_msg) {
            println!("[uid-agent] Challenge approval denied by user.");
            return Err("Challenge approval denied by user".into());
        }

        // Generate signature of the challenge token representing enclave presence
        let signature = self.keys.sign(challenge_token.as_bytes());
        let signature_base64 = base64_encode(&signature.to_bytes());
        
        println!("[uid-agent] Generated signature: {}", signature_base64);

        if let Some(token) = auth_token {
            let approve_url = format!("{}/api/v1/auth/challenges/{}/approve/", self.core_url, challenge_token);
            println!("[uid-agent] Posting approval to REST: {}", approve_url);
            
            let client = reqwest_client_simple()?;
            let response = client.post(&approve_url)
                .header("Authorization", &format!("Bearer {}", token))
                .json(&json!({
                    "encrypted_payload": signature_base64,
                }))
                .send()
                .await;

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        println!("[uid-agent] Challenge approved successfully via REST.");
                    } else {
                        let text = res.text().await.unwrap_or_default();
                        eprintln!("[uid-agent] Failed to approve challenge: {} - {}", approve_url, text);
                    }
                }
                Err(e) => {
                    eprintln!("[uid-agent] Error posting approval: {:?}", e);
                }
            }
        } else {
            println!("[uid-agent] No auth token provided. Skipping REST auto-approval. Use CLI 'sign' to output signature.");
        }

        // Keep listening for broadcasts on the websocket
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    println!("[uid-agent] Received WebSocket broadcast: {}", text);
                    if text.contains("APPROVED") {
                        println!("[uid-agent] Challenge approval confirmed. Closing connection.");
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("[uid-agent] WebSocket connection closed by remote host.");
                    break;
                }
                Err(e) => {
                    eprintln!("[uid-agent] WebSocket error: {:?}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

// Simple base64 encoder to avoid heavy dependencies
fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    
    let mut i = 0;
    while i < input.len() {
        let chunk = &input[i..std::cmp::min(i + 3, input.len())];
        let mut val = 0u32;
        for (idx, &byte) in chunk.iter().enumerate() {
            val |= (byte as u32) << (16 - idx * 8);
        }
        
        let chars_to_write = match chunk.len() {
            1 => 2,
            2 => 3,
            _ => 4,
        };
        
        for idx in 0..chars_to_write {
            let char_idx = ((val >> (18 - idx * 6)) & 0x3F) as usize;
            result.push(CHARSET[char_idx] as char);
        }
        
        for _ in chars_to_write..4 {
            result.push('=');
        }
        
        i += 3;
    }
    
    result
}

fn reqwest_client_simple() -> Result<SimpleHttpClient, Box<dyn std::error::Error>> {
    Ok(SimpleHttpClient {})
}

struct SimpleHttpClient;

impl SimpleHttpClient {
    fn post(&self, url: &str) -> SimpleHttpRequest {
        SimpleHttpRequest {
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
        }
    }
}

struct SimpleHttpRequest {
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

struct SimpleHttpResponse {
    status_code: u16,
    body: String,
}

impl SimpleHttpResponse {
    fn status(&self) -> SimpleHttpStatus {
        SimpleHttpStatus { code: self.status_code }
    }
    
    async fn text(self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.body)
    }
}

struct SimpleHttpStatus {
    code: u16,
}

impl SimpleHttpStatus {
    fn is_success(&self) -> bool {
        self.code >= 200 && self.code < 300
    }
}

impl SimpleHttpRequest {
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    fn json(mut self, val: &serde_json::Value) -> Self {
        self.body = Some(val.to_string());
        self.headers.push(("Content-Type".to_string(), "application/json".to_string()));
        self
    }

    async fn send(self) -> Result<SimpleHttpResponse, Box<dyn std::error::Error>> {
        // Parse URL manually to avoid extra crate dependency
        let temp = self.url.trim_start_matches("https://").trim_start_matches("http://");
        let (host_and_port, path) = match temp.find('/') {
            Some(idx) => (&temp[..idx], &temp[idx..]),
            None => (temp, "/"),
        };
        
        let (host, port) = match host_and_port.find(':') {
            Some(idx) => {
                let h = &host_and_port[..idx];
                let p = host_and_port[idx+1..].parse::<u16>().unwrap_or(80);
                (h, p)
            }
            None => {
                let p = if self.url.starts_with("https://") { 443 } else { 80 };
                (host_and_port, p)
            }
        };
        
        let addr = format!("{}:{}", host, port);
        
        // Connect via TCP
        let mut stream = tokio::net::TcpStream::connect(addr).await?;
        
        // Construct request
        let mut req_str = format!("POST {} HTTP/1.1\r\n", path);
        req_str.push_str(&format!("Host: {}\r\n", host));
        req_str.push_str("Connection: close\r\n");
        
        for (name, val) in &self.headers {
            req_str.push_str(&format!("{}: {}\r\n", name, val));
        }
        
        if let Some(ref body) = self.body {
            req_str.push_str(&format!("Content-Length: {}\r\n", body.len()));
            req_str.push_str("\r\n");
            req_str.push_str(body);
        } else {
            req_str.push_str("\r\n");
        }
        
        stream.write_all(req_str.as_bytes()).await?;
        
        // Read response
        let mut response_bytes = Vec::new();
        stream.read_to_end(&mut response_bytes).await?;
        
        let response_str = String::from_utf8_lossy(&response_bytes);
        
        // Parse status code
        let mut lines = response_str.lines();
        let status_line = lines.next().ok_or("Empty response")?;
        let parts: Vec<&str> = status_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("Invalid status line".into());
        }
        let status_code = parts[1].parse::<u16>()?;
        
        // Extract body (find double CRLF)
        let body = if let Some(pos) = response_str.find("\r\n\r\n") {
            response_str[pos + 4..].to_string()
        } else {
            String::new()
        };
        
        Ok(SimpleHttpResponse { status_code, body })
    }
}

fn new_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

fn prompt_gui_approval(message: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        let output = new_command("zenity")
            .args([
                "--question",
                "--title=UID.one Enclave Approval",
                &format!("--text={}", message)
            ])
            .output();
        if let Ok(out) = output {
            return out.status.success();
        }
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" buttons {{\"Deny\", \"Approve\"}} default button \"Approve\" with title \"UID.one\"",
            message
        );
        let output = new_command("osascript")
            .args(["-e", &script])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                return stdout.contains("button returned:Approve");
            }
        }
    }
    true
}
