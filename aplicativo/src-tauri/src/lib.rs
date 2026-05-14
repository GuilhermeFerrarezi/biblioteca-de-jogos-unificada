use tauri::{Emitter, Manager};

const LIBRARY_BOOTSTRAP_COMPLETE_EVENT: &str = "library-bootstrap-complete";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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

            app.manage(AppState {
                connection: std::sync::Arc::clone(&connection),
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
            commands::sync_steam_account_games,
            commands::list_steam_account_config,
            commands::save_steam_account_config,
            commands::disconnect_steam_account_config,
            commands::get_steam_api_key_status,
            commands::save_steam_api_key,
            commands::delete_steam_api_key,
            commands::set_library_entry_archived,
            commands::launch_library_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct AppState {
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
}

fn bootstrap_library(
    handle: tauri::AppHandle,
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
) {
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

mod auth_vault {
    use serde::{Deserialize, Serialize};
    use std::fmt;

    const KEYRING_SERVICE: &str = "com.bibliotecajogos.unificada";
    const STEAM_API_KEY_ACCOUNT: &str = "steam-web-api-key";
    const STEAM_PROVIDER_ID: &str = "steam";
    const STEAM_API_KEY_KIND: &str = "steam_web_api_key";

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct SteamApiKeyInput {
        api_key: String,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamApiKeyStatusDto {
        provider_id: String,
        secret_kind: String,
        is_configured: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum AuthVaultError {
        InvalidSteamApiKey,
        SecureStorageUnavailable,
    }

    impl fmt::Display for AuthVaultError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                AuthVaultError::InvalidSteamApiKey => {
                    write!(formatter, "Steam Web API key invalida.")
                }
                AuthVaultError::SecureStorageUnavailable => {
                    write!(formatter, "Cofre seguro indisponivel para a chave Steam.")
                }
            }
        }
    }

    pub trait SecretVault {
        fn has_secret(&self, account: &str) -> Result<bool, AuthVaultError>;
        fn get_secret(&self, account: &str) -> Result<Option<String>, AuthVaultError>;
        fn set_secret(&self, account: &str, secret: &str) -> Result<(), AuthVaultError>;
        fn delete_secret(&self, account: &str) -> Result<(), AuthVaultError>;
    }

    #[derive(Default)]
    pub struct SystemSecretVault;

    impl SecretVault for SystemSecretVault {
        fn has_secret(&self, account: &str) -> Result<bool, AuthVaultError> {
            let entry = keyring::Entry::new(KEYRING_SERVICE, account)
                .map_err(|_| AuthVaultError::SecureStorageUnavailable)?;

            match entry.get_password() {
                Ok(_) => Ok(true),
                Err(keyring::Error::NoEntry) => Ok(false),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable),
            }
        }

        fn get_secret(&self, account: &str) -> Result<Option<String>, AuthVaultError> {
            let entry = keyring::Entry::new(KEYRING_SERVICE, account)
                .map_err(|_| AuthVaultError::SecureStorageUnavailable)?;

            match entry.get_password() {
                Ok(secret) => Ok(Some(secret)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable),
            }
        }

        fn set_secret(&self, account: &str, secret: &str) -> Result<(), AuthVaultError> {
            let entry = keyring::Entry::new(KEYRING_SERVICE, account)
                .map_err(|_| AuthVaultError::SecureStorageUnavailable)?;

            entry
                .set_password(secret)
                .map_err(|_| AuthVaultError::SecureStorageUnavailable)
        }

        fn delete_secret(&self, account: &str) -> Result<(), AuthVaultError> {
            let entry = keyring::Entry::new(KEYRING_SERVICE, account)
                .map_err(|_| AuthVaultError::SecureStorageUnavailable)?;

            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable),
            }
        }
    }

    pub fn get_steam_api_key_status<Vault: SecretVault>(
        vault: &Vault,
    ) -> Result<SteamApiKeyStatusDto, AuthVaultError> {
        Ok(steam_api_key_status(
            vault.has_secret(STEAM_API_KEY_ACCOUNT)?,
        ))
    }

    pub fn get_steam_api_key<Vault: SecretVault>(
        vault: &Vault,
    ) -> Result<Option<String>, AuthVaultError> {
        vault.get_secret(STEAM_API_KEY_ACCOUNT)
    }

    pub fn save_steam_api_key<Vault: SecretVault>(
        vault: &Vault,
        input: SteamApiKeyInput,
    ) -> Result<SteamApiKeyStatusDto, AuthVaultError> {
        let api_key = normalize_steam_api_key(input.api_key)?;

        vault.set_secret(STEAM_API_KEY_ACCOUNT, &api_key)?;
        if vault.get_secret(STEAM_API_KEY_ACCOUNT)? != Some(api_key) {
            return Err(AuthVaultError::SecureStorageUnavailable);
        }

        Ok(steam_api_key_status(true))
    }

    pub fn delete_steam_api_key<Vault: SecretVault>(
        vault: &Vault,
    ) -> Result<SteamApiKeyStatusDto, AuthVaultError> {
        vault.delete_secret(STEAM_API_KEY_ACCOUNT)?;
        Ok(steam_api_key_status(false))
    }

    fn steam_api_key_status(is_configured: bool) -> SteamApiKeyStatusDto {
        SteamApiKeyStatusDto {
            provider_id: STEAM_PROVIDER_ID.to_string(),
            secret_kind: STEAM_API_KEY_KIND.to_string(),
            is_configured,
        }
    }

    fn normalize_steam_api_key(value: String) -> Result<String, AuthVaultError> {
        let api_key = value.trim();

        if api_key.len() == 32 && api_key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(api_key.to_ascii_uppercase())
        } else {
            Err(AuthVaultError::InvalidSteamApiKey)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::Mutex;

        #[derive(Default)]
        struct FakeSecretVault {
            secret: Mutex<Option<String>>,
        }

        impl FakeSecretVault {
            fn stored_secret(&self) -> Option<String> {
                self.secret.lock().expect("lock fake vault").clone()
            }
        }

        impl SecretVault for FakeSecretVault {
            fn has_secret(&self, _account: &str) -> Result<bool, AuthVaultError> {
                Ok(self.secret.lock().expect("lock fake vault").is_some())
            }

            fn get_secret(&self, _account: &str) -> Result<Option<String>, AuthVaultError> {
                Ok(self.secret.lock().expect("lock fake vault").clone())
            }

            fn set_secret(&self, _account: &str, secret: &str) -> Result<(), AuthVaultError> {
                *self.secret.lock().expect("lock fake vault") = Some(secret.to_string());
                Ok(())
            }

            fn delete_secret(&self, _account: &str) -> Result<(), AuthVaultError> {
                *self.secret.lock().expect("lock fake vault") = None;
                Ok(())
            }
        }

        #[test]
        fn steam_api_key_status_starts_unconfigured() {
            let vault = FakeSecretVault::default();
            let status = get_steam_api_key_status(&vault).expect("get status");

            assert_eq!(status.provider_id, "steam");
            assert_eq!(status.secret_kind, "steam_web_api_key");
            assert!(!status.is_configured);
        }

        #[test]
        fn save_steam_api_key_stores_normalized_secret_without_returning_it() {
            let vault = FakeSecretVault::default();
            let status = save_steam_api_key(
                &vault,
                SteamApiKeyInput {
                    api_key: " 0123456789abcdef0123456789abcdef ".to_string(),
                },
            )
            .expect("save api key");

            assert!(status.is_configured);
            assert_eq!(
                vault.stored_secret(),
                Some("0123456789ABCDEF0123456789ABCDEF".to_string())
            );
        }

        #[test]
        fn save_steam_api_key_rejects_invalid_secret() {
            let vault = FakeSecretVault::default();
            let result = save_steam_api_key(
                &vault,
                SteamApiKeyInput {
                    api_key: "not-a-valid-key".to_string(),
                },
            );

            assert!(matches!(result, Err(AuthVaultError::InvalidSteamApiKey)));
            assert_eq!(vault.stored_secret(), None);
        }

        #[test]
        fn delete_steam_api_key_clears_secret() {
            let vault = FakeSecretVault::default();
            save_steam_api_key(
                &vault,
                SteamApiKeyInput {
                    api_key: "0123456789abcdef0123456789abcdef".to_string(),
                },
            )
            .expect("save api key");

            let status = delete_steam_api_key(&vault).expect("delete api key");

            assert!(!status.is_configured);
            assert_eq!(vault.stored_secret(), None);
        }
    }
}

mod steam_web_api {
    use serde::Deserialize;
    use std::fmt;

    const GET_OWNED_GAMES_URL: &str =
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/";

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SteamOwnedGame {
        pub app_id: String,
        pub title: String,
        pub playtime_total_minutes: i64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SteamWebApiError {
        AccessDenied,
        RateLimited,
        RequestFailed,
        InvalidResponse,
    }

    impl fmt::Display for SteamWebApiError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SteamWebApiError::AccessDenied => {
                    write!(
                        formatter,
                        "A Steam recusou a consulta. Verifique a chave Web API, o SteamID64 e a visibilidade da biblioteca."
                    )
                }
                SteamWebApiError::RateLimited => {
                    write!(
                        formatter,
                        "A Steam limitou temporariamente as consultas. Aguarde alguns minutos e tente novamente."
                    )
                }
                SteamWebApiError::RequestFailed => {
                    write!(formatter, "Nao foi possivel consultar a Steam Web API.")
                }
                SteamWebApiError::InvalidResponse => {
                    write!(formatter, "A Steam Web API retornou uma resposta invalida.")
                }
            }
        }
    }

    pub trait SteamWebApiClient {
        fn get_owned_games(
            &self,
            api_key: &str,
            steam_id64: &str,
        ) -> Result<SteamOwnedGamesResponse, SteamWebApiError>;
    }

    #[derive(Default)]
    pub struct UreqSteamWebApiClient;

    impl SteamWebApiClient for UreqSteamWebApiClient {
        fn get_owned_games(
            &self,
            api_key: &str,
            steam_id64: &str,
        ) -> Result<SteamOwnedGamesResponse, SteamWebApiError> {
            ureq::get(GET_OWNED_GAMES_URL)
                .query("key", api_key)
                .query("steamid", steam_id64)
                .query("include_appinfo", "1")
                .query("include_played_free_games", "1")
                .query("format", "json")
                .call()
                .map_err(|error| match error {
                    ureq::Error::Status(401 | 403, _) => SteamWebApiError::AccessDenied,
                    ureq::Error::Status(429, _) => SteamWebApiError::RateLimited,
                    _ => SteamWebApiError::RequestFailed,
                })?
                .into_json::<SteamOwnedGamesResponse>()
                .map_err(|_| SteamWebApiError::InvalidResponse)
        }
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamOwnedGamesResponse {
        response: SteamOwnedGamesPayload,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamOwnedGamesPayload {
        games: Option<Vec<SteamOwnedGamePayload>>,
    }

    #[derive(Clone, Debug, Deserialize)]
    struct SteamOwnedGamePayload {
        appid: u64,
        name: Option<String>,
        playtime_forever: Option<i64>,
    }

    pub fn fetch_owned_games<Client: SteamWebApiClient>(
        client: &Client,
        api_key: &str,
        steam_id64: &str,
    ) -> Result<Vec<SteamOwnedGame>, SteamWebApiError> {
        let response = client.get_owned_games(api_key, steam_id64)?;
        let games = response
            .response
            .games
            .unwrap_or_default()
            .into_iter()
            .filter_map(|game| {
                let title = game.name?.trim().to_string();

                if title.is_empty() {
                    return None;
                }

                Some(SteamOwnedGame {
                    app_id: game.appid.to_string(),
                    title,
                    playtime_total_minutes: game.playtime_forever.unwrap_or(0).max(0),
                })
            })
            .collect::<Vec<_>>();

        Ok(games)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        struct FakeSteamWebApiClient {
            response: SteamOwnedGamesResponse,
        }

        impl SteamWebApiClient for FakeSteamWebApiClient {
            fn get_owned_games(
                &self,
                api_key: &str,
                steam_id64: &str,
            ) -> Result<SteamOwnedGamesResponse, SteamWebApiError> {
                assert_eq!(api_key, "SECRET");
                assert_eq!(steam_id64, "76561198000000000");

                Ok(SteamOwnedGamesResponse {
                    response: SteamOwnedGamesPayload {
                        games: self.response.response.games.clone(),
                    },
                })
            }
        }

        #[test]
        fn fetch_owned_games_normalizes_valid_response() {
            let client = FakeSteamWebApiClient {
                response: SteamOwnedGamesResponse {
                    response: SteamOwnedGamesPayload {
                        games: Some(vec![
                            SteamOwnedGamePayload {
                                appid: 413150,
                                name: Some("Stardew Valley".to_string()),
                                playtime_forever: Some(120),
                            },
                            SteamOwnedGamePayload {
                                appid: 10,
                                name: None,
                                playtime_forever: Some(1),
                            },
                        ]),
                    },
                },
            };

            let games =
                fetch_owned_games(&client, "SECRET", "76561198000000000").expect("fetch games");

            assert_eq!(
                games,
                vec![SteamOwnedGame {
                    app_id: "413150".to_string(),
                    title: "Stardew Valley".to_string(),
                    playtime_total_minutes: 120,
                }]
            );
        }
    }
}

mod commands {
    use super::{auth_vault, launcher, steam_web_api, storage, AppState};
    use tauri::State;

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
    pub fn sync_steam_account_games(
        state: State<'_, AppState>,
    ) -> Result<storage::SyncSummaryDto, String> {
        let steam_id64 = {
            let connection = state
                .connection
                .lock()
                .map_err(|_| "failed to lock local database".to_string())?;
            let account_config = storage::list_steam_account_config(&connection)
                .map_err(|error| error.to_string())?;

            account_config
                .steam_id64()
                .ok_or_else(|| {
                    "Configure um SteamID64 antes de sincronizar a conta Steam.".to_string()
                })?
                .to_string()
        };
        let vault = auth_vault::SystemSecretVault;
        let api_key = auth_vault::get_steam_api_key(&vault)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| {
                "Salve a chave Steam Web API no cofre antes de sincronizar a conta Steam."
                    .to_string()
            })?;
        let client = steam_web_api::UreqSteamWebApiClient::default();
        let remote_games = steam_web_api::fetch_owned_games(&client, &api_key, &steam_id64)
            .map_err(|error| error.to_string())?;
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::sync_steam_account_games(&mut connection, remote_games)
            .map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn list_steam_account_config(
        state: State<'_, AppState>,
    ) -> Result<storage::SteamAccountConfigDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::list_steam_account_config(&connection).map_err(|error| error.to_string())
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
    pub fn disconnect_steam_account_config(
        state: State<'_, AppState>,
    ) -> Result<storage::SteamAccountConfigDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;
        let vault = auth_vault::SystemSecretVault;

        auth_vault::delete_steam_api_key(&vault).map_err(|error| error.to_string())?;

        storage::disconnect_steam_account_config(&mut connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn get_steam_api_key_status() -> Result<auth_vault::SteamApiKeyStatusDto, String> {
        let vault = auth_vault::SystemSecretVault;

        auth_vault::get_steam_api_key_status(&vault).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_steam_api_key(
        input: auth_vault::SteamApiKeyInput,
    ) -> Result<auth_vault::SteamApiKeyStatusDto, String> {
        let vault = auth_vault::SystemSecretVault;

        auth_vault::save_steam_api_key(&vault, input).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn delete_steam_api_key() -> Result<auth_vault::SteamApiKeyStatusDto, String> {
        let vault = auth_vault::SystemSecretVault;

        auth_vault::delete_steam_api_key(&vault).map_err(|error| error.to_string())
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
}

mod launcher {
    use rusqlite::{params, Connection, OptionalExtension};
    use serde::Serialize;
    use std::fmt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

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
        NoExecutableAction,
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
                LaunchValidationError::NoExecutableAction => {
                    "Nenhuma acao executavel persistida foi encontrada para este jogo."
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
            .ok_or_else(|| LaunchValidationError::NoExecutableAction.to_string())?;
        let target = validate_executable_path(Path::new(&action.target))
            .map_err(|error| error.to_string())?;
        let working_directory =
            resolve_working_directory(&target, action.working_directory.as_deref())
                .map_err(|error| error.to_string())?;

        Command::new(&target)
            .current_dir(working_directory)
            .spawn()
            .map_err(|_| "Nao foi possivel iniciar o executavel local.".to_string())?;

        Ok(LaunchResultDto {
            started: true,
            message: "Inicializacao do executavel solicitada.".to_string(),
        })
    }

    struct ExecutableAction {
        target: String,
        working_directory: Option<String>,
    }

    fn find_executable_action(
        connection: &Connection,
        entry_id: &str,
    ) -> rusqlite::Result<Option<ExecutableAction>> {
        connection
            .query_row(
                r#"
                SELECT launch_actions.target, launch_actions.working_directory
                FROM library_entries
                JOIN launch_actions ON launch_actions.game_id = library_entries.game_id
                WHERE library_entries.id = ?1
                  AND library_entries.primary_platform_id IN ('manual', 'local')
                  AND library_entries.is_archived = 0
                  AND launch_actions.kind = 'executable'
                  AND launch_actions.is_primary = 1
                "#,
                params![entry_id],
                |row| {
                    Ok(ExecutableAction {
                        target: row.get(0)?,
                        working_directory: row.get(1)?,
                    })
                },
            )
            .optional()
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
        fn resolves_default_working_directory_from_target_parent() {
            let target = std::env::temp_dir().join("biblioteca-jogos-launch-test.exe");

            assert_eq!(
                resolve_working_directory(&target, None).expect("resolve working directory"),
                std::env::temp_dir(),
            );
        }
    }
}

mod storage {
    use super::steam_web_api::SteamOwnedGame;
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
        ensure_provider_account_config_columns(&connection)?;
        ensure_active_entries_index(&connection)?;
        ensure_local_cleanup_indexes(&connection)?;
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
        auth_state TEXT NOT NULL,
        steam_id64 TEXT,
        configured_at TEXT,
        disconnected_at TEXT,
        updated_at TEXT NOT NULL,
        CHECK (provider_id <> 'steam' OR steam_id64 IS NULL OR (length(steam_id64) = 17 AND steam_id64 NOT GLOB '*[^0-9]*')),
        CHECK (auth_state IN ('configured', 'disconnected'))
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
        connection.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (2, ?1)",
            params![now_iso()],
        )?;

        Ok(())
    }

    fn ensure_provider_account_config_columns(connection: &Connection) -> rusqlite::Result<()> {
        if !table_exists(connection, "provider_account_configs")? {
            return Ok(());
        }

        if !table_has_column(connection, "provider_account_configs", "auth_state")? {
            connection.execute(
                "ALTER TABLE provider_account_configs ADD COLUMN auth_state TEXT NOT NULL DEFAULT 'disconnected'",
                [],
            )?;
        }

        if !table_has_column(connection, "provider_account_configs", "configured_at")? {
            connection.execute(
                "ALTER TABLE provider_account_configs ADD COLUMN configured_at TEXT",
                [],
            )?;
        }

        if !table_has_column(connection, "provider_account_configs", "disconnected_at")? {
            connection.execute(
                "ALTER TABLE provider_account_configs ADD COLUMN disconnected_at TEXT",
                [],
            )?;
        }

        Ok(())
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct SteamAccountConfigInput {
        steam_id64: Option<String>,
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
        discovered: usize,
        inserted: usize,
        updated: usize,
        archived: usize,
        unavailable: usize,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamAccountConfigDto {
        provider_id: String,
        auth_state: String,
        steam_id64: Option<String>,
        configured_at: Option<String>,
        disconnected_at: Option<String>,
        updated_at: Option<String>,
    }

    impl SteamAccountConfigDto {
        pub fn steam_id64(&self) -> Option<&str> {
            self.steam_id64.as_deref()
        }
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
            })
        })?;

        rows.map(|row| row.and_then(|entry| hydrate_entry(connection, entry)))
            .collect()
    }

    pub fn list_steam_account_config(
        connection: &Connection,
    ) -> rusqlite::Result<SteamAccountConfigDto> {
        find_steam_account_config(connection)
            .map(|config| config.unwrap_or_else(default_steam_config))
    }

    pub fn save_steam_account_config(
        connection: &mut Connection,
        input: SteamAccountConfigInput,
    ) -> rusqlite::Result<SteamAccountConfigDto> {
        let steam_id64 = normalize_steam_id64(input.steam_id64)?;
        let now = now_iso();
        let transaction = connection.transaction()?;

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id, auth_state, steam_id64, configured_at, disconnected_at, updated_at
            ) VALUES ('steam', 'configured', ?1, ?2, NULL, ?2)
            ON CONFLICT(provider_id) DO UPDATE SET
              auth_state = 'configured',
              steam_id64 = excluded.steam_id64,
              configured_at = COALESCE(provider_account_configs.configured_at, excluded.configured_at),
              disconnected_at = NULL,
              updated_at = excluded.updated_at
            "#,
            params![steam_id64, now],
        )?;
        transaction.commit()?;

        list_steam_account_config(connection)
    }

    pub fn disconnect_steam_account_config(
        connection: &mut Connection,
    ) -> rusqlite::Result<SteamAccountConfigDto> {
        let now = now_iso();
        let transaction = connection.transaction()?;

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id, auth_state, steam_id64, configured_at, disconnected_at, updated_at
            ) VALUES ('steam', 'disconnected', NULL, NULL, ?1, ?1)
            ON CONFLICT(provider_id) DO UPDATE SET
              auth_state = 'disconnected',
              steam_id64 = NULL,
              disconnected_at = excluded.disconnected_at,
              updated_at = excluded.updated_at
            "#,
            params![now],
        )?;
        transaction.commit()?;

        list_steam_account_config(connection)
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

    fn table_exists(connection: &Connection, table_name: &str) -> rusqlite::Result<bool> {
        connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table_name],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
    }

    fn find_steam_account_config(
        connection: &Connection,
    ) -> rusqlite::Result<Option<SteamAccountConfigDto>> {
        connection
            .query_row(
                r#"
                SELECT provider_id, auth_state, steam_id64, configured_at, disconnected_at, updated_at
                FROM provider_account_configs
                WHERE provider_id = 'steam'
                "#,
                [],
                |row| {
                    Ok(SteamAccountConfigDto {
                        provider_id: row.get(0)?,
                        auth_state: row.get(1)?,
                        steam_id64: row.get(2)?,
                        configured_at: row.get(3)?,
                        disconnected_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .optional()
    }

    fn default_steam_config() -> SteamAccountConfigDto {
        SteamAccountConfigDto {
            provider_id: "steam".to_string(),
            auth_state: "disconnected".to_string(),
            steam_id64: None,
            configured_at: None,
            disconnected_at: None,
            updated_at: None,
        }
    }

    fn normalize_steam_id64(value: Option<String>) -> rusqlite::Result<Option<String>> {
        value
            .map(|steam_id64| {
                let steam_id64 = steam_id64.trim();

                if steam_id64.is_empty() {
                    return Ok(None);
                }

                if is_valid_steam_id64(steam_id64) {
                    Ok(Some(steam_id64.to_string()))
                } else {
                    Err(rusqlite::Error::InvalidParameterName(
                        "SteamID64 deve conter 17 digitos validos.".to_string(),
                    ))
                }
            })
            .unwrap_or(Ok(None))
    }

    fn is_valid_steam_id64(value: &str) -> bool {
        const STEAM_ID64_BASE: u64 = 76_561_197_960_265_728;

        value.len() == 17
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && value
                .parse::<u64>()
                .map(|id| id >= STEAM_ID64_BASE)
                .unwrap_or(false)
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
        is_installed: bool,
        playtime_total_minutes: Option<i64>,
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
        let roots = collect_steam_roots();
        sync_steam_games_from_roots(connection, &roots)
    }

    pub fn sync_steam_account_games(
        connection: &mut Connection,
        remote_games: Vec<SteamOwnedGame>,
    ) -> rusqlite::Result<SyncSummaryDto> {
        let candidates = remote_games
            .into_iter()
            .filter(|game| !is_rejected_steam_app(&game.app_id))
            .map(steam_account_game_candidate)
            .collect::<Vec<_>>();

        sync_steam_candidates(connection, candidates, false)
    }

    fn sync_steam_games_from_roots(
        connection: &mut Connection,
        roots: &[PathBuf],
    ) -> rusqlite::Result<SyncSummaryDto> {
        let candidates = discover_steam_game_candidates(roots);
        sync_steam_candidates(connection, candidates, true)
    }

    fn sync_steam_candidates(
        connection: &mut Connection,
        candidates: Vec<SteamGameCandidate>,
        reconcile_missing_as_unavailable: bool,
    ) -> rusqlite::Result<SyncSummaryDto> {
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
            if reconcile_missing_as_unavailable
                && !existing_row.is_archived
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

    fn steam_account_game_candidate(game: SteamOwnedGame) -> SteamGameCandidate {
        let slug = create_slug(&game.title);

        SteamGameCandidate {
            source_id: format!("source-steam-{}", game.app_id),
            game_id: format!("game-steam-{slug}-{}", game.app_id),
            entry_id: format!("entry-steam-{slug}-{}", game.app_id),
            launch_id: format!("launch-steam-{}", game.app_id),
            app_id: game.app_id,
            title: game.title.clone(),
            install_path: None,
            is_installed: false,
            playtime_total_minutes: Some(game.playtime_total_minutes),
            accent_color: deterministic_accent_color(&game.title),
        }
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

    fn collect_steam_roots() -> Vec<PathBuf> {
        if let Some(raw_roots) = std::env::var_os("BIBLIOTECA_JOGOS_STEAM_ROOTS") {
            let roots = raw_roots
                .to_string_lossy()
                .split(';')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .filter(|root| root.exists())
                .collect::<Vec<_>>();

            if !roots.is_empty() {
                return roots;
            }
        }

        let mut roots = Vec::new();

        if let Some(program_files_x86) = std::env::var_os("PROGRAMFILES(X86)") {
            roots.push(PathBuf::from(program_files_x86).join("Steam"));
        }

        if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
            roots.push(PathBuf::from(program_files).join("Steam"));
        }

        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            roots.push(PathBuf::from(local_app_data).join("Steam"));
        }

        roots.into_iter().filter(|root| root.exists()).collect()
    }

    fn discover_local_game_candidates(roots: &[PathBuf]) -> Vec<LocalGameCandidate> {
        let mut candidates = HashMap::new();

        for root in roots {
            for candidate_dir in candidate_directories(root) {
                if let Some(executable_path) = find_local_executable(&candidate_dir) {
                    let normalized_target = normalize_path_string(&executable_path);

                    candidates
                        .entry(normalized_target.clone())
                        .or_insert_with(|| {
                            let title = candidate_title(&candidate_dir, &executable_path);
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
        let default_steamapps = steam_root.join("steamapps");

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
            is_installed: true,
            playtime_total_minutes: None,
            accent_color,
        })
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
        let normalized_path = path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .map(normalize_name)
            .collect::<Vec<_>>()
            .join("/");

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
              games.accent_color
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
              games.accent_color
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
        let install_status = if candidate.is_installed {
            "installed"
        } else {
            "not_installed"
        };
        let playtime_total_minutes = candidate.playtime_total_minutes.unwrap_or(0);
        transaction.execute(
            r#"
            INSERT INTO games (
              id, title, sort_title, installed, playtime_total_minutes, accent_color, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            "#,
            params![
                candidate.game_id,
                candidate.title,
                candidate.title,
                candidate.is_installed as i64,
                playtime_total_minutes,
                candidate.accent_color,
                now,
            ],
        )?;
        transaction.execute(
            r#"
            INSERT INTO library_entries (
              id, game_id, primary_platform_id, install_status, last_played_label, is_archived, added_at, updated_at
            ) VALUES (?1, ?2, 'steam', ?3, 'Nunca', 0, ?4, ?4)
            "#,
            params![candidate.entry_id, candidate.game_id, install_status, now],
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

    fn update_steam_entry(
        transaction: &rusqlite::Transaction<'_>,
        existing_row: &EntryRow,
        candidate: &SteamGameCandidate,
    ) -> rusqlite::Result<bool> {
        let launch_target = format!("steam://rungameid/{}", candidate.app_id);
        let current_action = find_steam_primary_action(transaction, &existing_row.game_id)?;
        let next_working_directory = candidate.install_path.clone().or_else(|| {
            current_action
                .as_ref()
                .and_then(|action| action.working_directory.clone())
        });
        let next_installed = existing_row.installed || candidate.is_installed;
        let next_install_status = if next_installed {
            "installed"
        } else {
            "not_installed"
        };
        let next_playtime_total_minutes = candidate
            .playtime_total_minutes
            .unwrap_or(existing_row.playtime_total_minutes);
        let needs_action_update = current_action
            .as_ref()
            .map(|action| {
                action.kind != "uri"
                    || action.label != launch_target
                    || action.target != launch_target
                    || action.working_directory != next_working_directory
            })
            .unwrap_or(true);
        let needs_entry_update = existing_row.title != candidate.title
            || existing_row.sort_title != candidate.title
            || existing_row.installed != next_installed
            || existing_row.install_status != next_install_status
            || existing_row.playtime_total_minutes != next_playtime_total_minutes
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
                    installed = ?4,
                    playtime_total_minutes = ?5,
                    accent_color = ?6,
                    updated_at = ?7
                WHERE id = ?1
                "#,
                params![
                    existing_row.game_id,
                    candidate.title,
                    candidate.title,
                    next_installed as i64,
                    next_playtime_total_minutes,
                    candidate.accent_color,
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
                params![existing_row.entry_id, next_install_status, updated_at],
            )?;
        }

        if needs_action_update {
            upsert_steam_primary_action(
                transaction,
                &existing_row.game_id,
                &candidate.launch_id,
                &launch_target,
                next_working_directory.as_deref(),
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

        fn table_exists(connection: &Connection, table_name: &str) -> rusqlite::Result<bool> {
            connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table_name],
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
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get(0)
                })
                .expect("read schema version");

            assert_eq!(version, 2);
            assert!(table_exists(&connection, "provider_account_configs")
                .expect("check account config table"));
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
        fn open_database_starts_with_disconnected_steam_account_config() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-config-empty-{}.sqlite3",
                timestamp_millis()
            ));

            let connection = open_database(&path).expect("open empty database");
            let config = list_steam_account_config(&connection).expect("list default steam config");

            assert_eq!(config.provider_id, "steam");
            assert_eq!(config.auth_state, "disconnected");
            assert_eq!(config.steam_id64, None);
            assert_eq!(config.configured_at, None);

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn open_database_upgrades_legacy_provider_account_config_schema() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-legacy-provider-config-{}.sqlite3",
                timestamp_millis()
            ));
            {
                let connection = Connection::open(&path).expect("open legacy database");
                connection
                    .execute_batch(
                        r#"
                        CREATE TABLE provider_account_configs (
                          provider_id TEXT PRIMARY KEY,
                          account_id TEXT,
                          steam_id64 TEXT,
                          config_json TEXT,
                          updated_at TEXT NOT NULL
                        );
                        INSERT INTO provider_account_configs (
                          provider_id, account_id, steam_id64, config_json, updated_at
                        ) VALUES (
                          'steam', NULL, '76561198000000000', '{}', '2026-05-14T00:00:00Z'
                        );
                        "#,
                    )
                    .expect("create legacy provider config table");
            }

            let connection = open_database(&path).expect("upgrade legacy provider config database");
            let config = list_steam_account_config(&connection).expect("list upgraded config");

            assert_eq!(config.auth_state, "disconnected");
            assert_eq!(config.steam_id64, Some("76561198000000000".to_string()));
            assert!(
                table_has_column(&connection, "provider_account_configs", "auth_state")
                    .expect("check auth_state column")
            );
            assert!(
                table_has_column(&connection, "provider_account_configs", "configured_at")
                    .expect("check configured_at column")
            );
            assert!(
                table_has_column(&connection, "provider_account_configs", "disconnected_at")
                    .expect("check disconnected_at column")
            );

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn save_steam_account_config_accepts_valid_steam_id64() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            let config = save_steam_account_config(
                &mut connection,
                SteamAccountConfigInput {
                    steam_id64: Some("76561198000000000".to_string()),
                },
            )
            .expect("save steam config");

            assert_eq!(config.provider_id, "steam");
            assert_eq!(config.auth_state, "configured");
            assert_eq!(config.steam_id64, Some("76561198000000000".to_string()));
            assert!(config.configured_at.is_some());
            assert!(config.disconnected_at.is_none());
            assert!(config.updated_at.is_some());
        }

        #[test]
        fn save_steam_account_config_rejects_invalid_steam_id64() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            let result = save_steam_account_config(
                &mut connection,
                SteamAccountConfigInput {
                    steam_id64: Some("not-a-steam-id".to_string()),
                },
            );
            let config =
                list_steam_account_config(&connection).expect("list steam config after rejection");

            assert!(result.is_err());
            assert_eq!(config.auth_state, "disconnected");
            assert_eq!(config.steam_id64, None);
        }

        #[test]
        fn disconnect_steam_account_config_preserves_steam_library_entries() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-config-disconnect-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_seeded_database(&path);

            save_steam_account_config(
                &mut connection,
                SteamAccountConfigInput {
                    steam_id64: Some("76561198000000000".to_string()),
                },
            )
            .expect("save steam config");
            let before = list_library_entries(&connection).expect("list entries before disconnect");
            let config = disconnect_steam_account_config(&mut connection)
                .expect("disconnect steam account config");
            let after = list_library_entries(&connection).expect("list entries after disconnect");

            assert_eq!(config.auth_state, "disconnected");
            assert_eq!(config.steam_id64, None);
            assert!(config.disconnected_at.is_some());
            assert_eq!(
                before
                    .iter()
                    .filter(|entry| entry.primary_platform_id == "steam")
                    .count(),
                after
                    .iter()
                    .filter(|entry| entry.primary_platform_id == "steam")
                    .count()
            );
            assert!(after
                .iter()
                .any(|entry| entry.primary_platform_id == "steam"));

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
        fn sync_steam_account_games_imports_remote_library_as_not_installed() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-account-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");
            let summary = sync_steam_account_games(
                &mut connection,
                vec![SteamOwnedGame {
                    app_id: "413150".to_string(),
                    title: "Stardew Valley".to_string(),
                    playtime_total_minutes: 480,
                }],
            )
            .expect("sync steam account games");
            let entries = list_library_entries(&connection).expect("list steam account entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(summary.updated, 0);
            assert_eq!(summary.unavailable, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].primary_platform_id, "steam");
            assert_eq!(entries[0].install_status, "not_installed");
            assert!(!entries[0].game.installed);
            assert_eq!(entries[0].game.playtime.total_minutes, 480);
            assert_eq!(
                entries[0].game.launch_actions[0].target,
                "steam://rungameid/413150"
            );

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_steam_account_games_preserves_local_install_state_and_updates_playtime() {
            let steam_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-steam-account-local-root-{}",
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
                "biblioteca-jogos-steam-account-local-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            sync_steam_games_from_roots(&mut connection, std::slice::from_ref(&steam_root))
                .expect("sync local steam games");
            let summary = sync_steam_account_games(
                &mut connection,
                vec![SteamOwnedGame {
                    app_id: "620".to_string(),
                    title: "Portal 2".to_string(),
                    playtime_total_minutes: 240,
                }],
            )
            .expect("sync account steam games");
            let entries = list_library_entries(&connection).expect("list merged steam entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 0);
            assert_eq!(summary.updated, 1);
            assert_eq!(summary.unavailable, 0);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].install_status, "installed");
            assert!(entries[0].game.installed);
            assert_eq!(entries[0].game.playtime.total_minutes, 240);
            assert_eq!(
                entries[0].game.launch_actions[0]
                    .working_directory
                    .as_deref(),
                Some(common.to_string_lossy().as_ref())
            );

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
                is_installed: true,
                playtime_total_minutes: None,
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
