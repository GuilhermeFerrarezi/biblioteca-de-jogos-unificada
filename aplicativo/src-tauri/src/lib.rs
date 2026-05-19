use tauri::{Emitter, Manager};

const LIBRARY_BOOTSTRAP_COMPLETE_EVENT: &str = "library-bootstrap-complete";
const STEAM_SYNC_FAILED_EVENT: &str = "steam-sync-failed";
const XBOX_SYNC_FAILED_EVENT: &str = "xbox-sync-failed";
const XBOX_ACHIEVEMENTS_SYNC_FAILED_EVENT: &str = "xbox-achievements-sync-failed";
const XBOX_TITLE_HISTORY_IMPORT_FAILED_EVENT: &str = "xbox-title-history-import-failed";

mod xbox_provider;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
            std::fs::create_dir_all(&app_data_dir)
                .map_err(|error| format!("failed to create app data dir: {error}"))?;
            let db_path = app_data_dir.join("library.sqlite3");
            let connection = storage::open_database(&db_path)
                .map_err(|error| format!("failed to open local database: {error}"))?;
            let connection = std::sync::Arc::new(std::sync::Mutex::new(connection));
            let auth_vault_dir = app_data_dir.join("auth-vault");
            let auth_vault = std::sync::Arc::new(security::AuthVault::system(auth_vault_dir));

            app.manage(AppState {
                connection: std::sync::Arc::clone(&connection),
                auth_vault,
            });
            bootstrap_library(app.handle().clone(), connection);

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_library_entries,
            commands::list_manual_games,
            commands::add_manual_game,
            commands::update_manual_game,
            commands::sync_local_games,
            commands::sync_steam_games,
            commands::sync_xbox_games,
            commands::sync_xbox_achievement_games,
            commands::import_xbox_achievement_title_history,
            commands::sync_steam_account_games,
            commands::get_steam_account_config,
            commands::get_steam_library_roots,
            commands::start_steam_openid_login,
            commands::save_steam_account_config,
            commands::save_steam_library_roots,
            commands::get_library_settings,
            commands::save_library_settings,
            commands::save_steam_web_api_key,
            commands::get_steam_web_api_key_state,
            commands::disconnect_steam_web_api_key,
            commands::set_library_entry_archived,
            commands::launch_library_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct AppState {
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    auth_vault: std::sync::Arc<security::AuthVault>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderErrorDto {
    code: &'static str,
    message: String,
    recoverable: bool,
    provider_id: &'static str,
    phase: &'static str,
    details_sanitized: Option<String>,
}

impl ProviderErrorDto {
    fn steam(
        code: &'static str,
        message: impl Into<String>,
        recoverable: bool,
        phase: &'static str,
        details_sanitized: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
            provider_id: "steam",
            phase,
            details_sanitized,
        }
    }

    pub(crate) fn xbox(
        code: &'static str,
        message: impl Into<String>,
        recoverable: bool,
        phase: &'static str,
        details_sanitized: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
            provider_id: "xbox",
            phase,
            details_sanitized,
        }
    }
}

fn bootstrap_library(
    handle: tauri::AppHandle,
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
) {
    if !cfg!(debug_assertions) {
        let _ = handle.emit(LIBRARY_BOOTSTRAP_COMPLETE_EVENT, true);
        return;
    }

    std::thread::spawn(move || {
        let result = connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())
            .and_then(|mut connection| {
                storage::seed_mock_library(&mut connection).map_err(|error| error.to_string())?;
                Ok(())
            });

        if let Err(error) = &result {
            eprintln!("library bootstrap failed: {error}");
        }

        let _ = handle.emit(LIBRARY_BOOTSTRAP_COMPLETE_EVENT, result.is_ok());
    });
}

mod commands {
    use super::{
        launcher, security, steam_openid, steam_web_api, storage, xbox_provider, AppState,
    };
    use crate::ProviderErrorDto;
    use tauri::{AppHandle, Emitter, State};

    #[tauri::command]
    pub fn list_library_entries(
        state: State<'_, AppState>,
    ) -> Result<Vec<storage::LibraryEntryDto>, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::list_library_entries(&connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn list_manual_games(
        state: State<'_, AppState>,
    ) -> Result<Vec<storage::LibraryEntryDto>, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::list_manual_games(&connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn add_manual_game(
        input: storage::ManualGameInput,
        state: State<'_, AppState>,
    ) -> Result<storage::LibraryEntryDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::add_manual_game(&mut connection, input).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn sync_local_games(state: State<'_, AppState>) -> Result<storage::SyncSummaryDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::sync_local_games(&mut connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn sync_steam_games(state: State<'_, AppState>) -> Result<storage::SyncSummaryDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::sync_steam_games(&mut connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn sync_xbox_games(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<storage::SyncSummaryDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        xbox_provider::sync_xbox_games(&mut connection).map_err(|error| {
            let provider_error = error.into_provider_error();
            let provider_error_json = serde_json::to_string(&provider_error)
                .unwrap_or_else(|_| provider_error.message.clone());
            log::warn!(
                "xbox sync failed: code={} phase={} recoverable={} provider_id={} details_sanitized={:?}",
                provider_error.code,
                provider_error.phase,
                provider_error.recoverable,
                provider_error.provider_id,
                provider_error.details_sanitized
            );
            let _ = app.emit(super::XBOX_SYNC_FAILED_EVENT, provider_error.clone());
            provider_error_json
        })
    }

    #[tauri::command]
    pub fn sync_xbox_achievement_games(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<storage::SyncSummaryDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        xbox_provider::sync_xbox_achievement_games(&mut connection).map_err(|error| {
            let provider_error = error.into_provider_error();
            let provider_error_json = serde_json::to_string(&provider_error)
                .unwrap_or_else(|_| provider_error.message.clone());
            log::warn!(
                "xbox achievements sync failed: code={} phase={} recoverable={} provider_id={} details_sanitized={:?}",
                provider_error.code,
                provider_error.phase,
                provider_error.recoverable,
                provider_error.provider_id,
                provider_error.details_sanitized
            );
            let _ = app.emit(
                super::XBOX_ACHIEVEMENTS_SYNC_FAILED_EVENT,
                provider_error.clone(),
            );
            provider_error_json
        })
    }

    #[tauri::command]
    pub fn import_xbox_achievement_title_history(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<storage::SyncSummaryDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        xbox_provider::import_xbox_achievement_title_history(&mut connection).map_err(|error| {
            let provider_error = error.into_provider_error();
            let provider_error_json = serde_json::to_string(&provider_error)
                .unwrap_or_else(|_| provider_error.message.clone());
            log::warn!(
                "xbox title history import failed: code={} phase={} recoverable={} provider_id={} details_sanitized={:?}",
                provider_error.code,
                provider_error.phase,
                provider_error.recoverable,
                provider_error.provider_id,
                provider_error.details_sanitized
            );
            let _ = app.emit(
                super::XBOX_TITLE_HISTORY_IMPORT_FAILED_EVENT,
                provider_error.clone(),
            );
            provider_error_json
        })
    }

    #[tauri::command]
    pub fn sync_steam_account_games(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<storage::SyncSummaryDto, String> {
        sync_steam_account_games_impl(state.inner()).map_err(|error| {
            log::warn!(
                "steam sync failed: code={} phase={} recoverable={} provider_id={} details_sanitized={:?}",
                error.code,
                error.phase,
                error.recoverable,
                error.provider_id,
                error.details_sanitized
            );
            let _ = app.emit(super::STEAM_SYNC_FAILED_EVENT, error.clone());
            error.message
        })
    }

    #[tauri::command]
    pub fn get_steam_account_config(
        state: State<'_, AppState>,
    ) -> Result<storage::SteamAccountConfigDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::get_steam_account_config(&connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn start_steam_openid_login(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<steam_openid::SteamOpenIdLoginStartDto, String> {
        steam_openid::start_login(app, state.connection.clone()).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_steam_account_config(
        input: storage::SteamAccountConfigInput,
        state: State<'_, AppState>,
    ) -> Result<storage::SteamAccountConfigDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::save_steam_account_config(&mut connection, input)
            .map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn get_steam_library_roots(
        state: State<'_, AppState>,
    ) -> Result<storage::SteamLibraryRootsDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::get_steam_library_roots(&connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_steam_library_roots(
        input: storage::SteamLibraryRootsInput,
        state: State<'_, AppState>,
    ) -> Result<storage::SteamLibraryRootsDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::save_steam_library_roots(&mut connection, input).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn get_library_settings(
        state: State<'_, AppState>,
    ) -> Result<storage::LibrarySettingsDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::get_library_settings(&connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_library_settings(
        input: storage::LibrarySettingsInput,
        state: State<'_, AppState>,
    ) -> Result<storage::LibrarySettingsDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::save_library_settings(&mut connection, input).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_steam_web_api_key(
        input: security::SteamWebApiKeyInput,
        state: State<'_, AppState>,
    ) -> Result<security::SteamWebApiKeyStateDto, String> {
        let state_dto = state
            .auth_vault
            .save_steam_web_api_key(input)
            .map_err(|error| error.to_string())?;
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::set_steam_web_api_key_config(&mut connection, true)
            .map_err(|error| error.to_string())?;

        Ok(state_dto)
    }

    #[tauri::command]
    pub fn get_steam_web_api_key_state(
        state: State<'_, AppState>,
    ) -> Result<security::SteamWebApiKeyStateDto, String> {
        let vault_state = state
            .auth_vault
            .steam_web_api_key_state()
            .map_err(|error| error.to_string())?;
        Ok(vault_state)
    }

    #[tauri::command]
    pub fn disconnect_steam_web_api_key(
        state: State<'_, AppState>,
    ) -> Result<security::SteamWebApiKeyStateDto, String> {
        let state_dto = state
            .auth_vault
            .disconnect_steam_web_api_key()
            .map_err(|error| error.to_string())?;
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::set_steam_web_api_key_config(&mut connection, false)
            .map_err(|error| error.to_string())?;

        Ok(state_dto)
    }

    fn resolve_steam_sync_credentials(
        steam_id: Option<String>,
        api_key: Result<Option<String>, security::AuthVaultError>,
    ) -> Result<(String, String), ProviderErrorDto> {
        let steam_id = steam_id.ok_or_else(|| {
            ProviderErrorDto::steam(
                "steam_account_missing",
                "Configure o SteamID64 da conta Steam antes de sincronizar pela Web API.",
                true,
                "preflight",
                Some("steam_id64 ausente".to_string()),
            )
        })?;

        let api_key = api_key
            .map_err(|error| {
                ProviderErrorDto::steam(
                    "steam_web_api_key_unavailable",
                    "Nao foi possivel acessar a Steam Web API key no cofre seguro.",
                    true,
                    "preflight",
                    Some(error.to_string()),
                )
            })?
            .ok_or_else(|| {
                ProviderErrorDto::steam(
                    "steam_web_api_key_missing",
                    "Configure a Steam Web API key no cofre seguro antes de sincronizar.",
                    true,
                    "preflight",
                    Some("credencial ausente no AuthVault".to_string()),
                )
            })?;

        Ok((steam_id, api_key))
    }

    fn sync_steam_account_games_impl(
        state: &AppState,
    ) -> Result<storage::SyncSummaryDto, ProviderErrorDto> {
        let steam_id = {
            let connection = state.connection.lock().map_err(|_| {
                ProviderErrorDto::steam(
                    "local_database_lock_unavailable",
                    "Nao foi possivel acessar o banco local para ler a conta Steam.",
                    true,
                    "preflight",
                    Some("falha ao adquirir o lock local".to_string()),
                )
            })?;

            storage::read_steam_account_config(&connection).map_err(|error| {
                ProviderErrorDto::steam(
                    "steam_account_read_failed",
                    "Nao foi possivel ler a conta Steam configurada.",
                    true,
                    "preflight",
                    Some(error.to_string()),
                )
            })?
        };

        let (steam_id, api_key) =
            resolve_steam_sync_credentials(steam_id, state.auth_vault.steam_web_api_key())?;

        let client = steam_web_api::ReqwestSteamWebApiClient::default();
        let remote_games = steam_web_api::fetch_owned_games(&client, &api_key, &steam_id)
            .map_err(|error| error.into_provider_error())?;

        let mut connection = state.connection.lock().map_err(|_| {
            ProviderErrorDto::steam(
                "local_database_lock_unavailable",
                "Nao foi possivel acessar o banco local para aplicar a sincronizacao.",
                true,
                "merge",
                Some("falha ao adquirir o lock local".to_string()),
            )
        })?;

        let summary = storage::sync_steam_account_games(&mut connection, &steam_id, &remote_games)
            .map_err(|error| {
                ProviderErrorDto::steam(
                    "steam_account_merge_failed",
                    "Nao foi possivel aplicar a sincronizacao da conta Steam no banco local.",
                    false,
                    "merge",
                    Some(error.to_string()),
                )
            })?;

        storage::record_steam_account_sync_metadata(
            &mut connection,
            &steam_id,
            remote_games.len(),
            &summary,
        )
        .map_err(|error| {
            ProviderErrorDto::steam(
                "steam_account_metadata_update_failed",
                "Nao foi possivel atualizar os metadados da conta Steam sincronizada.",
                true,
                "merge",
                Some(error.to_string()),
            )
        })?;

        Ok(summary)
    }

    #[tauri::command]
    pub fn update_manual_game(
        entry_id: String,
        input: storage::ManualGameInput,
        state: State<'_, AppState>,
    ) -> Result<storage::LibraryEntryDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::update_manual_game(&mut connection, &entry_id, input)
            .map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn set_library_entry_archived(
        entry_id: String,
        is_archived: bool,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::set_library_entry_archived(&mut connection, &entry_id, is_archived)
            .map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn launch_library_entry(
        entry_id: String,
        state: State<'_, AppState>,
    ) -> Result<launcher::LaunchResultDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        launcher::launch_library_entry(&connection, &entry_id).map_err(|error| error.to_string())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn resolve_steam_sync_credentials_rejects_missing_account() {
            let error = resolve_steam_sync_credentials(None, Ok(Some("abcdef".to_string())))
                .expect_err("missing steam account");

            assert_eq!(error.code, "steam_account_missing");
            assert_eq!(error.phase, "preflight");
            assert!(error.recoverable);
            assert_eq!(error.provider_id, "steam");
            assert_eq!(
                error.details_sanitized.as_deref(),
                Some("steam_id64 ausente")
            );
        }

        #[test]
        fn resolve_steam_sync_credentials_rejects_missing_api_key() {
            let error =
                resolve_steam_sync_credentials(Some("76561198000000000".to_string()), Ok(None))
                    .expect_err("missing api key");

            assert_eq!(error.code, "steam_web_api_key_missing");
            assert_eq!(error.phase, "preflight");
            assert!(error.recoverable);
            assert_eq!(error.provider_id, "steam");
            assert_eq!(
                error.details_sanitized.as_deref(),
                Some("credencial ausente no AuthVault")
            );
        }

        #[test]
        fn resolve_steam_sync_credentials_maps_vault_errors() {
            let error = resolve_steam_sync_credentials(
                Some("76561198000000000".to_string()),
                Err(security::AuthVaultError::LockUnavailable),
            )
            .expect_err("vault error");

            assert_eq!(error.code, "steam_web_api_key_unavailable");
            assert_eq!(error.phase, "preflight");
            assert!(error.recoverable);
            assert_eq!(error.provider_id, "steam");
            assert!(!error
                .details_sanitized
                .as_deref()
                .expect("details")
                .is_empty());
        }
    }
}

mod security {
    use keyring::{Entry, Error as KeyringError};
    use serde::{Deserialize, Serialize};
    use std::fmt;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    const STEAM_WEB_API_SERVICE: &str = "com.bibliotecajogos.unificada";
    const STEAM_WEB_API_USER: &str = "steam-web-api-key";
    const STEAM_WEB_API_KEY_LENGTH: usize = 32;
    const STEAM_WEB_API_KEY_FILE: &str = "steam-web-api-key.dpapi";
    const STEAM_WEB_API_DPAPI_ENTROPY: &[u8] = b"com.bibliotecajogos.unificada/steam-web-api-key";

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct SteamWebApiKeyInput {
        api_key: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamWebApiKeyStateDto {
        configured: bool,
        provider_id: &'static str,
        storage: &'static str,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum AuthVaultError {
        InvalidSteamWebApiKey,
        SecureStorageUnavailable { operation: &'static str },
        LockUnavailable,
    }

    impl fmt::Display for AuthVaultError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            let message = match self {
                AuthVaultError::InvalidSteamWebApiKey => {
                    "Chave Steam Web API invalida. Informe uma chave hexadecimal de 32 caracteres."
                }
                AuthVaultError::SecureStorageUnavailable { operation } => {
                    return write!(
                        formatter,
                        "Nao foi possivel acessar o cofre seguro do sistema operacional durante {operation}."
                    );
                }
                AuthVaultError::LockUnavailable => {
                    "Nao foi possivel atualizar o estado de autenticacao local."
                }
            };

            formatter.write_str(message)
        }
    }

    pub struct AuthVault {
        store: Arc<dyn SecretStore>,
        state: Mutex<AuthVaultState>,
    }

    #[derive(Debug, Clone, Copy)]
    struct AuthVaultState {
        steam_web_api_key_configured: bool,
    }

    impl AuthVault {
        pub fn system(vault_dir: PathBuf) -> Self {
            Self::new(Arc::new(SystemSecretStore::new(vault_dir)))
        }

        fn new(store: Arc<dyn SecretStore>) -> Self {
            let steam_web_api_key_configured = store.exists().unwrap_or(false);

            Self {
                store,
                state: Mutex::new(AuthVaultState {
                    steam_web_api_key_configured,
                }),
            }
        }

        pub fn save_steam_web_api_key(
            &self,
            input: SteamWebApiKeyInput,
        ) -> Result<SteamWebApiKeyStateDto, AuthVaultError> {
            let api_key = validate_steam_web_api_key(&input.api_key)?;
            self.store.set(api_key)?;
            if self.store.get()?.as_deref() != Some(api_key) {
                return Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "validacao da credencial",
                });
            }
            self.set_steam_web_api_key_configured(true)
        }

        pub fn steam_web_api_key_state(&self) -> Result<SteamWebApiKeyStateDto, AuthVaultError> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AuthVaultError::LockUnavailable)?;
            state.steam_web_api_key_configured = self.store.exists()?;

            Ok(steam_web_api_key_state_dto(
                state.steam_web_api_key_configured,
                "auth_vault",
            ))
        }

        pub fn steam_web_api_key(&self) -> Result<Option<String>, AuthVaultError> {
            self.store.get()
        }

        pub fn disconnect_steam_web_api_key(
            &self,
        ) -> Result<SteamWebApiKeyStateDto, AuthVaultError> {
            self.store.delete()?;
            self.set_steam_web_api_key_configured(false)
        }

        fn set_steam_web_api_key_configured(
            &self,
            configured: bool,
        ) -> Result<SteamWebApiKeyStateDto, AuthVaultError> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AuthVaultError::LockUnavailable)?;
            state.steam_web_api_key_configured = configured;

            Ok(steam_web_api_key_state_dto(configured, "auth_vault"))
        }
    }

    fn steam_web_api_key_state_dto(
        configured: bool,
        storage: &'static str,
    ) -> SteamWebApiKeyStateDto {
        SteamWebApiKeyStateDto {
            configured,
            provider_id: "steam",
            storage,
        }
    }

    fn validate_steam_web_api_key(input: &str) -> Result<&str, AuthVaultError> {
        let api_key = input.trim();

        if api_key.len() == STEAM_WEB_API_KEY_LENGTH
            && api_key.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            Ok(api_key)
        } else {
            Err(AuthVaultError::InvalidSteamWebApiKey)
        }
    }

    trait SecretStore: Send + Sync {
        fn set(&self, secret: &str) -> Result<(), AuthVaultError>;
        fn get(&self) -> Result<Option<String>, AuthVaultError>;
        fn exists(&self) -> Result<bool, AuthVaultError>;
        fn delete(&self) -> Result<(), AuthVaultError>;
    }

    struct SystemSecretStore {
        fallback: DpapiFileSecretStore,
    }

    impl SystemSecretStore {
        fn new(vault_dir: PathBuf) -> Self {
            Self {
                fallback: DpapiFileSecretStore::new(vault_dir.join(STEAM_WEB_API_KEY_FILE)),
            }
        }

        fn entry(&self) -> Result<Entry, AuthVaultError> {
            Entry::new(STEAM_WEB_API_SERVICE, STEAM_WEB_API_USER).map_err(|_| {
                AuthVaultError::SecureStorageUnavailable {
                    operation: "preparacao do AuthVault",
                }
            })
        }

        fn set_keyring_secret(&self, secret: &str) -> Result<(), AuthVaultError> {
            self.entry()?.set_password(secret).map_err(|_| {
                AuthVaultError::SecureStorageUnavailable {
                    operation: "gravacao da credencial",
                }
            })
        }

        fn get_keyring_secret(&self) -> Result<Option<String>, AuthVaultError> {
            match self.entry()?.get_password() {
                Ok(secret) => Ok(Some(secret)),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "leitura da credencial",
                }),
            }
        }

        fn delete_keyring_secret(&self) -> Result<(), AuthVaultError> {
            match self.entry()?.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "remocao da credencial",
                }),
            }
        }
    }

    impl SecretStore for SystemSecretStore {
        fn set(&self, secret: &str) -> Result<(), AuthVaultError> {
            if self.set_keyring_secret(secret).is_ok()
                && self.get_keyring_secret().ok().flatten().as_deref() == Some(secret)
            {
                let _ = self.fallback.delete();
                return Ok(());
            }

            self.fallback.set(secret)?;
            if self.fallback.get()?.as_deref() == Some(secret) {
                match self.delete_keyring_secret() {
                    Ok(()) => Ok(()),
                    Err(error) if self.get_keyring_secret().ok().flatten().is_some() => Err(error),
                    Err(_) => Ok(()),
                }
            } else {
                Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "validacao da credencial",
                })
            }
        }

        fn get(&self) -> Result<Option<String>, AuthVaultError> {
            if let Ok(Some(secret)) = self.get_keyring_secret() {
                return Ok(Some(secret));
            }

            self.fallback.get()
        }

        fn exists(&self) -> Result<bool, AuthVaultError> {
            self.get().map(|secret| secret.is_some())
        }

        fn delete(&self) -> Result<(), AuthVaultError> {
            let keyring_result = self.delete_keyring_secret();
            let fallback_result = self.fallback.delete();

            fallback_result?;
            match keyring_result {
                Ok(()) => Ok(()),
                Err(error) if self.get_keyring_secret().ok().flatten().is_some() => Err(error),
                Err(_) => Ok(()),
            }
        }
    }

    struct DpapiFileSecretStore {
        path: PathBuf,
    }

    impl DpapiFileSecretStore {
        fn new(path: PathBuf) -> Self {
            Self { path }
        }
    }

    impl SecretStore for DpapiFileSecretStore {
        fn set(&self, secret: &str) -> Result<(), AuthVaultError> {
            let protected_secret = protect_secret(secret.as_bytes())?;
            let parent_dir =
                self.path
                    .parent()
                    .ok_or(AuthVaultError::SecureStorageUnavailable {
                        operation: "preparacao do cofre DPAPI",
                    })?;
            std::fs::create_dir_all(parent_dir).map_err(|_| {
                AuthVaultError::SecureStorageUnavailable {
                    operation: "preparacao do cofre DPAPI",
                }
            })?;

            let temp_path = self.path.with_extension("dpapi.tmp");
            let _ = std::fs::remove_file(&temp_path);
            {
                let mut file = std::fs::File::create(&temp_path).map_err(|_| {
                    let _ = std::fs::remove_file(&temp_path);
                    AuthVaultError::SecureStorageUnavailable {
                        operation: "gravacao da credencial DPAPI",
                    }
                })?;
                std::io::Write::write_all(&mut file, &protected_secret).map_err(|_| {
                    let _ = std::fs::remove_file(&temp_path);
                    AuthVaultError::SecureStorageUnavailable {
                        operation: "gravacao da credencial DPAPI",
                    }
                })?;
                file.sync_all().map_err(|_| {
                    let _ = std::fs::remove_file(&temp_path);
                    AuthVaultError::SecureStorageUnavailable {
                        operation: "gravacao da credencial DPAPI",
                    }
                })?;
            }
            std::fs::rename(&temp_path, &self.path).map_err(|_| {
                let _ = std::fs::remove_file(&temp_path);
                AuthVaultError::SecureStorageUnavailable {
                    operation: "gravacao da credencial DPAPI",
                }
            })
        }

        fn get(&self) -> Result<Option<String>, AuthVaultError> {
            let protected_secret = match std::fs::read(&self.path) {
                Ok(secret) => secret,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(_) => {
                    return Err(AuthVaultError::SecureStorageUnavailable {
                        operation: "leitura da credencial DPAPI",
                    });
                }
            };
            let secret = unprotect_secret(&protected_secret)?;

            String::from_utf8(secret).map(Some).map_err(|_| {
                AuthVaultError::SecureStorageUnavailable {
                    operation: "leitura da credencial DPAPI",
                }
            })
        }

        fn exists(&self) -> Result<bool, AuthVaultError> {
            self.get().map(|secret| secret.is_some())
        }

        fn delete(&self) -> Result<(), AuthVaultError> {
            let temp_path = self.path.with_extension("dpapi.tmp");
            let _ = std::fs::remove_file(temp_path);

            match std::fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "remocao da credencial DPAPI",
                }),
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn protect_secret(secret: &[u8]) -> Result<Vec<u8>, AuthVaultError> {
        use std::ptr;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Cryptography::{
            CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };

        let entropy = CRYPT_INTEGER_BLOB {
            cbData: STEAM_WEB_API_DPAPI_ENTROPY.len() as u32,
            pbData: STEAM_WEB_API_DPAPI_ENTROPY.as_ptr() as *mut u8,
        };
        let input = CRYPT_INTEGER_BLOB {
            cbData: secret.len() as u32,
            pbData: secret.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        let success = unsafe {
            CryptProtectData(
                &input,
                ptr::null(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if success == 0 || output.pbData.is_null() {
            return Err(AuthVaultError::SecureStorageUnavailable {
                operation: "criptografia da credencial DPAPI",
            });
        }

        let protected_secret =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe {
            LocalFree(output.pbData.cast());
        }

        Ok(protected_secret)
    }

    #[cfg(not(target_os = "windows"))]
    fn protect_secret(_secret: &[u8]) -> Result<Vec<u8>, AuthVaultError> {
        Err(AuthVaultError::SecureStorageUnavailable {
            operation: "criptografia da credencial DPAPI",
        })
    }

    #[cfg(target_os = "windows")]
    fn unprotect_secret(protected_secret: &[u8]) -> Result<Vec<u8>, AuthVaultError> {
        use std::ptr;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };

        let entropy = CRYPT_INTEGER_BLOB {
            cbData: STEAM_WEB_API_DPAPI_ENTROPY.len() as u32,
            pbData: STEAM_WEB_API_DPAPI_ENTROPY.as_ptr() as *mut u8,
        };
        let input = CRYPT_INTEGER_BLOB {
            cbData: protected_secret.len() as u32,
            pbData: protected_secret.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        let success = unsafe {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if success == 0 || output.pbData.is_null() {
            return Err(AuthVaultError::SecureStorageUnavailable {
                operation: "descriptografia da credencial DPAPI",
            });
        }

        let secret =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe {
            LocalFree(output.pbData.cast());
        }

        Ok(secret)
    }

    #[cfg(not(target_os = "windows"))]
    fn unprotect_secret(_protected_secret: &[u8]) -> Result<Vec<u8>, AuthVaultError> {
        Err(AuthVaultError::SecureStorageUnavailable {
            operation: "descriptografia da credencial DPAPI",
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn validates_steam_web_api_key_format() {
            assert!(validate_steam_web_api_key("0123456789abcdefABCDEF0123456789").is_ok());
            assert_eq!(
                validate_steam_web_api_key("").expect_err("reject empty"),
                AuthVaultError::InvalidSteamWebApiKey
            );
            assert_eq!(
                validate_steam_web_api_key("0123456789abcdef").expect_err("reject short"),
                AuthVaultError::InvalidSteamWebApiKey
            );
            assert_eq!(
                validate_steam_web_api_key("0123456789abcdefABCDEF01234567ZZ")
                    .expect_err("reject non hex"),
                AuthVaultError::InvalidSteamWebApiKey
            );
        }

        #[test]
        fn rejects_unknown_payload_fields() {
            let result = serde_json::from_str::<SteamWebApiKeyInput>(
                r#"{"apiKey":"0123456789abcdefABCDEF0123456789","unexpected":true}"#,
            );

            assert!(result.is_err());
        }

        #[test]
        fn save_state_and_disconnect_use_secret_store_without_exposing_secret() {
            let store = Arc::new(InMemorySecretStore::default());
            let vault = AuthVault::new(store.clone());

            assert_eq!(
                vault.steam_web_api_key_state().expect("read empty state"),
                steam_web_api_key_state_dto(false, "auth_vault")
            );

            let saved_state = vault
                .save_steam_web_api_key(SteamWebApiKeyInput {
                    api_key: "0123456789abcdefABCDEF0123456789".to_string(),
                })
                .expect("save api key");

            assert_eq!(saved_state, steam_web_api_key_state_dto(true, "auth_vault"));
            assert_eq!(
                store.read_secret(),
                Some("0123456789abcdefABCDEF0123456789".to_string())
            );
            assert_eq!(
                vault.steam_web_api_key_state().expect("read saved state"),
                steam_web_api_key_state_dto(true, "auth_vault")
            );

            let disconnected_state = vault
                .disconnect_steam_web_api_key()
                .expect("disconnect api key");

            assert_eq!(
                disconnected_state,
                steam_web_api_key_state_dto(false, "auth_vault")
            );
            assert_eq!(store.read_secret(), None);
        }

        #[test]
        fn failed_disconnect_keeps_local_state_configured() {
            let store = Arc::new(InMemorySecretStore::default());
            let vault = AuthVault::new(store.clone());
            vault
                .save_steam_web_api_key(SteamWebApiKeyInput {
                    api_key: "0123456789abcdefABCDEF0123456789".to_string(),
                })
                .expect("save api key");
            store.fail_deletes();

            assert_eq!(
                vault
                    .disconnect_steam_web_api_key()
                    .expect_err("delete should fail"),
                AuthVaultError::SecureStorageUnavailable {
                    operation: "remocao da credencial"
                }
            );
            assert_eq!(
                vault.steam_web_api_key_state().expect("read state"),
                steam_web_api_key_state_dto(true, "auth_vault")
            );
        }

        #[derive(Default)]
        struct InMemorySecretStore {
            secrets: Mutex<HashMap<String, String>>,
            delete_should_fail: Mutex<bool>,
        }

        impl InMemorySecretStore {
            fn read_secret(&self) -> Option<String> {
                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .get(STEAM_WEB_API_USER)
                    .cloned()
            }

            fn fail_deletes(&self) {
                *self.delete_should_fail.lock().expect("lock delete flag") = true;
            }
        }

        impl SecretStore for InMemorySecretStore {
            fn set(&self, secret: &str) -> Result<(), AuthVaultError> {
                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .insert(STEAM_WEB_API_USER.to_string(), secret.to_string());
                Ok(())
            }

            fn get(&self) -> Result<Option<String>, AuthVaultError> {
                Ok(self
                    .secrets
                    .lock()
                    .expect("lock secrets")
                    .get(STEAM_WEB_API_USER)
                    .cloned())
            }

            fn exists(&self) -> Result<bool, AuthVaultError> {
                Ok(self
                    .secrets
                    .lock()
                    .expect("lock secrets")
                    .contains_key(STEAM_WEB_API_USER))
            }

            fn delete(&self) -> Result<(), AuthVaultError> {
                if *self.delete_should_fail.lock().expect("lock delete flag") {
                    return Err(AuthVaultError::SecureStorageUnavailable {
                        operation: "remocao da credencial",
                    });
                }

                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .remove(STEAM_WEB_API_USER);
                Ok(())
            }
        }
    }
}

mod launcher {
    use rusqlite::{params, Connection, OptionalExtension};
    use serde::Serialize;
    use std::fmt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use url::Url;

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LaunchResultDto {
        started: bool,
        message: String,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum LaunchValidationError {
        EmptyTarget,
        RelativePath,
        NotFound,
        NotFile,
        UnsupportedExtension,
        InvalidWorkingDirectory,
        RemotePath,
        InvalidUri,
        UriLaunchFailed,
        UnsupportedLaunchKind,
        NoLaunchAction,
    }

    impl fmt::Display for LaunchValidationError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            let message = match self {
                LaunchValidationError::EmptyTarget => "Informe o caminho do executavel.",
                LaunchValidationError::RelativePath => {
                    "Use um caminho absoluto para o executavel local."
                }
                LaunchValidationError::NotFound => "Arquivo executavel nao encontrado.",
                LaunchValidationError::NotFile => "O caminho informado nao aponta para um arquivo.",
                LaunchValidationError::UnsupportedExtension => {
                    "Por seguranca, apenas arquivos .exe sao suportados nesta etapa."
                }
                LaunchValidationError::InvalidWorkingDirectory => {
                    "O diretorio de trabalho configurado nao existe."
                }
                LaunchValidationError::RemotePath => {
                    "Caminhos de rede nao sao suportados nesta etapa."
                }
                LaunchValidationError::InvalidUri => {
                    "Informe uma URI valida para esta acao de lancamento."
                }
                LaunchValidationError::UriLaunchFailed => {
                    "Nao foi possivel abrir a URI solicitada."
                }
                LaunchValidationError::UnsupportedLaunchKind => {
                    "A acao de lancamento persistida nao e suportada."
                }
                LaunchValidationError::NoLaunchAction => {
                    "Nenhuma acao de lancamento persistida foi encontrada para este jogo."
                }
            };

            formatter.write_str(message)
        }
    }

    pub fn launch_library_entry(
        connection: &Connection,
        entry_id: &str,
    ) -> Result<LaunchResultDto, String> {
        let action = find_executable_action(connection, entry_id)
            .map_err(|_| "Nao foi possivel consultar a acao de lancamento.".to_string())?
            .ok_or_else(|| LaunchValidationError::NoLaunchAction.to_string())?;

        match action.kind.as_str() {
            "executable" => {
                let target = validate_executable_path(Path::new(&action.target))
                    .map_err(|error| error.to_string())?;
                let working_directory =
                    resolve_working_directory(&target, action.working_directory.as_deref())
                        .map_err(|error| error.to_string())?;

                let mut command = Command::new(&target);
                if !action.arguments.is_empty() {
                    command.args(&action.arguments);
                }
                command
                    .current_dir(working_directory)
                    .spawn()
                    .map_err(|_| "Nao foi possivel iniciar o executavel local.".to_string())?;
            }
            "uri" => {
                let uri = validate_launch_uri(&action.target).map_err(|error| error.to_string())?;
                open_uri(&uri).map_err(|error| error.to_string())?;
            }
            _ => {
                return Err(LaunchValidationError::UnsupportedLaunchKind.to_string());
            }
        }

        Ok(LaunchResultDto {
            started: true,
            message: "Inicializacao do jogo solicitada.".to_string(),
        })
    }

    struct LaunchAction {
        kind: String,
        target: String,
        arguments: Vec<String>,
        working_directory: Option<String>,
    }

    fn find_executable_action(
        connection: &Connection,
        entry_id: &str,
    ) -> rusqlite::Result<Option<LaunchAction>> {
        connection
            .query_row(
                r#"
                SELECT launch_actions.kind, launch_actions.target, launch_actions.arguments_json, launch_actions.working_directory
                FROM library_entries
                JOIN launch_actions ON launch_actions.game_id = library_entries.game_id
                WHERE library_entries.id = ?1
                  AND library_entries.primary_platform_id IN ('manual', 'local', 'xbox')
                  AND library_entries.is_archived = 0
                  AND launch_actions.is_primary = 1
                "#,
                params![entry_id],
                |row| {
                    let arguments = row
                        .get::<_, Option<String>>(2)?
                        .and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
                        .unwrap_or_default();

                    Ok(LaunchAction {
                        kind: row.get(0)?,
                        target: row.get(1)?,
                        arguments,
                        working_directory: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    fn validate_launch_uri(target: &str) -> Result<Url, LaunchValidationError> {
        let uri = Url::parse(target).map_err(|_| LaunchValidationError::InvalidUri)?;

        if uri.scheme().is_empty() {
            return Err(LaunchValidationError::InvalidUri);
        }

        Ok(uri)
    }

    #[cfg(target_os = "windows")]
    fn open_uri(uri: &Url) -> Result<(), LaunchValidationError> {
        if uri.scheme() == "ms-windows-store" {
            Command::new("explorer.exe")
                .arg(uri.as_str())
                .spawn()
                .map_err(|_| LaunchValidationError::UriLaunchFailed)?;
        } else {
            Command::new("rundll32.exe")
                .args(["url.dll,FileProtocolHandler", uri.as_str()])
                .spawn()
                .map_err(|_| LaunchValidationError::UriLaunchFailed)?;
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn open_uri(uri: &Url) -> Result<(), LaunchValidationError> {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };

        Command::new(opener)
            .arg(uri.as_str())
            .spawn()
            .map_err(|_| LaunchValidationError::UriLaunchFailed)?;
        Ok(())
    }

    fn validate_executable_path(path: &Path) -> Result<PathBuf, LaunchValidationError> {
        if path.as_os_str().is_empty() {
            return Err(LaunchValidationError::EmptyTarget);
        }

        if is_remote_path(path) {
            return Err(LaunchValidationError::RemotePath);
        }

        if !path.is_absolute() {
            return Err(LaunchValidationError::RelativePath);
        }

        if !path.exists() {
            return Err(LaunchValidationError::NotFound);
        }

        if !path.is_file() {
            return Err(LaunchValidationError::NotFile);
        }

        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();

        if !extension.eq_ignore_ascii_case("exe") {
            return Err(LaunchValidationError::UnsupportedExtension);
        }

        path.canonicalize()
            .map_err(|_| LaunchValidationError::NotFound)
    }

    fn is_remote_path(path: &Path) -> bool {
        path.to_string_lossy().starts_with(r"\\")
    }

    fn resolve_working_directory(
        target: &Path,
        configured_working_directory: Option<&str>,
    ) -> Result<PathBuf, LaunchValidationError> {
        if let Some(working_directory) = configured_working_directory
            .map(str::trim)
            .filter(|working_directory| !working_directory.is_empty())
        {
            let path = PathBuf::from(working_directory);

            if path.is_absolute() && path.is_dir() {
                return Ok(path);
            }

            return Err(LaunchValidationError::InvalidWorkingDirectory);
        }

        Ok(target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;

        #[test]
        fn rejects_empty_target() {
            assert_eq!(
                validate_executable_path(Path::new("")).expect_err("reject empty"),
                LaunchValidationError::EmptyTarget,
            );
        }

        #[test]
        fn rejects_relative_target() {
            assert_eq!(
                validate_executable_path(Path::new("game.exe")).expect_err("reject relative"),
                LaunchValidationError::RelativePath,
            );
        }

        #[test]
        fn rejects_missing_target() {
            let missing = std::env::temp_dir().join("biblioteca-jogos-missing-game.exe");

            assert_eq!(
                validate_executable_path(&missing).expect_err("reject missing"),
                LaunchValidationError::NotFound,
            );
        }

        #[test]
        fn rejects_remote_target() {
            assert_eq!(
                validate_executable_path(Path::new(r"\\server\share\game.exe"))
                    .expect_err("reject remote"),
                LaunchValidationError::RemotePath,
            );
        }

        #[test]
        fn rejects_directory_target() {
            assert_eq!(
                validate_executable_path(&std::env::temp_dir()).expect_err("reject directory"),
                LaunchValidationError::NotFile,
            );
        }

        #[test]
        fn rejects_non_exe_target() {
            let path = std::env::temp_dir().join("biblioteca-jogos-launch-test.txt");
            fs::write(&path, "test").expect("write temp file");

            assert_eq!(
                validate_executable_path(&path).expect_err("reject non exe"),
                LaunchValidationError::UnsupportedExtension,
            );

            let _ = fs::remove_file(path);
        }

        #[test]
        fn accepts_existing_exe_target() {
            let path = std::env::temp_dir().join("biblioteca-jogos-launch-test.exe");
            fs::write(&path, "test").expect("write temp exe");

            assert!(validate_executable_path(&path).is_ok());

            let _ = fs::remove_file(path);
        }

        #[test]
        fn validates_launch_uri_accepts_steam_uri() {
            let uri = validate_launch_uri("steam://rungameid/1030300").expect("accept steam uri");

            assert_eq!(uri.scheme(), "steam");
            assert_eq!(uri.as_str(), "steam://rungameid/1030300");
        }

        #[test]
        fn validates_launch_uri_rejects_invalid_targets() {
            assert_eq!(
                validate_launch_uri("not-a-uri").expect_err("reject invalid uri"),
                LaunchValidationError::InvalidUri,
            );
        }

        #[test]
        fn validates_launch_uri_accepts_store_uri() {
            let uri = validate_launch_uri("ms-windows-store://search/?query=Halo%20Infinite")
                .expect("accept store uri");

            assert_eq!(uri.scheme(), "ms-windows-store");
            assert_eq!(
                uri.as_str(),
                "ms-windows-store://search/?query=Halo%20Infinite"
            );
        }

        #[test]
        fn parses_executable_launch_arguments_json() {
            let connection = Connection::open_in_memory().expect("open memory db");
            connection
                .execute_batch(
                    r#"
                    CREATE TABLE library_entries (
                        id TEXT PRIMARY KEY,
                        game_id TEXT NOT NULL UNIQUE,
                        primary_platform_id TEXT NOT NULL,
                        install_status TEXT NOT NULL,
                        last_played_label TEXT NOT NULL,
                        is_archived INTEGER NOT NULL DEFAULT 0,
                        added_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL
                    );

                    CREATE TABLE launch_actions (
                        id TEXT PRIMARY KEY,
                        game_id TEXT NOT NULL,
                        platform_id TEXT NOT NULL,
                        kind TEXT NOT NULL,
                        label TEXT NOT NULL,
                        target TEXT NOT NULL,
                        arguments_json TEXT,
                        working_directory TEXT,
                        is_primary INTEGER NOT NULL DEFAULT 0
                    );
                    "#,
                )
                .expect("create schema");
            connection
                .execute(
                    r#"
                    INSERT INTO library_entries (
                        id, game_id, primary_platform_id, install_status, last_played_label, is_archived, added_at, updated_at
                    ) VALUES ('entry-xbox-test', 'game-xbox-test', 'xbox', 'installed', 'Nunca', 0, '2026-05-18T00:00:00.000Z', '2026-05-18T00:00:00.000Z')
                    "#,
                    [],
                )
                .expect("insert entry");
            connection
                .execute(
                    r#"
                    INSERT INTO launch_actions (
                        id, game_id, platform_id, kind, label, target, arguments_json, working_directory, is_primary
                    ) VALUES ('launch-xbox-test', 'game-xbox-test', 'xbox', 'executable', 'Jogar no Xbox', 'explorer.exe', '["shell:AppsFolder\\Microsoft.HaloInfinite_8wekyb3d8bbwe!App"]', NULL, 1)
                    "#,
                    [],
                )
                .expect("insert launch action");

            let action = find_executable_action(&connection, "entry-xbox-test")
                .expect("query launch action")
                .expect("launch action exists");

            assert_eq!(action.kind, "executable");
            assert_eq!(action.target, "explorer.exe");
            assert_eq!(
                action.arguments,
                vec!["shell:AppsFolder\\Microsoft.HaloInfinite_8wekyb3d8bbwe!App".to_string()]
            );
        }

        #[test]
        fn resolves_default_working_directory_from_target_parent() {
            let target = std::env::temp_dir().join("biblioteca-jogos-launch-test.exe");

            assert_eq!(
                resolve_working_directory(&target, None).expect("resolve working directory"),
                std::env::temp_dir(),
            );
        }
    }
}

mod steam_web_api {
    use super::ProviderErrorDto;
    use serde::Deserialize;
    use std::fmt;
    use url::Url;

    const STEAM_OWNED_GAMES_ENDPOINT: &str =
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/";

    pub trait SteamWebApiClient {
        fn get_owned_games(&self, url: &Url) -> Result<String, SteamWebApiError>;
    }

    #[derive(Default)]
    pub struct ReqwestSteamWebApiClient {
        client: reqwest::blocking::Client,
    }

    impl SteamWebApiClient for ReqwestSteamWebApiClient {
        fn get_owned_games(&self, url: &Url) -> Result<String, SteamWebApiError> {
            let response = self
                .client
                .get(url.clone())
                .send()
                .map_err(|_| SteamWebApiError::network_unavailable())?;
            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(SteamWebApiError::rate_limited());
            }

            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
                || status == reqwest::StatusCode::BAD_REQUEST
            {
                return Err(SteamWebApiError::auth_required(Some(format!(
                    "resposta HTTP {} da Steam Web API",
                    status.as_u16()
                ))));
            }

            if !status.is_success() {
                return Err(SteamWebApiError::platform_unavailable(Some(format!(
                    "resposta HTTP {} da Steam Web API",
                    status.as_u16()
                ))));
            }

            response.text().map_err(|_| {
                SteamWebApiError::platform_unavailable(Some(
                    "nao foi possivel ler o corpo da resposta".to_string(),
                ))
            })
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SteamWebApiError {
        code: &'static str,
        message: &'static str,
        recoverable: bool,
        phase: &'static str,
        details_sanitized: Option<String>,
    }

    impl fmt::Display for SteamWebApiError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.message)
        }
    }

    impl SteamWebApiError {
        fn new(
            code: &'static str,
            message: &'static str,
            recoverable: bool,
            phase: &'static str,
            details_sanitized: Option<String>,
        ) -> Self {
            Self {
                code,
                message,
                recoverable,
                phase,
                details_sanitized,
            }
        }

        fn auth_required(details_sanitized: Option<String>) -> Self {
            Self::new(
                "steam_web_api_auth_required",
                "Nao foi possivel autenticar na Steam Web API. Verifique a chave salva no cofre.",
                true,
                "request",
                details_sanitized,
            )
        }

        fn network_unavailable() -> Self {
            Self::new(
                "steam_web_api_network_unavailable",
                "Nao foi possivel conectar a Steam Web API. Verifique a conexao e tente novamente.",
                true,
                "request",
                Some("falha de rede ao consultar a Steam Web API".to_string()),
            )
        }

        fn platform_unavailable(details_sanitized: Option<String>) -> Self {
            Self::new(
                "steam_web_api_platform_unavailable",
                "A Steam Web API nao respondeu como esperado. Tente novamente mais tarde.",
                true,
                "request",
                details_sanitized,
            )
        }

        fn rate_limited() -> Self {
            Self::new(
                "steam_web_api_rate_limited",
                "A Steam Web API limitou as requisicoes. Aguarde antes de sincronizar novamente.",
                true,
                "request",
                Some("resposta HTTP 429 da Steam Web API".to_string()),
            )
        }

        fn parse_failed(details_sanitized: Option<String>) -> Self {
            Self::new(
                "steam_web_api_parse_failed",
                "Nao foi possivel ler a resposta da Steam Web API.",
                true,
                "parse",
                details_sanitized,
            )
        }

        pub(crate) fn into_provider_error(self) -> ProviderErrorDto {
            ProviderErrorDto::steam(
                self.code,
                self.message,
                self.recoverable,
                self.phase,
                self.details_sanitized,
            )
        }

        pub(crate) fn code(&self) -> &'static str {
            self.code
        }

        pub(crate) fn recoverable(&self) -> bool {
            self.recoverable
        }

        pub(crate) fn phase(&self) -> &'static str {
            self.phase
        }

        pub(crate) fn details_sanitized(&self) -> Option<&str> {
            self.details_sanitized.as_deref()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RemoteSteamGame {
        pub app_id: String,
        pub title: String,
        pub playtime_forever: Option<i64>,
    }

    pub fn fetch_owned_games(
        client: &impl SteamWebApiClient,
        api_key: &str,
        steam_id: &str,
    ) -> Result<Vec<RemoteSteamGame>, SteamWebApiError> {
        let mut url = Url::parse(STEAM_OWNED_GAMES_ENDPOINT)
            .map_err(|_| SteamWebApiError::parse_failed(Some("endpoint invalido".to_string())))?;
        url.query_pairs_mut()
            .append_pair("key", api_key)
            .append_pair("steamid", steam_id)
            .append_pair("include_appinfo", "1")
            .append_pair("include_played_free_games", "1")
            .append_pair("format", "json");

        let body = client.get_owned_games(&url)?;
        parse_owned_games_response(&body)
    }

    fn parse_owned_games_response(body: &str) -> Result<Vec<RemoteSteamGame>, SteamWebApiError> {
        let payload: SteamOwnedGamesResponse = serde_json::from_str(body).map_err(|_| {
            SteamWebApiError::parse_failed(Some("payload JSON invalido".to_string()))
        })?;

        let mut games = Vec::new();
        let mut discarded_games = 0usize;

        for game in payload.response.games.unwrap_or_default() {
            if let Some(remote_game) = parse_owned_game(&game) {
                games.push(remote_game);
            } else {
                discarded_games += 1;
            }
        }

        if discarded_games > 0 {
            log::warn!(
                "steam web api discarded {} malformed owned games with sanitized payload",
                discarded_games
            );
        }

        Ok(games)
    }

    #[derive(Deserialize)]
    struct SteamOwnedGamesResponse {
        response: SteamOwnedGamesPayload,
    }

    #[derive(Deserialize)]
    struct SteamOwnedGamesPayload {
        games: Option<Vec<serde_json::Value>>,
    }

    fn parse_owned_game(game: &serde_json::Value) -> Option<RemoteSteamGame> {
        let app_id = game
            .get("appid")
            .and_then(parse_u64_value)
            .map(|value| value.to_string())?;
        let title = game
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|title| !title.is_empty())?
            .to_string();
        let playtime_forever = game.get("playtime_forever").and_then(parse_i64_value);

        Some(RemoteSteamGame {
            app_id,
            title,
            playtime_forever,
        })
    }

    fn parse_u64_value(value: &serde_json::Value) -> Option<u64> {
        value.as_u64().or_else(|| {
            value
                .as_str()
                .and_then(|raw| raw.trim().parse::<u64>().ok())
        })
    }

    fn parse_i64_value(value: &serde_json::Value) -> Option<i64> {
        value
            .as_i64()
            .or_else(|| {
                value
                    .as_u64()
                    .and_then(|raw| (raw <= i64::MAX as u64).then(|| raw as i64))
            })
            .or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<i64>().ok())
            })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::Mutex;

        #[test]
        fn fetch_owned_games_uses_required_query_parameters() {
            let client = FakeSteamWebApiClient::with_body(
                r#"{"response":{"games":[{"appid":413150,"name":"Stardew Valley","playtime_forever":321}]}}"#,
            );

            let games = fetch_owned_games(
                &client,
                "0123456789abcdefABCDEF0123456789",
                "76561198000000000",
            )
            .expect("fetch owned games");
            let url = client.last_url();

            assert_eq!(games.len(), 1);
            assert_eq!(games[0].app_id, "413150");
            assert_eq!(games[0].playtime_forever, Some(321));
            assert_eq!(url.path(), "/IPlayerService/GetOwnedGames/v1/");
            assert_eq!(
                url.query_pairs().find(|(key, _)| key == "key").unwrap().1,
                "0123456789abcdefABCDEF0123456789"
            );
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "steamid")
                    .unwrap()
                    .1,
                "76561198000000000"
            );
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "include_appinfo")
                    .unwrap()
                    .1,
                "1"
            );
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "include_played_free_games")
                    .unwrap()
                    .1,
                "1"
            );
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "format")
                    .unwrap()
                    .1,
                "json"
            );
        }

        #[test]
        fn fetch_owned_games_maps_client_errors_without_raw_payload() {
            let client = FakeSteamWebApiClient::with_error(SteamWebApiError::rate_limited());

            let error = fetch_owned_games(
                &client,
                "0123456789abcdefABCDEF0123456789",
                "76561198000000000",
            )
            .expect_err("rate limited");

            assert_eq!(error.code(), "steam_web_api_rate_limited");
            assert_eq!(error.phase(), "request");
            assert!(error.recoverable());
            assert_eq!(
                error.details_sanitized(),
                Some("resposta HTTP 429 da Steam Web API")
            );
            assert!(!error.to_string().contains("0123456789abcdef"));
        }

        #[test]
        fn fetch_owned_games_skips_invalid_games_without_failing_entire_response() {
            let client = FakeSteamWebApiClient::with_body(
                r#"{"response":{"games":[{"appid":413150,"name":"Stardew Valley","playtime_forever":321},{"appid":"invalid","name":"Broken Game"},{"appid":620,"name":" Portal 2 ","playtime_forever":"45"}]}}"#,
            );

            let games = fetch_owned_games(
                &client,
                "0123456789abcdefABCDEF0123456789",
                "76561198000000000",
            )
            .expect("fetch owned games");

            assert_eq!(games.len(), 2);
            assert_eq!(games[0].app_id, "413150");
            assert_eq!(games[0].title, "Stardew Valley");
            assert_eq!(games[1].app_id, "620");
            assert_eq!(games[1].title, "Portal 2");
            assert_eq!(games[1].playtime_forever, Some(45));
        }

        #[test]
        fn fetch_owned_games_returns_empty_vec_for_empty_library_payload() {
            let client = FakeSteamWebApiClient::with_body(r#"{"response":{"games":[]}}"#);

            let games = fetch_owned_games(
                &client,
                "0123456789abcdefABCDEF0123456789",
                "76561198000000000",
            )
            .expect("fetch owned games");

            assert!(games.is_empty());
        }

        #[derive(Default)]
        struct FakeSteamWebApiClient {
            body: Option<String>,
            error: Option<SteamWebApiError>,
            last_url: Mutex<Option<Url>>,
        }

        impl FakeSteamWebApiClient {
            fn with_body(body: &str) -> Self {
                Self {
                    body: Some(body.to_string()),
                    error: None,
                    last_url: Mutex::new(None),
                }
            }

            fn with_error(error: SteamWebApiError) -> Self {
                Self {
                    body: None,
                    error: Some(error),
                    last_url: Mutex::new(None),
                }
            }

            fn last_url(&self) -> Url {
                self.last_url
                    .lock()
                    .expect("lock url")
                    .clone()
                    .expect("url captured")
            }
        }

        impl SteamWebApiClient for FakeSteamWebApiClient {
            fn get_owned_games(&self, url: &Url) -> Result<String, SteamWebApiError> {
                *self.last_url.lock().expect("lock url") = Some(url.clone());

                if let Some(error) = &self.error {
                    return Err(error.clone());
                }

                Ok(self.body.clone().unwrap_or_default())
            }
        }
    }
}

mod steam_openid {
    use super::storage;
    use rand::{rngs::OsRng, RngCore};
    use serde::Serialize;
    use std::fmt;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Emitter};
    use url::Url;

    const STEAM_OPENID_ENDPOINT: &str = "https://steamcommunity.com/openid/login";
    const STEAM_OPENID_EVENT: &str = "steam-openid-login-complete";

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamOpenIdLoginStartDto {
        pending: bool,
        provider_id: &'static str,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SteamOpenIdLoginCompleteDto {
        success: bool,
        provider_id: &'static str,
        steam_id64: Option<String>,
        message: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SteamOpenIdError {
        CallbackUnavailable,
        BrowserOpenFailed,
    }

    impl fmt::Display for SteamOpenIdError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            let message = match self {
                SteamOpenIdError::CallbackUnavailable => {
                    "Nao foi possivel preparar o retorno local do login Steam."
                }
                SteamOpenIdError::BrowserOpenFailed => {
                    "Nao foi possivel abrir o navegador para login Steam."
                }
            };

            formatter.write_str(message)
        }
    }

    pub fn start_login(
        app: AppHandle,
        connection: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<SteamOpenIdLoginStartDto, SteamOpenIdError> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|_| SteamOpenIdError::CallbackUnavailable)?;
        let port = listener
            .local_addr()
            .map_err(|_| SteamOpenIdError::CallbackUnavailable)?
            .port();
        let state = generate_state();
        let return_to = format!("http://127.0.0.1:{port}/steam/openid/callback?state={state}");
        let realm = format!("http://127.0.0.1:{port}/");
        let login_url = build_login_url(&return_to, &realm);

        std::thread::spawn(move || {
            let result = wait_for_callback(listener, &state, &return_to, Duration::from_secs(300))
                .and_then(|steam_id64| {
                    let mut connection = connection
                        .lock()
                        .map_err(|_| "Nao foi possivel salvar a conta Steam.".to_string())?;
                    storage::save_verified_steam_account_config(&mut connection, &steam_id64)
                        .map_err(|_| "Nao foi possivel salvar a conta Steam.".to_string())?;
                    Ok(steam_id64)
                });

            let event = match result {
                Ok(steam_id64) => SteamOpenIdLoginCompleteDto {
                    success: true,
                    provider_id: "steam",
                    steam_id64: Some(steam_id64),
                    message: "Conta Steam conectada neste dispositivo.".to_string(),
                },
                Err(message) => SteamOpenIdLoginCompleteDto {
                    success: false,
                    provider_id: "steam",
                    steam_id64: None,
                    message,
                },
            };

            let _ = app.emit(STEAM_OPENID_EVENT, event);
        });

        open_browser(&login_url)?;

        Ok(SteamOpenIdLoginStartDto {
            pending: true,
            provider_id: "steam",
        })
    }

    fn build_login_url(return_to: &str, realm: &str) -> Url {
        let mut url = Url::parse(STEAM_OPENID_ENDPOINT).expect("valid Steam OpenID endpoint");
        url.query_pairs_mut()
            .append_pair("openid.ns", "http://specs.openid.net/auth/2.0")
            .append_pair("openid.mode", "checkid_setup")
            .append_pair("openid.return_to", return_to)
            .append_pair("openid.realm", realm)
            .append_pair(
                "openid.identity",
                "http://specs.openid.net/auth/2.0/identifier_select",
            )
            .append_pair(
                "openid.claimed_id",
                "http://specs.openid.net/auth/2.0/identifier_select",
            );

        url
    }

    #[cfg(target_os = "windows")]
    fn open_browser(url: &Url) -> Result<(), SteamOpenIdError> {
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url.as_str()])
            .spawn()
            .map_err(|_| SteamOpenIdError::BrowserOpenFailed)?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn open_browser(url: &Url) -> Result<(), SteamOpenIdError> {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(opener)
            .arg(url.as_str())
            .spawn()
            .map_err(|_| SteamOpenIdError::BrowserOpenFailed)?;
        Ok(())
    }

    fn wait_for_callback(
        listener: TcpListener,
        expected_state: &str,
        expected_return_to: &str,
        timeout: Duration,
    ) -> Result<String, String> {
        listener
            .set_nonblocking(true)
            .map_err(|_| "Nao foi possivel aguardar o retorno do login Steam.".to_string())?;

        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(Instant::now);

        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

                    let mut buffer = [0_u8; 8192];
                    let size = stream.read(&mut buffer).map_err(|_| {
                        "Nao foi possivel ler o retorno do login Steam.".to_string()
                    })?;
                    let request = String::from_utf8_lossy(&buffer[..size]);
                    let callback_url = parse_request_target(&request)
                        .and_then(|target| Url::parse(&format!("http://127.0.0.1{target}")).ok())
                        .ok_or_else(|| "Retorno do login Steam invalido.".to_string())?;

                    let result = verify_callback(&callback_url, expected_state, expected_return_to);
                    let html = if result.is_ok() {
                        "<!doctype html><title>Steam conectado</title><body>Login Steam concluido. Volte para o aplicativo.</body>"
                    } else {
                        "<!doctype html><title>Login Steam</title><body>Nao foi possivel concluir o login Steam. Volte para o aplicativo e tente novamente.</body>"
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    );
                    let _ = stream.write_all(response.as_bytes());

                    return result;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(
                            "Tempo expirado aguardando o retorno do login Steam.".to_string()
                        );
                    }

                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    return Err("Nao foi possivel receber o retorno do login Steam.".to_string());
                }
            }
        }
    }

    fn parse_request_target(request: &str) -> Option<&str> {
        let line = request.lines().next()?;
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next(), parts.next()) {
            (Some("GET"), Some(target), Some(_)) => Some(target),
            _ => None,
        }
    }

    fn verify_callback(
        callback_url: &Url,
        expected_state: &str,
        expected_return_to: &str,
    ) -> Result<String, String> {
        let pairs: Vec<(String, String)> = callback_url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();

        if query_value(&pairs, "state") != Some(expected_state) {
            return Err("Resposta do login Steam rejeitada.".to_string());
        }
        if query_value(&pairs, "openid.mode") != Some("id_res") {
            return Err("Login Steam cancelado ou incompleto.".to_string());
        }
        if query_value(&pairs, "openid.return_to") != Some(expected_return_to) {
            return Err("Resposta do login Steam nao corresponde a sessao iniciada.".to_string());
        }
        let claimed_id = query_value(&pairs, "openid.claimed_id")
            .ok_or_else(|| "Resposta do login Steam nao trouxe SteamID64.".to_string())?;
        let identity = query_value(&pairs, "openid.identity").ok_or_else(|| {
            "Resposta do login Steam nao trouxe identidade verificavel.".to_string()
        })?;
        if claimed_id != identity {
            return Err("Resposta do login Steam nao pode ser verificada.".to_string());
        }

        verify_with_steam(&pairs)?;
        steam_id64_from_claimed_id(claimed_id)
            .ok_or_else(|| "Resposta do login Steam nao trouxe SteamID64 valido.".to_string())
    }

    fn verify_with_steam(pairs: &[(String, String)]) -> Result<(), String> {
        let mut form: Vec<(String, String)> = pairs
            .iter()
            .filter(|(key, _)| key.starts_with("openid."))
            .map(|(key, value)| {
                let value = if key == "openid.mode" {
                    "check_authentication".to_string()
                } else {
                    value.clone()
                };
                (key.clone(), value)
            })
            .collect();
        if !form.iter().any(|(key, _)| key == "openid.mode") {
            form.push((
                "openid.mode".to_string(),
                "check_authentication".to_string(),
            ));
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| "Nao foi possivel verificar o login Steam.".to_string())?;
        let body = client
            .post(STEAM_OPENID_ENDPOINT)
            .form(&form)
            .send()
            .and_then(|response| response.error_for_status())
            .and_then(|response| response.text())
            .map_err(|_| "Nao foi possivel verificar o login Steam.".to_string())?;

        if body.lines().any(|line| line.trim() == "is_valid:true") {
            Ok(())
        } else {
            Err("A resposta do Steam nao pode ser verificada.".to_string())
        }
    }

    fn query_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
        pairs
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value.as_str()))
    }

    fn steam_id64_from_claimed_id(claimed_id: &str) -> Option<String> {
        let prefix_http = "http://steamcommunity.com/openid/id/";
        let prefix_https = "https://steamcommunity.com/openid/id/";
        let steam_id = claimed_id
            .strip_prefix(prefix_http)
            .or_else(|| claimed_id.strip_prefix(prefix_https))?;

        if steam_id.len() == 17 && steam_id.bytes().all(|byte| byte.is_ascii_digit()) {
            Some(steam_id.to_string())
        } else {
            None
        }
    }

    fn generate_state() -> String {
        let mut bytes = [0_u8; 24];
        OsRng.fill_bytes(&mut bytes);
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn extracts_steam_id64_from_documented_claimed_id_formats() {
            assert_eq!(
                steam_id64_from_claimed_id("http://steamcommunity.com/openid/id/76561198000000000"),
                Some("76561198000000000".to_string())
            );
            assert_eq!(
                steam_id64_from_claimed_id(
                    "https://steamcommunity.com/openid/id/76561198000000000"
                ),
                Some("76561198000000000".to_string())
            );
            assert_eq!(
                steam_id64_from_claimed_id("https://example.test/id/76561198000000000"),
                None
            );
        }

        #[test]
        fn login_url_targets_official_steam_openid_endpoint() {
            let url = build_login_url(
                "http://127.0.0.1:4000/steam/openid/callback?state=abc",
                "http://127.0.0.1:4000/",
            );

            assert_eq!(url.as_str().split('?').next(), Some(STEAM_OPENID_ENDPOINT));
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "openid.mode")
                    .unwrap()
                    .1,
                "checkid_setup"
            );
        }

        #[test]
        fn wait_for_callback_times_out_without_connection() {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
            let error = wait_for_callback(
                listener,
                "expected-state",
                "http://127.0.0.1/callback",
                Duration::from_millis(0),
            )
            .expect_err("timeout without callback");

            assert_eq!(error, "Tempo expirado aguardando o retorno do login Steam.");
        }
    }
}

mod storage {
    use super::steam_web_api;
    use chrono::{SecondsFormat, Utc};
    use rusqlite::{params, Connection, OptionalExtension};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn open_database(path: &Path) -> rusqlite::Result<Connection> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&connection)?;
        ensure_archived_column(&connection)?;
        ensure_active_entries_index(&connection)?;
        ensure_local_cleanup_indexes(&connection)?;
        ensure_provider_account_configs_table(&connection)?;
        archive_rejected_local_entries(&connection)?;
        Ok(connection)
    }

    fn migrate(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute_batch(
      r#"
      CREATE TABLE IF NOT EXISTS schema_migrations (
        version INTEGER PRIMARY KEY,
        applied_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS games (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        sort_title TEXT NOT NULL,
        installed INTEGER NOT NULL DEFAULT 0,
        playtime_total_minutes INTEGER NOT NULL DEFAULT 0,
        accent_color TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS library_entries (
        id TEXT PRIMARY KEY,
        game_id TEXT NOT NULL UNIQUE,
        primary_platform_id TEXT NOT NULL,
        install_status TEXT NOT NULL,
        last_played_label TEXT NOT NULL,
        is_archived INTEGER NOT NULL DEFAULT 0,
        added_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
      );

      CREATE TABLE IF NOT EXISTS game_sources (
        id TEXT PRIMARY KEY,
        game_id TEXT NOT NULL,
        platform_id TEXT NOT NULL,
        external_id TEXT NOT NULL,
        account_id TEXT,
        FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
        UNIQUE (platform_id, external_id)
      );

      CREATE TABLE IF NOT EXISTS launch_actions (
        id TEXT PRIMARY KEY,
        game_id TEXT NOT NULL,
        platform_id TEXT NOT NULL,
        kind TEXT NOT NULL,
        label TEXT NOT NULL,
        target TEXT NOT NULL,
        arguments_json TEXT,
        working_directory TEXT,
        is_primary INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
      );

      CREATE TABLE IF NOT EXISTS game_genres (
        game_id TEXT NOT NULL,
        genre TEXT NOT NULL,
        position INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
        PRIMARY KEY (game_id, genre)
      );

      CREATE TABLE IF NOT EXISTS provider_account_configs (
        provider_id TEXT PRIMARY KEY,
        account_id TEXT,
        steam_id64 TEXT,
        steam_web_api_key_configured INTEGER NOT NULL DEFAULT 0,
        config_json TEXT,
        updated_at TEXT NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_games_sort_title ON games(sort_title);
      CREATE INDEX IF NOT EXISTS idx_library_entries_install_status ON library_entries(install_status);
      CREATE INDEX IF NOT EXISTS idx_library_entries_platform ON library_entries(primary_platform_id);
      CREATE INDEX IF NOT EXISTS idx_launch_actions_game_primary ON launch_actions(game_id, is_primary);
      "#,
    )?;

        connection.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
            params![now_iso()],
        )?;

        Ok(())
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ManualGameInput {
        title: String,
        genre: Option<String>,
        install_status: String,
        launch_target: Option<String>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LibraryEntryDto {
        id: String,
        game: GameDto,
        primary_platform_id: String,
        install_status: String,
        last_played_label: String,
        is_archived: bool,
        added_at: String,
        updated_at: String,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SyncSummaryDto {
        pub(crate) discovered: usize,
        pub(crate) inserted: usize,
        pub(crate) updated: usize,
        pub(crate) archived: usize,
        pub(crate) unavailable: usize,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct SteamAccountConfigInput {
        steam_id64: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamAccountConfigDto {
        provider_id: &'static str,
        connected: bool,
        steam_id64: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct XboxAccountConfigInput {
        xuid: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct XboxAccountConfigDto {
        provider_id: &'static str,
        connected: bool,
        xuid: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct SteamLibraryRootsInput {
        roots: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamLibraryRootsDto {
        provider_id: &'static str,
        roots: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct LibrarySettingsInput {
        preferred_store_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LibrarySettingsDto {
        preferred_store_id: String,
    }

    pub fn get_steam_account_config(
        connection: &Connection,
    ) -> rusqlite::Result<SteamAccountConfigDto> {
        let steam_id64 = read_steam_account_config(connection)?;

        Ok(steam_account_config_dto(steam_id64))
    }

    pub fn get_xbox_account_config(
        connection: &Connection,
    ) -> rusqlite::Result<XboxAccountConfigDto> {
        let xuid = read_xbox_account_config(connection)?;

        Ok(xbox_account_config_dto(xuid))
    }

    pub fn save_steam_account_config(
        connection: &mut Connection,
        input: SteamAccountConfigInput,
    ) -> rusqlite::Result<SteamAccountConfigDto> {
        let steam_id64 =
            normalize_steam_id64(&input.steam_id64).ok_or_else(|| rusqlite::Error::InvalidQuery)?;
        save_verified_steam_account_config(connection, &steam_id64)?;

        Ok(steam_account_config_dto(Some(steam_id64)))
    }

    pub fn save_xbox_account_config(
        connection: &mut Connection,
        input: XboxAccountConfigInput,
    ) -> rusqlite::Result<XboxAccountConfigDto> {
        let xuid = normalize_xuid(&input.xuid).ok_or_else(|| rusqlite::Error::InvalidQuery)?;
        save_verified_xbox_account_config(connection, &xuid)?;

        Ok(xbox_account_config_dto(Some(xuid)))
    }

    pub fn get_steam_library_roots(
        connection: &Connection,
    ) -> rusqlite::Result<SteamLibraryRootsDto> {
        Ok(steam_library_roots_dto(read_steam_library_roots(
            connection,
        )?))
    }

    pub fn save_steam_library_roots(
        connection: &mut Connection,
        input: SteamLibraryRootsInput,
    ) -> rusqlite::Result<SteamLibraryRootsDto> {
        let roots = normalize_steam_library_roots(&input.roots)?;
        let transaction = connection.transaction()?;
        let existing_config_json = read_provider_config_json(&transaction, "steam")?;
        let mut config = existing_config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        let roots_json = roots
            .iter()
            .map(|root| serde_json::Value::String(root.clone()))
            .collect::<Vec<_>>();

        config.insert(
            "additionalLibraryRoots".to_string(),
            serde_json::Value::Array(roots_json),
        );

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              config_json,
              updated_at
            )
            VALUES ('steam', ?1, ?2)
            ON CONFLICT(provider_id) DO UPDATE SET
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![serde_json::Value::Object(config).to_string(), now_iso()],
        )?;
        transaction.commit()?;

        Ok(steam_library_roots_dto(roots))
    }

    pub fn get_library_settings(connection: &Connection) -> rusqlite::Result<LibrarySettingsDto> {
        let preferred_store_id = read_library_setting_value(connection, "preferredStoreId")?
            .unwrap_or_else(|| "steam".to_string());

        Ok(LibrarySettingsDto { preferred_store_id })
    }

    pub fn save_library_settings(
        connection: &mut Connection,
        input: LibrarySettingsInput,
    ) -> rusqlite::Result<LibrarySettingsDto> {
        let preferred_store_id = normalize_preferred_store_id(&input.preferred_store_id);
        let transaction = connection.transaction()?;
        let existing_config_json = transaction
            .query_row(
                r#"
                SELECT config_json
                FROM provider_account_configs
                WHERE provider_id = 'library'
                "#,
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let mut config = existing_config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        config.insert(
            "preferredStoreId".to_string(),
            serde_json::Value::String(preferred_store_id.clone()),
        );

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              config_json,
              updated_at
            )
            VALUES ('library', ?1, ?2)
            ON CONFLICT(provider_id) DO UPDATE SET
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![serde_json::Value::Object(config).to_string(), now_iso()],
        )?;
        transaction.commit()?;

        Ok(LibrarySettingsDto { preferred_store_id })
    }

    pub fn save_verified_steam_account_config(
        connection: &mut Connection,
        steam_id64: &str,
    ) -> rusqlite::Result<()> {
        let steam_id64 =
            normalize_steam_id64(steam_id64).ok_or_else(|| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.transaction()?;
        let existing_config_json = transaction
            .query_row(
                r#"
                SELECT config_json
                FROM provider_account_configs
                WHERE provider_id = 'steam'
                "#,
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let mut config = existing_config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        config.insert(
            "steamId64".to_string(),
            serde_json::Value::String(steam_id64.to_string()),
        );
        config.insert(
            "linkedBy".to_string(),
            serde_json::Value::String("steam_openid".to_string()),
        );

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              account_id,
              steam_id64,
              config_json,
              updated_at
            )
            VALUES ('steam', ?1, ?1, ?2, ?3)
            ON CONFLICT(provider_id) DO UPDATE SET
              account_id = excluded.account_id,
              steam_id64 = excluded.steam_id64,
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![
                steam_id64,
                serde_json::Value::Object(config).to_string(),
                now_iso()
            ],
        )?;
        transaction.commit()?;

        Ok(())
    }

    pub fn set_steam_web_api_key_config(
        connection: &mut Connection,
        configured: bool,
    ) -> rusqlite::Result<()> {
        connection.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              steam_web_api_key_configured,
              updated_at
            )
            VALUES ('steam', ?1, ?2)
            ON CONFLICT(provider_id) DO UPDATE SET
              steam_web_api_key_configured = excluded.steam_web_api_key_configured,
              updated_at = excluded.updated_at
            "#,
            params![configured, now_iso()],
        )?;

        Ok(())
    }

    pub fn record_xbox_achievement_sync_metadata(
        connection: &mut Connection,
        xuid: &str,
        discovered_titles_count: usize,
        summary: &SyncSummaryDto,
    ) -> rusqlite::Result<()> {
        let transaction = connection.transaction()?;
        let existing_config_json = transaction
            .query_row(
                r#"
                SELECT config_json
                FROM provider_account_configs
                WHERE provider_id = 'xbox'
                "#,
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let mut config = existing_config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        let sync_summary = serde_json::json!({
            "discovered": summary.discovered,
            "inserted": summary.inserted,
            "updated": summary.updated,
            "archived": summary.archived,
            "unavailable": summary.unavailable,
        });

        config.insert(
            "xuid".to_string(),
            serde_json::Value::String(xuid.to_string()),
        );
        if !config.contains_key("linkedBy") {
            config.insert(
                "linkedBy".to_string(),
                serde_json::Value::String("xbox_achievements".to_string()),
            );
        }
        config.insert(
            "lastAchievementsSyncAt".to_string(),
            serde_json::Value::String(now_iso()),
        );
        config.insert(
            "lastAchievementsCount".to_string(),
            serde_json::Value::Number(serde_json::Number::from(discovered_titles_count as u64)),
        );
        config.insert("lastAchievementsSummary".to_string(), sync_summary);

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id, account_id, config_json, updated_at
            )
            VALUES ('xbox', ?1, ?2, ?3)
            ON CONFLICT(provider_id) DO UPDATE SET
              account_id = excluded.account_id,
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![xuid, serde_json::Value::Object(config).to_string(), now_iso()],
        )?;
        transaction.commit()?;

        Ok(())
    }

    fn steam_account_config_dto(steam_id64: Option<String>) -> SteamAccountConfigDto {
        SteamAccountConfigDto {
            provider_id: "steam",
            connected: steam_id64.is_some(),
            steam_id64,
        }
    }

    fn xbox_account_config_dto(xuid: Option<String>) -> XboxAccountConfigDto {
        XboxAccountConfigDto {
            provider_id: "xbox",
            connected: xuid.is_some(),
            xuid,
        }
    }

    fn steam_library_roots_dto(roots: Vec<String>) -> SteamLibraryRootsDto {
        SteamLibraryRootsDto {
            provider_id: "steam",
            roots,
        }
    }

    fn normalize_preferred_store_id(value: &str) -> String {
        match value.trim().to_lowercase().as_str() {
            "xbox" => "xbox".to_string(),
            _ => "steam".to_string(),
        }
    }

    fn read_library_setting_value(
        connection: &Connection,
        key: &str,
    ) -> rusqlite::Result<Option<String>> {
        let columns = table_columns(connection, "provider_account_configs")?;
        if columns.is_empty() {
            return Ok(None);
        }

        let config_json = read_provider_config_json(connection, "library")?;

        let value = config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|json| json.as_object().cloned())
            .and_then(|object| object.get(key).cloned())
            .and_then(|value| value.as_str().map(|value| value.to_string()));

        Ok(value)
    }

    fn read_provider_config_json(
        connection: &Connection,
        provider_id: &str,
    ) -> rusqlite::Result<Option<String>> {
        connection
            .query_row(
                r#"
                SELECT config_json
                FROM provider_account_configs
                WHERE provider_id = ?1
                "#,
                params![provider_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(Option::flatten)
    }

    fn read_steam_library_roots(connection: &Connection) -> rusqlite::Result<Vec<String>> {
        let columns = table_columns(connection, "provider_account_configs")?;
        if columns.is_empty() {
            return Ok(Vec::new());
        }

        Ok(read_provider_config_json(connection, "steam")?
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .and_then(|object| object.get("additionalLibraryRoots").cloned())
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect::<Vec<_>>())
    }

    fn normalize_steam_library_roots(values: &[String]) -> rusqlite::Result<Vec<String>> {
        let mut roots = Vec::new();

        for value in values {
            let trimmed = value.trim().trim_matches('"');
            if trimmed.is_empty() {
                continue;
            }

            let root = normalize_steam_library_root(value)
                .ok_or_else(|| rusqlite::Error::InvalidPath(PathBuf::from(trimmed)))?;
            let normalized = root.to_string_lossy().to_string();
            let normalized_key = normalize_path_string(&root);

            if !roots.iter().any(|existing: &String| {
                normalize_path_string(Path::new(existing)) == normalized_key
            }) {
                roots.push(normalized);
            }
        }

        Ok(roots)
    }

    fn normalize_steam_library_root(value: &str) -> Option<PathBuf> {
        let trimmed = value.trim().trim_matches('"');
        if trimmed.is_empty() {
            return None;
        }

        let path = PathBuf::from(trimmed);
        let library_root = if is_steamapps_path(&path) {
            path.parent().map(Path::to_path_buf)?
        } else {
            path
        };
        let steamapps_dir = library_root.join("steamapps");

        if steamapps_dir.is_dir() {
            return Some(library_root.canonicalize().unwrap_or(library_root));
        }

        None
    }

    fn is_steamapps_path(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("steamapps"))
    }

    pub fn read_steam_account_config(connection: &Connection) -> rusqlite::Result<Option<String>> {
        let columns = table_columns(connection, "provider_account_configs")?;
        if columns.is_empty() {
            return Ok(None);
        }

        for column in [
            "steam_id64",
            "steamid64",
            "steam_id",
            "steamid",
            "account_id",
        ] {
            if !columns.iter().any(|existing| existing == column) {
                continue;
            }

            if let Some(value) =
                read_provider_account_config_value(connection, &columns, "steam", column)?
            {
                if let Some(steam_id) = normalize_steam_id64(&value) {
                    return Ok(Some(steam_id));
                }
            }
        }

        for column in ["config_json", "config"] {
            if !columns.iter().any(|existing| existing == column) {
                continue;
            }

            if let Some(value) =
                read_provider_account_config_value(connection, &columns, "steam", column)?
            {
                if let Some(steam_id) = steam_id64_from_config_json(&value) {
                    return Ok(Some(steam_id));
                }
            }
        }

        Ok(None)
    }

    pub fn read_xbox_account_config(connection: &Connection) -> rusqlite::Result<Option<String>> {
        let columns = table_columns(connection, "provider_account_configs")?;
        if columns.is_empty() {
            return Ok(None);
        }

        for column in ["account_id", "xuid", "xuid64"] {
            if !columns.iter().any(|existing| existing == column) {
                continue;
            }

            if let Some(value) =
                read_provider_account_config_value(connection, &columns, "xbox", column)?
            {
                if let Some(xuid) = normalize_xuid(&value) {
                    return Ok(Some(xuid));
                }
            }
        }

        for column in ["config_json", "config"] {
            if !columns.iter().any(|existing| existing == column) {
                continue;
            }

            if let Some(value) =
                read_provider_account_config_value(connection, &columns, "xbox", column)?
            {
                if let Some(xuid) = xuid_from_config_json(&value) {
                    return Ok(Some(xuid));
                }
            }
        }

        Ok(None)
    }

    pub fn save_verified_xbox_account_config(
        connection: &mut Connection,
        xuid: &str,
    ) -> rusqlite::Result<()> {
        let xuid = normalize_xuid(xuid).ok_or_else(|| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.transaction()?;
        let existing_config_json = transaction
            .query_row(
                r#"
                SELECT config_json
                FROM provider_account_configs
                WHERE provider_id = 'xbox'
                "#,
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let mut config = existing_config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        config.insert(
            "xuid".to_string(),
            serde_json::Value::String(xuid.clone()),
        );
        config.insert(
            "linkedBy".to_string(),
            serde_json::Value::String("xbox_manual_config".to_string()),
        );

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              account_id,
              config_json,
              updated_at
            )
            VALUES ('xbox', ?1, ?2, ?3)
            ON CONFLICT(provider_id) DO UPDATE SET
              account_id = excluded.account_id,
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![xuid, serde_json::Value::Object(config).to_string(), now_iso()],
        )?;
        transaction.commit()?;

        Ok(())
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct GameDto {
        internal_id: String,
        title: String,
        sort_title: String,
        platforms: Vec<String>,
        sources: Vec<GameSourceDto>,
        installed: bool,
        install_locations: Vec<String>,
        launch_actions: Vec<LaunchActionDto>,
        playtime: PlaytimeDto,
        artwork: GameArtworkDto,
        genres: Vec<String>,
        tags: Vec<String>,
        user_overrides: serde_json::Value,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct GameSourceDto {
        platform_id: String,
        external_id: String,
        account_id: Option<String>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct LaunchActionDto {
        id: String,
        platform_id: String,
        kind: String,
        label: String,
        target: String,
        arguments: Option<Vec<String>>,
        working_directory: Option<String>,
        is_primary: bool,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PlaytimeDto {
        total_minutes: i64,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct GameArtworkDto {
        accent_color: Option<String>,
    }

    pub fn list_library_entries(connection: &Connection) -> rusqlite::Result<Vec<LibraryEntryDto>> {
        let mut statement = connection.prepare(
            r#"
      SELECT
        library_entries.id,
        library_entries.primary_platform_id,
        library_entries.install_status,
        library_entries.last_played_label,
        library_entries.is_archived,
        library_entries.added_at,
        library_entries.updated_at,
        games.id,
        games.title,
        games.sort_title,
        games.installed,
        games.playtime_total_minutes,
        games.accent_color
      FROM library_entries
      JOIN games ON games.id = library_entries.game_id
      WHERE library_entries.is_archived = 0
      ORDER BY library_entries.added_at DESC, games.sort_title
      "#,
        )?;

        let rows = statement.query_map([], |row| {
            let game_id: String = row.get(7)?;
            Ok(EntryRow {
                entry_id: row.get(0)?,
                primary_platform_id: row.get(1)?,
                install_status: row.get(2)?,
                last_played_label: row.get(3)?,
                is_archived: row.get::<_, i64>(4)? == 1,
                added_at: row.get(5)?,
                updated_at: row.get(6)?,
                game_id,
                title: row.get(8)?,
                sort_title: row.get(9)?,
                installed: row.get::<_, i64>(10)? == 1,
                playtime_total_minutes: row.get(11)?,
                accent_color: row.get(12)?,
                source_account_id: None,
            })
        })?;

        rows.map(|row| row.and_then(|entry| hydrate_entry(connection, entry)))
            .collect()
    }

    pub fn list_manual_games(connection: &Connection) -> rusqlite::Result<Vec<LibraryEntryDto>> {
        let mut statement = connection.prepare(
            r#"
      SELECT
        library_entries.id,
        library_entries.primary_platform_id,
        library_entries.install_status,
        library_entries.last_played_label,
        library_entries.is_archived,
        library_entries.added_at,
        library_entries.updated_at,
        games.id,
        games.title,
        games.sort_title,
        games.installed,
        games.playtime_total_minutes,
        games.accent_color
      FROM library_entries
      JOIN games ON games.id = library_entries.game_id
      WHERE library_entries.primary_platform_id = 'manual'
        AND library_entries.is_archived = 0
      ORDER BY library_entries.added_at DESC
      "#,
        )?;

        let rows = statement.query_map([], |row| {
            let game_id: String = row.get(7)?;
            Ok(EntryRow {
                entry_id: row.get(0)?,
                primary_platform_id: row.get(1)?,
                install_status: row.get(2)?,
                last_played_label: row.get(3)?,
                is_archived: row.get::<_, i64>(4)? == 1,
                added_at: row.get(5)?,
                updated_at: row.get(6)?,
                game_id,
                title: row.get(8)?,
                sort_title: row.get(9)?,
                installed: row.get::<_, i64>(10)? == 1,
                playtime_total_minutes: row.get(11)?,
                accent_color: row.get(12)?,
                source_account_id: None,
            })
        })?;

        rows.map(|row| row.and_then(|entry| hydrate_entry(connection, entry)))
            .collect()
    }

    pub fn add_manual_game(
        connection: &mut Connection,
        input: ManualGameInput,
    ) -> rusqlite::Result<LibraryEntryDto> {
        let manual_game = normalize_manual_game_input(input)?;
        let title = manual_game.title.as_str();
        let slug = create_slug(title);
        let timestamp = timestamp_millis();
        let now = now_iso();
        let game_id = format!("game-manual-{slug}-{timestamp}");
        let entry_id = format!("entry-manual-{slug}-{timestamp}");
        let source_id = format!("source-manual-{slug}-{timestamp}");
        let launch_id = format!("launch-manual-{slug}-{timestamp}");
        let external_id = format!("manual-{slug}-{timestamp}");
        let accent_color = deterministic_accent_color(title);

        let transaction = connection.transaction()?;
        transaction.execute(
      r#"
      INSERT INTO games (
        id, title, sort_title, installed, playtime_total_minutes, accent_color, created_at, updated_at
      ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6)
      "#,
      params![game_id, title, title, manual_game.installed as i64, accent_color, now],
    )?;
        transaction.execute(
            r#"
      INSERT INTO library_entries (
        id, game_id, primary_platform_id, install_status, last_played_label, is_archived, added_at, updated_at
      ) VALUES (?1, ?2, 'manual', ?3, 'Nunca', 0, ?4, ?4)
      "#,
            params![entry_id, game_id, manual_game.install_status, now],
        )?;
        transaction.execute(
            r#"
      INSERT INTO game_sources (id, game_id, platform_id, external_id)
      VALUES (?1, ?2, 'manual', ?3)
      "#,
            params![source_id, game_id, external_id],
        )?;
        transaction.execute(
            r#"
      INSERT INTO launch_actions (
        id, game_id, platform_id, kind, label, target, arguments_json, is_primary
      ) VALUES (?1, ?2, 'manual', ?3, ?4, ?5, '[]', 1)
      "#,
            params![
                launch_id,
                game_id,
                manual_game.launch_kind,
                manual_game.launch_label,
                manual_game.launch_target
            ],
        )?;
        transaction.execute(
            "INSERT INTO game_genres (game_id, genre, position) VALUES (?1, ?2, 0)",
            params![game_id, manual_game.genre],
        )?;
        transaction.commit()?;

        let row = find_entry(connection, &entry_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        hydrate_entry(connection, row)
    }

    pub fn update_manual_game(
        connection: &mut Connection,
        entry_id: &str,
        input: ManualGameInput,
    ) -> rusqlite::Result<LibraryEntryDto> {
        let manual_game = normalize_manual_game_input(input)?;
        let NormalizedManualGame {
            title,
            sort_title,
            install_status,
            installed,
            genre,
            launch_target,
            launch_kind,
            launch_label,
        } = manual_game;
        let transaction = connection.transaction()?;
        let row =
            find_entry(&transaction, entry_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;

        if row.primary_platform_id != "manual" {
            return Err(rusqlite::Error::InvalidParameterName(
                "entry is not manual".to_string(),
            ));
        }

        let updated_at = now_iso();
        let accent_color = deterministic_accent_color(&title);

        transaction.execute(
            r#"
            UPDATE games
            SET title = ?2,
                sort_title = ?3,
                installed = ?4,
                accent_color = ?5,
                updated_at = ?6
            WHERE id = ?1
            "#,
            params![
                row.game_id,
                title,
                sort_title,
                installed as i64,
                accent_color,
                updated_at,
            ],
        )?;

        transaction.execute(
            r#"
            UPDATE library_entries
            SET install_status = ?2,
                updated_at = ?3
            WHERE id = ?1
            "#,
            params![entry_id, install_status, updated_at],
        )?;

        transaction.execute(
            r#"
            UPDATE launch_actions
            SET kind = ?2,
                label = ?3,
                target = ?4
            WHERE game_id = ?1
              AND is_primary = 1
            "#,
            params![row.game_id, launch_kind, launch_label, launch_target,],
        )?;

        transaction.execute(
            "DELETE FROM game_genres WHERE game_id = ?1",
            params![row.game_id],
        )?;
        transaction.execute(
            "INSERT INTO game_genres (game_id, genre, position) VALUES (?1, ?2, 0)",
            params![row.game_id, genre],
        )?;
        transaction.commit()?;

        let refreshed_row =
            find_entry(connection, entry_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        hydrate_entry(connection, refreshed_row)
    }

    #[derive(Clone)]
    struct EntryRow {
        entry_id: String,
        primary_platform_id: String,
        install_status: String,
        last_played_label: String,
        is_archived: bool,
        added_at: String,
        updated_at: String,
        game_id: String,
        title: String,
        sort_title: String,
        installed: bool,
        playtime_total_minutes: i64,
        accent_color: Option<String>,
        source_account_id: Option<String>,
    }

    fn find_entry(connection: &Connection, entry_id: &str) -> rusqlite::Result<Option<EntryRow>> {
        connection
            .query_row(
                r#"
        SELECT
          library_entries.id,
          library_entries.primary_platform_id,
          library_entries.install_status,
          library_entries.last_played_label,
          library_entries.is_archived,
          library_entries.added_at,
          library_entries.updated_at,
          games.id,
          games.title,
          games.sort_title,
          games.installed,
          games.playtime_total_minutes,
          games.accent_color
        FROM library_entries
        JOIN games ON games.id = library_entries.game_id
        WHERE library_entries.id = ?1
        "#,
                params![entry_id],
                |row| {
                    Ok(EntryRow {
                        entry_id: row.get(0)?,
                        primary_platform_id: row.get(1)?,
                        install_status: row.get(2)?,
                        last_played_label: row.get(3)?,
                        is_archived: row.get::<_, i64>(4)? == 1,
                        added_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        game_id: row.get(7)?,
                        title: row.get(8)?,
                        sort_title: row.get(9)?,
                        installed: row.get::<_, i64>(10)? == 1,
                        playtime_total_minutes: row.get(11)?,
                        accent_color: row.get(12)?,
                        source_account_id: None,
                    })
                },
            )
            .optional()
    }

    fn hydrate_entry(connection: &Connection, row: EntryRow) -> rusqlite::Result<LibraryEntryDto> {
        let sources = list_sources(connection, &row.game_id)?;
        let launch_actions = list_launch_actions(connection, &row.game_id)?;
        let genres = list_genres(connection, &row.game_id)?;
        let mut platforms: Vec<String> = sources
            .iter()
            .map(|source| source.platform_id.clone())
            .collect();

        if !platforms.contains(&row.primary_platform_id) {
            platforms.insert(0, row.primary_platform_id.clone());
        }

        Ok(LibraryEntryDto {
            id: row.entry_id,
            primary_platform_id: row.primary_platform_id.clone(),
            install_status: row.install_status,
            last_played_label: row.last_played_label,
            is_archived: row.is_archived,
            added_at: row.added_at,
            updated_at: row.updated_at,
            game: GameDto {
                internal_id: row.game_id,
                title: row.title,
                sort_title: row.sort_title,
                platforms,
                sources,
                installed: row.installed,
                install_locations: Vec::new(),
                launch_actions,
                playtime: PlaytimeDto {
                    total_minutes: row.playtime_total_minutes,
                },
                artwork: GameArtworkDto {
                    accent_color: row.accent_color,
                },
                genres,
                tags: Vec::new(),
                user_overrides: serde_json::json!({}),
            },
        })
    }

    fn list_sources(
        connection: &Connection,
        game_id: &str,
    ) -> rusqlite::Result<Vec<GameSourceDto>> {
        let mut statement = connection.prepare(
            r#"
      SELECT platform_id, external_id, account_id
      FROM game_sources
      WHERE game_id = ?1
      ORDER BY platform_id
      "#,
        )?;

        let sources = statement
            .query_map(params![game_id], |row| {
                Ok(GameSourceDto {
                    platform_id: row.get(0)?,
                    external_id: row.get(1)?,
                    account_id: row.get(2)?,
                })
            })?
            .collect();

        sources
    }

    fn list_launch_actions(
        connection: &Connection,
        game_id: &str,
    ) -> rusqlite::Result<Vec<LaunchActionDto>> {
        let mut statement = connection.prepare(
            r#"
      SELECT id, platform_id, kind, label, target, arguments_json, working_directory, is_primary
      FROM launch_actions
      WHERE game_id = ?1
      ORDER BY is_primary DESC, id
      "#,
        )?;

        let launch_actions = statement
            .query_map(params![game_id], |row| {
                let arguments_json: Option<String> = row.get(5)?;
                let arguments = arguments_json.and_then(|value| serde_json::from_str(&value).ok());

                Ok(LaunchActionDto {
                    id: row.get(0)?,
                    platform_id: row.get(1)?,
                    kind: row.get(2)?,
                    label: row.get(3)?,
                    target: row.get(4)?,
                    arguments,
                    working_directory: row.get(6)?,
                    is_primary: row.get::<_, i64>(7)? == 1,
                })
            })?
            .collect();

        launch_actions
    }

    fn list_genres(connection: &Connection, game_id: &str) -> rusqlite::Result<Vec<String>> {
        let mut statement = connection
            .prepare("SELECT genre FROM game_genres WHERE game_id = ?1 ORDER BY position, genre")?;

        let genres = statement
            .query_map(params![game_id], |row| row.get(0))?
            .collect();

        genres
    }

    struct NormalizedManualGame {
        title: String,
        sort_title: String,
        install_status: String,
        installed: bool,
        genre: String,
        launch_target: String,
        launch_kind: String,
        launch_label: String,
    }

    fn normalize_manual_game_input(
        input: ManualGameInput,
    ) -> rusqlite::Result<NormalizedManualGame> {
        let title = input.title.trim();
        if title.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "title is required".to_string(),
            ));
        }

        let install_status = match input.install_status.as_str() {
            "installed" => "installed".to_string(),
            _ => "not_installed".to_string(),
        };
        let installed = install_status == "installed";
        let genre = input
            .genre
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Sem genero")
            .to_string();
        let launch_target = input.launch_target.unwrap_or_default().trim().to_string();
        let launch_kind = launch_action_kind(&launch_target).to_string();
        let launch_label = if launch_target.is_empty() {
            "Sem acao configurada".to_string()
        } else {
            launch_target.clone()
        };

        Ok(NormalizedManualGame {
            title: title.to_string(),
            sort_title: title.to_string(),
            install_status,
            installed,
            genre,
            launch_target,
            launch_kind,
            launch_label,
        })
    }

    fn ensure_archived_column(connection: &Connection) -> rusqlite::Result<()> {
        if !table_has_column(connection, "library_entries", "is_archived")? {
            connection.execute(
                "ALTER TABLE library_entries ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        Ok(())
    }

    fn ensure_active_entries_index(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_library_entries_active_added_at ON library_entries(added_at DESC) WHERE is_archived = 0",
            [],
        )?;

        Ok(())
    }

    fn ensure_local_cleanup_indexes(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_library_entries_local_active_game ON library_entries(primary_platform_id, is_archived, game_id)",
            [],
        )?;
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_launch_actions_platform_kind_game ON launch_actions(platform_id, kind, game_id)",
            [],
        )?;

        Ok(())
    }

    fn ensure_provider_account_configs_table(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            r#"
            CREATE TABLE IF NOT EXISTS provider_account_configs (
              provider_id TEXT PRIMARY KEY,
              account_id TEXT,
              steam_id64 TEXT,
              steam_web_api_key_configured INTEGER NOT NULL DEFAULT 0,
              config_json TEXT,
              updated_at TEXT NOT NULL
            )
            "#,
            [],
        )?;

        let columns = table_columns(connection, "provider_account_configs")?;
        if !columns.iter().any(|column| column == "steam_id64") {
            connection.execute(
                "ALTER TABLE provider_account_configs ADD COLUMN steam_id64 TEXT",
                [],
            )?;
        }
        if !columns.iter().any(|column| column == "config_json") {
            connection.execute(
                "ALTER TABLE provider_account_configs ADD COLUMN config_json TEXT",
                [],
            )?;
        }
        if !columns
            .iter()
            .any(|column| column == "steam_web_api_key_configured")
        {
            connection.execute(
                "ALTER TABLE provider_account_configs ADD COLUMN steam_web_api_key_configured INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        Ok(())
    }

    fn read_provider_account_config_value(
        connection: &Connection,
        columns: &[String],
        provider_id: &str,
        value_column: &str,
    ) -> rusqlite::Result<Option<String>> {
        let filter_column = if columns.iter().any(|column| column == "provider_id") {
            Some("provider_id")
        } else if columns.iter().any(|column| column == "platform_id") {
            Some("platform_id")
        } else {
            None
        };
        let order_clause = if columns.iter().any(|column| column == "updated_at") {
            " ORDER BY updated_at DESC"
        } else {
            ""
        };
        let query = if let Some(filter_column) = filter_column {
            format!(
                "SELECT {value_column} FROM provider_account_configs WHERE {filter_column} = ?1 AND {value_column} IS NOT NULL{order_clause} LIMIT 1"
            )
        } else {
            format!(
                "SELECT {value_column} FROM provider_account_configs WHERE {value_column} IS NOT NULL{order_clause} LIMIT 1"
            )
        };

        if filter_column.is_some() {
            connection
                .query_row(&query, params![provider_id], |row| row.get::<_, String>(0))
                .optional()
        } else {
            connection
                .query_row(&query, [], |row| row.get::<_, String>(0))
                .optional()
        }
    }

    fn normalize_steam_id64(value: &str) -> Option<String> {
        let value = value.trim();

        if value.len() == 17 && value.bytes().all(|byte| byte.is_ascii_digit()) {
            Some(value.to_string())
        } else {
            None
        }
    }

    fn normalize_xuid(value: &str) -> Option<String> {
        let value = value.trim();
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }

        value.parse::<u64>().ok().map(|value| value.to_string())
    }

    fn xuid_from_config_json(value: &str) -> Option<String> {
        let value: serde_json::Value = serde_json::from_str(value).ok()?;

        ["xuid", "xUid", "accountId", "account_id"]
            .into_iter()
            .find_map(|key| value.get(key).and_then(|value| value.as_str()))
            .and_then(normalize_xuid)
    }

    fn steam_id64_from_config_json(value: &str) -> Option<String> {
        let value: serde_json::Value = serde_json::from_str(value).ok()?;

        [
            "steamId64",
            "steam_id64",
            "steamid64",
            "steamId",
            "steam_id",
            "steamid",
        ]
        .into_iter()
        .find_map(|key| value.get(key).and_then(|value| value.as_str()))
        .and_then(normalize_steam_id64)
    }

    fn table_has_column(
        connection: &Connection,
        table_name: &str,
        column_name: &str,
    ) -> rusqlite::Result<bool> {
        let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;

        for column in columns {
            if column? == column_name {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn table_columns(connection: &Connection, table_name: &str) -> rusqlite::Result<Vec<String>> {
        let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect();

        columns
    }

    struct SeedLibraryEntry {
        entry_id: &'static str,
        game_id: &'static str,
        source_id: &'static str,
        launch_id: &'static str,
        primary_platform_id: &'static str,
        external_id: &'static str,
        title: &'static str,
        sort_title: &'static str,
        install_status: &'static str,
        installed: bool,
        last_played_label: &'static str,
        added_at: &'static str,
        updated_at: &'static str,
        playtime_total_minutes: i64,
        accent_color: &'static str,
        genre: &'static str,
        launch_kind: &'static str,
        launch_label: &'static str,
        launch_target: &'static str,
    }

    pub(super) fn seed_mock_library(connection: &mut Connection) -> rusqlite::Result<()> {
        const SEED_ENTRIES: [SeedLibraryEntry; 4] = [
            SeedLibraryEntry {
                entry_id: "entry-steam-hades",
                game_id: "game-hades",
                source_id: "source-steam-hades",
                launch_id: "launch-steam-hades",
                primary_platform_id: "steam",
                external_id: "1145360",
                title: "Hades",
                sort_title: "Hades",
                install_status: "installed",
                installed: true,
                last_played_label: "Hoje",
                added_at: "2026-05-07T00:00:00.000Z",
                updated_at: "2026-05-07T00:00:00.000Z",
                playtime_total_minutes: 2520,
                accent_color: "#c2410c",
                genre: "Roguelike",
                launch_kind: "uri",
                launch_label: "steam://rungameid/1145360",
                launch_target: "steam://rungameid/1145360",
            },
            SeedLibraryEntry {
                entry_id: "entry-steam-cyberpunk",
                game_id: "game-cyberpunk-2077",
                source_id: "source-steam-cyberpunk",
                launch_id: "launch-steam-cyberpunk",
                primary_platform_id: "steam",
                external_id: "1091500",
                title: "Cyberpunk 2077",
                sort_title: "Cyberpunk 2077",
                install_status: "not_installed",
                installed: false,
                last_played_label: "12 dias",
                added_at: "2026-05-07T00:00:00.000Z",
                updated_at: "2026-05-07T00:00:00.000Z",
                playtime_total_minutes: 5220,
                accent_color: "#0f766e",
                genre: "RPG",
                launch_kind: "uri",
                launch_label: "steam://rungameid/1091500",
                launch_target: "steam://rungameid/1091500",
            },
            SeedLibraryEntry {
                entry_id: "entry-local-minecraft",
                game_id: "game-minecraft",
                source_id: "source-local-minecraft",
                launch_id: "launch-local-minecraft",
                primary_platform_id: "local",
                external_id: "local-minecraft",
                title: "Minecraft",
                sort_title: "Minecraft",
                install_status: "installed",
                installed: true,
                last_played_label: "Ontem",
                added_at: "2026-05-07T00:00:00.000Z",
                updated_at: "2026-05-07T00:00:00.000Z",
                playtime_total_minutes: 7680,
                accent_color: "#15803d",
                genre: "Sandbox",
                launch_kind: "executable",
                launch_label: "C:/Games/Minecraft/Launcher.exe",
                launch_target: "C:/Games/Minecraft/Launcher.exe",
            },
            SeedLibraryEntry {
                entry_id: "entry-manual-silksong",
                game_id: "game-hollow-knight-silksong",
                source_id: "source-manual-silksong",
                launch_id: "launch-manual-silksong",
                primary_platform_id: "manual",
                external_id: "manual-silksong",
                title: "Hollow Knight: Silksong",
                sort_title: "Hollow Knight: Silksong",
                install_status: "not_installed",
                installed: false,
                last_played_label: "Nunca",
                added_at: "2026-05-07T00:00:00.000Z",
                updated_at: "2026-05-07T00:00:00.000Z",
                playtime_total_minutes: 0,
                accent_color: "#be123c",
                genre: "Metroidvania",
                launch_kind: "manual",
                launch_label: "Sem acao configurada",
                launch_target: "",
            },
        ];

        let transaction = connection.transaction()?;

        for entry in SEED_ENTRIES {
            transaction.execute(
                r#"
        INSERT OR IGNORE INTO games (
          id, title, sort_title, installed, playtime_total_minutes, accent_color, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
                params![
                    entry.game_id,
                    entry.title,
                    entry.sort_title,
                    entry.installed as i64,
                    entry.playtime_total_minutes,
                    entry.accent_color,
                    entry.added_at,
                    entry.updated_at
                ],
            )?;
            transaction.execute(
                r#"
        INSERT OR IGNORE INTO library_entries (
          id, game_id, primary_platform_id, install_status, last_played_label, is_archived, added_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)
        "#,
                params![
                    entry.entry_id,
                    entry.game_id,
                    entry.primary_platform_id,
                    entry.install_status,
                    entry.last_played_label,
                    entry.added_at,
                    entry.updated_at
                ],
            )?;
            transaction.execute(
                r#"
        INSERT OR IGNORE INTO game_sources (id, game_id, platform_id, external_id)
        VALUES (?1, ?2, ?3, ?4)
        "#,
                params![
                    entry.source_id,
                    entry.game_id,
                    entry.primary_platform_id,
                    entry.external_id
                ],
            )?;
            transaction.execute(
                r#"
        INSERT OR IGNORE INTO launch_actions (
          id, game_id, platform_id, kind, label, target, arguments_json, is_primary
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '[]', 1)
        "#,
                params![
                    entry.launch_id,
                    entry.game_id,
                    entry.primary_platform_id,
                    entry.launch_kind,
                    entry.launch_label,
                    entry.launch_target
                ],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, ?2, 0)",
                params![entry.game_id, entry.genre],
            )?;
        }

        transaction.commit()
    }

    #[derive(Clone)]
    struct LocalGameCandidate {
        source_external_id: String,
        title: String,
        launch_target: String,
        source_id: String,
        game_id: String,
        entry_id: String,
        launch_id: String,
        accent_color: &'static str,
    }

    #[derive(Clone)]
    struct SteamGameCandidate {
        app_id: String,
        title: String,
        install_path: Option<String>,
        source_id: String,
        game_id: String,
        entry_id: String,
        launch_id: String,
        accent_color: &'static str,
    }

    struct RemoteSteamGameCandidate {
        app_id: String,
        title: String,
        playtime_total_minutes: i64,
        has_playtime: bool,
        source_id: String,
        game_id: String,
        entry_id: String,
        launch_id: String,
        accent_color: &'static str,
    }

    pub fn sync_local_games(connection: &mut Connection) -> rusqlite::Result<SyncSummaryDto> {
        let roots = collect_local_game_roots();
        sync_local_games_from_roots(connection, &roots)
    }

    pub fn sync_steam_games(connection: &mut Connection) -> rusqlite::Result<SyncSummaryDto> {
        let roots = collect_steam_roots(connection);
        sync_steam_games_from_roots(connection, &roots)
    }

    pub fn sync_steam_account_games(
        connection: &mut Connection,
        steam_id: &str,
        remote_games: &[steam_web_api::RemoteSteamGame],
    ) -> rusqlite::Result<SyncSummaryDto> {
        let candidates = remote_games
            .iter()
            .filter(|game| !is_rejected_steam_app(&game.app_id))
            .map(remote_steam_game_candidate)
            .collect::<Vec<_>>();
        let existing_entries = list_steam_entries_by_source(connection)?;
        let mut summary = SyncSummaryDto {
            discovered: candidates.len(),
            inserted: 0,
            updated: 0,
            archived: 0,
            unavailable: 0,
        };

        let transaction = connection.transaction()?;
        summary.archived = archive_rejected_steam_entries(&transaction)?;

        for candidate in candidates {
            if let Some(existing_row) = existing_entries.get(&candidate.app_id) {
                if update_remote_steam_entry(&transaction, steam_id, existing_row, &candidate)? {
                    summary.updated += 1;
                }
            } else {
                insert_remote_steam_entry(&transaction, steam_id, &candidate)?;
                summary.inserted += 1;
            }
        }

        transaction.commit()?;
        Ok(summary)
    }

    pub fn record_steam_account_sync_metadata(
        connection: &mut Connection,
        steam_id: &str,
        owned_games_count: usize,
        summary: &SyncSummaryDto,
    ) -> rusqlite::Result<()> {
        let transaction = connection.transaction()?;
        let existing_config_json = transaction
            .query_row(
                r#"
                SELECT config_json
                FROM provider_account_configs
                WHERE provider_id = 'steam'
                "#,
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let mut config = existing_config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        let sync_summary = serde_json::json!({
            "discovered": summary.discovered,
            "inserted": summary.inserted,
            "updated": summary.updated,
            "archived": summary.archived,
            "unavailable": summary.unavailable,
        });

        config.insert(
            "steamId64".to_string(),
            serde_json::Value::String(steam_id.to_string()),
        );
        if !config.contains_key("linkedBy") {
            config.insert(
                "linkedBy".to_string(),
                serde_json::Value::String("steam_web_api".to_string()),
            );
        }
        config.insert(
            "lastOwnedGamesSyncAt".to_string(),
            serde_json::Value::String(now_iso()),
        );
        config.insert(
            "lastOwnedGamesCount".to_string(),
            serde_json::Value::Number(serde_json::Number::from(owned_games_count as u64)),
        );
        config.insert("lastOwnedGamesSummary".to_string(), sync_summary);

        let config_json = serde_json::Value::Object(config).to_string();

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id, account_id, steam_id64, config_json, updated_at
            ) VALUES ('steam', ?1, ?1, ?2, ?3)
            ON CONFLICT(provider_id) DO UPDATE SET
              account_id = excluded.account_id,
              steam_id64 = excluded.steam_id64,
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![steam_id, config_json, now_iso()],
        )?;
        transaction.commit()?;

        Ok(())
    }

    fn sync_steam_games_from_roots(
        connection: &mut Connection,
        roots: &[PathBuf],
    ) -> rusqlite::Result<SyncSummaryDto> {
        let candidates = discover_steam_game_candidates(roots);
        let existing_entries = list_steam_entries_by_source(connection)?;
        let mut summary = SyncSummaryDto {
            discovered: candidates.len(),
            inserted: 0,
            updated: 0,
            archived: 0,
            unavailable: 0,
        };

        let transaction = connection.transaction()?;
        summary.archived = archive_rejected_steam_entries(&transaction)?;
        let discovered_app_ids = candidates
            .iter()
            .map(|candidate| candidate.app_id.clone())
            .collect::<std::collections::HashSet<_>>();

        for candidate in candidates {
            if let Some(existing_row) = existing_entries.get(&candidate.app_id) {
                if update_steam_entry(&transaction, existing_row, &candidate)? {
                    summary.updated += 1;
                }
            } else {
                insert_steam_entry(&transaction, &candidate)?;
                summary.inserted += 1;
            }
        }

        for (app_id, existing_row) in &existing_entries {
            if !existing_row.is_archived
                && !is_rejected_steam_app(app_id)
                && !discovered_app_ids.contains(app_id)
                && mark_steam_entry_unavailable(&transaction, existing_row)?
            {
                summary.unavailable += 1;
            }
        }

        transaction.commit()?;
        Ok(summary)
    }

    fn sync_local_games_from_roots(
        connection: &mut Connection,
        roots: &[PathBuf],
    ) -> rusqlite::Result<SyncSummaryDto> {
        let candidates = discover_local_game_candidates(roots);
        let archived = archive_rejected_local_entries(connection)?;
        let existing_entries = list_local_entries_by_source(connection)?;
        let mut summary = SyncSummaryDto {
            discovered: candidates.len(),
            inserted: 0,
            updated: 0,
            archived,
            unavailable: 0,
        };

        if candidates.is_empty() {
            return Ok(summary);
        }

        let transaction = connection.transaction()?;

        for candidate in candidates {
            if let Some(existing_row) = existing_entries.get(&candidate.source_external_id) {
                update_local_entry(&transaction, existing_row, &candidate)?;
                summary.updated += 1;
            } else {
                insert_local_entry(&transaction, &candidate)?;
                summary.inserted += 1;
            }
        }

        transaction.commit()?;
        Ok(summary)
    }

    fn collect_local_game_roots() -> Vec<PathBuf> {
        if let Some(raw_roots) = std::env::var_os("BIBLIOTECA_JOGOS_LOCAL_ROOTS") {
            let roots = raw_roots
                .to_string_lossy()
                .split(';')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .collect::<Vec<_>>();

            if !roots.is_empty() {
                return roots;
            }
        }

        let mut roots = Vec::new();

        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let user_profile = PathBuf::from(user_profile);
            roots.push(user_profile.join("Games"));
            roots.push(user_profile.join("Desktop").join("Games"));
            roots.push(user_profile.join("Documents").join("Games"));
            roots.push(user_profile.join("AppData").join("Local").join("osu!"));
        }

        if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
            let program_files = PathBuf::from(program_files);
            roots.push(program_files.join("GOG Games"));
            roots.push(program_files.join("Epic Games"));
            roots.push(program_files.join("EA Games"));
            roots.push(program_files.join("Ubisoft"));
            roots.push(program_files.join("Battle.net"));
        }

        if let Some(program_files_x86) = std::env::var_os("PROGRAMFILES(X86)") {
            let program_files_x86 = PathBuf::from(program_files_x86);
            roots.push(program_files_x86.join("GOG Games"));
            roots.push(program_files_x86.join("Epic Games"));
            roots.push(program_files_x86.join("Battle.net"));
        }

        if let Some(public) = std::env::var_os("PUBLIC") {
            roots.push(PathBuf::from(public).join("Games"));
        }

        roots.into_iter().filter(|root| root.exists()).collect()
    }

    fn collect_steam_roots(connection: &Connection) -> Vec<PathBuf> {
        let mut roots = Vec::new();

        if let Some(raw_roots) = std::env::var_os("BIBLIOTECA_JOGOS_STEAM_ROOTS") {
            for root in raw_roots
                .to_string_lossy()
                .split(';')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .filter(|root| root.exists())
            {
                push_unique_path(&mut roots, root);
            }
        }

        if let Ok(saved_roots) = read_steam_library_roots(connection) {
            for root in saved_roots {
                let root = PathBuf::from(root);
                if root.exists() {
                    push_unique_path(&mut roots, root);
                }
            }
        }

        if let Some(program_files_x86) = std::env::var_os("PROGRAMFILES(X86)") {
            push_unique_path(&mut roots, PathBuf::from(program_files_x86).join("Steam"));
        }

        if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
            push_unique_path(&mut roots, PathBuf::from(program_files).join("Steam"));
        }

        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            push_unique_path(&mut roots, PathBuf::from(local_app_data).join("Steam"));
        }

        roots.into_iter().filter(|root| root.exists()).collect()
    }

    fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
        let key = normalize_path_string(&path);

        if !paths
            .iter()
            .any(|existing| normalize_path_string(existing) == key)
        {
            paths.push(path);
        }
    }

    fn discover_local_game_candidates(roots: &[PathBuf]) -> Vec<LocalGameCandidate> {
        let mut candidates = HashMap::new();

        for root in roots {
            for candidate_dir in candidate_directories(root) {
                if let Some(executable_path) = find_local_executable(&candidate_dir) {
                    let title = candidate_title(&candidate_dir, &executable_path);

                    if is_rejected_local_title(&title) {
                        continue;
                    }

                    let normalized_target = normalize_path_string(&executable_path);

                    candidates
                        .entry(normalized_target.clone())
                        .or_insert_with(|| {
                            let slug = create_slug(&title);
                            let hash = stable_hash_hex(&normalized_target);
                            let accent_color = deterministic_accent_color(&title);

                            LocalGameCandidate {
                                source_external_id: normalized_target,
                                title,
                                launch_target: executable_path.to_string_lossy().to_string(),
                                source_id: format!("source-local-{slug}-{hash}"),
                                game_id: format!("game-local-{slug}-{hash}"),
                                entry_id: format!("entry-local-{slug}-{hash}"),
                                launch_id: format!("launch-local-{slug}-{hash}"),
                                accent_color,
                            }
                        });
                }
            }
        }

        candidates.into_values().collect()
    }

    fn discover_steam_game_candidates(roots: &[PathBuf]) -> Vec<SteamGameCandidate> {
        let mut candidates = HashMap::new();

        for steam_root in roots {
            for steamapps_dir in steamapps_directories(steam_root) {
                let Ok(entries) = fs::read_dir(&steamapps_dir) else {
                    continue;
                };

                for entry in entries.flatten().take(2048) {
                    let manifest_path = entry.path();
                    if !is_steam_app_manifest(&manifest_path) {
                        continue;
                    }

                    if let Some(candidate) = parse_steam_manifest_candidate(&manifest_path)
                        .filter(|candidate| !is_rejected_steam_app(&candidate.app_id))
                    {
                        candidates
                            .entry(candidate.app_id.clone())
                            .or_insert(candidate);
                    }
                }
            }
        }

        candidates.into_values().collect()
    }

    fn steamapps_directories(steam_root: &Path) -> Vec<PathBuf> {
        let mut directories = Vec::new();
        let default_steamapps = if is_steamapps_path(steam_root) {
            steam_root.to_path_buf()
        } else {
            steam_root.join("steamapps")
        };

        if default_steamapps.is_dir() {
            directories.push(default_steamapps.clone());
        }

        let libraryfolders_path = default_steamapps.join("libraryfolders.vdf");
        let Ok(contents) = fs::read_to_string(libraryfolders_path) else {
            return directories;
        };

        for library_path in parse_steam_library_paths(&contents) {
            let steamapps_dir = library_path.join("steamapps");
            if steamapps_dir.is_dir() && !directories.iter().any(|path| path == &steamapps_dir) {
                directories.push(steamapps_dir);
            }
        }

        directories
    }

    fn parse_steam_library_paths(contents: &str) -> Vec<PathBuf> {
        key_value_pairs(contents)
            .into_iter()
            .filter_map(|(key, value)| {
                if key.eq_ignore_ascii_case("path") {
                    Some(PathBuf::from(value.replace("\\\\", "\\")))
                } else {
                    None
                }
            })
            .collect()
    }

    fn is_steam_app_manifest(path: &Path) -> bool {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        file_name.starts_with("appmanifest_") && file_name.ends_with(".acf")
    }

    fn parse_steam_manifest_candidate(manifest_path: &Path) -> Option<SteamGameCandidate> {
        let contents = fs::read_to_string(manifest_path).ok()?;
        let pairs = key_value_pairs(&contents);
        let app_id = get_key_value(&pairs, "appid")?.trim().to_string();
        let title = get_key_value(&pairs, "name")?.trim().to_string();

        if app_id.is_empty() || title.is_empty() {
            return None;
        }

        let install_dir = get_key_value(&pairs, "installdir")
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let install_path = install_dir.and_then(|directory| {
            manifest_path
                .parent()
                .map(|steamapps_dir| steamapps_dir.join("common").join(directory))
                .filter(|path| path.exists())
                .map(|path| path.to_string_lossy().to_string())
        });
        let slug = create_slug(&title);
        let accent_color = deterministic_accent_color(&title);

        Some(SteamGameCandidate {
            source_id: format!("source-steam-{app_id}"),
            game_id: format!("game-steam-{slug}-{app_id}"),
            entry_id: format!("entry-steam-{slug}-{app_id}"),
            launch_id: format!("launch-steam-{app_id}"),
            app_id,
            title,
            install_path,
            accent_color,
        })
    }

    fn remote_steam_game_candidate(
        game: &steam_web_api::RemoteSteamGame,
    ) -> RemoteSteamGameCandidate {
        let slug = create_slug(&game.title);
        let accent_color = deterministic_accent_color(&game.title);

        RemoteSteamGameCandidate {
            source_id: format!("source-steam-{}", game.app_id),
            game_id: format!("game-steam-{slug}-{}", game.app_id),
            entry_id: format!("entry-steam-{slug}-{}", game.app_id),
            launch_id: format!("launch-steam-{}", game.app_id),
            app_id: game.app_id.clone(),
            title: game.title.clone(),
            playtime_total_minutes: game.playtime_forever.unwrap_or_default(),
            has_playtime: game.playtime_forever.is_some(),
            accent_color,
        }
    }

    fn is_rejected_steam_app(app_id: &str) -> bool {
        app_id == "228980"
    }

    fn key_value_pairs(contents: &str) -> Vec<(String, String)> {
        contents
            .lines()
            .filter_map(|line| {
                let parts = line
                    .split('"')
                    .filter(|part| !part.trim().is_empty())
                    .collect::<Vec<_>>();

                if parts.len() >= 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_key_value<'a>(pairs: &'a [(String, String)], expected_key: &str) -> Option<&'a str> {
        pairs
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(expected_key))
            .map(|(_, value)| value.as_str())
    }

    fn candidate_directories(root: &Path) -> Vec<PathBuf> {
        let mut directories = Vec::new();

        if root.is_dir() {
            if !is_helper_directory(root) && has_direct_executable(root) {
                directories.push(root.to_path_buf());
            }

            if let Ok(children) = fs::read_dir(root) {
                for child in children.flatten().take(256) {
                    let path = child.path();
                    if path.is_dir() && !is_helper_directory(&path) {
                        directories.push(path);
                    }
                }
            }
        }

        directories
    }

    fn has_direct_executable(path: &Path) -> bool {
        fs::read_dir(path)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.flatten().take(256))
            .any(|entry| {
                let path = entry.path();
                path.is_file() && is_executable_file(&path)
            })
    }

    fn find_local_executable(candidate_dir: &Path) -> Option<PathBuf> {
        if is_helper_directory(candidate_dir) {
            return None;
        }

        let mut executables = Vec::new();
        collect_candidate_executables(candidate_dir, candidate_dir, 0, &mut executables);

        choose_best_executable(candidate_dir, executables)
    }

    fn collect_candidate_executables(
        root: &Path,
        current_dir: &Path,
        depth: usize,
        executables: &mut Vec<PathBuf>,
    ) {
        const MAX_SCAN_DEPTH: usize = 5;
        const MAX_ENTRIES_PER_DIR: usize = 256;
        const MAX_EXECUTABLES_PER_CANDIDATE: usize = 128;

        if depth > MAX_SCAN_DEPTH
            || executables.len() >= MAX_EXECUTABLES_PER_CANDIDATE
            || (current_dir != root && is_helper_directory(current_dir))
        {
            return;
        }

        let Ok(entries) = fs::read_dir(current_dir) else {
            return;
        };

        for entry in entries.flatten().take(MAX_ENTRIES_PER_DIR) {
            if executables.len() >= MAX_EXECUTABLES_PER_CANDIDATE {
                return;
            }

            let path = entry.path();
            if path.is_file() && is_executable_file(&path) {
                executables.push(path);
            } else if path.is_dir() {
                collect_candidate_executables(root, &path, depth + 1, executables);
            }
        }
    }

    fn choose_best_executable(candidate_dir: &Path, executables: Vec<PathBuf>) -> Option<PathBuf> {
        let directory_name = candidate_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let normalized_directory_name = normalize_name(directory_name);

        executables
            .into_iter()
            .filter_map(|path| {
                executable_score(candidate_dir, &path, &normalized_directory_name)
                    .map(|score| (score, path))
            })
            .max_by(|(left_score, left_path), (right_score, right_path)| {
                left_score.cmp(right_score).then_with(|| {
                    right_path
                        .to_string_lossy()
                        .cmp(&left_path.to_string_lossy())
                })
            })
            .map(|(_, path)| path)
    }

    fn executable_score(
        candidate_dir: &Path,
        executable_path: &Path,
        normalized_directory_name: &str,
    ) -> Option<i32> {
        if is_helper_executable(executable_path) || path_contains_helper_component(executable_path)
        {
            return None;
        }

        let stem = executable_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let normalized_stem = normalize_game_executable_name(stem);
        let mut score = 10;

        if normalized_stem == normalized_directory_name {
            score += 100;
        } else if !normalized_directory_name.is_empty()
            && (normalized_stem.contains(normalized_directory_name)
                || normalized_directory_name.contains(&normalized_stem))
        {
            score += 70;
        }

        if executable_path.parent() == Some(candidate_dir) {
            score += 30;
        }

        let normalized_path = executable_path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .map(normalize_name)
            .collect::<Vec<_>>();

        if normalized_path
            .iter()
            .any(|component| component == "binaries" || component == "win64")
        {
            score += 20;
        }

        Some(score)
    }

    fn is_helper_executable(path: &Path) -> bool {
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_lowercase();

        [
            "setup",
            "unins",
            "uninstall",
            "vc_redist",
            "dxsetup",
            "directx",
            "epiconlineservices",
            "redistributable",
            "redist",
            "prereq",
            "runtime",
            "service",
            "support",
            "installer",
            "repair",
            "crash",
            "helper",
        ]
        .iter()
        .any(|keyword| stem.contains(keyword))
    }

    fn is_helper_directory(path: &Path) -> bool {
        let normalized_components = path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .map(normalize_name)
            .collect::<Vec<_>>();

        if normalized_components.iter().any(|component| {
            matches!(
                component.as_str(),
                "staging" | "build" | "helper" | "helpers"
            )
        }) {
            return true;
        }

        let normalized_path = normalized_components.join("/");

        [
            "setup",
            "unins",
            "uninstall",
            "dxsetup",
            "directx",
            "epiconlineservices",
            "redistributable",
            "redist",
            "prereq",
            "runtime",
            "service",
            "support",
            "installer",
            "repair",
            "engine",
            "extras",
            "supportfiles",
            "thirdparty",
        ]
        .iter()
        .any(|keyword| normalized_path.contains(keyword))
    }

    fn path_contains_helper_component(path: &Path) -> bool {
        path.parent().is_some_and(is_helper_directory)
    }

    fn is_executable_file(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
    }

    fn candidate_title(candidate_dir: &Path, executable_path: &Path) -> String {
        if let Some(title) = candidate_dir
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
        {
            return title.to_string();
        }

        executable_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Jogo Local")
            .to_string()
    }

    fn is_rejected_local_title(title: &str) -> bool {
        let normalized_title = normalize_name(title);

        normalized_title.ends_with("launcher")
    }

    fn normalize_game_executable_name(value: &str) -> String {
        let mut normalized = normalize_name(value);

        for suffix in [
            "win64shipping",
            "win32shipping",
            "shipping",
            "win64",
            "win32",
            "x64",
            "x86",
            "eaceos",
            "eac",
            "eos",
        ] {
            if let Some(stripped) = normalized.strip_suffix(suffix) {
                normalized = stripped.to_string();
            }
        }

        normalized
    }

    fn normalize_name(value: &str) -> String {
        value
            .trim()
            .to_lowercase()
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .collect()
    }

    fn normalize_path_string(path: &Path) -> String {
        path.to_string_lossy().replace('/', "\\").to_lowercase()
    }

    fn stable_hash_hex(value: &str) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;

        for byte in value.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        format!("{hash:016x}")
    }

    fn list_local_entries_by_source(
        connection: &Connection,
    ) -> rusqlite::Result<HashMap<String, EntryRow>> {
        let mut statement = connection.prepare(
            r#"
            SELECT
              game_sources.external_id,
              library_entries.id,
              library_entries.primary_platform_id,
              library_entries.install_status,
              library_entries.last_played_label,
              library_entries.is_archived,
              library_entries.added_at,
              library_entries.updated_at,
              games.id,
              games.title,
              games.sort_title,
              games.installed,
              games.playtime_total_minutes,
              games.accent_color,
              game_sources.account_id
            FROM library_entries
            JOIN games ON games.id = library_entries.game_id
            JOIN game_sources ON game_sources.game_id = library_entries.game_id
            WHERE library_entries.primary_platform_id = 'local'
              AND game_sources.platform_id = 'local'
            "#,
        )?;

        let entries = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    EntryRow {
                        entry_id: row.get(1)?,
                        primary_platform_id: row.get(2)?,
                        install_status: row.get(3)?,
                        last_played_label: row.get(4)?,
                        is_archived: row.get::<_, i64>(5)? == 1,
                        added_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        game_id: row.get(8)?,
                        title: row.get(9)?,
                        sort_title: row.get(10)?,
                        installed: row.get::<_, i64>(11)? == 1,
                        playtime_total_minutes: row.get(12)?,
                        accent_color: row.get(13)?,
                        source_account_id: row.get(14)?,
                    },
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(entries.into_iter().collect())
    }

    fn list_steam_entries_by_source(
        connection: &Connection,
    ) -> rusqlite::Result<HashMap<String, EntryRow>> {
        let mut statement = connection.prepare(
            r#"
            SELECT
              game_sources.external_id,
              library_entries.id,
              library_entries.primary_platform_id,
              library_entries.install_status,
              library_entries.last_played_label,
              library_entries.is_archived,
              library_entries.added_at,
              library_entries.updated_at,
              games.id,
              games.title,
              games.sort_title,
              games.installed,
              games.playtime_total_minutes,
              games.accent_color,
              game_sources.account_id
            FROM library_entries
            JOIN games ON games.id = library_entries.game_id
            JOIN game_sources ON game_sources.game_id = library_entries.game_id
            WHERE library_entries.primary_platform_id = 'steam'
              AND game_sources.platform_id = 'steam'
            "#,
        )?;

        let entries = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    EntryRow {
                        entry_id: row.get(1)?,
                        primary_platform_id: row.get(2)?,
                        install_status: row.get(3)?,
                        last_played_label: row.get(4)?,
                        is_archived: row.get::<_, i64>(5)? == 1,
                        added_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        game_id: row.get(8)?,
                        title: row.get(9)?,
                        sort_title: row.get(10)?,
                        installed: row.get::<_, i64>(11)? == 1,
                        playtime_total_minutes: row.get(12)?,
                        accent_color: row.get(13)?,
                        source_account_id: row.get(14)?,
                    },
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(entries.into_iter().collect())
    }

    fn insert_steam_entry(
        transaction: &rusqlite::Transaction<'_>,
        candidate: &SteamGameCandidate,
    ) -> rusqlite::Result<()> {
        let now = now_iso();
        transaction.execute(
            r#"
            INSERT INTO games (
              id, title, sort_title, installed, playtime_total_minutes, accent_color, created_at, updated_at
            ) VALUES (?1, ?2, ?3, 1, 0, ?4, ?5, ?5)
            "#,
            params![
                candidate.game_id,
                candidate.title,
                candidate.title,
                candidate.accent_color,
                now,
            ],
        )?;
        transaction.execute(
            r#"
            INSERT INTO library_entries (
              id, game_id, primary_platform_id, install_status, last_played_label, is_archived, added_at, updated_at
            ) VALUES (?1, ?2, 'steam', 'installed', 'Nunca', 0, ?3, ?3)
            "#,
            params![candidate.entry_id, candidate.game_id, now],
        )?;
        transaction.execute(
            r#"
            INSERT INTO game_sources (id, game_id, platform_id, external_id)
            VALUES (?1, ?2, 'steam', ?3)
            "#,
            params![candidate.source_id, candidate.game_id, candidate.app_id],
        )?;
        transaction.execute(
            r#"
            INSERT INTO launch_actions (
              id, game_id, platform_id, kind, label, target, arguments_json, working_directory, is_primary
            ) VALUES (?1, ?2, 'steam', 'uri', ?3, ?4, '[]', ?5, 1)
            "#,
            params![
                candidate.launch_id,
                candidate.game_id,
                format!("steam://rungameid/{}", candidate.app_id),
                format!("steam://rungameid/{}", candidate.app_id),
                candidate.install_path.as_deref(),
            ],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, 'Steam', 0)",
            params![candidate.game_id],
        )?;

        Ok(())
    }

    fn insert_remote_steam_entry(
        transaction: &rusqlite::Transaction<'_>,
        steam_id: &str,
        candidate: &RemoteSteamGameCandidate,
    ) -> rusqlite::Result<()> {
        let now = now_iso();
        transaction.execute(
            r#"
            INSERT INTO games (
              id, title, sort_title, installed, playtime_total_minutes, accent_color, created_at, updated_at
            ) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?6)
            "#,
            params![
                candidate.game_id,
                candidate.title,
                candidate.title,
                candidate.playtime_total_minutes,
                candidate.accent_color,
                now,
            ],
        )?;
        transaction.execute(
            r#"
            INSERT INTO library_entries (
              id, game_id, primary_platform_id, install_status, last_played_label, is_archived, added_at, updated_at
            ) VALUES (?1, ?2, 'steam', 'not_installed', 'Nunca', 0, ?3, ?3)
            "#,
            params![candidate.entry_id, candidate.game_id, now],
        )?;
        transaction.execute(
            r#"
            INSERT INTO game_sources (id, game_id, platform_id, external_id, account_id)
            VALUES (?1, ?2, 'steam', ?3, ?4)
            "#,
            params![
                candidate.source_id,
                candidate.game_id,
                candidate.app_id,
                steam_id
            ],
        )?;
        let launch_target = format!("steam://rungameid/{}", candidate.app_id);
        transaction.execute(
            r#"
            INSERT INTO launch_actions (
              id, game_id, platform_id, kind, label, target, arguments_json, working_directory, is_primary
            ) VALUES (?1, ?2, 'steam', 'uri', ?3, ?3, '[]', NULL, 1)
            "#,
            params![candidate.launch_id, candidate.game_id, launch_target],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, 'Steam', 0)",
            params![candidate.game_id],
        )?;

        Ok(())
    }

    fn update_steam_entry(
        transaction: &rusqlite::Transaction<'_>,
        existing_row: &EntryRow,
        candidate: &SteamGameCandidate,
    ) -> rusqlite::Result<bool> {
        let launch_target = format!("steam://rungameid/{}", candidate.app_id);
        let current_action = find_steam_primary_action(transaction, &existing_row.game_id)?;
        let needs_action_update = current_action
            .as_ref()
            .map(|action| {
                action.kind != "uri"
                    || action.label != launch_target
                    || action.target != launch_target
                    || action.working_directory != candidate.install_path
            })
            .unwrap_or(true);
        let needs_entry_update = existing_row.title != candidate.title
            || existing_row.sort_title != candidate.title
            || !existing_row.installed
            || existing_row.install_status != "installed"
            || existing_row.is_archived
            || existing_row.accent_color.as_deref() != Some(candidate.accent_color);

        if !needs_entry_update && !needs_action_update {
            return Ok(false);
        }

        let updated_at = now_iso();

        if needs_entry_update {
            transaction.execute(
                r#"
                UPDATE games
                SET title = ?2,
                    sort_title = ?3,
                    installed = 1,
                    accent_color = ?4,
                    updated_at = ?5
                WHERE id = ?1
                "#,
                params![
                    existing_row.game_id,
                    candidate.title,
                    candidate.title,
                    candidate.accent_color,
                    updated_at,
                ],
            )?;
            transaction.execute(
                r#"
                UPDATE library_entries
                SET install_status = 'installed',
                    is_archived = 0,
                    updated_at = ?2
                WHERE id = ?1
                "#,
                params![existing_row.entry_id, updated_at],
            )?;
        }

        if needs_action_update {
            upsert_steam_primary_action(
                transaction,
                &existing_row.game_id,
                &candidate.launch_id,
                &launch_target,
                candidate.install_path.as_deref(),
            )?;
        }

        transaction.execute(
            "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, 'Steam', 0)",
            params![existing_row.game_id],
        )?;

        Ok(true)
    }

    fn update_remote_steam_entry(
        transaction: &rusqlite::Transaction<'_>,
        steam_id: &str,
        existing_row: &EntryRow,
        candidate: &RemoteSteamGameCandidate,
    ) -> rusqlite::Result<bool> {
        let launch_target = format!("steam://rungameid/{}", candidate.app_id);
        let current_action = find_steam_primary_action(transaction, &existing_row.game_id)?;
        let preserved_working_directory = current_action
            .as_ref()
            .and_then(|action| action.working_directory.as_deref())
            .filter(|_| existing_row.installed);
        let needs_action_update = current_action
            .as_ref()
            .map(|action| {
                action.kind != "uri"
                    || action.label != launch_target
                    || action.target != launch_target
            })
            .unwrap_or(true);
        let new_install_status = if existing_row.installed {
            "installed"
        } else {
            "not_installed"
        };
        let needs_entry_update = existing_row.title != candidate.title
            || existing_row.sort_title != candidate.title
            || existing_row.install_status != new_install_status
            || existing_row.is_archived
            || existing_row.accent_color.as_deref() != Some(candidate.accent_color)
            || (candidate.has_playtime
                && existing_row.playtime_total_minutes != candidate.playtime_total_minutes);
        let needs_account_update = existing_row.source_account_id.as_deref() != Some(steam_id);

        if !needs_entry_update && !needs_action_update && !needs_account_update {
            return Ok(false);
        }

        let updated_at = now_iso();

        if needs_entry_update {
            transaction.execute(
                r#"
                UPDATE games
                SET title = ?2,
                    sort_title = ?3,
                    playtime_total_minutes = CASE WHEN ?4 THEN ?5 ELSE playtime_total_minutes END,
                    accent_color = ?6,
                    updated_at = ?7
                WHERE id = ?1
                "#,
                params![
                    existing_row.game_id,
                    candidate.title,
                    candidate.title,
                    candidate.has_playtime,
                    candidate.playtime_total_minutes,
                    candidate.accent_color,
                    updated_at,
                ],
            )?;
            transaction.execute(
                r#"
                UPDATE library_entries
                SET install_status = ?2,
                    is_archived = 0,
                    updated_at = ?3
                WHERE id = ?1
                "#,
                params![existing_row.entry_id, new_install_status, updated_at],
            )?;
        }

        if needs_action_update {
            upsert_steam_primary_action(
                transaction,
                &existing_row.game_id,
                &candidate.launch_id,
                &launch_target,
                preserved_working_directory,
            )?;
        }

        if needs_account_update {
            transaction.execute(
                r#"
                UPDATE game_sources
                SET account_id = ?2
                WHERE game_id = ?1
                  AND platform_id = 'steam'
                "#,
                params![existing_row.game_id, steam_id],
            )?;
        }

        transaction.execute(
            "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, 'Steam', 0)",
            params![existing_row.game_id],
        )?;

        Ok(true)
    }

    struct SteamPrimaryActionRow {
        kind: String,
        label: String,
        target: String,
        working_directory: Option<String>,
    }

    fn find_steam_primary_action(
        transaction: &rusqlite::Transaction<'_>,
        game_id: &str,
    ) -> rusqlite::Result<Option<SteamPrimaryActionRow>> {
        transaction
            .query_row(
                r#"
                SELECT kind, label, target, working_directory
                FROM launch_actions
                WHERE game_id = ?1
                  AND platform_id = 'steam'
                  AND is_primary = 1
                ORDER BY id
                LIMIT 1
                "#,
                params![game_id],
                |row| {
                    Ok(SteamPrimaryActionRow {
                        kind: row.get(0)?,
                        label: row.get(1)?,
                        target: row.get(2)?,
                        working_directory: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    fn upsert_steam_primary_action(
        transaction: &rusqlite::Transaction<'_>,
        game_id: &str,
        fallback_launch_id: &str,
        launch_target: &str,
        install_path: Option<&str>,
    ) -> rusqlite::Result<()> {
        let changed = transaction.execute(
            r#"
            UPDATE launch_actions
            SET kind = 'uri',
                label = ?2,
                target = ?2,
                working_directory = ?3
            WHERE game_id = ?1
              AND platform_id = 'steam'
              AND is_primary = 1
            "#,
            params![game_id, launch_target, install_path],
        )?;

        if changed == 0 {
            transaction.execute(
                r#"
                INSERT INTO launch_actions (
                  id, game_id, platform_id, kind, label, target, arguments_json, working_directory, is_primary
                ) VALUES (?1, ?2, 'steam', 'uri', ?3, ?3, '[]', ?4, 1)
                "#,
                params![fallback_launch_id, game_id, launch_target, install_path],
            )?;
        }

        Ok(())
    }

    fn mark_steam_entry_unavailable(
        transaction: &rusqlite::Transaction<'_>,
        existing_row: &EntryRow,
    ) -> rusqlite::Result<bool> {
        if !existing_row.installed && existing_row.install_status == "not_installed" {
            return Ok(false);
        }

        let updated_at = now_iso();
        transaction.execute(
            "UPDATE games SET installed = 0, updated_at = ?2 WHERE id = ?1",
            params![existing_row.game_id, updated_at],
        )?;
        transaction.execute(
            r#"
            UPDATE library_entries
            SET install_status = 'not_installed',
                updated_at = ?2
            WHERE id = ?1
            "#,
            params![existing_row.entry_id, updated_at],
        )?;

        Ok(true)
    }

    fn archive_rejected_steam_entries(
        transaction: &rusqlite::Transaction<'_>,
    ) -> rusqlite::Result<usize> {
        let updated_at = now_iso();

        transaction.execute(
            r#"
            UPDATE library_entries
            SET is_archived = 1,
                updated_at = ?1
            WHERE primary_platform_id = 'steam'
              AND is_archived = 0
              AND game_id IN (
                SELECT games.id
                FROM games
                JOIN game_sources ON game_sources.game_id = games.id
                WHERE game_sources.platform_id = 'steam'
                  AND game_sources.external_id = '228980'
              )
            "#,
            params![updated_at],
        )
    }

    fn insert_local_entry(
        transaction: &rusqlite::Transaction<'_>,
        candidate: &LocalGameCandidate,
    ) -> rusqlite::Result<()> {
        let now = now_iso();
        transaction.execute(
            r#"
            INSERT INTO games (
              id, title, sort_title, installed, playtime_total_minutes, accent_color, created_at, updated_at
            ) VALUES (?1, ?2, ?3, 1, 0, ?4, ?5, ?5)
            "#,
            params![
                candidate.game_id,
                candidate.title,
                candidate.title,
                candidate.accent_color,
                now,
            ],
        )?;
        transaction.execute(
            r#"
            INSERT INTO library_entries (
              id, game_id, primary_platform_id, install_status, last_played_label, is_archived, added_at, updated_at
            ) VALUES (?1, ?2, 'local', 'installed', 'Nunca', 0, ?3, ?3)
            "#,
            params![candidate.entry_id, candidate.game_id, now],
        )?;
        transaction.execute(
            r#"
            INSERT INTO game_sources (id, game_id, platform_id, external_id)
            VALUES (?1, ?2, 'local', ?3)
            "#,
            params![
                candidate.source_id,
                candidate.game_id,
                candidate.source_external_id
            ],
        )?;
        transaction.execute(
            r#"
            INSERT INTO launch_actions (
              id, game_id, platform_id, kind, label, target, arguments_json, is_primary
            ) VALUES (?1, ?2, 'local', 'executable', ?3, ?4, '[]', 1)
            "#,
            params![
                candidate.launch_id,
                candidate.game_id,
                candidate.title,
                candidate.launch_target,
            ],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, 'Local', 0)",
            params![candidate.game_id],
        )?;

        Ok(())
    }

    fn update_local_entry(
        transaction: &rusqlite::Transaction<'_>,
        existing_row: &EntryRow,
        candidate: &LocalGameCandidate,
    ) -> rusqlite::Result<()> {
        let updated_at = now_iso();
        transaction.execute(
            r#"
            UPDATE games
            SET title = ?2,
                sort_title = ?3,
                installed = 1,
                accent_color = ?4,
                updated_at = ?5
            WHERE id = ?1
            "#,
            params![
                existing_row.game_id,
                candidate.title,
                candidate.title,
                candidate.accent_color,
                updated_at,
            ],
        )?;
        transaction.execute(
            r#"
            UPDATE library_entries
            SET install_status = 'installed',
                is_archived = 0,
                updated_at = ?2
            WHERE id = ?1
            "#,
            params![existing_row.entry_id, updated_at],
        )?;
        transaction.execute(
            r#"
            UPDATE launch_actions
            SET kind = 'executable',
                label = ?2,
                target = ?3
            WHERE game_id = ?1
              AND is_primary = 1
            "#,
            params![
                existing_row.game_id,
                candidate.title,
                candidate.launch_target
            ],
        )?;
        transaction.execute(
            "DELETE FROM game_genres WHERE game_id = ?1",
            params![existing_row.game_id],
        )?;
        transaction.execute(
            "INSERT INTO game_genres (game_id, genre, position) VALUES (?1, 'Local', 0)",
            params![existing_row.game_id],
        )?;

        Ok(())
    }

    fn archive_rejected_local_entries(connection: &Connection) -> rusqlite::Result<usize> {
        let active_local_entries = connection.query_row(
            r#"
            SELECT COUNT(*)
            FROM library_entries
            WHERE primary_platform_id = 'local'
              AND is_archived = 0
            "#,
            [],
            |row| row.get::<_, i64>(0),
        )?;

        if active_local_entries == 0 {
            return Ok(0);
        }

        connection.execute(
            r#"
            UPDATE library_entries
            SET is_archived = 1,
                updated_at = ?1
            WHERE primary_platform_id = 'local'
              AND is_archived = 0
              AND EXISTS (
                SELECT 1
                FROM launch_actions
                WHERE platform_id = 'local'
                  AND kind = 'executable'
                  AND launch_actions.game_id = library_entries.game_id
                  AND (
                    lower(target) LIKE '%directx%'
                    OR lower(target) LIKE '%dxsetup%'
                    OR lower(target) LIKE '%epiconlineservices%'
                    OR lower(target) LIKE '%_commonredist%'
                    OR lower(target) LIKE '%redistributable%'
                    OR lower(target) LIKE '%vc_redist%'
                    OR lower(target) LIKE '%redist%'
                    OR lower(target) LIKE '%prereq%'
                    OR lower(target) LIKE '%installer%'
                    OR lower(label) LIKE '%directx%'
                    OR lower(label) LIKE '%dxsetup%'
                    OR lower(label) LIKE '%epiconlineservices%'
                    OR lower(label) LIKE '%installer%'
                  )
              )
            "#,
            params![now_iso()],
        )
    }

    pub fn set_library_entry_archived(
        connection: &mut Connection,
        entry_id: &str,
        is_archived: bool,
    ) -> rusqlite::Result<()> {
        let updated_at = now_iso();
        let affected = connection.execute(
            r#"
            UPDATE library_entries
            SET is_archived = ?2,
                updated_at = ?3
            WHERE id = ?1
            "#,
            params![entry_id, is_archived as i64, updated_at],
        )?;

        if affected == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        Ok(())
    }

    fn create_slug(value: &str) -> String {
        let mut slug = String::new();
        let mut last_was_dash = false;

        for character in value.trim().to_lowercase().chars() {
            if character.is_ascii_alphanumeric() {
                slug.push(character);
                last_was_dash = false;
            } else if !last_was_dash {
                slug.push('-');
                last_was_dash = true;
            }
        }

        let slug = slug.trim_matches('-').to_string();
        if slug.is_empty() {
            "jogo-manual".to_string()
        } else {
            slug
        }
    }

    fn deterministic_accent_color(value: &str) -> &'static str {
        const PALETTE: [&str; 8] = [
            "#0d9488", "#2563eb", "#7c3aed", "#be123c", "#c2410c", "#15803d", "#9333ea", "#b45309",
        ];
        let hash: usize = value.chars().map(|character| character as usize).sum();

        PALETTE[hash % PALETTE.len()]
    }

    fn launch_action_kind(target: &str) -> &'static str {
        if target.is_empty() {
            "manual"
        } else if target.contains("://") {
            "uri"
        } else {
            "executable"
        }
    }

    fn now_iso() -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    fn timestamp_millis() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::Path;

        fn open_seeded_database(path: &Path) -> Connection {
            let mut connection = open_database(path).expect("open database");
            seed_mock_library(&mut connection).expect("seed mock library");
            connection
        }

        fn index_exists(connection: &Connection, index_name: &str) -> rusqlite::Result<bool> {
            connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    params![index_name],
                    |_| Ok(()),
                )
                .optional()
                .map(|value| value.is_some())
        }

        #[test]
        fn migration_creates_schema_version() {
            let connection = Connection::open_in_memory().expect("open in-memory database");

            migrate(&connection).expect("apply migration");

            let version: i64 = connection
                .query_row("SELECT version FROM schema_migrations", [], |row| {
                    row.get(0)
                })
                .expect("read schema version");

            assert_eq!(version, 1);
        }

        #[test]
        fn open_database_seeds_mock_library_once() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-seed-{}.sqlite3",
                timestamp_millis()
            ));

            {
                let connection = open_seeded_database(&path);
                let entries = list_library_entries(&connection).expect("list seeded entries");

                assert_eq!(entries.len(), 4);
                assert!(entries.iter().any(|entry| entry.id == "entry-steam-hades"));
                assert!(entries
                    .iter()
                    .any(|entry| entry.id == "entry-local-minecraft"));
                assert!(entries
                    .iter()
                    .any(|entry| entry.id == "entry-manual-silksong"));
            }

            {
                let connection = open_seeded_database(&path);
                let entries = list_library_entries(&connection).expect("list seeded entries again");

                assert_eq!(entries.len(), 4);
            }

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn open_database_starts_without_seeded_library() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-empty-{}.sqlite3",
                timestamp_millis()
            ));

            let connection = open_database(&path).expect("open empty database");
            let entries = list_library_entries(&connection).expect("list empty entries");

            assert!(entries.is_empty());

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn open_database_creates_local_cleanup_indexes() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-cleanup-indexes-{}.sqlite3",
                timestamp_millis()
            ));

            let connection = open_database(&path).expect("open database");

            assert!(
                index_exists(&connection, "idx_library_entries_local_active_game")
                    .expect("check local entry cleanup index")
            );
            assert!(
                index_exists(&connection, "idx_launch_actions_platform_kind_game")
                    .expect("check launch action cleanup index")
            );

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_steam_games_imports_app_manifests_once() {
            let steam_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-root-{}",
                timestamp_millis()
            ));
            let steamapps = steam_root.join("steamapps");
            let common = steamapps.join("common").join("Stardew Valley");
            let manifest = steamapps.join("appmanifest_413150.acf");

            std::fs::create_dir_all(&common).expect("create steam library");
            std::fs::write(
                &manifest,
                r#""AppState"
{
  "appid" "413150"
  "name" "Stardew Valley"
  "installdir" "Stardew Valley"
}
"#,
            )
            .expect("write steam manifest");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary =
                sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                    .expect("sync steam games");
            let entries = list_library_entries(&connection).expect("list steam entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert_eq!(summary.unavailable, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].primary_platform_id, "steam");
            assert_eq!(entries[0].game.title, "Stardew Valley");
            assert_eq!(entries[0].game.sources[0].external_id, "413150");
            assert_eq!(entries[0].game.launch_actions[0].kind, "uri");
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                "steam://rungameid/413150"
            );
            assert_eq!(
                entries[0].game.launch_actions[0]
                    .working_directory
                    .as_deref(),
                Some(common.to_string_lossy().as_ref())
            );

            let second_summary =
                sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                    .expect("resync steam games");
            let entries_again =
                list_library_entries(&connection).expect("list steam entries again");

            assert_eq!(second_summary.discovered, 1);
            assert_eq!(second_summary.inserted, 0);
            assert_eq!(second_summary.updated, 0);
            assert_eq!(second_summary.unavailable, 0);
            assert_eq!(entries_again.len(), 1);

            let _ = std::fs::remove_dir_all(steam_root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_steam_games_marks_removed_manifest_not_installed() {
            let steam_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-removed-root-{}",
                timestamp_millis()
            ));
            let steamapps = steam_root.join("steamapps");
            let common = steamapps.join("common").join("Portal 2");
            let manifest = steamapps.join("appmanifest_620.acf");

            std::fs::create_dir_all(&common).expect("create steam library");
            std::fs::write(
                &manifest,
                r#""AppState"
{
  "appid" "620"
  "name" "Portal 2"
  "installdir" "Portal 2"
}
"#,
            )
            .expect("write steam manifest");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-removed-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                .expect("sync steam games");
            std::fs::remove_file(&manifest).expect("remove manifest");

            let summary =
                sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                    .expect("resync removed manifest");
            let entries = list_library_entries(&connection).expect("list steam entries");

            assert_eq!(summary.discovered, 0);
            assert_eq!(summary.inserted, 0);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.unavailable, 1);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].primary_platform_id, "steam");
            assert_eq!(entries[0].install_status, "not_installed");
            assert!(!entries[0].game.installed);

            let _ = std::fs::remove_dir_all(steam_root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_steam_games_restores_archived_entries_when_manifest_returns() {
            let steam_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-restore-root-{}",
                timestamp_millis()
            ));
            let steamapps = steam_root.join("steamapps");
            let common = steamapps.join("common").join("Portal 2");
            let manifest = steamapps.join("appmanifest_620.acf");

            std::fs::create_dir_all(&common).expect("create steam library");
            std::fs::write(
                &manifest,
                r#""AppState"
{
  "appid" "620"
  "name" "Portal 2"
  "installdir" "Portal 2"
}
"#,
            )
            .expect("write steam manifest");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-restore-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");
            let candidate = SteamGameCandidate {
                app_id: "620".to_string(),
                title: "Portal 2".to_string(),
                install_path: Some(common.to_string_lossy().to_string()),
                source_id: "source-steam-620".to_string(),
                game_id: "game-steam-portal-2-620".to_string(),
                entry_id: "entry-steam-portal-2-620".to_string(),
                launch_id: "launch-steam-620".to_string(),
                accent_color: "#2563eb",
            };

            {
                let transaction = connection.transaction().expect("start transaction");
                insert_steam_entry(&transaction, &candidate).expect("insert stale steam entry");
                transaction.commit().expect("commit stale steam entry");
            }
            set_library_entry_archived(&mut connection, "entry-steam-portal-2-620", true)
                .expect("archive steam entry");

            let summary =
                sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                    .expect("resync steam manifest");
            let stored_entry = find_entry(&connection, "entry-steam-portal-2-620")
                .expect("read stored entry")
                .expect("entry exists");
            let visible_entries = list_library_entries(&connection).expect("list visible entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.updated, 1);
            assert_eq!(summary.archived, 0);
            assert!(!stored_entry.is_archived);
            assert!(visible_entries
                .iter()
                .any(|entry| entry.id == "entry-steam-portal-2-620"));

            let _ = std::fs::remove_dir_all(steam_root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_steam_games_ignores_and_archives_common_redistributables() {
            let steam_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-redist-root-{}",
                timestamp_millis()
            ));
            let steamapps = steam_root.join("steamapps");
            let common = steamapps.join("common").join("Steamworks Shared");
            let manifest = steamapps.join("appmanifest_228980.acf");

            std::fs::create_dir_all(&common).expect("create steam redist library");
            std::fs::write(
                &manifest,
                r#""AppState"
{
  "appid" "228980"
  "name" "Steamworks Common Redistributables"
  "installdir" "Steamworks Shared"
}
"#,
            )
            .expect("write steam redist manifest");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-redist-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");
            let stale_candidate = SteamGameCandidate {
                app_id: "228980".to_string(),
                title: "Steamworks Common Redistributables".to_string(),
                install_path: Some(common.to_string_lossy().to_string()),
                source_id: "source-steam-228980".to_string(),
                game_id: "game-steam-common-redistributables-228980".to_string(),
                entry_id: "entry-steam-common-redistributables-228980".to_string(),
                launch_id: "launch-steam-228980".to_string(),
                accent_color: "#64748b",
            };

            {
                let transaction = connection.transaction().expect("start transaction");
                insert_steam_entry(&transaction, &stale_candidate)
                    .expect("insert stale redist entry");
                transaction.commit().expect("commit stale redist entry");
            }

            let summary =
                sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                    .expect("sync steam redist");
            let entries = list_library_entries(&connection).expect("list steam entries");

            assert_eq!(summary.discovered, 0);
            assert_eq!(summary.inserted, 0);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 1);
            assert_eq!(summary.unavailable, 0);
            assert!(entries.is_empty());

            let _ = std::fs::remove_dir_all(steam_root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_steam_games_reads_additional_libraryfolders() {
            let steam_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-primary-{}",
                timestamp_millis()
            ));
            let extra_library = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-extra-{}",
                timestamp_millis()
            ));
            let primary_steamapps = steam_root.join("steamapps");
            let extra_steamapps = extra_library.join("steamapps");
            let extra_common = extra_steamapps.join("common").join("Hades");

            std::fs::create_dir_all(&primary_steamapps).expect("create primary steamapps");
            std::fs::create_dir_all(&extra_common).expect("create extra steamapps");
            std::fs::write(
                primary_steamapps.join("libraryfolders.vdf"),
                format!(
                    r#""libraryfolders"
{{
  "0"
  {{
    "path" "{}"
  }}
}}
"#,
                    extra_library.to_string_lossy().replace('\\', "\\\\")
                ),
            )
            .expect("write libraryfolders");
            std::fs::write(
                extra_steamapps.join("appmanifest_1145360.acf"),
                r#""AppState"
{
  "appid" "1145360"
  "name" "Hades"
  "installdir" "Hades"
}
"#,
            )
            .expect("write extra manifest");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-extra-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary =
                sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                    .expect("sync steam libraries");
            let entries = list_library_entries(&connection).expect("list steam entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Hades");
            assert_eq!(entries[0].game.sources[0].external_id, "1145360");

            let _ = std::fs::remove_dir_all(steam_root);
            let _ = std::fs::remove_dir_all(extra_library);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn collect_steam_roots_includes_saved_additional_library_roots() {
            let extra_library = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-saved-extra-{}",
                timestamp_millis()
            ));
            let extra_steamapps = extra_library.join("steamapps");
            let extra_common = extra_steamapps.join("common").join("Hollow Knight");

            std::fs::create_dir_all(&extra_common).expect("create saved steamapps");
            std::fs::write(
                extra_steamapps.join("appmanifest_367520.acf"),
                r#""AppState"
{
  "appid" "367520"
  "name" "Hollow Knight"
  "installdir" "Hollow Knight"
}
"#,
            )
            .expect("write saved manifest");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-saved-extra-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            save_steam_library_roots(
                &mut connection,
                SteamLibraryRootsInput {
                    roots: vec![extra_library.to_string_lossy().to_string()],
                },
            )
            .expect("save additional steam root");
            let collected_roots = collect_steam_roots(&connection);

            let expected_root = normalize_path_string(
                &extra_library
                    .canonicalize()
                    .unwrap_or(extra_library.clone()),
            );
            assert!(collected_roots
                .iter()
                .any(|root| normalize_path_string(root) == expected_root));

            let summary =
                sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&extra_library))
                    .expect("sync saved steam roots");
            let entries = list_library_entries(&connection).expect("list steam entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Hollow Knight");
            assert_eq!(entries[0].game.sources[0].external_id, "367520");

            let _ = std::fs::remove_dir_all(extra_library);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn save_steam_library_roots_accepts_steamapps_path_and_preserves_account_config() {
            let extra_library = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steamapps-input-{}",
                timestamp_millis()
            ));
            let extra_steamapps = extra_library.join("steamapps");

            std::fs::create_dir_all(&extra_steamapps).expect("create steamapps");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steamapps-input-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            save_verified_steam_account_config(&mut connection, "76561198000000000")
                .expect("save steam account");
            let dto = save_steam_library_roots(
                &mut connection,
                SteamLibraryRootsInput {
                    roots: vec![extra_steamapps.to_string_lossy().to_string()],
                },
            )
            .expect("save steamapps path");

            assert_eq!(dto.roots.len(), 1);
            assert_eq!(
                read_steam_account_config(&connection).expect("read steam account"),
                Some("76561198000000000".to_string())
            );
            assert_eq!(
                read_steam_library_roots(&connection).expect("read steam roots"),
                dto.roots
            );

            let _ = std::fs::remove_dir_all(extra_library);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn read_steam_account_config_accepts_direct_column_and_json() {
            let connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            ensure_provider_account_configs_table(&connection).expect("ensure account config");

            connection
                .execute(
                    r#"
                    INSERT INTO provider_account_configs (
                      provider_id, account_id, steam_id64, config_json, updated_at
                    ) VALUES ('steam', NULL, '76561198000000000', NULL, ?1)
                    "#,
                    params![now_iso()],
                )
                .expect("insert direct steam id");

            assert_eq!(
                read_steam_account_config(&connection).expect("read direct steam id"),
                Some("76561198000000000".to_string())
            );

            connection
                .execute("DELETE FROM provider_account_configs", [])
                .expect("clear config");
            connection
                .execute(
                    r#"
                    INSERT INTO provider_account_configs (
                      provider_id, account_id, steam_id64, config_json, updated_at
                    ) VALUES ('steam', NULL, NULL, '{"steamId64":"76561198000000001"}', ?1)
                    "#,
                    params![now_iso()],
                )
                .expect("insert json steam id");

            assert_eq!(
                read_steam_account_config(&connection).expect("read json steam id"),
                Some("76561198000000001".to_string())
            );
        }

        #[test]
        fn steam_web_api_key_config_tracks_metadata_without_losing_account() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            ensure_provider_account_configs_table(&connection).expect("ensure account config");
            save_verified_steam_account_config(&mut connection, "76561198000000000")
                .expect("save steam account");

            let columns =
                table_columns(&connection, "provider_account_configs").expect("read columns");
            assert!(!columns
                .iter()
                .any(|column| column == "steam_web_api_key_plaintext_dev"));

            set_steam_web_api_key_config(&mut connection, true).expect("save key metadata");

            assert_eq!(
                connection
                    .query_row(
                        "SELECT steam_web_api_key_configured FROM provider_account_configs WHERE provider_id = 'steam'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .expect("read key metadata"),
                1
            );
            assert_eq!(
                read_steam_account_config(&connection).expect("read steam account"),
                Some("76561198000000000".to_string())
            );

            set_steam_web_api_key_config(&mut connection, false).expect("clear key metadata");

            assert_eq!(
                connection
                    .query_row(
                        "SELECT steam_web_api_key_configured FROM provider_account_configs WHERE provider_id = 'steam'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .expect("read cleared metadata"),
                0
            );
            assert_eq!(
                read_steam_account_config(&connection).expect("account remains"),
                Some("76561198000000000".to_string())
            );
        }

        #[test]
        fn save_verified_steam_account_config_preserves_sync_metadata() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            save_verified_steam_account_config(&mut connection, "76561198000000000")
                .expect("save steam account");
            record_steam_account_sync_metadata(
                &mut connection,
                "76561198000000000",
                3,
                &SyncSummaryDto {
                    discovered: 3,
                    inserted: 2,
                    updated: 1,
                    archived: 0,
                    unavailable: 0,
                },
            )
            .expect("record sync metadata");

            save_verified_steam_account_config(&mut connection, "76561198000000000")
                .expect("save steam account again");

            let config_json: String = connection
                .query_row(
                    "SELECT config_json FROM provider_account_configs WHERE provider_id = 'steam'",
                    [],
                    |row| row.get(0),
                )
                .expect("read config json");

            assert!(config_json.contains("\"steamId64\":\"76561198000000000\""));
            assert!(config_json.contains("\"lastOwnedGamesSyncAt\""));
            assert!(config_json.contains("\"lastOwnedGamesSummary\""));
            assert!(config_json.contains("\"linkedBy\":\"steam_openid\""));
        }

        #[test]
        fn save_verified_xbox_account_config_preserves_sync_metadata() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            save_verified_xbox_account_config(&mut connection, "2533274791234567")
                .expect("save xbox account");
            record_xbox_achievement_sync_metadata(
                &mut connection,
                "2533274791234567",
                2,
                &SyncSummaryDto {
                    discovered: 2,
                    inserted: 1,
                    updated: 1,
                    archived: 0,
                    unavailable: 0,
                },
            )
            .expect("record xbox metadata");

            let config_json: String = connection
                .query_row(
                    "SELECT config_json FROM provider_account_configs WHERE provider_id = 'xbox'",
                    [],
                    |row| row.get(0),
                )
                .expect("read xbox config json");

            assert!(config_json.contains("\"xuid\":\"2533274791234567\""));
            assert!(config_json.contains("\"lastAchievementsSyncAt\""));
            assert!(config_json.contains("\"lastAchievementsSummary\""));
            assert!(config_json.contains("\"linkedBy\":\"xbox_manual_config\""));
        }

        #[test]
        fn read_xbox_account_config_accepts_direct_column_and_json() {
            let connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            ensure_provider_account_configs_table(&connection).expect("ensure account config");

            connection
                .execute(
                    r#"
                    INSERT INTO provider_account_configs (
                      provider_id, account_id, config_json, updated_at
                    ) VALUES ('xbox', '2533274791234567', NULL, ?1)
                    "#,
                    params![now_iso()],
                )
                .expect("insert direct xbox xuid");

            assert_eq!(
                read_xbox_account_config(&connection).expect("read direct xbox xuid"),
                Some("2533274791234567".to_string())
            );

            connection
                .execute("DELETE FROM provider_account_configs", [])
                .expect("clear xbox config");
            connection
                .execute(
                    r#"
                    INSERT INTO provider_account_configs (
                      provider_id, account_id, config_json, updated_at
                    ) VALUES ('xbox', NULL, '{"xuid":"2533274791234568"}', ?1)
                    "#,
                    params![now_iso()],
                )
                .expect("insert json xbox xuid");

            assert_eq!(
                read_xbox_account_config(&connection).expect("read json xbox xuid"),
                Some("2533274791234568".to_string())
            );
        }

        #[test]
        fn sync_steam_account_games_imports_remote_games_as_not_installed() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: "413150".to_string(),
                title: "Stardew Valley".to_string(),
                playtime_forever: Some(321),
            }];

            let summary =
                sync_steam_account_games(&mut connection, "76561198000000000", &remote_games)
                    .expect("sync remote steam account");
            let entries = list_library_entries(&connection).expect("list entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].primary_platform_id, "steam");
            assert_eq!(entries[0].install_status, "not_installed");
            assert_eq!(entries[0].game.sources[0].external_id, "413150");
            assert_eq!(
                entries[0].game.sources[0].account_id,
                Some("76561198000000000".to_string())
            );
            assert!(!entries[0].game.installed);
            assert_eq!(entries[0].game.playtime.total_minutes, 321);
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                "steam://rungameid/413150"
            );
            assert_eq!(entries[0].game.launch_actions[0].working_directory, None);
        }

        #[test]
        fn sync_steam_account_games_preserves_locally_installed_steam_entry() {
            let steam_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-remote-preserve-{}",
                timestamp_millis()
            ));
            let steamapps = steam_root.join("steamapps");
            let common = steamapps.join("common").join("Portal 2");
            let manifest = steamapps.join("appmanifest_620.acf");

            std::fs::create_dir_all(&common).expect("create steam library");
            std::fs::write(
                &manifest,
                r#""AppState"
                {
                  "appid" "620"
                  "name" "Portal 2"
                  "installdir" "Portal 2"
                }
                "#,
            )
            .expect("write steam manifest");

            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                .expect("sync local steam manifest");

            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: "620".to_string(),
                title: "Portal 2".to_string(),
                playtime_forever: Some(45),
            }];
            let summary =
                sync_steam_account_games(&mut connection, "76561198000000000", &remote_games)
                    .expect("sync remote steam account");
            let entries = list_library_entries(&connection).expect("list entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.updated, 1);
            assert_eq!(entries.len(), 1);
            assert!(entries[0].game.installed);
            assert_eq!(entries[0].install_status, "installed");
            assert_eq!(
                entries[0].game.sources[0].account_id,
                Some("76561198000000000".to_string())
            );
            assert_eq!(entries[0].game.playtime.total_minutes, 45);
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                "steam://rungameid/620"
            );
            assert_eq!(
                entries[0].game.launch_actions[0].working_directory,
                Some(common.to_string_lossy().to_string())
            );

            let _ = std::fs::remove_dir_all(steam_root);
        }

        #[test]
        fn sync_steam_account_games_is_idempotent_for_same_remote_library() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: "413150".to_string(),
                title: "Stardew Valley".to_string(),
                playtime_forever: Some(321),
            }];

            let first_summary =
                sync_steam_account_games(&mut connection, "76561198000000000", &remote_games)
                    .expect("first sync");
            let second_summary =
                sync_steam_account_games(&mut connection, "76561198000000000", &remote_games)
                    .expect("second sync");
            let entries = list_library_entries(&connection).expect("list entries");

            assert_eq!(first_summary.discovered, 1);
            assert_eq!(first_summary.inserted, 1);
            assert_eq!(second_summary.discovered, 1);
            assert_eq!(second_summary.inserted, 0);
            assert_eq!(second_summary.updated, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Stardew Valley");
            assert_eq!(entries[0].game.playtime.total_minutes, 321);
        }

        #[test]
        fn sync_steam_account_games_updates_metadata_without_losing_account() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            save_verified_steam_account_config(&mut connection, "76561198000000000")
                .expect("save steam account");

            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: "413150".to_string(),
                title: "Stardew Valley".to_string(),
                playtime_forever: Some(321),
            }];
            let summary =
                sync_steam_account_games(&mut connection, "76561198000000000", &remote_games)
                    .expect("sync remote steam account");

            record_steam_account_sync_metadata(
                &mut connection,
                "76561198000000000",
                remote_games.len(),
                &summary,
            )
            .expect("record metadata");

            let config_json: String = connection
                .query_row(
                    "SELECT config_json FROM provider_account_configs WHERE provider_id = 'steam'",
                    [],
                    |row| row.get(0),
                )
                .expect("read config json");

            assert!(config_json.contains("\"steamId64\":\"76561198000000000\""));
            assert!(config_json.contains("\"lastOwnedGamesCount\":1"));
            assert!(config_json.contains("\"lastOwnedGamesSummary\""));
        }

        #[test]
        fn sync_steam_account_games_handles_empty_remote_library() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            let summary = sync_steam_account_games(&mut connection, "76561198000000000", &[])
                .expect("sync empty remote library");
            let entries = list_library_entries(&connection).expect("list entries");

            assert_eq!(summary.discovered, 0);
            assert_eq!(summary.inserted, 0);
            assert_eq!(summary.updated, 0);
            assert!(entries.is_empty());
        }

        #[test]
        fn sync_local_games_imports_executable_once() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-root-{}",
                timestamp_millis()
            ));
            let game_dir = root.join("Nightfall");
            let executable = game_dir.join("Nightfall.exe");
            std::fs::create_dir_all(&game_dir).expect("create local game dir");
            std::fs::write(&executable, b"fake exe").expect("create fake exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-sync-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].primary_platform_id, "local");
            assert_eq!(entries[0].game.title, "Nightfall");
            assert_eq!(entries[0].game.launch_actions[0].kind, "executable");
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                executable.to_string_lossy()
            );

            let second_summary =
                sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                    .expect("resync local games");
            let entries_again =
                list_library_entries(&connection).expect("list local entries again");

            assert_eq!(second_summary.discovered, 1);
            assert_eq!(second_summary.inserted, 0);
            assert_eq!(second_summary.updated, 1);
            assert_eq!(second_summary.archived, 0);
            assert_eq!(entries_again.len(), 1);

            let _ = std::fs::remove_file(executable);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_ignores_generic_launcher_title() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-launcher-root-{}",
                timestamp_millis()
            ));
            let launcher_dir = root.join("launcher");
            let executable = launcher_dir.join("launcher.exe");
            std::fs::create_dir_all(&launcher_dir).expect("create launcher dir");
            std::fs::write(&executable, b"fake exe").expect("create launcher exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-launcher-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 0);
            assert_eq!(summary.inserted, 0);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert!(entries.is_empty());

            let _ = std::fs::remove_file(executable);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_keeps_legit_game_with_launcher_executable() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-legit-launcher-root-{}",
                timestamp_millis()
            ));
            let game_dir = root.join("Silksong");
            let executable = game_dir.join("Launcher.exe");

            std::fs::create_dir_all(&game_dir).expect("create game dir");
            std::fs::write(&executable, b"fake exe").expect("create launcher exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-legit-launcher-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Silksong");
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                executable.to_string_lossy()
            );

            let _ = std::fs::remove_file(executable);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_skips_staging_helper_directories() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-helper-regression-root-{}",
                timestamp_millis()
            ));
            let staging_dir = root.join("_staging");
            let staging_exe = staging_dir.join("Build.exe");
            let game_dir = root.join("Skybound");
            let game_exe = game_dir.join("Skybound.exe");

            std::fs::create_dir_all(&staging_dir).expect("create staging dir");
            std::fs::create_dir_all(&game_dir).expect("create game dir");
            std::fs::write(&staging_exe, b"fake exe").expect("create staging exe");
            std::fs::write(&game_exe, b"fake exe").expect("create game exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-helper-regression-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Skybound");
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                game_exe.to_string_lossy()
            );

            let _ = std::fs::remove_file(staging_exe);
            let _ = std::fs::remove_file(game_exe);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_skips_helper_directories() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-helper-root-{}",
                timestamp_millis()
            ));
            let directx_dir = root.join("DirectX");
            let directx_exe = directx_dir.join("DirectXSetup.exe");
            let eos_dir = root.join("EpicOnlineServices");
            let eos_exe = eos_dir.join("EpicOnlineServices.exe");
            let game_dir = root.join("Skybound");
            let game_exe = game_dir.join("Skybound.exe");

            std::fs::create_dir_all(&directx_dir).expect("create directx dir");
            std::fs::create_dir_all(&eos_dir).expect("create eos dir");
            std::fs::create_dir_all(&game_dir).expect("create game dir");
            std::fs::write(&directx_exe, b"fake exe").expect("create directx exe");
            std::fs::write(&eos_exe, b"fake exe").expect("create eos exe");
            std::fs::write(&game_exe, b"fake exe").expect("create game exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-helper-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Skybound");

            let _ = std::fs::remove_file(directx_exe);
            let _ = std::fs::remove_file(eos_exe);
            let _ = std::fs::remove_file(game_exe);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_does_not_import_only_runtime_helpers() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-only-helpers-root-{}",
                timestamp_millis()
            ));
            let directx_dir = root.join("_CommonRedist").join("DirectX").join("Jun2010");
            let eos_dir = root.join("EpicOnlineServices");
            let directx_exe = directx_dir.join("DXSETUP.exe");
            let eos_exe = eos_dir.join("EpicOnlineServicesInstaller.exe");

            std::fs::create_dir_all(&directx_dir).expect("create directx dir");
            std::fs::create_dir_all(&eos_dir).expect("create eos dir");
            std::fs::write(&directx_exe, b"fake exe").expect("create directx exe");
            std::fs::write(&eos_exe, b"fake exe").expect("create eos exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-only-helpers-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 0);
            assert_eq!(summary.inserted, 0);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert!(entries.is_empty());

            let _ = std::fs::remove_file(directx_exe);
            let _ = std::fs::remove_file(eos_exe);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_archives_previously_imported_runtime_helpers() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-archive-helper-root-{}",
                timestamp_millis()
            ));
            let directx_dir = root.join("_CommonRedist").join("DirectX");
            let directx_exe = directx_dir.join("DXSETUP.exe");

            std::fs::create_dir_all(&directx_dir).expect("create directx dir");
            std::fs::write(&directx_exe, b"fake exe").expect("create directx exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-archive-helper-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");
            let source_external_id = normalize_path_string(&directx_exe);
            let helper_candidate = LocalGameCandidate {
                source_external_id,
                title: "DXSETUP".to_string(),
                launch_target: directx_exe.to_string_lossy().to_string(),
                source_id: "source-local-dxsetup-test".to_string(),
                game_id: "game-local-dxsetup-test".to_string(),
                entry_id: "entry-local-dxsetup-test".to_string(),
                launch_id: "launch-local-dxsetup-test".to_string(),
                accent_color: "#0d9488",
            };

            {
                let transaction = connection.transaction().expect("start transaction");
                insert_local_entry(&transaction, &helper_candidate).expect("insert helper entry");
                transaction.commit().expect("commit helper entry");
            }

            let entries_before =
                list_library_entries(&connection).expect("list helper entry before cleanup");
            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries_after =
                list_library_entries(&connection).expect("list helper entry after cleanup");

            assert_eq!(entries_before.len(), 1);
            assert_eq!(summary.discovered, 0);
            assert_eq!(summary.inserted, 0);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 1);
            assert!(entries_after.is_empty());

            let _ = std::fs::remove_file(directx_exe);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn open_database_archives_previously_imported_runtime_helpers() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-boot-cleanup-root-{}",
                timestamp_millis()
            ));
            let directx_dir = root.join("_CommonRedist").join("DirectX");
            let directx_exe = directx_dir.join("DXSETUP.exe");

            std::fs::create_dir_all(&directx_dir).expect("create directx dir");
            std::fs::write(&directx_exe, b"fake exe").expect("create directx exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-boot-cleanup-db-{}.sqlite3",
                timestamp_millis()
            ));
            let helper_candidate = LocalGameCandidate {
                source_external_id: normalize_path_string(&directx_exe),
                title: "DXSETUP".to_string(),
                launch_target: directx_exe.to_string_lossy().to_string(),
                source_id: "source-local-dxsetup-boot-test".to_string(),
                game_id: "game-local-dxsetup-boot-test".to_string(),
                entry_id: "entry-local-dxsetup-boot-test".to_string(),
                launch_id: "launch-local-dxsetup-boot-test".to_string(),
                accent_color: "#0d9488",
            };

            {
                let mut connection = open_database(&path).expect("open empty database");
                let transaction = connection.transaction().expect("start transaction");
                insert_local_entry(&transaction, &helper_candidate).expect("insert helper entry");
                transaction.commit().expect("commit helper entry");
                let entries_before =
                    list_library_entries(&connection).expect("list helper entry before reopen");
                assert_eq!(entries_before.len(), 1);
            }

            {
                let connection = open_database(&path).expect("reopen database");
                let entries_after =
                    list_library_entries(&connection).expect("list helper entry after reopen");
                assert!(entries_after.is_empty());
            }

            let _ = std::fs::remove_file(directx_exe);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_finds_nested_game_executable() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-nested-root-{}",
                timestamp_millis()
            ));
            let game_dir = root.join("Deep Space");
            let executable_dir = game_dir.join("DeepSpace").join("Binaries").join("Win64");
            let executable = executable_dir.join("DeepSpace-Win64-Shipping.exe");

            std::fs::create_dir_all(&executable_dir).expect("create nested game dir");
            std::fs::write(&executable, b"fake exe").expect("create nested game exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-nested-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.archived, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Deep Space");
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                executable.to_string_lossy()
            );

            let _ = std::fs::remove_file(executable);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_preserves_manual_entries() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-preserve-{}",
                timestamp_millis()
            ));
            let game_dir = root.join("IndieGame");
            let executable = game_dir.join("IndieGame.exe");
            std::fs::create_dir_all(&game_dir).expect("create local game dir");
            std::fs::write(&executable, b"fake exe").expect("create fake exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-preserve-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            add_manual_game(
                &mut connection,
                ManualGameInput {
                    title: "Manual Preservado".to_string(),
                    genre: Some("Teste".to_string()),
                    install_status: "installed".to_string(),
                    launch_target: Some("C:\\Games\\Manual\\Manual.exe".to_string()),
                },
            )
            .expect("add manual game");
            sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");

            let entries = list_library_entries(&connection).expect("list unified library");

            assert!(entries
                .iter()
                .any(|entry| entry.primary_platform_id == "manual"));
            assert!(entries
                .iter()
                .any(|entry| entry.primary_platform_id == "local"));

            let _ = std::fs::remove_file(executable);
            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn open_database_upgrades_legacy_library_schema() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-legacy-{}.sqlite3",
                timestamp_millis()
            ));

            {
                let connection = Connection::open(&path).expect("open legacy database");
                connection
                    .execute_batch(
                        r#"
                        CREATE TABLE schema_migrations (
                          version INTEGER PRIMARY KEY,
                          applied_at TEXT NOT NULL
                        );

                        CREATE TABLE games (
                          id TEXT PRIMARY KEY,
                          title TEXT NOT NULL,
                          sort_title TEXT NOT NULL,
                          installed INTEGER NOT NULL DEFAULT 0,
                          playtime_total_minutes INTEGER NOT NULL DEFAULT 0,
                          accent_color TEXT,
                          created_at TEXT NOT NULL,
                          updated_at TEXT NOT NULL
                        );

                        CREATE TABLE library_entries (
                          id TEXT PRIMARY KEY,
                          game_id TEXT NOT NULL UNIQUE,
                          primary_platform_id TEXT NOT NULL,
                          install_status TEXT NOT NULL,
                          last_played_label TEXT NOT NULL,
                          added_at TEXT NOT NULL,
                          updated_at TEXT NOT NULL,
                          FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
                        );

                        CREATE TABLE game_sources (
                          id TEXT PRIMARY KEY,
                          game_id TEXT NOT NULL,
                          platform_id TEXT NOT NULL,
                          external_id TEXT NOT NULL,
                          account_id TEXT,
                          FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
                          UNIQUE (platform_id, external_id)
                        );

                        CREATE TABLE launch_actions (
                          id TEXT PRIMARY KEY,
                          game_id TEXT NOT NULL,
                          platform_id TEXT NOT NULL,
                          kind TEXT NOT NULL,
                          label TEXT NOT NULL,
                          target TEXT NOT NULL,
                          arguments_json TEXT,
                          working_directory TEXT,
                          is_primary INTEGER NOT NULL DEFAULT 0,
                          FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
                        );

                        CREATE TABLE game_genres (
                          game_id TEXT NOT NULL,
                          genre TEXT NOT NULL,
                          position INTEGER NOT NULL DEFAULT 0,
                          FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
                          PRIMARY KEY (game_id, genre)
                        );
                        "#,
                    )
                    .expect("create legacy schema");
            }

            let mut connection = open_database(&path).expect("upgrade legacy database");
            seed_mock_library(&mut connection).expect("seed legacy database");
            let entries = list_library_entries(&connection).expect("list upgraded entries");

            assert_eq!(entries.len(), 4);
            assert!(
                table_has_column(&connection, "library_entries", "is_archived")
                    .expect("check archived column")
            );

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn set_library_entry_archived_toggles_visibility() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-archive-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_seeded_database(&path);

            set_library_entry_archived(&mut connection, "entry-manual-silksong", true)
                .expect("archive entry");
            let archived_entries = list_library_entries(&connection).expect("list archived state");

            assert_eq!(archived_entries.len(), 3);
            assert!(!archived_entries
                .iter()
                .any(|entry| entry.id == "entry-manual-silksong"));

            set_library_entry_archived(&mut connection, "entry-manual-silksong", false)
                .expect("restore entry");
            let restored_entries = list_library_entries(&connection).expect("list restored state");

            assert_eq!(restored_entries.len(), 4);
            assert!(restored_entries
                .iter()
                .any(|entry| entry.id == "entry-manual-silksong"));

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn add_manual_game_persists_entry_with_launch_action() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            let entry = add_manual_game(
                &mut connection,
                ManualGameInput {
                    title: "Teste Persistente".to_string(),
                    genre: Some("Teste".to_string()),
                    install_status: "installed".to_string(),
                    launch_target: Some("C:\\Games\\Teste\\game.exe".to_string()),
                },
            )
            .expect("add manual game");
            let entries = list_manual_games(&connection).expect("list manual games");

            assert_eq!(entries.len(), 1);
            assert_eq!(entry.game.title, "Teste Persistente");
            assert_eq!(entries[0].game.title, "Teste Persistente");
            assert_eq!(entries[0].install_status, "installed");
            assert_eq!(entries[0].game.genres, vec!["Teste"]);
            assert_eq!(entries[0].game.launch_actions[0].kind, "executable");
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                "C:\\Games\\Teste\\game.exe"
            );
        }

        #[test]
        fn list_library_entries_returns_seed_and_manual_games() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-library-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_seeded_database(&path);

            add_manual_game(
                &mut connection,
                ManualGameInput {
                    title: "Jogo Manual Novo".to_string(),
                    genre: Some("Teste".to_string()),
                    install_status: "installed".to_string(),
                    launch_target: Some("steam://rungameid/123".to_string()),
                },
            )
            .expect("add manual game");
            let entries = list_library_entries(&connection).expect("list unified library");

            assert_eq!(entries.len(), 5);
            assert!(entries
                .iter()
                .any(|entry| entry.primary_platform_id == "steam"));
            assert!(entries
                .iter()
                .any(|entry| entry.primary_platform_id == "local"));
            assert!(entries
                .iter()
                .any(|entry| entry.game.title == "Jogo Manual Novo"));

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn add_manual_game_detects_uri_launch_action() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            add_manual_game(
                &mut connection,
                ManualGameInput {
                    title: "Steam Manual".to_string(),
                    genre: Some("Teste".to_string()),
                    install_status: "not_installed".to_string(),
                    launch_target: Some("steam://rungameid/1030300".to_string()),
                },
            )
            .expect("add manual game");
            let entries = list_manual_games(&connection).expect("list manual games");

            assert_eq!(entries[0].game.launch_actions[0].kind, "uri");
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                "steam://rungameid/1030300"
            );
        }

        #[test]
        fn update_manual_game_updates_existing_entry() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-update-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_seeded_database(&path);

            let entry = update_manual_game(
                &mut connection,
                "entry-manual-silksong",
                ManualGameInput {
                    title: "Hollow Knight: Silksong Deluxe".to_string(),
                    genre: Some("Metroidvania".to_string()),
                    install_status: "installed".to_string(),
                    launch_target: Some("C:\\Games\\Silksong\\Launcher.exe".to_string()),
                },
            )
            .expect("update manual game");
            let entries = list_library_entries(&connection).expect("list unified library");

            assert_eq!(entry.game.title, "Hollow Knight: Silksong Deluxe");
            assert_eq!(entry.install_status, "installed");
            assert_eq!(entry.game.genres, vec!["Metroidvania"]);
            assert_eq!(
                entry.game.launch_actions[0].target,
                "C:\\Games\\Silksong\\Launcher.exe"
            );
            assert!(entries
                .iter()
                .any(|library_entry| library_entry.game.title == "Hollow Knight: Silksong Deluxe"));

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn update_manual_game_preserves_archived_state() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-update-archive-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_seeded_database(&path);

            set_library_entry_archived(&mut connection, "entry-manual-silksong", true)
                .expect("archive entry");
            update_manual_game(
                &mut connection,
                "entry-manual-silksong",
                ManualGameInput {
                    title: "Silksong Archivado".to_string(),
                    genre: Some("Metroidvania".to_string()),
                    install_status: "not_installed".to_string(),
                    launch_target: Some("steam://rungameid/1030300".to_string()),
                },
            )
            .expect("update archived entry");

            let stored_entry = find_entry(&connection, "entry-manual-silksong")
                .expect("read stored entry")
                .expect("stored entry exists");
            let entries = list_library_entries(&connection).expect("list visible entries");

            assert!(stored_entry.is_archived);
            assert!(!entries
                .iter()
                .any(|library_entry| library_entry.id == "entry-manual-silksong"));

            let _ = std::fs::remove_file(path);
        }
    }
}
