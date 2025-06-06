use crate::core::minecraft_account::MinecraftAccount;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct AccountsManager {
    pub accounts: Vec<MinecraftAccount>,
    accounts_file: PathBuf,
}

impl AccountsManager {
    pub fn new() -> Self {
        let accounts_file = config_dir()
            .expect("Failed to get config directory")
            .join("dev.alexitoo.modpackstore")
            .join("accounts.json");
        if !accounts_file.exists() {
            let default_accounts = json!([]);
            fs::write(
                &accounts_file,
                serde_json::to_string_pretty(&default_accounts).unwrap(),
            )
            .expect("Failed to create accounts.json file");
        }

        let mut manager = AccountsManager {
            accounts: Vec::new(),
            accounts_file,
        };
        manager.load();
        manager
    }

    pub fn add_microsoft_account(
        username: &str,
        access_token: &str,
        uuid: &str,
    ) -> Result<MinecraftAccount, String> {
        let accounts_manager = get_accounts_manager();
        let mut manager = accounts_manager.lock().unwrap();
        let account = MinecraftAccount::new(
            username.to_string(),
            uuid.to_string(),
            Some(access_token.to_string()),
            "Microsoft".to_string(),
        );
        if manager.accounts.iter().any(|a| a.uuid() == uuid) {
            return Err(format!("Account with UUID {} already exists", uuid));
        }
        manager.accounts.push(account.clone());
        manager.save();
        Ok(account)
    }

    pub fn add_offline_account(&mut self, username: &str) -> Result<MinecraftAccount, String> {
        let uuid = Self::get_offline_player_uuid(username)?;
        let account = MinecraftAccount::new(
            username.to_string(),
            uuid.clone(),
            None,
            "offline".to_string(),
        );
        if self.accounts.iter().any(|a| a.uuid() == uuid) {
            return Err(format!("Account with UUID {} already exists", uuid));
        }
        self.accounts.push(account.clone());
        self.save();
        Ok(account)
    }

    pub fn remove_account(&mut self, uuid: &str) {
        if let Some(pos) = self.accounts.iter().position(|a| a.uuid() == uuid) {
            self.accounts.remove(pos);
            self.save();
        } else {
            println!("Account with UUID {} not found", uuid);
        }
    }

    pub fn get_all_accounts(&self) -> Vec<MinecraftAccount> {
        println!("Loading Minecraft accounts...");
        self.accounts.clone()
    }

    pub fn get_minecraft_account(&self, uuid: &str) -> Option<MinecraftAccount> {
        self.accounts.iter().find(|a| a.uuid() == uuid).cloned()
    }

    pub fn get_minecraft_account_by_uuid(&self, uuid: &str) -> Option<MinecraftAccount> {
        self.accounts.iter().find(|a| a.uuid() == uuid).cloned()
    }

    fn load(&mut self) {
        if !self.accounts_file.exists() {
            println!("accounts.json file doesn't exist. Creating a new one...");
            self.save();
            return;
        }

        match fs::read_to_string(&self.accounts_file) {
            Ok(contents) => match serde_json::from_str::<Vec<MinecraftAccount>>(&contents) {
                Ok(loaded_accounts) => {
                    self.accounts = loaded_accounts;
                    println!("Accounts loaded successfully: {}", self.accounts.len());
                }
                Err(e) => {
                    eprintln!("Error parsing accounts.json: {}", e);
                }
            },
            Err(e) => {
                eprintln!("Error reading accounts.json: {}", e);
            }
        }
    }

    pub fn save(&self) {
        if let Some(parent) = self.accounts_file.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    eprintln!("Error creating directory: {}", e);
                    return;
                }
            }
        }

        match serde_json::to_string_pretty(&self.accounts) {
            Ok(json) => {
                if let Err(e) = fs::write(&self.accounts_file, json) {
                    eprintln!("Error writing to accounts.json: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error serializing accounts: {}", e);
            }
        }
    }

    /// Calculates the UUID for an offline player
    pub fn get_offline_player_uuid(username: &str) -> Result<String, String> {
        // Validation
        if username.is_empty() {
            return Err("Username cannot be null or empty".to_string());
        }

        if username.len() < 3 || username.len() > 16 {
            return Err("Username must be between 3 and 16 characters".to_string());
        }

        if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(
                "Username can only contain letters (a-z, A-Z), numbers (0-9), and underscores (_)"
                    .to_string(),
            );
        }

        // Create the string to hash
        let string_to_hash = format!("OfflinePlayer:{}", username);

        // Generate the UUID (Version 3, name-based)
        let offline_uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, string_to_hash.as_bytes());

        Ok(offline_uuid.to_string())
    }
}

// Singleton implementation to easily access the AccountsManager from anywhere
lazy_static::lazy_static! {
    static ref ACCOUNTS_MANAGER: Arc<Mutex<AccountsManager>> = Arc::new(Mutex::new(AccountsManager::new()));
}

pub fn get_accounts_manager() -> Arc<Mutex<AccountsManager>> {
    ACCOUNTS_MANAGER.clone()
}

#[tauri::command]
pub fn add_offline_account(username: &str) -> Result<MinecraftAccount, String> {
    let accounts_manager = get_accounts_manager();
    let mut manager = accounts_manager.lock().unwrap();
    match manager.add_offline_account(username) {
        Ok(account) => Ok(account),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn remove_account(uuid: &str) -> Result<(), String> {
    let accounts_manager = get_accounts_manager();
    let mut manager = accounts_manager.lock().unwrap();
    manager.remove_account(uuid);
    Ok(())
}

#[tauri::command]
pub fn get_all_accounts() -> Result<Vec<MinecraftAccount>, String> {
    let accounts_manager = get_accounts_manager();
    let manager = accounts_manager.lock().unwrap();
    Ok(manager.get_all_accounts())
}


#[tauri::command]
pub fn ensure_account_exists(uuid: &str) -> Result<bool, String> {
    let accounts_manager = get_accounts_manager();
    let manager = accounts_manager.lock().unwrap();
    if manager.accounts.iter().any(|a| a.uuid() == uuid) {
        Ok(true)
    } else {
        Ok(false)
    }
}
