use std::env;
use std::sync::Arc;
use uid_agent::crypto::AgentKeys;
#[cfg(unix)]
use uid_agent::ssh_agent::SshAgent;
use uid_agent::websocket::AgentWebSocketClient;
use uid_agent::posture;
use uid_agent::server;

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
        #[cfg(unix)]
        "daemon" => {
            println!("[uid-agent] Starting system endpoint security agent daemon...");
            let keys = Arc::new(AgentKeys::load_or_create()?);
            
            // Set default socket path in user's standard data directory
            let base_dir = uid_agent::get_uid_data_dir();
            let socket_path = format!("{}/agent.sock", base_dir);
            
            let agent = SshAgent::new(keys.clone(), socket_path.clone());
            
            println!("[uid-agent] System environment configuration setup:");
            println!("  export SSH_AUTH_SOCK={}", socket_path);
            println!("  To test the agent key, run: ssh-add -l");
            
            // Start local signing HTTP server in the background
            let keys_clone = keys.clone();
            tokio::spawn(async move {
                if let Err(e) = server::start_web_server(keys_clone).await {
                    eprintln!("[uid-agent] Local signing HTTP server error: {:?}", e);
                }
            });
            
            // Run SSH agent
            agent.run().await?;
        }
        #[cfg(not(unix))]
        "daemon" => {
            println!("[uid-agent] Starting system endpoint security agent daemon...");
            let keys = Arc::new(AgentKeys::load_or_create()?);
            println!("[uid-agent] Note: SSH agent mode is currently only supported on Unix-like systems (macOS / Linux).");
            
            // Run local signing HTTP server synchronously
            server::start_web_server(keys).await?;
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
            let core_url = env::var("UID_CORE_URL").unwrap_or_else(|_| "https://api.uid.one".to_string());
            
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
