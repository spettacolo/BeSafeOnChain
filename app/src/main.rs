use std::process::{Command, Child};
use std::time::Duration;
use std::error::Error;
use reqwest::{Client, Proxy};
use tokio::time::sleep;

// Importiamo il modulo secp256k1 dal crate bitcoin per evitare conflitti di versioni.
use bitcoin::network::constants::Network;
use bitcoin::util::address::Address;
use bitcoin::util::key::PrivateKey;
use bitcoin::secp256k1::{Secp256k1, SecretKey};

use monero::util::key::{PrivateKey as MoneroPrivateKey, PublicKey as MoneroPublicKey};
use monero::Address as MoneroAddress;
use monero::Network as MoneroNetwork;

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use web3::types::Address as EthAddress;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer; // Per usare il metodo pubkey() sul Keypair

/// Avvia il demone Tor e crea un client HTTP configurato per usare il proxy Tor.
async fn start_tor() -> Result<(Child, Client), Box<dyn Error>> {
    let mut tor_process: Child = Command::new("tor")
        .spawn()
        .expect("Errore durante l'avvio di Tor");

    println!("Attendo l'avvio di Tor...");
    sleep(Duration::from_secs(5)).await;

    let proxy = Proxy::all("socks5://127.0.0.1:9050")?;
    let client = Client::builder()
        .proxy(proxy)
        .build()?;

    let response = client
        .get("http://check.torproject.org")
        .send()
        .await?;
    let body = response.text().await?;
    println!("Risposta di test da Tor:\n{}", body);

    Ok((tor_process, client))
}

// ----------------------------
// Funzionalità Wallet e Scambio
// ----------------------------

#[derive(Debug, Serialize, Deserialize)]
struct StealthExQuote {
    from_currency: String,
    to_currency: String,
    from_amount: f64,
    estimated_amount: f64,
    address: String, // Indirizzo di deposito per StealthEX
}

struct Wallet {
    btc: (PrivateKey, Address),              // Bitcoin
    ltc: (PrivateKey, Address),              // Litecoin (placeholder: utilizza il network Bitcoin)
    eth: (String, EthAddress),               // Ethereum (chiave privata come stringa)
    sol: (Keypair, Pubkey),                  // Solana
    ton: (String, String),                   // TON (simulato: chiave privata, indirizzo)
    xmr: (MoneroPrivateKey, MoneroAddress),    // Monero
}

fn generate_btc_wallet(network: Network) -> Result<(PrivateKey, Address), Box<dyn Error>> {
    let mut rng = OsRng;
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&secret_bytes)?;
    let private_key = PrivateKey::new(secret_key, network);
    let public_key = private_key.public_key(&secp);
    let address = Address::p2pkh(&public_key, network);
    Ok((private_key, address))
}

fn generate_eth_wallet() -> Result<(String, EthAddress), Box<dyn Error>> {
    let mut rng = OsRng;
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);
    let private_key = hex::encode(secret_bytes);
    let address = EthAddress::from_slice(&secret_bytes[0..20]);
    Ok((private_key, address))
}

fn generate_sol_wallet() -> (Keypair, Pubkey) {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    (keypair, pubkey)
}

fn generate_ton_wallet() -> Result<(String, String), Box<dyn Error>> {
    let mut rng = OsRng;
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);
    let private_key = hex::encode(secret_bytes);
    let address = format!("TON{}", hex::encode(&secret_bytes[0..10]));
    Ok((private_key, address))
}

fn generate_xmr_wallet() -> Result<(MoneroPrivateKey, MoneroAddress), Box<dyn Error>> {
    let mut rng = OsRng;
    let mut spend_bytes = [0u8; 32];
    rng.fill_bytes(&mut spend_bytes);
    let spend_key = MoneroPrivateKey::from_slice(&spend_bytes)?;
    let mut view_bytes = [0u8; 32];
    rng.fill_bytes(&mut view_bytes);
    let view_key = MoneroPrivateKey::from_slice(&view_bytes)?;

    // Convertiamo le chiavi private in chiavi pubbliche
    let public_spend = MoneroPublicKey::from_slice(spend_key.as_bytes())?;
    let public_view = MoneroPublicKey::from_slice(view_key.as_bytes())?;
    let address = MoneroAddress::standard(MoneroNetwork::Mainnet, public_spend, public_view);
    Ok((spend_key, address))
}

fn generate_wallets() -> Result<Wallet, Box<dyn Error>> {
    Ok(Wallet {
        btc: generate_btc_wallet(Network::Bitcoin)?,
        ltc: generate_btc_wallet(Network::Bitcoin)?, // Placeholder per Litecoin
        eth: generate_eth_wallet()?,
        sol: generate_sol_wallet(),
        ton: generate_ton_wallet()?,
        xmr: generate_xmr_wallet()?,
    })
}

// Simula il monitoraggio delle transazioni (il parametro _wallet non viene usato)
async fn check_transactions(_wallet: &Wallet) -> Result<Vec<(&'static str, f64)>, Box<dyn Error>> {
    let mut received = Vec::new();
    if rand::random::<f32>() > 0.8 {
        received.push(("BTC", 0.001));
    }
    if rand::random::<f32>() > 0.9 {
        received.push(("ETH", 0.01));
    }
    Ok(received)
}

async fn get_stealthex_quote(
    client: &Client,
    api_key: &str,
    from: &str,
    to: &str,
    amount: f64,
) -> Result<StealthExQuote, Box<dyn Error>> {
    let url = format!(
        "https://api.stealthex.io/api/v2/estimate/{}/{}?amount={}&api_key={}",
        from, to, amount, api_key
    );
    let response = client.get(&url).send().await?.json::<StealthExQuote>().await?;
    Ok(response)
}

async fn create_stealthex_swap(
    client: &Client,
    api_key: &str,
    from: &str,
    to: &str,
    amount: f64,
    destination_address: &str,
) -> Result<String, Box<dyn Error>> {
    let url = "https://api.stealthex.io/api/v2/exchange";
    let body = serde_json::json!({
        "from": from,
        "to": to,
        "amount": amount,
        "address": destination_address,
        "api_key": api_key
    });
    let response = client.post(url).json(&body).send().await?;
    let swap_id: serde_json::Value = response.json().await?;
    Ok(swap_id["id"].as_str().unwrap_or("").to_string())
}

async fn send_to_stealthex(
    crypto: &str,
    amount: f64,
    wallet: &Wallet,
    stealthex_address: &str,
) -> Result<(), Box<dyn Error>> {
    match crypto {
        "BTC" => println!("Invio {} BTC da {} a {}", amount, wallet.btc.1, stealthex_address),
        "LTC" => println!("Invio {} LTC da {} a {}", amount, wallet.ltc.1, stealthex_address),
        "ETH" => println!("Invio {} ETH da {:?} a {}", amount, wallet.eth.1, stealthex_address),
        "SOL" => println!("Invio {} SOL da {} a {}", amount, wallet.sol.1, stealthex_address),
        "TON" => println!("Invio {} TON da {} a {}", amount, wallet.ton.1, stealthex_address),
        _ => println!("Criptovaluta non supportata: {}", crypto),
    }
    Ok(())
}

/// Gestisce il ciclo di monitoraggio e scambio, utilizzando il client configurato per Tor.
async fn run_wallet_exchange(client: &Client) -> Result<(), Box<dyn Error>> {
    let api_key = "YOUR_STEALTHEX_API_KEY";

    let wallet = generate_wallets()?;
    println!("BTC Address: {}", wallet.btc.1);
    println!("LTC Address: {}", wallet.ltc.1);
    println!("ETH Address: {:?}", wallet.eth.1);
    println!("SOL Address: {}", wallet.sol.1);
    println!("TON Address: {}", wallet.ton.1);
    println!("XMR Address: {}", wallet.xmr.1);

    loop {
        let received = check_transactions(&wallet).await?;
        for (crypto, amount) in received {
            if crypto != "XMR" {
                println!("Ricevuti {} {}", amount, crypto);
                let quote = get_stealthex_quote(client, api_key, crypto, "XMR", amount).await?;
                println!("Riceverai circa {} XMR", quote.estimated_amount);
                let swap_id = create_stealthex_swap(
                    client,
                    api_key,
                    crypto,
                    "XMR",
                    amount,
                    &wallet.xmr.1.to_string(),
                ).await?;
                println!("Swap ID: {}", swap_id);
                send_to_stealthex(crypto, amount, &wallet, &quote.address).await?;
            }
        }
        sleep(Duration::from_secs(60)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (mut tor_process, client) = start_tor().await?;
    
    let exchange_result = run_wallet_exchange(&client).await;

    tor_process.kill().expect("Impossibile terminare il processo Tor");
    
    exchange_result
}
