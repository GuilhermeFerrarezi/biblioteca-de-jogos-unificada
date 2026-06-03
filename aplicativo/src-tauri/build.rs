const COMMANDS: &[&str] = &[
    "list_library_entries",
    "record_boot_marker",
    "add_manual_game",
    "update_manual_game",
    "sync_local_games",
    "sync_steam_games",
    "sync_epic_games",
    "sync_xbox_games",
    "sync_xbox_achievement_games",
    "import_xbox_achievement_title_history",
    "sync_steam_account_games",
    "get_xbox_account_config",
    "save_xbox_account_config",
    "get_xbox_live_client_config",
    "get_xbox_live_client_secret_state",
    "get_xbox_library_roots",
    "save_xbox_library_roots",
    "get_steam_account_config",
    "get_steam_library_roots",
    "get_epic_library_roots",
    "start_steam_openid_login",
    "start_xbox_live_login",
    "get_xbox_live_auth_state",
    "save_xbox_live_client_config",
    "save_xbox_live_client_secret",
    "save_steam_account_config",
    "save_steam_library_roots",
    "save_epic_library_roots",
    "get_library_settings",
    "save_library_settings",
    "save_steam_web_api_key",
    "get_steam_web_api_key_state",
    "disconnect_steam_web_api_key",
    "set_library_entry_archived",
    "set_library_entry_favorite",
    "launch_library_entry",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to build Tauri app manifest");
}
