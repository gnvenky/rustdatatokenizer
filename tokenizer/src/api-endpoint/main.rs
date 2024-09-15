use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use std::sync::Mutex;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sled::Db;

// ... (Keep the existing TokenVault, PersistentTokenVault, and other functions)

// New struct for the tokenization request
#[derive(Deserialize)]
struct TokenizationRequest {
    input: String,
}

// New struct for the tokenization response
#[derive(Serialize)]
struct TokenizationResponse {
    tokenized: String,
}

// Wrap PersistentTokenVault in a Mutex for thread-safe access
struct AppState {
    vault: Mutex<PersistentTokenVault>,
}

// Handler for the tokenization endpoint
async fn tokenize_handler(
    data: web::Data<AppState>,
    req: web::Json<TokenizationRequest>,
) -> impl Responder {
    let mut vault = data.vault.lock().unwrap();
    match tokenize(&mut vault, &req.input) {
        Ok(tokenized) => HttpResponse::Ok().json(TokenizationResponse { tokenized }),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let vault = PersistentTokenVault::new("token_vault.db").expect("Failed to create vault");
    let app_state = web::Data::new(AppState {
        vault: Mutex::new(vault),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/tokenize", web::post().to(tokenize_handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// Define a struct to hold the token mappings, with Serialize and Deserialize traits for easy conversion
#[derive(Serialize, Deserialize)]
struct TokenVault {
    tokens: HashMap<String, String>,
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
            Some(data) => bincode::deserialize(&data).unwrap_or(TokenVault { tokens: HashMap::new() }),
            None => TokenVault { tokens: HashMap::new() },
        };
        Ok(Self { db, vault })
    }

    // Set a new token or update an existing one
    fn set_token(&mut self, key: &str, value: &str) -> Result<(), sled::Error> {
        self.vault.tokens.insert(key.to_string(), value.to_string());
        self.save()
    }

    // Retrieve a token if it exists
    fn get_token(&self, key: &str) -> Option<String> {
        self.vault.tokens.get(key).cloned()
    }

    // Save the current state of the vault to disk
    fn save(&self) -> Result<(), sled::Error> {
        let encoded = bincode::serialize(&self.vault).unwrap();
        self.db.insert("vault", encoded)?;
        self.db.flush().map(|_| ())
    }
}

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
                let new_token = generate_token();
                vault.set_token(&new_token, &word)?;
                new_token
            }
        };
        tokenized.push(token);
    }

    Ok(tokenized.join(" "))
}
