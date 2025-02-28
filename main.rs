use std::process::{Command, Child};
use std::time::Duration;
use reqwest::{Client, Proxy};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Avvia il demone Tor come processo figlio.
    // Assicurati che il comando "tor" sia disponibile nel PATH del sistema.
    let mut tor_process: Child = Command::new("tor")
        .spawn()
        .expect("Errore durante l'avvio di Tor");
    
    // Attendi qualche secondo per dare tempo a Tor di avviarsi.
    // In alternativa, potresti implementare un controllo per verificare la disponibilità della porta.
    println!("Attendo l'avvio di Tor...");
    sleep(Duration::from_secs(5)).await;

    // Configura il proxy SOCKS5 (Tor solitamente ascolta su 127.0.0.1:9050, ma potrebbe variare)
    let proxy = Proxy::all("socks5://127.0.0.1:9050")?;
    
    // Crea un client HTTP configurato per usare il proxy
    let client = Client::builder()
        .proxy(proxy)
        .build()?;

    // Invia una richiesta GET ad esempio verso il sito di verifica di Tor
    let response = client
        .get("http://check.torproject.org")
        .send()
        .await?;
    
    // Stampa il contenuto della risposta
    let body = response.text().await?;
    println!("Risposta:\n{}", body);

    // Termina il processo Tor
    tor_process.kill().expect("Impossibile terminare il processo Tor");
    
    Ok(())
}
