// Import necessary libraries
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sled::Db;

// Define a struct to hold the token mappings, with Serialize and Deserialize traits for easy conversion
#[derive(Serialize, Deserialize)]
struct TokenVault {
    word_to_token: HashMap<String, String>,
    token_to_word: HashMap<String, String>
}

// Define the main struct for persistent token storage
struct PersistentTokenVault {
    db: Db,
    vault: TokenVault,
}

impl PersistentTokenVault {
    // Create a new PersistentTokenVault or load an existing one
		fn new(path: &str) -> Result<Self, sled::Error> {
				let db = sled::open(path)?;
				let vault = match db.get("vault")? {
						Some(data) => bincode::deserialize(&data).unwrap_or(TokenVault {
								word_to_token: HashMap::new(),
								token_to_word: HashMap::new(),
						}),
						None => TokenVault {
								word_to_token: HashMap::new(),
								token_to_word: HashMap::new(),
						},
				};
				Ok(Self { db, vault })
		}

    // Set a new token or update an existing one
		fn set_token(&mut self, word: &str, token: &str) -> Result<(), sled::Error> {
				self.vault.word_to_token.insert(word.to_string(), token.to_string());
				self.vault.token_to_word.insert(token.to_string(), word.to_string());
				self.save()
    }

    // Retrieve a token if it exists
		fn get_token(&self, word: &str) -> Option<String> {
				self.vault.word_to_token.get(word).cloned()
		}

		fn get_word(&self, token: &str) -> Option<String> {
				self.vault.token_to_word.get(token).cloned()
		}

    // Save the current state of the vault to disk
    fn save(&self) -> Result<(), sled::Error> {
        let encoded = bincode::serialize(&self.vault).unwrap();
        self.db.insert("vault", encoded)?;
        self.db.flush().map(|_| ())
    }
}

// Generate a random token
/*
fn generate_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const TOKEN_LEN: usize = 8;
    let mut rng = rand::thread_rng();
    (0..TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
*/

use rand::Rng;
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

fn generate_token() -> String {
    const TOKEN_LEN: usize = 16; // Increased for better security
    
    // Use OsRng for cryptographically secure random numbers
    let mut rng = OsRng;
    
    // Generate random bytes
    let random_bytes: Vec<u8> = (0..TOKEN_LEN)
        .map(|_| rng.gen::<u8>())
        .collect();
    
    // Hash the random bytes using SHA-256
    let mut hasher = Sha256::new();
    hasher.update(&random_bytes);
    let result = hasher.finalize();
    
    // Encode the hash in URL-safe Base64
    general_purpose::URL_SAFE_NO_PAD.encode(&result)[0..TOKEN_LEN].to_string()
}

fn tokenize(vault: &mut PersistentTokenVault, input: &str) -> Result<String, sled::Error> {
    let words: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();
    let mut tokenized = Vec::new();

    for word in words {
        let token = match vault.get_token(&word) {
            Some(t) => t,
            None => {
                let mut new_token;
                loop {
                    new_token = generate_token();
                    if vault.get_word(&new_token).is_none() {
                        break;
                    }
                }
                vault.set_token(&word, &new_token)?;
                new_token
            }
        };
        tokenized.push(token);
    }

    Ok(tokenized.join(" "))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vault = PersistentTokenVault::new("token_vault.db")?;

    // Example usage
    let sensitive_data = "My age is 43. My ssn is 110-110-10101";
    let tokenized = tokenize(&mut vault, sensitive_data)?;
    println!("Original: {}", sensitive_data);
    println!("Tokenized: {}", tokenized);

    // Retrieve original data
    let retrieved: Vec<String> = tokenized
        .split_whitespace()
        .filter_map(|token| vault.get_token(token))
        .collect();
    println!("Retrieved: {}", retrieved.join(" "));

    Ok(())
}
