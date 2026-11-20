use std::{net::{Ipv4Addr, SocketAddrV4}, path::PathBuf};
use cadentrix_server::{Store, router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::env::var_os("CADENTRIX_WORKSPACE").map(PathBuf::from).unwrap_or(std::env::current_dir()?).canonicalize()?;
    let data = std::env::var_os("CADENTRIX_DATA_DIR").map(PathBuf::from).unwrap_or_else(|| workspace.join(".cadentrix"));
    std::fs::create_dir_all(&data)?;
    let store = Store::open(&data.join("state.sqlite3"), &workspace)?;
    let frontend = workspace.join("dist/client");
    let listener = tokio::net::TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 4180)).await?;
    println!("CADENTRIX backend listening on http://127.0.0.1:4180");
    println!("Workspace: {} · native read-only worker · SQLite persistence", store.workspace_name());
    axum::serve(listener, router(store, frontend)).with_graceful_shutdown(async { if let Err(error) = tokio::signal::ctrl_c().await { eprintln!("shutdown signal error: {error}"); } }).await?;
    Ok(())
}
