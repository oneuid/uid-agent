mod crypto;
mod posture;
mod ssh_agent;
mod websocket;

use std::env;
use std::sync::Arc;
use crate::crypto::AgentKeys;
use crate::ssh_agent::SshAgent;
use crate::websocket::AgentWebSocketClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = args[1].as_str();

    match command {
        "register" => {
            println!("[uid-agent] Initializing hardware-bound identity keys...");
            let keys = AgentKeys::load_or_create()?;
            let pubkey_hex = hex::encode(keys.public_key().to_bytes());
            println!("[uid-agent] Keypair successfully registered.");
            println!("[uid-agent] Public Key Attestation: {}", pubkey_hex);
        }
        "posture" => {
            let posture_data = posture::get_posture();
            let json_output = serde_json::to_string_pretty(&posture_data)?;
            println!("{}", json_output);
        }
        "sign" => {
            if args.len() < 3 {
                eprintln!("Usage: uid-agent sign <data_string>");
                std::process::exit(1);
            }
            let keys = AgentKeys::load_or_create()?;
            let data = args[2].as_bytes();
            let signature = keys.sign(data);
            println!("{}", hex::encode(signature.to_bytes()));
        }
        "daemon" => {
            println!("[uid-agent] Starting system endpoint security agent daemon...");
            let keys = Arc::new(AgentKeys::load_or_create()?);
            
            // Set default socket path in user's home folder ~/.uid/agent.sock
            let home = env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
            let socket_path = format!("{}/.uid/agent.sock", home);
            
            let agent = SshAgent::new(keys.clone(), socket_path.clone());
            
            println!("[uid-agent] System environment configuration setup:");
            println!("  export SSH_AUTH_SOCK={}", socket_path);
            println!("  To test the agent key, run: ssh-add -l");
            
            // Run SSH agent
            agent.run().await?;
        }
        "approve" => {
            if args.len() < 3 {
                eprintln!("Usage: uid-agent approve <challenge_token> [--token <auth_token>]");
                std::process::exit(1);
            }
            let challenge_token = &args[2];
            let mut auth_token = None;
            
            // Look for --token parameter
            for i in 3..args.len() {
                if args[i] == "--token" && i + 1 < args.len() {
                    auth_token = Some(args[i + 1].as_str());
                    break;
                }
            }

            let keys = Arc::new(AgentKeys::load_or_create()?);
            let core_url = env::var("UID_CORE_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());
            
            let ws_client = AgentWebSocketClient::new(keys, core_url);
            ws_client.listen_and_sign(challenge_token, auth_token).await?;
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_usage() {
    println!("UID Agent (Endpoint OS Security) CLI");
    println!("Usage:");
    println!("  uid-agent register                  Generate and register hardware-bound keypair");
    println!("  uid-agent posture                   Collect and display device compliance posture (SOC 2)");
    println!("  uid-agent sign <data>               Cryptographically sign a string payload");
    println!("  uid-agent approve <challenge_token> Connect, sign, and approve authentication challenge");
    println!("  uid-agent daemon                    Run background service hosting SSH agent socket");
    println!("  uid-agent help                      Show this help menu");
}
