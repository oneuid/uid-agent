use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::env;
use ed25519_dalek::{SigningKey, Signature, Signer, VerifyingKey};
use rand::rngs::OsRng;

pub struct AgentKeys {
    pub signing_key: SigningKey,
}

impl AgentKeys {
    fn get_key_path() -> PathBuf {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_else(|_| "/home/s".to_string());
        PathBuf::from(home).join(".uid").join("agent.key")
    }

    pub fn load_or_create() -> Result<Self, Box<dyn std::error::Error>> {
        let key_path = Self::get_key_path();
        
        if key_path.exists() {
            Self::load_from_file(&key_path)
        } else {
            Self::create_new(&key_path)
        }
    }

    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut bytes = [0u8; 32];
        file.read_exact(&mut bytes)?;
        
        let signing_key = SigningKey::from_bytes(&bytes);
        Ok(Self { signing_key })
    }

    pub fn create_new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        
        let mut file = File::create(path)?;
        // Set permissions to 0600 (owner read/write) on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o600);
            file.set_permissions(perms)?;
        }
        
        file.write_all(&signing_key.to_bytes())?;
        
        Ok(Self { signing_key })
    }

    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }
}
