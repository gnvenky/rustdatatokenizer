use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use rand::Rng;
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

#[derive(Serialize, Deserialize)]
struct TokenVault {
    tokens: HashMap<String, String>,
}

struct PersistentTokenVault {
    client: Client<tokio_util::compat::Compat<TcpStream>>,
    vault: TokenVault,
}

impl PersistentTokenVault {
    async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;

        let client = Client::connect(config, tcp.compat_write()).await?;
        
        // Create table if not exists
        client.execute(
            "IF NOT EXISTS (SELECT * FROM sysobjects WHERE name='TokenVault' AND xtype='U')
             CREATE TABLE TokenVault (token VARCHAR(255) PRIMARY KEY, value VARCHAR(MAX))",
            &[]
        ).await?;

        let vault = TokenVault { tokens: HashMap::new() };
        Ok(Self { client, vault })
    }

    async fn set_token(&mut self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .execute(
                "INSERT INTO TokenVault (token, value) VALUES (@P1, @P2)
                 ON DUPLICATE KEY UPDATE value = @P2",
                &[&key, &value]
            )
            .await?;
        self.vault.tokens.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn get_token(&mut self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if let Some(value) = self.vault.tokens.get(key) {
            return Ok(Some(value.clone()));
        }

        let result = self.client
            .query("SELECT value FROM TokenVault WHERE token = @P1", &[&key])
            .await?
            .into_row()
            .await?;

        if let Some(row) = result {
            let value: String = row.get("value")?;
            self.vault.tokens.insert(key.to_string(), value.clone());
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

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

async fn tokenize(vault: &mut PersistentTokenVault, input: &str) -> Result<String, Box<dyn std::error::Error>> {
    let words: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();
    let mut tokenized = Vec::new();

    for word in words {
        let token = match vault.get_token(&word).await? {
            Some(t) => t,
            None => {
                let new_token = generate_token();
                vault.set_token(&new_token, &word).await?;
                new_token
            }
        };
        tokenized.push(token);
    }

    Ok(tokenized.join(" "))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.host("localhost");
    config.port(1433);
    config.database("YourDatabase");
    config.authentication(tiberius::AuthMethod::sql_server("YourUsername", "YourPassword"));

    let mut vault = PersistentTokenVault::new(config).await?;

    let sensitive_data = "This is a secret message";
    let tokenized = tokenize(&mut vault, sensitive_data).await?;
    println!("Original: {}", sensitive_data);
    println!("Tokenized: {}", tokenized);

    let retrieved: Vec<String> = futures::future::join_all(
        tokenized.split_whitespace().map(|token| vault.get_token(token))
    ).await
     .into_iter()
     .filter_map(|r| r.ok().flatten())
     .collect();
    println!("Retrieved: {}", retrieved.join(" "));

    Ok(())
}
