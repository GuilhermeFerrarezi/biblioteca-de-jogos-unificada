const COMMANDS: &[&str] = &[
    "list_library_entries",
    "add_manual_game",
    "update_manual_game",
    "sync_local_games",
    "sync_steam_games",
    "sync_xbox_games",
    "sync_steam_account_games",
    "get_steam_account_config",
    "start_steam_openid_login",
    "save_steam_account_config",
    "get_library_settings",
    "save_library_settings",
    "save_steam_web_api_key",
    "get_steam_web_api_key_state",
    "disconnect_steam_web_api_key",
    "set_library_entry_archived",
    "launch_library_entry",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to build Tauri app manifest");
}
