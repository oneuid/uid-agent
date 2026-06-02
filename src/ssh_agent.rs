#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::crypto::AgentKeys;

// SSH Agent Protocol constants
const SSH2_AGENTC_REQUEST_IDENTITIES: u8 = 11;
const SSH2_AGENT_IDENTITIES_ANSWER: u8 = 12;
const SSH2_AGENTC_SIGN_REQUEST: u8 = 13;
const SSH2_AGENT_SIGN_RESPONSE: u8 = 14;
const SSH_AGENT_FAILURE: u8 = 5;

pub struct SshAgent {
    keys: Arc<AgentKeys>,
    socket_path: String,
}

impl SshAgent {
    pub fn new(keys: Arc<AgentKeys>, socket_path: String) -> Self {
        Self { keys, socket_path }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&self.socket_path);
        
        // Remove existing socket if it exists
        if path.exists() {
            fs::remove_file(path)?;
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(path)?;
        println!("[uid-agent] SSH Agent socket listening on {}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let keys_clone = self.keys.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, keys_clone).await {
                            eprintln!("[uid-agent] Error handling SSH connection: {:?}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[uid-agent] SSH Accept error: {:?}", e);
                }
            }
        }
    }
}

async fn handle_connection(mut stream: UnixStream, keys: Arc<AgentKeys>) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; 4096];

    loop {
        // Read message length (4 bytes)
        let mut length_buf = [0u8; 4];
        if stream.read_exact(&mut length_buf).await.is_err() {
            break; // Connection closed
        }
        
        let length = u32::from_be_bytes(length_buf) as usize;
        if length > buffer.len() {
            buffer.resize(length, 0);
        }

        // Read the message body
        stream.read_exact(&mut buffer[..length]).await?;
        
        let msg_type = buffer[0];
        let mut response = Vec::new();

        match msg_type {
            SSH2_AGENTC_REQUEST_IDENTITIES => {
                // Respond with our Ed25519 identity
                let pubkey_bytes = keys.public_key().to_bytes();
                
                // Construct the public key blob:
                // string format: "ssh-ed25519"
                // string key_payload: 32 bytes
                let mut key_blob = Vec::new();
                write_ssh_string(&mut key_blob, b"ssh-ed25519");
                write_ssh_string(&mut key_blob, &pubkey_bytes);

                let mut answer_body = Vec::new();
                answer_body.push(SSH2_AGENT_IDENTITIES_ANSWER);
                answer_body.extend_from_slice(&1u32.to_be_bytes()); // number of keys
                write_ssh_string(&mut answer_body, &key_blob);
                write_ssh_string(&mut answer_body, b"uid-agent@hardware");

                // Write length-prefixed response
                response.extend_from_slice(&(answer_body.len() as u32).to_be_bytes());
                response.extend_from_slice(&answer_body);
            }
            SSH2_AGENTC_SIGN_REQUEST => {
                // Parse signature request
                let mut cursor = 1; // Skip message type byte
                
                // Read requested key blob
                let key_blob_len = read_u32(&buffer, &mut cursor)? as usize;
                let _requested_key = &buffer[cursor..cursor + key_blob_len];
                cursor += key_blob_len;

                // Read data to sign
                let data_len = read_u32(&buffer, &mut cursor)? as usize;
                let data_to_sign = &buffer[cursor..cursor + data_len];
                cursor += data_len;

                // Read flags (unused here)
                let _flags = read_u32(&buffer, &mut cursor)?;

                // Perform the cryptographic signature
                let signature = keys.sign(data_to_sign);
                let sig_bytes = signature.to_bytes();

                // Construct signature payload:
                // string format: "ssh-ed25519"
                // string signature_raw: 64 bytes
                let mut sig_blob = Vec::new();
                write_ssh_string(&mut sig_blob, b"ssh-ed25519");
                write_ssh_string(&mut sig_blob, &sig_bytes);

                let mut answer_body = Vec::new();
                answer_body.push(SSH2_AGENT_SIGN_RESPONSE);
                write_ssh_string(&mut answer_body, &sig_blob);

                response.extend_from_slice(&(answer_body.len() as u32).to_be_bytes());
                response.extend_from_slice(&answer_body);
            }
            _ => {
                // Send standard failure code
                let mut answer_body = Vec::new();
                answer_body.push(SSH_AGENT_FAILURE);
                response.extend_from_slice(&(answer_body.len() as u32).to_be_bytes());
                response.extend_from_slice(&answer_body);
            }
        }

        stream.write_all(&response).await?;
    }

    Ok(())
}

fn write_ssh_string(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
    buf.extend_from_slice(data);
}

fn read_u32(buf: &[u8], cursor: &mut usize) -> Result<u32, Box<dyn std::error::Error>> {
    if *cursor + 4 > buf.len() {
        return Err("Unexpected end of buffer".into());
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&buf[*cursor..*cursor + 4]);
    *cursor += 4;
    Ok(u32::from_be_bytes(bytes))
}
