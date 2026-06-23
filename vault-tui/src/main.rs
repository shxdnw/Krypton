mod actions;
mod app;
mod config;
mod events;
mod views;

use std::sync::Arc;

use app::{App, AppState, FirstRunState, LockedState};
use config::KryptonConfig;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Resolve the store path: ~/.local/share/krypton/vault.db
    let data_dir = dirs::data_dir()
        .ok_or_else(|| color_eyre::eyre::eyre!("no data directory"))?
        .join("krypton");
    std::fs::create_dir_all(&data_dir)?;
    let store_path = data_dir.join("vault.db");

    // Load user config from ~/.config/krypton/config.json
    let config = KryptonConfig::load();

    // Wire dependencies.
    let deriver = Arc::new(vault_crypto::Argon2IdDeriver::new());
    let extensions = Arc::new(vault_ext::Registry::new());
    let service = Arc::new(vault_service::VaultService::new(
        store_path,
        deriver,
        extensions,
    ));

    // Determine initial state.
    let initial_state = if service.vault_exists() {
        AppState::Locked(LockedState::default())
    } else {
        AppState::FirstRun(FirstRunState::default())
    };

    let mut app = App::new(service, config, initial_state);
    app.service
        .encrypt_metadata
        .store(app.config.encrypt_metadata, std::sync::atomic::Ordering::Relaxed);
    events::run(&mut app).await?;

    Ok(())
}
