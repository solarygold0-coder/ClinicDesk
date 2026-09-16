use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address: SocketAddr = std::env::var("CLINICDESK_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("ClinicDesk server listening on {address}");
    axum::serve(listener, clinicdesk_server::app()).await?;
    Ok(())
}
