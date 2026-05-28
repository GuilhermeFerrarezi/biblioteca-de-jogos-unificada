#![cfg_attr(test, allow(dead_code))]

#[cfg(not(test))]
use tauri::{webview::PageLoadEvent, Emitter, Manager};

mod events;
mod xbox_live_auth;
mod xbox_provider;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[cfg(not(test))]
pub fn run() {
    let process_started_at = std::time::Instant::now();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let setup_started_at = std::time::Instant::now();
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            log::info!("[library-boot] backend.setup.start");
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
            std::fs::create_dir_all(&app_data_dir)
                .map_err(|error| format!("failed to create app data dir: {error}"))?;
            let db_path = app_data_dir.join("library.sqlite3");
            let database_open_started_at = std::time::Instant::now();
            let connection = storage::open_database(&db_path)
                .map_err(|error| format!("failed to open local database: {error}"))?;
            log::info!(
                "[library-boot] backend.database_open.ready elapsed_ms={}",
                database_open_started_at.elapsed().as_millis()
            );
            let connection = std::sync::Arc::new(std::sync::Mutex::new(connection));
            let auth_vault_dir = app_data_dir.join("auth-vault");
            let auth_vault = std::sync::Arc::new(security::AuthVault::system(auth_vault_dir));

            app.manage(AppState {
                connection: std::sync::Arc::clone(&connection),
                auth_vault,
                steam_account_sync_in_progress: std::sync::Arc::new(
                    std::sync::atomic::AtomicBool::new(false),
                ),
                steam_enrichment_in_progress: std::sync::Arc::new(
                    std::sync::atomic::AtomicBool::new(false),
                ),
            });
            if let Some(main_window) = app.get_webview_window("main") {
                let fallback_window = main_window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(12));
                    if matches!(fallback_window.is_visible(), Ok(false)) {
                        log::warn!(
                            "[library-boot] backend.main_window.fallback_show elapsed_ms={}",
                            process_started_at.elapsed().as_millis()
                        );
                        let _ = fallback_window.show();
                    }
                });
            }
            bootstrap_library(app.handle().clone(), connection);
            log::info!(
                "[library-boot] backend.setup.complete elapsed_ms={}",
                setup_started_at.elapsed().as_millis()
            );
            Ok(())
        })
        .on_page_load(move |webview, payload| {
            let event = match payload.event() {
                PageLoadEvent::Started => "started",
                PageLoadEvent::Finished => "finished",
            };
            log::info!(
                "[library-boot] webview.page_load.{} url={} elapsed_ms={}",
                event,
                payload.url(),
                process_started_at.elapsed().as_millis()
            );

            let window = webview.window();
            if payload.event() == PageLoadEvent::Finished
                && matches!(window.is_visible(), Ok(false))
            {
                log::info!(
                    "[library-boot] webview.page_load.finished_show elapsed_ms={}",
                    process_started_at.elapsed().as_millis()
                );
                let _ = window.show();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_library_entries,
            commands::record_boot_marker,
            commands::list_manual_games,
            commands::add_manual_game,
            commands::update_manual_game,
            commands::sync_local_games,
            commands::sync_steam_games,
            commands::sync_xbox_games,
            commands::sync_xbox_achievement_games,
            commands::import_xbox_achievement_title_history,
            commands::sync_steam_account_games,
            commands::get_steam_enrichment_retry_summary,
            commands::get_steam_account_config,
            commands::get_steam_library_roots,
            commands::start_steam_openid_login,
            commands::start_xbox_live_login,
            commands::get_xbox_live_auth_state,
            commands::get_xbox_live_client_config,
            commands::get_xbox_live_client_secret_state,
            commands::save_steam_account_config,
            commands::save_steam_library_roots,
            commands::get_xbox_library_roots,
            commands::save_xbox_library_roots,
            commands::save_xbox_live_client_config,
            commands::save_xbox_live_client_secret,
            commands::get_library_settings,
            commands::save_library_settings,
            commands::save_steam_web_api_key,
            commands::get_steam_web_api_key_state,
            commands::disconnect_steam_web_api_key,
            commands::set_library_entry_archived,
            commands::set_library_entry_favorite,
            commands::launch_library_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
pub fn run() {}

struct AppState {
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    auth_vault: std::sync::Arc<security::AuthVault>,
    steam_account_sync_in_progress: std::sync::Arc<std::sync::atomic::AtomicBool>,
    steam_enrichment_in_progress: std::sync::Arc<std::sync::atomic::AtomicBool>,
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

#[cfg(not(test))]
fn bootstrap_library(
    handle: tauri::AppHandle,
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
) {
    if !cfg!(debug_assertions) {
        let _ = handle.emit(events::LIBRARY_BOOTSTRAP_COMPLETE, true);
        return;
    }

    std::thread::spawn(move || {
        let bootstrap_started_at = std::time::Instant::now();
        log::info!("[library-boot] backend.bootstrap_seed.start critical=false");
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

        log::info!(
            "[library-boot] backend.bootstrap_seed.complete critical=false ok={} elapsed_ms={}",
            result.is_ok(),
            bootstrap_started_at.elapsed().as_millis()
        );
        let _ = handle.emit(events::LIBRARY_BOOTSTRAP_COMPLETE, result.is_ok());
    });
}

mod command_logic {
    use super::{security, steam_web_api, storage, AppState, ProviderErrorDto};
    use std::sync::atomic::{AtomicBool, Ordering};

    pub(super) fn prepare_steam_account_sync(
        state: &AppState,
    ) -> Result<(SteamAccountSyncGuard<'_>, String, String), ProviderErrorDto> {
        let sync_guard = SteamAccountSyncGuard::acquire(&state.steam_account_sync_in_progress)?;
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

        Ok((sync_guard, steam_id, api_key))
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct SteamEnrichmentFailureDecision {
        pub stop_round: bool,
        pub record_retry: bool,
        pub retry_after_days: i64,
        pub outcome: &'static str,
    }

    pub(super) fn classify_steam_enrichment_failure(
        phase: &str,
        error: &steam_web_api::SteamWebApiError,
    ) -> SteamEnrichmentFailureDecision {
        let retry_after_days = if phase == storage::STEAM_ENRICHMENT_PHASE_ARTWORK {
            storage::STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS
        } else {
            storage::STEAM_ENRICHMENT_ACHIEVEMENT_RETRY_DAYS
        };

        match error.code() {
            "steam_web_api_rate_limited" => SteamEnrichmentFailureDecision {
                stop_round: true,
                record_retry: false,
                retry_after_days,
                outcome: error.code(),
            },
            "steam_web_api_network_unavailable" => SteamEnrichmentFailureDecision {
                stop_round: false,
                record_retry: false,
                retry_after_days,
                outcome: error.code(),
            },
            _ => SteamEnrichmentFailureDecision {
                stop_round: false,
                record_retry: true,
                retry_after_days,
                outcome: error.code(),
            },
        }
    }

    #[derive(Debug)]
    pub(super) struct SteamAccountSyncGuard<'a> {
        flag: &'a AtomicBool,
    }

    impl<'a> SteamAccountSyncGuard<'a> {
        fn acquire(flag: &'a AtomicBool) -> Result<Self, ProviderErrorDto> {
            flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| Self { flag })
                .map_err(|_| {
                    ProviderErrorDto::steam(
                        "steam_sync_already_running",
                        "A sincronizacao da conta Steam ja esta em andamento. Aguarde terminar antes de iniciar novamente.",
                        true,
                        "preflight",
                        Some("tentativa duplicada de sincronizacao Steam".to_string()),
                    )
                })
        }
    }

    impl Drop for SteamAccountSyncGuard<'_> {
        fn drop(&mut self) {
            self.flag.store(false, Ordering::Release);
        }
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

        #[test]
        fn steam_account_sync_guard_rejects_duplicate_and_releases_on_drop() {
            let flag = AtomicBool::new(false);
            let guard = SteamAccountSyncGuard::acquire(&flag).expect("acquire first sync guard");
            let duplicate =
                SteamAccountSyncGuard::acquire(&flag).expect_err("reject duplicate sync guard");

            assert_eq!(duplicate.code, "steam_sync_already_running");
            assert_eq!(duplicate.phase, "preflight");
            assert!(flag.load(Ordering::Acquire));

            drop(guard);

            assert!(!flag.load(Ordering::Acquire));
            assert!(SteamAccountSyncGuard::acquire(&flag).is_ok());
        }

        #[test]
        fn prepare_steam_account_sync_rejects_duplicate_sync() {
            let state = test_app_state_with_database(
                std::env::temp_dir().join(format!(
                    "biblioteca-jogos-steam-duplicate-sync-{}.sqlite3",
                    test_timestamp_millis()
                )),
                true,
            );

            let error =
                prepare_steam_account_sync(&state).expect_err("reject duplicate steam sync");

            assert_eq!(error.code, "steam_sync_already_running");
            assert_eq!(error.phase, "preflight");
            assert!(error.recoverable);
        }

        #[test]
        fn prepare_steam_account_sync_releases_guard_after_preflight_error() {
            let state = test_app_state_with_database(
                std::env::temp_dir().join(format!(
                    "biblioteca-jogos-steam-preflight-sync-{}.sqlite3",
                    test_timestamp_millis()
                )),
                false,
            );

            let first_error =
                prepare_steam_account_sync(&state).expect_err("missing steam account");
            let second_error =
                prepare_steam_account_sync(&state).expect_err("missing steam account again");

            assert_eq!(first_error.code, "steam_account_missing");
            assert_eq!(second_error.code, "steam_account_missing");
            assert!(!state.steam_account_sync_in_progress.load(Ordering::Acquire));
        }

        #[test]
        fn steam_enrichment_failure_rate_limit_stops_round_without_retry_marker() {
            let decision = classify_steam_enrichment_failure(
                storage::STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS,
                &steam_web_api::SteamWebApiError::rate_limited(),
            );

            assert!(decision.stop_round);
            assert!(!decision.record_retry);
            assert_eq!(
                decision.retry_after_days,
                storage::STEAM_ENRICHMENT_ACHIEVEMENT_RETRY_DAYS
            );
            assert_eq!(decision.outcome, "steam_web_api_rate_limited");
        }

        #[test]
        fn steam_enrichment_failure_marks_recoverable_provider_errors_for_backoff() {
            let decision = classify_steam_enrichment_failure(
                storage::STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA,
                &steam_web_api::SteamWebApiError::auth_required(Some(
                    "resposta HTTP 403 da Steam Web API".to_string(),
                )),
            );

            assert!(!decision.stop_round);
            assert!(decision.record_retry);
            assert_eq!(
                decision.retry_after_days,
                storage::STEAM_ENRICHMENT_ACHIEVEMENT_RETRY_DAYS
            );
            assert_eq!(decision.outcome, "steam_web_api_auth_required");
        }

        #[test]
        fn steam_enrichment_failure_keeps_network_errors_transient() {
            let decision = classify_steam_enrichment_failure(
                storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                &steam_web_api::SteamWebApiError::network_unavailable_for_test(),
            );

            assert!(!decision.stop_round);
            assert!(!decision.record_retry);
            assert_eq!(
                decision.retry_after_days,
                storage::STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS
            );
            assert_eq!(decision.outcome, "steam_web_api_network_unavailable");
        }

        fn test_app_state_with_database(path: std::path::PathBuf, sync_running: bool) -> AppState {
            let connection = storage::open_database(&path).expect("open test database");
            AppState {
                connection: std::sync::Arc::new(std::sync::Mutex::new(connection)),
                auth_vault: std::sync::Arc::new(security::AuthVault::system(
                    std::env::temp_dir().join(format!(
                        "biblioteca-jogos-auth-vault-{}",
                        test_timestamp_millis()
                    )),
                )),
                steam_account_sync_in_progress: std::sync::Arc::new(AtomicBool::new(sync_running)),
                steam_enrichment_in_progress: std::sync::Arc::new(AtomicBool::new(false)),
            }
        }

        fn test_timestamp_millis() -> u128 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after unix epoch")
                .as_millis()
        }
    }
}

#[cfg(not(test))]
mod commands {
    use super::{
        command_logic, launcher, security, steam_openid, steam_web_api, storage, xbox_live_auth,
        xbox_provider, AppState,
    };
    use crate::ProviderErrorDto;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::{AppHandle, Emitter, State};

    #[derive(Debug, Clone, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BootMarkerInput {
        step: String,
        elapsed_ms: u64,
        #[serde(default)]
        details: std::collections::HashMap<String, serde_json::Value>,
    }

    #[tauri::command]
    pub fn record_boot_marker(input: BootMarkerInput) -> Result<(), String> {
        let step = input
            .step
            .chars()
            .filter(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '.' | '_' | '-' | ':' | '=')
            })
            .take(80)
            .collect::<String>();
        let mut details = input
            .details
            .iter()
            .filter_map(|(key, value)| {
                let sanitized_key = key
                    .chars()
                    .filter(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':')
                    })
                    .take(48)
                    .collect::<String>();

                if sanitized_key.is_empty() {
                    return None;
                }

                match value {
                    serde_json::Value::Bool(flag) => Some(format!("{sanitized_key}={flag}")),
                    serde_json::Value::Number(number) => Some(format!("{sanitized_key}={number}")),
                    serde_json::Value::String(text) => {
                        let sanitized_text = text
                            .chars()
                            .filter(|character| {
                                character.is_ascii_alphanumeric()
                                    || matches!(character, '.' | '_' | '-' | ':' | '=')
                            })
                            .take(48)
                            .collect::<String>();
                        Some(format!("{sanitized_key}={sanitized_text}"))
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        details.sort();
        log::info!(
            "[library-boot] frontend.marker step={} elapsed_ms={} details={}",
            step,
            input.elapsed_ms,
            details.join(",")
        );

        Ok(())
    }

    #[tauri::command]
    pub fn list_library_entries(
        state: State<'_, AppState>,
    ) -> Result<Vec<storage::LibraryEntryDto>, String> {
        let command_started_at = std::time::Instant::now();
        let lock_started_at = std::time::Instant::now();
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;
        let lock_wait_ms = lock_started_at.elapsed().as_millis();
        let query_started_at = std::time::Instant::now();

        let entries =
            storage::list_library_entries(&connection).map_err(|error| error.to_string())?;
        log::info!(
            "[library-boot] backend.list_library_entries.complete entries={} lock_wait_ms={} query_ms={} total_ms={}",
            entries.len(),
            lock_wait_ms,
            query_started_at.elapsed().as_millis(),
            command_started_at.elapsed().as_millis()
        );

        Ok(entries)
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
        let shared_connection = state.connection.clone();
        let (summary, steam_app_ids) = {
            let mut connection = state
                .connection
                .lock()
                .map_err(|_| "failed to lock local database".to_string())?;
            let summary =
                storage::sync_steam_games(&mut connection).map_err(|error| error.to_string())?;
            let steam_app_ids = storage::list_steam_app_ids(&connection).unwrap_or_else(|error| {
                log::warn!("failed to list steam app ids for artwork cache: {error}");
                Vec::new()
            });

            (summary, steam_app_ids)
        };

        warm_steam_artwork_cache_best_effort(shared_connection, steam_app_ids);

        Ok(summary)
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
            let _ = app.emit(super::events::XBOX_SYNC_FAILED, provider_error.clone());
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

        xbox_provider::sync_xbox_achievement_games(&mut connection, state.auth_vault.as_ref()).map_err(|error| {
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
                super::events::XBOX_ACHIEVEMENTS_SYNC_FAILED,
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

        xbox_provider::import_xbox_achievement_title_history(&mut connection, state.auth_vault.as_ref()).map_err(|error| {
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
                super::events::XBOX_TITLE_HISTORY_IMPORT_FAILED,
                provider_error.clone(),
            );
            provider_error_json
        })
    }

    #[tauri::command]
    pub fn sync_steam_account_games(
        app: AppHandle,
        state: State<'_, AppState>,
        input: Option<SteamAccountSyncInput>,
    ) -> Result<storage::SyncSummaryDto, String> {
        let retry_marked_enrichment = input
            .as_ref()
            .is_some_and(|input| input.retry_marked_enrichment);
        sync_steam_account_games_impl(Some(app.clone()), state.inner(), retry_marked_enrichment)
            .map_err(|error| {
                log::warn!(
                    "steam sync failed: code={} phase={} recoverable={} provider_id={} details_sanitized={:?}",
                    error.code,
                    error.phase,
                    error.recoverable,
                    error.provider_id,
                    error.details_sanitized
                );
                let _ = app.emit(super::events::STEAM_SYNC_FAILED, error.clone());
                error.message
            })
    }

    #[derive(Debug, Clone, Default, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamAccountSyncInput {
        #[serde(default)]
        retry_marked_enrichment: bool,
    }

    #[tauri::command]
    pub fn get_steam_enrichment_retry_summary(
        state: State<'_, AppState>,
    ) -> Result<storage::SteamEnrichmentRetrySummaryDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;
        let steam_id64 = storage::read_steam_account_config(&connection)
            .map_err(|error| error.to_string())?
            .unwrap_or_default();

        storage::get_steam_enrichment_retry_summary(&connection, &steam_id64)
            .map_err(|error| error.to_string())
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
    pub fn start_xbox_live_login(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<xbox_live_auth::XboxLiveLoginStartDto, String> {
        xbox_live_auth::start_login(app, state.connection.clone(), state.auth_vault.clone())
            .map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn get_xbox_live_auth_state(
        state: State<'_, AppState>,
    ) -> Result<security::XboxLiveAuthStateDto, String> {
        state
            .auth_vault
            .xbox_live_auth_state()
            .map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn get_xbox_live_client_config(
        state: State<'_, AppState>,
    ) -> Result<storage::XboxLiveClientConfigDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::get_xbox_live_client_config(&connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn get_xbox_live_client_secret_state(
        state: State<'_, AppState>,
    ) -> Result<security::XboxLiveClientSecretStateDto, String> {
        // Legacy compatibility only. The Xbox public-client login flow no longer depends on this.
        state
            .auth_vault
            .xbox_live_client_secret_state()
            .map_err(|error| error.to_string())
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
    pub fn get_xbox_library_roots(
        state: State<'_, AppState>,
    ) -> Result<storage::XboxLibraryRootsDto, String> {
        let connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::get_xbox_library_roots(&connection).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_xbox_library_roots(
        input: storage::XboxLibraryRootsInput,
        state: State<'_, AppState>,
    ) -> Result<storage::XboxLibraryRootsDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::save_xbox_library_roots(&mut connection, input).map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_xbox_live_client_config(
        input: storage::XboxLiveClientConfigInput,
        state: State<'_, AppState>,
    ) -> Result<storage::XboxLiveClientConfigDto, String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::save_xbox_live_client_config(&mut connection, input)
            .map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn save_xbox_live_client_secret(
        input: security::XboxLiveClientSecretInput,
        state: State<'_, AppState>,
    ) -> Result<security::XboxLiveClientSecretStateDto, String> {
        // Legacy compatibility only. The Xbox public-client login flow no longer depends on this.
        state
            .auth_vault
            .save_xbox_live_client_secret(input)
            .map_err(|error| error.to_string())
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

    fn warm_steam_artwork_cache_best_effort(
        connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
        app_ids: Vec<String>,
    ) {
        static ARTWORK_CACHE_WARMING: std::sync::OnceLock<AtomicBool> = std::sync::OnceLock::new();
        let warming = ARTWORK_CACHE_WARMING.get_or_init(|| AtomicBool::new(false));
        if warming
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            log::debug!("steam artwork cache warm already running; skipping duplicate request");
            return;
        }

        let app_ids = app_ids
            .into_iter()
            .filter(|app_id| steam_web_api::is_valid_steam_app_id(app_id))
            .take(512)
            .collect::<Vec<_>>();

        if app_ids.is_empty() {
            warming.store(false, Ordering::Release);
            return;
        }

        if let Err(error) = std::thread::Builder::new()
            .name("steam-artwork-cache".to_string())
            .spawn(move || {
                struct WarmFlagGuard(&'static AtomicBool);

                impl Drop for WarmFlagGuard {
                    fn drop(&mut self) {
                        self.0.store(false, Ordering::Release);
                    }
                }

                let _guard = WarmFlagGuard(warming);
                let client = steam_web_api::ReqwestSteamWebApiClient::default();

                for app_id in app_ids {
                    let already_cached = connection
                        .lock()
                        .ok()
                        .and_then(|connection| {
                            storage::read_steam_artwork_header_image(&connection, &app_id).ok()
                        })
                        .flatten()
                        .or_else(|| steam_web_api::cached_appdetails_header_image(&app_id))
                        .is_some();

                    if already_cached {
                        continue;
                    }

                    match steam_web_api::fetch_appdetails_header_image(&client, &app_id) {
                                Ok(Some(header_image)) => {
                                    steam_web_api::cache_appdetails_header_image(
                                        &app_id,
                                        header_image.clone(),
                                    );

                                    if let Ok(connection) = connection.lock() {
                                        if let Err(error) = storage::save_steam_artwork_header_image(
                                            &connection,
                                            &app_id,
                                            &header_image,
                                        ) {
                                    log::debug!(
                                        "failed to persist steam artwork cache: app_id={} error={error}",
                                        app_id
                                    );
                                }
                            }
                                }
                                Ok(None) => {
                                    if let Ok(connection) = connection.lock() {
                                        if let Err(error) =
                                            storage::record_steam_enrichment_attempt(
                                                &connection,
                                                None,
                                                &app_id,
                                                storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                                                storage::STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS,
                                                "not_available",
                                            )
                                        {
                                            log::debug!(
                                                "failed to persist steam artwork negative cache: app_id={} error={error}",
                                                app_id
                                            );
                                        } else if let Err(error) =
                                            storage::clear_steam_enrichment_attempt(
                                                &connection,
                                                None,
                                                &app_id,
                                                storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                                            )
                                        {
                                            log::debug!(
                                                "failed to clear steam artwork attempt cache: app_id={} error={error}",
                                                app_id
                                            );
                                        }
                                    }
                                }
                        Err(error) => {
                            if error.code() != "steam_web_api_network_unavailable" {
                                if let Ok(connection) = connection.lock() {
                                    if let Err(storage_error) =
                                        storage::record_steam_enrichment_attempt(
                                            &connection,
                                            None,
                                            &app_id,
                                            storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                                            storage::STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS,
                                            error.code(),
                                        )
                                    {
                                        log::debug!(
                                            "failed to persist steam artwork error cache: app_id={} error={storage_error}",
                                            app_id
                                        );
                                    }
                                }
                            }
                            log::debug!(
                                "steam appdetails artwork fetch failed: app_id={} code={} phase={} details_sanitized={:?}",
                                app_id,
                                error.code(),
                                error.phase(),
                                error.details_sanitized()
                            );
                        }
                    }
                }
            })
        {
            warming.store(false, Ordering::Release);
            log::debug!("failed to start steam artwork cache thread: {error}");
        }
    }

    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SteamEnrichmentJobEvent {
        job_id: String,
        provider_id: &'static str,
        total_candidates: usize,
        batch_size: usize,
        phases: Vec<&'static str>,
    }

    #[derive(Debug, Clone, Default, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SteamEnrichmentJobSummaryDto {
        job_id: String,
        provider_id: &'static str,
        total_candidates: usize,
        completed: usize,
        skipped_cached: usize,
        fetched_artwork: usize,
        fetched_achievement_schemas: usize,
        fetched_player_achievements: usize,
        updated: usize,
        failed: usize,
        rate_limited: bool,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SteamEnrichmentProgressEvent {
        job_id: String,
        provider_id: &'static str,
        phase: &'static str,
        completed: usize,
        total: usize,
        total_candidates: usize,
        batch_completed: usize,
        batch_total: usize,
        batches_completed: usize,
        total_batches: usize,
        skipped_cached: usize,
        failed: usize,
        rate_limited: bool,
    }

    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SteamEnrichmentFailedEvent {
        job_id: String,
        provider_id: &'static str,
        error: ProviderErrorDto,
        partial_summary: SteamEnrichmentJobSummaryDto,
    }

    const STEAM_ENRICHMENT_BATCH_SIZE: usize = 50;
    const STEAM_ENRICHMENT_MAX_INPUT_APPS: usize = 2000;
    const STEAM_ENRICHMENT_MAX_APPS_PER_JOB: usize = 1000;
    const STEAM_ENRICHMENT_REQUEST_DELAY_MS: u64 = 650;
    const STEAM_ENRICHMENT_BATCH_DELAY_MS: u64 = 1500;

    fn spawn_steam_enrichment_job_best_effort(
        app: AppHandle,
        connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
        auth_vault: std::sync::Arc<security::AuthVault>,
        enrichment_in_progress: std::sync::Arc<AtomicBool>,
        steam_id: String,
        app_ids: Vec<String>,
        retry_marked_enrichment: bool,
    ) {
        if enrichment_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            let summary = SteamEnrichmentJobSummaryDto {
                job_id: "steam-enrichment-skipped".to_string(),
                provider_id: "steam",
                ..SteamEnrichmentJobSummaryDto::default()
            };
            let error = ProviderErrorDto::steam(
                "steam_enrichment_already_running",
                "A atualizacao de capas e conquistas da Steam ja esta em andamento.",
                true,
                "preflight",
                Some("tentativa duplicada de enrichment Steam".to_string()),
            );
            let _ = app.emit(
                super::events::STEAM_ENRICHMENT_FAILED,
                SteamEnrichmentFailedEvent {
                    job_id: summary.job_id.clone(),
                    provider_id: "steam",
                    error,
                    partial_summary: summary,
                },
            );
            return;
        }

        let app_ids = app_ids
            .into_iter()
            .filter(|app_id| steam_web_api::is_valid_steam_app_id(app_id))
            .take(STEAM_ENRICHMENT_MAX_INPUT_APPS)
            .collect::<Vec<_>>();

        if app_ids.is_empty() {
            enrichment_in_progress.store(false, Ordering::Release);
            return;
        }

        let enrichment_flag_for_thread = enrichment_in_progress.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("steam-enrichment".to_string())
            .spawn(move || {
                struct EnrichmentFlagGuard(std::sync::Arc<AtomicBool>);

                impl Drop for EnrichmentFlagGuard {
                    fn drop(&mut self) {
                        self.0.store(false, Ordering::Release);
                    }
                }

                let _guard = EnrichmentFlagGuard(enrichment_flag_for_thread);
                let job_id = format!("steam-enrichment-{}", storage::timestamp_millis());
                let candidates = connection
                    .lock()
                    .map_err(|_| {
                        ProviderErrorDto::steam(
                            "steam_enrichment_database_lock_unavailable",
                            "Nao foi possivel acessar o banco local para preparar a atualizacao Steam.",
                            true,
                            "preflight",
                            Some("falha ao adquirir o lock local".to_string()),
                        )
                    })
                    .and_then(|connection| {
                        storage::list_steam_enrichment_candidates(
                            &connection,
                            &steam_id,
                            &app_ids,
                            STEAM_ENRICHMENT_MAX_APPS_PER_JOB,
                            retry_marked_enrichment,
                        )
                        .map_err(|error| {
                            ProviderErrorDto::steam(
                                "steam_enrichment_candidate_read_failed",
                                "Nao foi possivel preparar a fila de atualizacao Steam.",
                                true,
                                "preflight",
                                Some(error.to_string()),
                            )
                        })
                    });
                let candidates = match candidates {
                    Ok(candidates) => candidates,
                    Err(error) => {
                        let summary = SteamEnrichmentJobSummaryDto {
                            job_id: job_id.clone(),
                            provider_id: "steam",
                            ..SteamEnrichmentJobSummaryDto::default()
                        };
                        let _ = app.emit(
                            super::events::STEAM_ENRICHMENT_FAILED,
                            SteamEnrichmentFailedEvent {
                                job_id,
                                provider_id: "steam",
                                error,
                                partial_summary: summary,
                            },
                        );
                        return;
                    }
                };

                if candidates.is_empty() {
                    let summary = SteamEnrichmentJobSummaryDto {
                        job_id,
                        provider_id: "steam",
                        skipped_cached: app_ids.len(),
                        ..SteamEnrichmentJobSummaryDto::default()
                    };
                    let _ = app.emit(super::events::STEAM_ENRICHMENT_COMPLETED, summary);
                    return;
                }

                let _ = app.emit(
                    super::events::STEAM_ENRICHMENT_STARTED,
                    SteamEnrichmentJobEvent {
                        job_id: job_id.clone(),
                        provider_id: "steam",
                        total_candidates: candidates.len(),
                        batch_size: STEAM_ENRICHMENT_BATCH_SIZE,
                        phases: vec!["artwork", "achievements"],
                    },
                );

                let api_key = auth_vault.steam_web_api_key().ok().flatten();
                let client = steam_web_api::ReqwestSteamWebApiClient::default();
                let mut summary = SteamEnrichmentJobSummaryDto {
                    job_id: job_id.clone(),
                    provider_id: "steam",
                    total_candidates: candidates.len(),
                    ..SteamEnrichmentJobSummaryDto::default()
                };
                let total_batches =
                    candidates.len().div_ceil(STEAM_ENRICHMENT_BATCH_SIZE).max(1);

                for (batch_index, batch) in
                    candidates.chunks(STEAM_ENRICHMENT_BATCH_SIZE).enumerate()
                {
                    for (batch_item_index, candidate) in batch.iter().enumerate() {
                        let mut changed = false;

                        if candidate.needs_artwork {
                            match steam_web_api::fetch_appdetails_header_image(
                                &client,
                                &candidate.app_id,
                            ) {
                                Ok(Some(header_image)) => {
                                    steam_web_api::cache_appdetails_header_image(
                                        &candidate.app_id,
                                        header_image.clone(),
                                    );
                                    if let Ok(connection) = connection.lock() {
                                        if storage::save_steam_artwork_header_image(
                                            &connection,
                                            &candidate.app_id,
                                            &header_image,
                                        )
                                        .is_ok()
                                        {
                                            let _ = storage::clear_steam_enrichment_attempt(
                                                &connection,
                                                None,
                                                &candidate.app_id,
                                                storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                                            );
                                            summary.fetched_artwork += 1;
                                            changed = true;
                                        }
                                    }
                                }
                                Ok(None) => {
                                    if let Ok(connection) = connection.lock() {
                                        if storage::record_steam_enrichment_attempt(
                                            &connection,
                                            None,
                                            &candidate.app_id,
                                            storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                                            storage::STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS,
                                            "not_available",
                                        )
                                        .is_ok()
                                        {
                                            changed = true;
                                        }
                                    }
                                }
                                Err(error) => {
                                    let decision =
                                        command_logic::classify_steam_enrichment_failure(
                                            storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                                            &error,
                                        );
                                    summary.failed += 1;
                                    if decision.stop_round {
                                        summary.rate_limited = true;
                                        emit_steam_enrichment_rate_limited(
                                            &app, &job_id, &summary, error,
                                        );
                                        return;
                                    }
                                    if decision.record_retry {
                                        if let Ok(connection) = connection.lock() {
                                            let _ = storage::record_steam_enrichment_attempt(
                                                &connection,
                                                None,
                                                &candidate.app_id,
                                                storage::STEAM_ENRICHMENT_PHASE_ARTWORK,
                                                decision.retry_after_days,
                                                decision.outcome,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(api_key) = api_key.as_deref() {
                            if candidate.needs_achievement_schema {
                                match steam_web_api::fetch_achievement_schema(
                                    &client,
                                    api_key,
                                    &candidate.app_id,
                                ) {
                                    Ok(schema) => {
                                        if let Ok(connection) = connection.lock() {
                                            if storage::save_steam_achievement_schema_cache(
                                                &connection,
                                                &candidate.app_id,
                                                &schema.raw_json,
                                                schema.total_count,
                                            )
                                            .is_ok()
                                            {
                                                let _ = storage::clear_steam_enrichment_attempt(
                                                    &connection,
                                                    None,
                                                    &candidate.app_id,
                                                    storage::STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA,
                                                );
                                                summary.fetched_achievement_schemas += 1;
                                                changed = true;
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        let decision =
                                            command_logic::classify_steam_enrichment_failure(
                                                storage::STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA,
                                                &error,
                                            );
                                        summary.failed += 1;
                                        if decision.stop_round {
                                            summary.rate_limited = true;
                                            emit_steam_enrichment_rate_limited(
                                                &app, &job_id, &summary, error,
                                            );
                                            return;
                                        }
                                        if decision.record_retry {
                                            if let Ok(connection) = connection.lock() {
                                                let _ = storage::record_steam_enrichment_attempt(
                                                    &connection,
                                                    None,
                                                    &candidate.app_id,
                                                    storage::STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA,
                                                    decision.retry_after_days,
                                                    decision.outcome,
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            if candidate.needs_player_achievements {
                                match steam_web_api::fetch_player_achievements(
                                    &client,
                                    api_key,
                                    &steam_id,
                                    &candidate.app_id,
                                ) {
                                    Ok(player_achievements) => {
                                        if let Ok(connection) = connection.lock() {
                                            if storage::save_steam_player_achievement_cache(
                                                &connection,
                                                &steam_id,
                                                &candidate.app_id,
                                                &player_achievements.raw_json,
                                                player_achievements.unlocked_count,
                                                player_achievements.total_count,
                                            )
                                            .is_ok()
                                            {
                                                let _ = storage::clear_steam_enrichment_attempt(
                                                    &connection,
                                                    Some(&steam_id),
                                                    &candidate.app_id,
                                                    storage::STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS,
                                                );
                                                summary.fetched_player_achievements += 1;
                                                changed = true;
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        let decision =
                                            command_logic::classify_steam_enrichment_failure(
                                                storage::STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS,
                                                &error,
                                            );
                                        summary.failed += 1;
                                        if decision.stop_round {
                                            summary.rate_limited = true;
                                            emit_steam_enrichment_rate_limited(
                                                &app, &job_id, &summary, error,
                                            );
                                            return;
                                        }
                                        if decision.record_retry {
                                            if let Ok(connection) = connection.lock() {
                                                let _ = storage::record_steam_enrichment_attempt(
                                                    &connection,
                                                    Some(&steam_id),
                                                    &candidate.app_id,
                                                    storage::STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS,
                                                    decision.retry_after_days,
                                                    decision.outcome,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if changed {
                            summary.updated += 1;
                        } else if !candidate.needs_artwork
                            && !candidate.needs_achievement_schema
                            && !candidate.needs_player_achievements
                        {
                            summary.skipped_cached += 1;
                        }

                        summary.completed += 1;
                        let _ = app.emit(
                            super::events::STEAM_ENRICHMENT_PROGRESS,
                            SteamEnrichmentProgressEvent {
                                job_id: job_id.clone(),
                                provider_id: "steam",
                                phase: "enrichment",
                                completed: summary.completed,
                                total: summary.total_candidates,
                                total_candidates: summary.total_candidates,
                                batch_completed: batch_item_index + 1,
                                batch_total: batch.len(),
                                batches_completed: batch_index,
                                total_batches,
                                skipped_cached: summary.skipped_cached,
                                failed: summary.failed,
                                rate_limited: summary.rate_limited,
                            },
                        );
                        std::thread::sleep(std::time::Duration::from_millis(
                            STEAM_ENRICHMENT_REQUEST_DELAY_MS,
                        ));
                    }

                    if batch_index + 1 < total_batches {
                        std::thread::sleep(std::time::Duration::from_millis(
                            STEAM_ENRICHMENT_BATCH_DELAY_MS,
                        ));
                    }
                }

                let _ = app.emit(super::events::STEAM_ENRICHMENT_COMPLETED, summary);
            })
        {
            enrichment_in_progress.store(false, Ordering::Release);
            log::debug!("failed to start steam enrichment thread: {error}");
        }
    }

    fn emit_steam_enrichment_rate_limited(
        app: &AppHandle,
        job_id: &str,
        summary: &SteamEnrichmentJobSummaryDto,
        error: steam_web_api::SteamWebApiError,
    ) {
        let _ = app.emit(
            super::events::STEAM_ENRICHMENT_FAILED,
            SteamEnrichmentFailedEvent {
                job_id: job_id.to_string(),
                provider_id: "steam",
                error: error.into_provider_error(),
                partial_summary: summary.clone(),
            },
        );
    }

    fn sync_steam_account_games_impl(
        app: Option<AppHandle>,
        state: &AppState,
        retry_marked_enrichment: bool,
    ) -> Result<storage::SyncSummaryDto, ProviderErrorDto> {
        let (_sync_guard, steam_id, api_key) = command_logic::prepare_steam_account_sync(state)?;

        let client = steam_web_api::ReqwestSteamWebApiClient::default();
        let remote_games = steam_web_api::fetch_owned_games(&client, &api_key, &steam_id)
            .map_err(|error| error.into_provider_error())?;
        let artwork_app_ids = remote_games
            .iter()
            .map(|game| game.app_id.clone())
            .collect::<Vec<_>>();

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

        drop(connection);
        if let Some(app) = app {
            spawn_steam_enrichment_job_best_effort(
                app,
                state.connection.clone(),
                state.auth_vault.clone(),
                state.steam_enrichment_in_progress.clone(),
                steam_id,
                artwork_app_ids,
                retry_marked_enrichment,
            );
        }

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
    pub fn set_library_entry_favorite(
        entry_id: String,
        is_favorite: bool,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        let mut connection = state
            .connection
            .lock()
            .map_err(|_| "failed to lock local database".to_string())?;

        storage::set_library_entry_favorite(&mut connection, &entry_id, is_favorite)
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
    const XBOX_LIVE_REFRESH_TOKEN_USER: &str = "xbox-live-refresh-token";
    const XBOX_LIVE_REFRESH_TOKEN_FILE: &str = "xbox-live-refresh-token.dpapi";
    const XBOX_LIVE_REFRESH_TOKEN_DPAPI_ENTROPY: &[u8] =
        b"com.bibliotecajogos.unificada/xbox-live-refresh-token";
    const XBOX_LIVE_CLIENT_SECRET_USER: &str = "xbox-live-client-secret";
    const XBOX_LIVE_CLIENT_SECRET_FILE: &str = "xbox-live-client-secret.dpapi";
    const XBOX_LIVE_CLIENT_SECRET_DPAPI_ENTROPY: &[u8] =
        b"com.bibliotecajogos.unificada/xbox-live-client-secret";

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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct XboxLiveAuthStateDto {
        configured: bool,
        provider_id: &'static str,
        storage: &'static str,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct XboxLiveClientSecretStateDto {
        configured: bool,
        provider_id: &'static str,
        storage: &'static str,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct XboxLiveRefreshTokenInput {
        pub refresh_token: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct XboxLiveClientSecretInput {
        pub client_secret: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum AuthVaultError {
        InvalidSteamWebApiKey,
        InvalidXboxLiveRefreshToken,
        InvalidXboxLiveClientSecret,
        SecureStorageUnavailable { operation: &'static str },
        LockUnavailable,
    }

    impl fmt::Display for AuthVaultError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            let message = match self {
                AuthVaultError::InvalidSteamWebApiKey => {
                    "Chave Steam Web API invalida. Informe uma chave hexadecimal de 32 caracteres."
                }
                AuthVaultError::InvalidXboxLiveRefreshToken => {
                    "Credencial Xbox Live invalida. Conclua o login Xbox Live novamente."
                }
                AuthVaultError::InvalidXboxLiveClientSecret => {
                    "Segredo Microsoft invalido. Informe um valor valido no campo Xbox Live."
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
        xbox_live_auth_configured: bool,
        xbox_live_client_secret_configured: bool,
    }

    impl AuthVault {
        pub fn system(vault_dir: PathBuf) -> Self {
            Self::new(Arc::new(SystemSecretStore::new(vault_dir)))
        }

        fn new(store: Arc<dyn SecretStore>) -> Self {
            let steam_web_api_key_configured = store.exists().unwrap_or(false);
            let xbox_live_auth_configured = store.xbox_live_refresh_token_exists().unwrap_or(false);
            let xbox_live_client_secret_configured =
                store.xbox_live_client_secret_exists().unwrap_or(false);

            Self {
                store,
                state: Mutex::new(AuthVaultState {
                    steam_web_api_key_configured,
                    xbox_live_auth_configured,
                    xbox_live_client_secret_configured,
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

        pub fn save_xbox_live_refresh_token(
            &self,
            input: XboxLiveRefreshTokenInput,
        ) -> Result<XboxLiveAuthStateDto, AuthVaultError> {
            let refresh_token = validate_xbox_live_refresh_token(&input.refresh_token)?;
            self.store.set_xbox_live_refresh_token(refresh_token)?;
            if self.store.get_xbox_live_refresh_token()?.as_deref() != Some(refresh_token) {
                return Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "validacao da credencial",
                });
            }

            self.set_xbox_live_auth_configured(true)
        }

        pub fn xbox_live_auth_state(&self) -> Result<XboxLiveAuthStateDto, AuthVaultError> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AuthVaultError::LockUnavailable)?;
            state.xbox_live_auth_configured = self.store.xbox_live_refresh_token_exists()?;

            Ok(xbox_live_auth_state_dto(
                state.xbox_live_auth_configured,
                "auth_vault",
            ))
        }

        pub fn xbox_live_refresh_token(&self) -> Result<Option<String>, AuthVaultError> {
            self.store.get_xbox_live_refresh_token()
        }

        pub fn save_xbox_live_client_secret(
            &self,
            input: XboxLiveClientSecretInput,
        ) -> Result<XboxLiveClientSecretStateDto, AuthVaultError> {
            let client_secret = validate_xbox_live_client_secret(&input.client_secret)?;
            self.store.set_xbox_live_client_secret(client_secret)?;
            if self.store.get_xbox_live_client_secret()?.as_deref() != Some(client_secret) {
                return Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "validacao da credencial",
                });
            }

            self.set_xbox_live_client_secret_configured(true)
        }

        pub fn xbox_live_client_secret_state(
            &self,
        ) -> Result<XboxLiveClientSecretStateDto, AuthVaultError> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AuthVaultError::LockUnavailable)?;
            state.xbox_live_client_secret_configured =
                self.store.xbox_live_client_secret_exists()?;

            Ok(xbox_live_client_secret_state_dto(
                state.xbox_live_client_secret_configured,
                "auth_vault",
            ))
        }

        pub fn xbox_live_client_secret(&self) -> Result<Option<String>, AuthVaultError> {
            self.store.get_xbox_live_client_secret()
        }

        pub fn disconnect_xbox_live_client_secret(
            &self,
        ) -> Result<XboxLiveClientSecretStateDto, AuthVaultError> {
            self.store.delete_xbox_live_client_secret()?;
            self.set_xbox_live_client_secret_configured(false)
        }

        pub fn disconnect_xbox_live_auth(&self) -> Result<XboxLiveAuthStateDto, AuthVaultError> {
            self.store.delete_xbox_live_refresh_token()?;
            self.set_xbox_live_auth_configured(false)
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

        fn set_xbox_live_auth_configured(
            &self,
            configured: bool,
        ) -> Result<XboxLiveAuthStateDto, AuthVaultError> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AuthVaultError::LockUnavailable)?;
            state.xbox_live_auth_configured = configured;

            Ok(xbox_live_auth_state_dto(configured, "auth_vault"))
        }

        fn set_xbox_live_client_secret_configured(
            &self,
            configured: bool,
        ) -> Result<XboxLiveClientSecretStateDto, AuthVaultError> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AuthVaultError::LockUnavailable)?;
            state.xbox_live_client_secret_configured = configured;

            Ok(xbox_live_client_secret_state_dto(configured, "auth_vault"))
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

    fn xbox_live_auth_state_dto(configured: bool, storage: &'static str) -> XboxLiveAuthStateDto {
        XboxLiveAuthStateDto {
            configured,
            provider_id: "xbox",
            storage,
        }
    }

    fn xbox_live_client_secret_state_dto(
        configured: bool,
        storage: &'static str,
    ) -> XboxLiveClientSecretStateDto {
        XboxLiveClientSecretStateDto {
            configured,
            provider_id: "xbox",
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

    fn validate_xbox_live_refresh_token(input: &str) -> Result<&str, AuthVaultError> {
        let refresh_token = input.trim();

        if refresh_token.is_empty() {
            Err(AuthVaultError::InvalidXboxLiveRefreshToken)
        } else {
            Ok(refresh_token)
        }
    }

    fn validate_xbox_live_client_secret(input: &str) -> Result<&str, AuthVaultError> {
        let client_secret = input.trim();

        if client_secret.is_empty() {
            Err(AuthVaultError::InvalidXboxLiveClientSecret)
        } else {
            Ok(client_secret)
        }
    }

    trait SecretStore: Send + Sync {
        fn set(&self, secret: &str) -> Result<(), AuthVaultError>;
        fn get(&self) -> Result<Option<String>, AuthVaultError>;
        fn exists(&self) -> Result<bool, AuthVaultError>;
        fn delete(&self) -> Result<(), AuthVaultError>;
        fn set_xbox_live_refresh_token(&self, secret: &str) -> Result<(), AuthVaultError>;
        fn get_xbox_live_refresh_token(&self) -> Result<Option<String>, AuthVaultError>;
        fn xbox_live_refresh_token_exists(&self) -> Result<bool, AuthVaultError>;
        fn delete_xbox_live_refresh_token(&self) -> Result<(), AuthVaultError>;
        fn set_xbox_live_client_secret(&self, secret: &str) -> Result<(), AuthVaultError>;
        fn get_xbox_live_client_secret(&self) -> Result<Option<String>, AuthVaultError>;
        fn xbox_live_client_secret_exists(&self) -> Result<bool, AuthVaultError>;
        fn delete_xbox_live_client_secret(&self) -> Result<(), AuthVaultError>;
    }

    struct SystemSecretStore {
        fallback: DpapiFileSecretStore,
        xbox_live_fallback: DpapiFileSecretStore,
        xbox_live_client_secret_fallback: DpapiFileSecretStore,
    }

    impl SystemSecretStore {
        fn new(vault_dir: PathBuf) -> Self {
            Self {
                fallback: DpapiFileSecretStore::new(
                    vault_dir.join(STEAM_WEB_API_KEY_FILE),
                    STEAM_WEB_API_DPAPI_ENTROPY,
                ),
                xbox_live_fallback: DpapiFileSecretStore::new(
                    vault_dir.join(XBOX_LIVE_REFRESH_TOKEN_FILE),
                    XBOX_LIVE_REFRESH_TOKEN_DPAPI_ENTROPY,
                ),
                xbox_live_client_secret_fallback: DpapiFileSecretStore::new(
                    vault_dir.join(XBOX_LIVE_CLIENT_SECRET_FILE),
                    XBOX_LIVE_CLIENT_SECRET_DPAPI_ENTROPY,
                ),
            }
        }

        fn entry(&self) -> Result<Entry, AuthVaultError> {
            Entry::new(STEAM_WEB_API_SERVICE, STEAM_WEB_API_USER).map_err(|_| {
                AuthVaultError::SecureStorageUnavailable {
                    operation: "preparacao do AuthVault",
                }
            })
        }

        fn xbox_live_entry(&self) -> Result<Entry, AuthVaultError> {
            Entry::new(STEAM_WEB_API_SERVICE, XBOX_LIVE_REFRESH_TOKEN_USER).map_err(|_| {
                AuthVaultError::SecureStorageUnavailable {
                    operation: "preparacao do AuthVault",
                }
            })
        }

        fn xbox_live_client_secret_entry(&self) -> Result<Entry, AuthVaultError> {
            Entry::new(STEAM_WEB_API_SERVICE, XBOX_LIVE_CLIENT_SECRET_USER).map_err(|_| {
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

        fn set_xbox_live_keyring_secret(&self, secret: &str) -> Result<(), AuthVaultError> {
            self.xbox_live_entry()?.set_password(secret).map_err(|_| {
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

        fn get_xbox_live_keyring_secret(&self) -> Result<Option<String>, AuthVaultError> {
            match self.xbox_live_entry()?.get_password() {
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

        fn delete_xbox_live_keyring_secret(&self) -> Result<(), AuthVaultError> {
            match self.xbox_live_entry()?.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "remocao da credencial",
                }),
            }
        }

        fn set_xbox_live_client_secret_keyring_secret(
            &self,
            secret: &str,
        ) -> Result<(), AuthVaultError> {
            self.xbox_live_client_secret_entry()?
                .set_password(secret)
                .map_err(|_| AuthVaultError::SecureStorageUnavailable {
                    operation: "gravacao da credencial",
                })
        }

        fn get_xbox_live_client_secret_keyring_secret(
            &self,
        ) -> Result<Option<String>, AuthVaultError> {
            match self.xbox_live_client_secret_entry()?.get_password() {
                Ok(secret) => Ok(Some(secret)),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(_) => Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "leitura da credencial",
                }),
            }
        }

        fn delete_xbox_live_client_secret_keyring_secret(&self) -> Result<(), AuthVaultError> {
            match self.xbox_live_client_secret_entry()?.delete_credential() {
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

        fn set_xbox_live_refresh_token(&self, secret: &str) -> Result<(), AuthVaultError> {
            if self.set_xbox_live_keyring_secret(secret).is_ok()
                && self
                    .get_xbox_live_keyring_secret()
                    .ok()
                    .flatten()
                    .as_deref()
                    == Some(secret)
            {
                let _ = self.xbox_live_fallback.delete();
                return Ok(());
            }

            self.xbox_live_fallback.set(secret)?;
            if self.xbox_live_fallback.get()?.as_deref() == Some(secret) {
                match self.delete_xbox_live_keyring_secret() {
                    Ok(()) => Ok(()),
                    Err(error) if self.get_xbox_live_keyring_secret().ok().flatten().is_some() => {
                        Err(error)
                    }
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

        fn get_xbox_live_refresh_token(&self) -> Result<Option<String>, AuthVaultError> {
            if let Ok(Some(secret)) = self.get_xbox_live_keyring_secret() {
                return Ok(Some(secret));
            }

            self.xbox_live_fallback.get()
        }

        fn xbox_live_refresh_token_exists(&self) -> Result<bool, AuthVaultError> {
            self.get_xbox_live_refresh_token()
                .map(|secret| secret.is_some())
        }

        fn delete_xbox_live_refresh_token(&self) -> Result<(), AuthVaultError> {
            let keyring_result = self.delete_xbox_live_keyring_secret();
            let fallback_result = self.xbox_live_fallback.delete();

            fallback_result?;
            match keyring_result {
                Ok(()) => Ok(()),
                Err(error) if self.get_xbox_live_keyring_secret().ok().flatten().is_some() => {
                    Err(error)
                }
                Err(_) => Ok(()),
            }
        }

        fn set_xbox_live_client_secret(&self, secret: &str) -> Result<(), AuthVaultError> {
            if self
                .set_xbox_live_client_secret_keyring_secret(secret)
                .is_ok()
                && self
                    .get_xbox_live_client_secret_keyring_secret()
                    .ok()
                    .flatten()
                    .as_deref()
                    == Some(secret)
            {
                let _ = self.xbox_live_client_secret_fallback.delete();
                return Ok(());
            }

            self.xbox_live_client_secret_fallback.set(secret)?;
            if self.xbox_live_client_secret_fallback.get()?.as_deref() == Some(secret) {
                match self.delete_xbox_live_client_secret_keyring_secret() {
                    Ok(()) => Ok(()),
                    Err(error)
                        if self
                            .get_xbox_live_client_secret_keyring_secret()
                            .ok()
                            .flatten()
                            .is_some() =>
                    {
                        Err(error)
                    }
                    Err(_) => Ok(()),
                }
            } else {
                Err(AuthVaultError::SecureStorageUnavailable {
                    operation: "validacao da credencial",
                })
            }
        }

        fn get_xbox_live_client_secret(&self) -> Result<Option<String>, AuthVaultError> {
            if let Ok(Some(secret)) = self.get_xbox_live_client_secret_keyring_secret() {
                return Ok(Some(secret));
            }

            self.xbox_live_client_secret_fallback.get()
        }

        fn xbox_live_client_secret_exists(&self) -> Result<bool, AuthVaultError> {
            self.get_xbox_live_client_secret()
                .map(|secret| secret.is_some())
        }

        fn delete_xbox_live_client_secret(&self) -> Result<(), AuthVaultError> {
            let keyring_result = self.delete_xbox_live_client_secret_keyring_secret();
            let fallback_result = self.xbox_live_client_secret_fallback.delete();

            fallback_result?;
            match keyring_result {
                Ok(()) => Ok(()),
                Err(error)
                    if self
                        .get_xbox_live_client_secret_keyring_secret()
                        .ok()
                        .flatten()
                        .is_some() =>
                {
                    Err(error)
                }
                Err(_) => Ok(()),
            }
        }
    }

    struct DpapiFileSecretStore {
        path: PathBuf,
        entropy: &'static [u8],
    }

    impl DpapiFileSecretStore {
        fn new(path: PathBuf, entropy: &'static [u8]) -> Self {
            Self { path, entropy }
        }
    }

    impl SecretStore for DpapiFileSecretStore {
        fn set(&self, secret: &str) -> Result<(), AuthVaultError> {
            let protected_secret = protect_secret(secret.as_bytes(), self.entropy)?;
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
            let secret = unprotect_secret(&protected_secret, self.entropy)?;

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

        fn set_xbox_live_refresh_token(&self, secret: &str) -> Result<(), AuthVaultError> {
            self.set(secret)
        }

        fn get_xbox_live_refresh_token(&self) -> Result<Option<String>, AuthVaultError> {
            self.get()
        }

        fn xbox_live_refresh_token_exists(&self) -> Result<bool, AuthVaultError> {
            self.exists()
        }

        fn delete_xbox_live_refresh_token(&self) -> Result<(), AuthVaultError> {
            self.delete()
        }

        fn set_xbox_live_client_secret(&self, secret: &str) -> Result<(), AuthVaultError> {
            self.set(secret)
        }

        fn get_xbox_live_client_secret(&self) -> Result<Option<String>, AuthVaultError> {
            self.get()
        }

        fn xbox_live_client_secret_exists(&self) -> Result<bool, AuthVaultError> {
            self.exists()
        }

        fn delete_xbox_live_client_secret(&self) -> Result<(), AuthVaultError> {
            self.delete()
        }
    }

    #[cfg(target_os = "windows")]
    fn protect_secret(secret: &[u8], entropy_bytes: &[u8]) -> Result<Vec<u8>, AuthVaultError> {
        use std::ptr;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Cryptography::{
            CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };

        let entropy = CRYPT_INTEGER_BLOB {
            cbData: entropy_bytes.len() as u32,
            pbData: entropy_bytes.as_ptr() as *mut u8,
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
    fn protect_secret(_secret: &[u8], _entropy_bytes: &[u8]) -> Result<Vec<u8>, AuthVaultError> {
        Err(AuthVaultError::SecureStorageUnavailable {
            operation: "criptografia da credencial DPAPI",
        })
    }

    #[cfg(target_os = "windows")]
    fn unprotect_secret(
        protected_secret: &[u8],
        entropy_bytes: &[u8],
    ) -> Result<Vec<u8>, AuthVaultError> {
        use std::ptr;
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };

        let entropy = CRYPT_INTEGER_BLOB {
            cbData: entropy_bytes.len() as u32,
            pbData: entropy_bytes.as_ptr() as *mut u8,
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
    fn unprotect_secret(
        _protected_secret: &[u8],
        _entropy_bytes: &[u8],
    ) -> Result<Vec<u8>, AuthVaultError> {
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

        #[test]
        fn save_and_disconnect_xbox_live_client_secret_use_secret_store_without_exposing_secret() {
            let store = Arc::new(InMemorySecretStore::default());
            let vault = AuthVault::new(store.clone());

            assert_eq!(
                vault
                    .xbox_live_client_secret_state()
                    .expect("read empty state"),
                xbox_live_client_secret_state_dto(false, "auth_vault")
            );

            let saved_state = vault
                .save_xbox_live_client_secret(XboxLiveClientSecretInput {
                    client_secret: "super-secret-value".to_string(),
                })
                .expect("save client secret");

            assert_eq!(
                saved_state,
                xbox_live_client_secret_state_dto(true, "auth_vault")
            );
            assert_eq!(
                store.read_secret_for(XBOX_LIVE_CLIENT_SECRET_USER),
                Some("super-secret-value".to_string())
            );
            assert_eq!(
                vault
                    .xbox_live_client_secret_state()
                    .expect("read saved state"),
                xbox_live_client_secret_state_dto(true, "auth_vault")
            );

            let disconnected_state = vault
                .disconnect_xbox_live_client_secret()
                .expect("disconnect client secret");

            assert_eq!(
                disconnected_state,
                xbox_live_client_secret_state_dto(false, "auth_vault")
            );
            assert_eq!(store.read_secret_for(XBOX_LIVE_CLIENT_SECRET_USER), None);
        }

        #[derive(Default)]
        struct InMemorySecretStore {
            secrets: Mutex<HashMap<String, String>>,
            delete_should_fail: Mutex<bool>,
        }

        impl InMemorySecretStore {
            fn read_secret(&self) -> Option<String> {
                self.read_secret_for(STEAM_WEB_API_USER)
            }

            fn read_secret_for(&self, user: &str) -> Option<String> {
                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .get(user)
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

            fn set_xbox_live_refresh_token(&self, secret: &str) -> Result<(), AuthVaultError> {
                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .insert(XBOX_LIVE_REFRESH_TOKEN_USER.to_string(), secret.to_string());
                Ok(())
            }

            fn get_xbox_live_refresh_token(&self) -> Result<Option<String>, AuthVaultError> {
                Ok(self
                    .secrets
                    .lock()
                    .expect("lock secrets")
                    .get(XBOX_LIVE_REFRESH_TOKEN_USER)
                    .cloned())
            }

            fn xbox_live_refresh_token_exists(&self) -> Result<bool, AuthVaultError> {
                Ok(self
                    .secrets
                    .lock()
                    .expect("lock secrets")
                    .contains_key(XBOX_LIVE_REFRESH_TOKEN_USER))
            }

            fn delete_xbox_live_refresh_token(&self) -> Result<(), AuthVaultError> {
                if *self.delete_should_fail.lock().expect("lock delete flag") {
                    return Err(AuthVaultError::SecureStorageUnavailable {
                        operation: "remocao da credencial",
                    });
                }

                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .remove(XBOX_LIVE_REFRESH_TOKEN_USER);
                Ok(())
            }

            fn set_xbox_live_client_secret(&self, secret: &str) -> Result<(), AuthVaultError> {
                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .insert(XBOX_LIVE_CLIENT_SECRET_USER.to_string(), secret.to_string());
                Ok(())
            }

            fn get_xbox_live_client_secret(&self) -> Result<Option<String>, AuthVaultError> {
                Ok(self
                    .secrets
                    .lock()
                    .expect("lock secrets")
                    .get(XBOX_LIVE_CLIENT_SECRET_USER)
                    .cloned())
            }

            fn xbox_live_client_secret_exists(&self) -> Result<bool, AuthVaultError> {
                Ok(self
                    .secrets
                    .lock()
                    .expect("lock secrets")
                    .contains_key(XBOX_LIVE_CLIENT_SECRET_USER))
            }

            fn delete_xbox_live_client_secret(&self) -> Result<(), AuthVaultError> {
                if *self.delete_should_fail.lock().expect("lock delete flag") {
                    return Err(AuthVaultError::SecureStorageUnavailable {
                        operation: "remocao da credencial",
                    });
                }

                self.secrets
                    .lock()
                    .expect("lock secrets")
                    .remove(XBOX_LIVE_CLIENT_SECRET_USER);
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
                        is_favorite INTEGER NOT NULL DEFAULT 0,
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
    use std::collections::HashMap;
    use std::fmt;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    use url::Url;

    const STEAM_OWNED_GAMES_ENDPOINT: &str =
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/";
    const STEAM_PLAYER_ACHIEVEMENTS_ENDPOINT: &str =
        "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/";
    const STEAM_ACHIEVEMENT_SCHEMA_ENDPOINT: &str =
        "https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/";
    const STEAM_APPDETAILS_ENDPOINT: &str = "https://store.steampowered.com/api/appdetails";
    const STEAM_OWNED_GAMES_TIMEOUT: Duration = Duration::from_secs(15);
    const STEAM_APPDETAILS_TIMEOUT: Duration = Duration::from_millis(1200);

    pub trait SteamWebApiClient {
        fn get_owned_games(&self, url: &Url) -> Result<String, SteamWebApiError>;
        fn get_appdetails(&self, url: &Url) -> Result<String, SteamWebApiError>;
        fn get_player_achievements(&self, url: &Url) -> Result<String, SteamWebApiError>;
        fn get_schema_for_game(&self, url: &Url) -> Result<String, SteamWebApiError>;
    }

    pub struct ReqwestSteamWebApiClient {
        client: reqwest::blocking::Client,
    }

    impl Default for ReqwestSteamWebApiClient {
        fn default() -> Self {
            let client = reqwest::blocking::Client::builder()
                .timeout(STEAM_OWNED_GAMES_TIMEOUT)
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new());

            Self { client }
        }
    }

    impl SteamWebApiClient for ReqwestSteamWebApiClient {
        fn get_owned_games(&self, url: &Url) -> Result<String, SteamWebApiError> {
            let response = match self
                .client
                .get(url.clone())
                .timeout(STEAM_OWNED_GAMES_TIMEOUT)
                .send()
            {
                Ok(response) => response,
                Err(error) if error.is_timeout() => {
                    std::thread::sleep(Duration::from_millis(350));
                    self.client
                        .get(url.clone())
                        .timeout(STEAM_OWNED_GAMES_TIMEOUT)
                        .send()
                        .map_err(|error| {
                            SteamWebApiError::network_unavailable_from_reqwest(Some(&error))
                        })?
                }
                Err(error) => {
                    return Err(SteamWebApiError::network_unavailable_from_reqwest(Some(
                        &error,
                    )));
                }
            };
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

        fn get_appdetails(&self, url: &Url) -> Result<String, SteamWebApiError> {
            let response = self
                .client
                .get(url.clone())
                .timeout(STEAM_APPDETAILS_TIMEOUT)
                .send()
                .map_err(|error| {
                    SteamWebApiError::network_unavailable_from_reqwest(Some(&error))
                })?;
            let status = response.status();

            if !status.is_success() {
                return Err(SteamWebApiError::platform_unavailable(Some(format!(
                    "resposta HTTP {} da Steam Store API",
                    status.as_u16()
                ))));
            }

            response.text().map_err(|_| {
                SteamWebApiError::platform_unavailable(Some(
                    "nao foi possivel ler o corpo da resposta da Steam Store API".to_string(),
                ))
            })
        }

        fn get_player_achievements(&self, url: &Url) -> Result<String, SteamWebApiError> {
            request_steam_web_api_text(&self.client, url, STEAM_OWNED_GAMES_TIMEOUT)
        }

        fn get_schema_for_game(&self, url: &Url) -> Result<String, SteamWebApiError> {
            request_steam_web_api_text(&self.client, url, STEAM_OWNED_GAMES_TIMEOUT)
        }
    }

    fn request_steam_web_api_text(
        client: &reqwest::blocking::Client,
        url: &Url,
        timeout: Duration,
    ) -> Result<String, SteamWebApiError> {
        let response = client
            .get(url.clone())
            .timeout(timeout)
            .send()
            .map_err(|error| SteamWebApiError::network_unavailable_from_reqwest(Some(&error)))?;
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

        pub(crate) fn auth_required(details_sanitized: Option<String>) -> Self {
            Self::new(
                "steam_web_api_auth_required",
                "Nao foi possivel autenticar na Steam Web API. Verifique a chave salva no cofre.",
                true,
                "request",
                details_sanitized,
            )
        }

        fn network_unavailable_from_reqwest(error: Option<&reqwest::Error>) -> Self {
            let detail = error
                .map(|error| {
                    if error.is_timeout() {
                        "timeout ao consultar a Steam Web API"
                    } else if error.is_connect() {
                        "falha de conexao ao consultar a Steam Web API"
                    } else if error.is_request() {
                        "falha ao montar requisicao para a Steam Web API"
                    } else {
                        "falha de rede ao consultar a Steam Web API"
                    }
                })
                .unwrap_or("falha de rede ao consultar a Steam Web API")
                .to_string();

            Self::new(
                "steam_web_api_network_unavailable",
                "Nao foi possivel conectar a Steam Web API. Verifique a conexao e tente novamente.",
                true,
                "request",
                Some(detail),
            )
        }

        #[cfg(test)]
        pub(crate) fn network_unavailable_for_test() -> Self {
            Self::network_unavailable_from_reqwest(None)
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

        pub(crate) fn rate_limited() -> Self {
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SteamAchievementSchema {
        pub raw_json: String,
        pub total_count: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SteamPlayerAchievements {
        pub raw_json: String,
        pub total_count: usize,
        pub unlocked_count: usize,
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

    pub fn cached_appdetails_header_image(app_id: &str) -> Option<String> {
        appdetails_header_image_cache()
            .lock()
            .ok()
            .and_then(|cache| cache.get(app_id).cloned())
    }

    pub fn fetch_appdetails_header_image(
        client: &impl SteamWebApiClient,
        app_id: &str,
    ) -> Result<Option<String>, SteamWebApiError> {
        if !is_valid_steam_app_id(app_id) {
            return Ok(None);
        }

        let mut url = Url::parse(STEAM_APPDETAILS_ENDPOINT)
            .map_err(|_| SteamWebApiError::parse_failed(Some("endpoint invalido".to_string())))?;
        url.query_pairs_mut()
            .append_pair("appids", app_id)
            .append_pair("filters", "basic");

        let body = client.get_appdetails(&url)?;
        parse_appdetails_header_image(app_id, &body)
    }

    pub fn fetch_achievement_schema(
        client: &impl SteamWebApiClient,
        api_key: &str,
        app_id: &str,
    ) -> Result<SteamAchievementSchema, SteamWebApiError> {
        if !is_valid_steam_app_id(app_id) {
            return Err(SteamWebApiError::parse_failed(Some(
                "appid Steam invalido".to_string(),
            )));
        }

        let mut url = Url::parse(STEAM_ACHIEVEMENT_SCHEMA_ENDPOINT)
            .map_err(|_| SteamWebApiError::parse_failed(Some("endpoint invalido".to_string())))?;
        url.query_pairs_mut()
            .append_pair("key", api_key)
            .append_pair("appid", app_id)
            .append_pair("format", "json");

        let body = client.get_schema_for_game(&url)?;
        parse_achievement_schema_response(&body)
    }

    pub fn fetch_player_achievements(
        client: &impl SteamWebApiClient,
        api_key: &str,
        steam_id: &str,
        app_id: &str,
    ) -> Result<SteamPlayerAchievements, SteamWebApiError> {
        if !is_valid_steam_app_id(app_id) {
            return Err(SteamWebApiError::parse_failed(Some(
                "appid Steam invalido".to_string(),
            )));
        }

        let mut url = Url::parse(STEAM_PLAYER_ACHIEVEMENTS_ENDPOINT)
            .map_err(|_| SteamWebApiError::parse_failed(Some("endpoint invalido".to_string())))?;
        url.query_pairs_mut()
            .append_pair("key", api_key)
            .append_pair("steamid", steam_id)
            .append_pair("appid", app_id)
            .append_pair("l", "brazilian")
            .append_pair("format", "json");

        let body = client.get_player_achievements(&url)?;
        parse_player_achievements_response(&body)
    }

    fn parse_appdetails_header_image(
        app_id: &str,
        body: &str,
    ) -> Result<Option<String>, SteamWebApiError> {
        let payload: serde_json::Value = serde_json::from_str(body).map_err(|_| {
            SteamWebApiError::parse_failed(Some(
                "payload JSON invalido da Steam Store API".to_string(),
            ))
        })?;
        let header_image = payload
            .get(app_id)
            .and_then(|entry| entry.get("success"))
            .and_then(|success| success.as_bool())
            .filter(|success| *success)
            .and_then(|_| payload.get(app_id))
            .and_then(|entry| entry.get("data"))
            .and_then(|data| data.get("header_image"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| is_safe_image_url(value))
            .map(str::to_string);

        Ok(header_image)
    }

    pub(crate) fn cache_appdetails_header_image(app_id: &str, header_image: String) {
        if let Ok(mut cache) = appdetails_header_image_cache().lock() {
            cache.insert(app_id.to_string(), header_image);
        }
    }

    fn appdetails_header_image_cache() -> &'static Mutex<HashMap<String, String>> {
        static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

        CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn is_valid_steam_app_id(app_id: &str) -> bool {
        !app_id.is_empty() && app_id.bytes().all(|byte| byte.is_ascii_digit())
    }

    fn is_safe_image_url(value: &str) -> bool {
        Url::parse(value)
            .ok()
            .filter(|url| url.scheme() == "https")
            .and_then(|url| {
                url.host_str().map(str::to_ascii_lowercase).filter(|host| {
                    matches!(
                        host.as_str(),
                        "cdn.akamai.steamstatic.com"
                            | "shared.akamai.steamstatic.com"
                            | "steamcdn-a.akamaihd.net"
                    )
                })
            })
            .is_some()
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

    fn parse_achievement_schema_response(
        body: &str,
    ) -> Result<SteamAchievementSchema, SteamWebApiError> {
        let payload: serde_json::Value = serde_json::from_str(body).map_err(|_| {
            SteamWebApiError::parse_failed(Some(
                "payload JSON invalido do schema de achievements Steam".to_string(),
            ))
        })?;
        let achievements = payload
            .get("game")
            .and_then(|game| game.get("availableGameStats"))
            .and_then(|stats| stats.get("achievements"))
            .and_then(|achievements| achievements.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(SteamAchievementSchema {
            raw_json: serde_json::Value::Array(achievements.clone()).to_string(),
            total_count: achievements.len(),
        })
    }

    fn parse_player_achievements_response(
        body: &str,
    ) -> Result<SteamPlayerAchievements, SteamWebApiError> {
        let payload: serde_json::Value = serde_json::from_str(body).map_err(|_| {
            SteamWebApiError::parse_failed(Some(
                "payload JSON invalido dos achievements Steam".to_string(),
            ))
        })?;
        if payload
            .get("playerstats")
            .and_then(|stats| stats.get("success"))
            .and_then(|success| success.as_bool())
            .is_some_and(|success| !success)
        {
            return Ok(SteamPlayerAchievements {
                raw_json: "[]".to_string(),
                total_count: 0,
                unlocked_count: 0,
            });
        }
        let achievements = payload
            .get("playerstats")
            .and_then(|stats| stats.get("achievements"))
            .and_then(|achievements| achievements.as_array())
            .cloned()
            .unwrap_or_default();
        let unlocked_count = achievements
            .iter()
            .filter(|achievement| {
                achievement
                    .get("achieved")
                    .and_then(parse_i64_value)
                    .unwrap_or_default()
                    > 0
            })
            .count();

        Ok(SteamPlayerAchievements {
            raw_json: serde_json::Value::Array(achievements.clone()).to_string(),
            total_count: achievements.len(),
            unlocked_count,
        })
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

        #[test]
        fn fetch_appdetails_header_image_reads_store_header_image() {
            let client = FakeSteamWebApiClient::with_appdetails_body(
                r#"{"413150":{"success":true,"data":{"header_image":"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/413150/header.jpg"}}}"#,
            );

            let header_image = fetch_appdetails_header_image(&client, "413150")
                .expect("fetch appdetails header image");
            let url = client.last_appdetails_url();

            assert_eq!(
                header_image.as_deref(),
                Some("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/413150/header.jpg")
            );
            assert_eq!(url.path(), "/api/appdetails");
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "appids")
                    .unwrap()
                    .1,
                "413150"
            );
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "filters")
                    .unwrap()
                    .1,
                "basic"
            );
        }

        #[test]
        fn parse_appdetails_header_image_ignores_failed_or_unsafe_payloads() {
            assert_eq!(
                parse_appdetails_header_image(
                    "413150",
                    r#"{"413150":{"success":false,"data":{"header_image":"https://example.test/header.jpg"}}}"#,
                )
                .expect("parse failed appdetails"),
                None
            );
            assert_eq!(
                parse_appdetails_header_image(
                    "413150",
                    r#"{"413150":{"success":true,"data":{"header_image":"javascript:alert(1)"}}}"#,
                )
                .expect("parse unsafe appdetails"),
                None
            );
            assert_eq!(
                parse_appdetails_header_image(
                    "413150",
                    r#"{"413150":{"success":true,"data":{"header_image":"http://shared.akamai.steamstatic.com/store_item_assets/steam/apps/413150/header.jpg"}}}"#,
                )
                .expect("parse insecure appdetails"),
                None
            );
            assert_eq!(
                parse_appdetails_header_image(
                    "413150",
                    r#"{"413150":{"success":true,"data":{"header_image":"https://example.test/header.jpg"}}}"#,
                )
                .expect("parse non-steam appdetails"),
                None
            );
        }

        #[test]
        fn fetch_achievement_schema_counts_definitions() {
            let mut client = FakeSteamWebApiClient::default();
            client.schema_body = Some(
                r#"{"game":{"availableGameStats":{"achievements":[{"name":"FIRST"},{"name":"SECOND"}]}}}"#
                    .to_string(),
            );

            let schema =
                fetch_achievement_schema(&client, "0123456789abcdefABCDEF0123456789", "413150")
                    .expect("fetch achievement schema");

            assert_eq!(schema.total_count, 2);
            assert!(schema.raw_json.contains("FIRST"));
        }

        #[test]
        fn fetch_player_achievements_counts_unlocked_items() {
            let mut client = FakeSteamWebApiClient::default();
            client.achievements_body = Some(
                r#"{"playerstats":{"achievements":[{"apiname":"FIRST","achieved":1},{"apiname":"SECOND","achieved":0},{"apiname":"THIRD","achieved":"1"}]}}"#
                    .to_string(),
            );

            let achievements = fetch_player_achievements(
                &client,
                "0123456789abcdefABCDEF0123456789",
                "76561198000000000",
                "413150",
            )
            .expect("fetch player achievements");

            assert_eq!(achievements.total_count, 3);
            assert_eq!(achievements.unlocked_count, 2);
            assert!(achievements.raw_json.contains("THIRD"));
        }

        #[test]
        fn fetch_player_achievements_treats_private_or_missing_data_as_empty_cache() {
            let mut client = FakeSteamWebApiClient::default();
            client.achievements_body = Some(
                r#"{"playerstats":{"success":false,"error":"Profile is not public"}}"#.to_string(),
            );

            let achievements = fetch_player_achievements(
                &client,
                "0123456789abcdefABCDEF0123456789",
                "76561198000000000",
                "413150",
            )
            .expect("private achievements should not fail enrichment");

            assert_eq!(achievements.total_count, 0);
            assert_eq!(achievements.unlocked_count, 0);
            assert_eq!(achievements.raw_json, "[]");
            assert!(!achievements.raw_json.contains("Profile is not public"));
        }

        #[derive(Default)]
        struct FakeSteamWebApiClient {
            body: Option<String>,
            appdetails_body: Option<String>,
            achievements_body: Option<String>,
            schema_body: Option<String>,
            error: Option<SteamWebApiError>,
            last_url: Mutex<Option<Url>>,
            last_appdetails_url: Mutex<Option<Url>>,
            last_achievements_url: Mutex<Option<Url>>,
            last_schema_url: Mutex<Option<Url>>,
        }

        impl FakeSteamWebApiClient {
            fn with_body(body: &str) -> Self {
                Self {
                    body: Some(body.to_string()),
                    appdetails_body: None,
                    achievements_body: None,
                    schema_body: None,
                    error: None,
                    last_url: Mutex::new(None),
                    last_appdetails_url: Mutex::new(None),
                    last_achievements_url: Mutex::new(None),
                    last_schema_url: Mutex::new(None),
                }
            }

            fn with_appdetails_body(body: &str) -> Self {
                Self {
                    body: None,
                    appdetails_body: Some(body.to_string()),
                    achievements_body: None,
                    schema_body: None,
                    error: None,
                    last_url: Mutex::new(None),
                    last_appdetails_url: Mutex::new(None),
                    last_achievements_url: Mutex::new(None),
                    last_schema_url: Mutex::new(None),
                }
            }

            fn with_error(error: SteamWebApiError) -> Self {
                Self {
                    body: None,
                    appdetails_body: None,
                    achievements_body: None,
                    schema_body: None,
                    error: Some(error),
                    last_url: Mutex::new(None),
                    last_appdetails_url: Mutex::new(None),
                    last_achievements_url: Mutex::new(None),
                    last_schema_url: Mutex::new(None),
                }
            }

            fn last_url(&self) -> Url {
                self.last_url
                    .lock()
                    .expect("lock url")
                    .clone()
                    .expect("url captured")
            }

            fn last_appdetails_url(&self) -> Url {
                self.last_appdetails_url
                    .lock()
                    .expect("lock appdetails url")
                    .clone()
                    .expect("appdetails url captured")
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

            fn get_appdetails(&self, url: &Url) -> Result<String, SteamWebApiError> {
                *self
                    .last_appdetails_url
                    .lock()
                    .expect("lock appdetails url") = Some(url.clone());

                if let Some(error) = &self.error {
                    return Err(error.clone());
                }

                Ok(self.appdetails_body.clone().unwrap_or_default())
            }

            fn get_player_achievements(&self, url: &Url) -> Result<String, SteamWebApiError> {
                *self
                    .last_achievements_url
                    .lock()
                    .expect("lock achievements url") = Some(url.clone());

                if let Some(error) = &self.error {
                    return Err(error.clone());
                }

                Ok(self.achievements_body.clone().unwrap_or_default())
            }

            fn get_schema_for_game(&self, url: &Url) -> Result<String, SteamWebApiError> {
                *self.last_schema_url.lock().expect("lock schema url") = Some(url.clone());

                if let Some(error) = &self.error {
                    return Err(error.clone());
                }

                Ok(self.schema_body.clone().unwrap_or_default())
            }
        }
    }
}

mod steam_openid {
    #[cfg(not(test))]
    use super::storage;
    use rand::{rngs::OsRng, RngCore};
    use serde::Serialize;
    use std::fmt;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    #[cfg(not(test))]
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    #[cfg(not(test))]
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

    #[cfg(not(test))]
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
        let open_started_at = std::time::Instant::now();
        log::info!("[library-boot] backend.open_database.start");
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let migrate_started_at = std::time::Instant::now();
        migrate(&connection)?;
        log::info!(
            "[library-boot] backend.open_database.migrate_complete elapsed_ms={}",
            migrate_started_at.elapsed().as_millis()
        );
        let compatibility_started_at = std::time::Instant::now();
        ensure_archived_column(&connection)?;
        ensure_favorite_column(&connection)?;
        ensure_active_entries_index(&connection)?;
        ensure_favorite_entries_index(&connection)?;
        ensure_local_cleanup_indexes(&connection)?;
        ensure_provider_account_configs_table(&connection)?;
        log::info!(
            "[library-boot] backend.open_database.compatibility_complete elapsed_ms={}",
            compatibility_started_at.elapsed().as_millis()
        );
        let cleanup_started_at = std::time::Instant::now();
        let cleaned_entries = archive_rejected_local_entries(&connection)?;
        log::info!(
            "[library-boot] backend.local_cleanup.complete affected={} elapsed_ms={}",
            cleaned_entries,
            cleanup_started_at.elapsed().as_millis()
        );
        log::info!(
            "[library-boot] backend.open_database.complete elapsed_ms={}",
            open_started_at.elapsed().as_millis()
        );
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
        is_favorite INTEGER NOT NULL DEFAULT 0,
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

      CREATE TABLE IF NOT EXISTS steam_artwork_cache (
        app_id TEXT PRIMARY KEY,
        header_image_url TEXT NOT NULL,
        fetched_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS steam_achievement_schema_cache (
        app_id TEXT PRIMARY KEY,
        schema_json TEXT NOT NULL,
        achievement_count INTEGER NOT NULL DEFAULT 0,
        fetched_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS steam_player_achievement_cache (
        steam_id64 TEXT NOT NULL,
        app_id TEXT NOT NULL,
        achievements_json TEXT NOT NULL,
        unlocked_count INTEGER NOT NULL DEFAULT 0,
        total_count INTEGER NOT NULL DEFAULT 0,
        fetched_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (steam_id64, app_id)
      );

      CREATE TABLE IF NOT EXISTS steam_enrichment_attempt_cache (
        steam_id64 TEXT NOT NULL DEFAULT '',
        app_id TEXT NOT NULL,
        phase TEXT NOT NULL,
        outcome TEXT NOT NULL,
        attempted_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (steam_id64, app_id, phase)
      );

      CREATE INDEX IF NOT EXISTS idx_games_sort_title ON games(sort_title);
      CREATE INDEX IF NOT EXISTS idx_library_entries_install_status ON library_entries(install_status);
      CREATE INDEX IF NOT EXISTS idx_library_entries_platform ON library_entries(primary_platform_id);
      CREATE INDEX IF NOT EXISTS idx_launch_actions_game_primary ON launch_actions(game_id, is_primary);
      CREATE INDEX IF NOT EXISTS idx_steam_achievement_schema_expires ON steam_achievement_schema_cache(expires_at);
      CREATE INDEX IF NOT EXISTS idx_steam_player_achievement_expires ON steam_player_achievement_cache(steam_id64, expires_at);
      CREATE INDEX IF NOT EXISTS idx_steam_enrichment_attempt_expires ON steam_enrichment_attempt_cache(phase, expires_at);
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
        connection.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (3, ?1)",
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
        is_favorite: bool,
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
    pub struct XboxLiveClientConfigInput {
        client_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct XboxLiveClientConfigDto {
        provider_id: &'static str,
        configured: bool,
        client_id: Option<String>,
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
    pub struct XboxLibraryRootsInput {
        roots: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct XboxLibraryRootsDto {
        provider_id: &'static str,
        roots: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct LibrarySettingsInput {
        #[serde(default)]
        preferred_store_id: String,
        #[serde(default)]
        local_scan_mode: String,
        #[serde(default)]
        local_scan_roots: Vec<String>,
        #[serde(default)]
        local_scan_excluded_roots: Vec<String>,
        #[serde(default)]
        microsoft_client_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LibrarySettingsDto {
        preferred_store_id: String,
        local_scan_mode: String,
        local_scan_roots: Vec<String>,
        local_scan_excluded_roots: Vec<String>,
        microsoft_client_id: String,
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

    pub fn get_xbox_live_client_config(
        connection: &Connection,
    ) -> rusqlite::Result<XboxLiveClientConfigDto> {
        let client_id = read_xbox_live_client_id(connection)?;

        Ok(xbox_live_client_config_dto(client_id))
    }

    pub fn save_xbox_live_client_config(
        connection: &mut Connection,
        input: XboxLiveClientConfigInput,
    ) -> rusqlite::Result<XboxLiveClientConfigDto> {
        let client_id = normalize_xbox_live_client_id(&input.client_id)
            .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
        save_xbox_live_client_id(connection, &client_id)?;

        Ok(xbox_live_client_config_dto(Some(client_id)))
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

    pub fn get_xbox_library_roots(
        connection: &Connection,
    ) -> rusqlite::Result<XboxLibraryRootsDto> {
        Ok(xbox_library_roots_dto(read_xbox_library_roots(connection)?))
    }

    pub fn save_xbox_library_roots(
        connection: &mut Connection,
        input: XboxLibraryRootsInput,
    ) -> rusqlite::Result<XboxLibraryRootsDto> {
        let roots = normalize_xbox_library_roots(&input.roots)?;
        let transaction = connection.transaction()?;
        let existing_config_json = read_provider_config_json(&transaction, "xbox")?;
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
            "additionalGameRoots".to_string(),
            serde_json::Value::Array(roots_json),
        );

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              config_json,
              updated_at
            )
            VALUES ('xbox', ?1, ?2)
            ON CONFLICT(provider_id) DO UPDATE SET
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![serde_json::Value::Object(config).to_string(), now_iso()],
        )?;
        transaction.commit()?;

        Ok(xbox_library_roots_dto(roots))
    }

    pub fn get_library_settings(connection: &Connection) -> rusqlite::Result<LibrarySettingsDto> {
        let config = read_library_settings_config(connection)?;

        Ok(LibrarySettingsDto {
            preferred_store_id: normalize_preferred_store_id(
                config
                    .as_ref()
                    .and_then(|value| value.get("preferredStoreId"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("steam"),
            ),
            local_scan_mode: normalize_local_scan_mode(
                config
                    .as_ref()
                    .and_then(|value| value.get("localScanMode"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("automatic"),
            ),
            local_scan_roots: read_library_settings_string_array(&config, "localScanRoots"),
            local_scan_excluded_roots: read_library_settings_string_array(
                &config,
                "localScanExcludedRoots",
            ),
            microsoft_client_id: normalize_xbox_live_client_id(
                config
                    .as_ref()
                    .and_then(|value| value.get("microsoftClientId"))
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            )
            .unwrap_or_default(),
        })
    }

    pub fn save_library_settings(
        connection: &mut Connection,
        input: LibrarySettingsInput,
    ) -> rusqlite::Result<LibrarySettingsDto> {
        let preferred_store_id = normalize_preferred_store_id(&input.preferred_store_id);
        let local_scan_mode = normalize_local_scan_mode(&input.local_scan_mode);
        let local_scan_roots = normalize_library_scan_roots(&input.local_scan_roots)?;
        let local_scan_excluded_roots =
            normalize_library_scan_roots(&input.local_scan_excluded_roots)?;
        let microsoft_client_id = normalize_xbox_live_client_id(&input.microsoft_client_id)
            .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.transaction()?;
        let existing_config_json = read_provider_config_json(&transaction, "library")?;
        let mut config = existing_config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        config.insert(
            "preferredStoreId".to_string(),
            serde_json::Value::String(preferred_store_id.clone()),
        );
        config.insert(
            "localScanMode".to_string(),
            serde_json::Value::String(local_scan_mode.clone()),
        );
        config.insert(
            "localScanRoots".to_string(),
            serde_json::Value::Array(
                local_scan_roots
                    .iter()
                    .map(|root| serde_json::Value::String(root.clone()))
                    .collect(),
            ),
        );
        config.insert(
            "localScanExcludedRoots".to_string(),
            serde_json::Value::Array(
                local_scan_excluded_roots
                    .iter()
                    .map(|root| serde_json::Value::String(root.clone()))
                    .collect(),
            ),
        );
        config.insert(
            "microsoftClientId".to_string(),
            serde_json::Value::String(microsoft_client_id.clone()),
        );

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              config_json,
              updated_at
            )
            VALUES ('xbox', ?1, ?2)
            ON CONFLICT(provider_id) DO UPDATE SET
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![serde_json::Value::Object(config).to_string(), now_iso()],
        )?;
        transaction.commit()?;

        Ok(LibrarySettingsDto {
            preferred_store_id,
            local_scan_mode,
            local_scan_roots,
            local_scan_excluded_roots,
            microsoft_client_id,
        })
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
            params![
                xuid,
                serde_json::Value::Object(config).to_string(),
                now_iso()
            ],
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

    fn xbox_live_client_config_dto(client_id: Option<String>) -> XboxLiveClientConfigDto {
        XboxLiveClientConfigDto {
            provider_id: "xbox",
            configured: client_id.is_some(),
            client_id,
        }
    }

    fn steam_library_roots_dto(roots: Vec<String>) -> SteamLibraryRootsDto {
        SteamLibraryRootsDto {
            provider_id: "steam",
            roots,
        }
    }

    fn xbox_library_roots_dto(roots: Vec<String>) -> XboxLibraryRootsDto {
        XboxLibraryRootsDto {
            provider_id: "xbox",
            roots,
        }
    }

    fn normalize_preferred_store_id(value: &str) -> String {
        match value.trim().to_lowercase().as_str() {
            "xbox" => "xbox".to_string(),
            _ => "steam".to_string(),
        }
    }

    fn normalize_local_scan_mode(value: &str) -> String {
        match value.trim().to_lowercase().as_str() {
            "selected_only" => "selected_only".to_string(),
            "automatic_plus_extra" => "automatic_plus_extra".to_string(),
            _ => "automatic".to_string(),
        }
    }

    fn read_library_settings_config(
        connection: &Connection,
    ) -> rusqlite::Result<Option<serde_json::Map<String, serde_json::Value>>> {
        let columns = table_columns(connection, "provider_account_configs")?;
        if columns.is_empty() {
            return Ok(None);
        }

        let config_json = read_provider_config_json(connection, "xbox")?
            .or(read_provider_config_json(connection, "library")?);

        Ok(config_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|json| json.as_object().cloned()))
    }

    fn read_library_settings_string_array(
        config: &Option<serde_json::Map<String, serde_json::Value>>,
        key: &str,
    ) -> Vec<String> {
        config
            .as_ref()
            .and_then(|object| object.get(key))
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect::<Vec<_>>()
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

    pub(crate) fn read_xbox_library_roots(
        connection: &Connection,
    ) -> rusqlite::Result<Vec<String>> {
        let columns = table_columns(connection, "provider_account_configs")?;
        if columns.is_empty() {
            return Ok(Vec::new());
        }

        Ok(read_provider_config_json(connection, "xbox")?
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .and_then(|object| object.get("additionalGameRoots").cloned())
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect::<Vec<_>>())
    }

    pub(crate) fn read_xbox_live_client_id(
        connection: &Connection,
    ) -> rusqlite::Result<Option<String>> {
        let columns = table_columns(connection, "provider_account_configs")?;
        if columns.is_empty() {
            return Ok(None);
        }

        for column in ["config_json", "config"] {
            if !columns.iter().any(|existing| existing == column) {
                continue;
            }

            if let Some(value) =
                read_provider_account_config_value(connection, &columns, "xbox", column)?
            {
                if let Some(client_id) = xbox_live_client_id_from_config_json(&value) {
                    return Ok(Some(client_id));
                }
            }
        }

        Ok(None)
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

    fn normalize_library_scan_roots(values: &[String]) -> rusqlite::Result<Vec<String>> {
        let mut roots = Vec::new();

        for value in values {
            let trimmed = value.trim().trim_matches('"');
            if trimmed.is_empty() {
                continue;
            }

            let root = normalize_local_scan_root(value)
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

    fn normalize_local_scan_root(value: &str) -> Option<PathBuf> {
        let trimmed = value.trim().trim_matches('"');
        if trimmed.is_empty() {
            return None;
        }

        let path = PathBuf::from(trimmed);
        if !path.is_dir() {
            return None;
        }

        Some(path.canonicalize().unwrap_or(path))
    }

    fn normalize_xbox_library_roots(values: &[String]) -> rusqlite::Result<Vec<String>> {
        let mut roots = Vec::new();

        for value in values {
            let trimmed = value.trim().trim_matches('"');
            if trimmed.is_empty() {
                continue;
            }

            let root = normalize_xbox_library_root(value)
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

    fn normalize_xbox_library_root(value: &str) -> Option<PathBuf> {
        let trimmed = value.trim().trim_matches('"');
        if trimmed.is_empty() {
            return None;
        }

        let path = PathBuf::from(trimmed);
        let library_root = if is_xbox_games_path(&path) {
            path.parent().map(Path::to_path_buf)?
        } else {
            path
        };
        let xbox_games_dir = library_root.join("XboxGames");

        if xbox_games_dir.is_dir() {
            return Some(library_root.canonicalize().unwrap_or(library_root));
        }

        None
    }

    fn is_xbox_games_path(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("xboxgames"))
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

        config.insert("xuid".to_string(), serde_json::Value::String(xuid.clone()));
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
            params![
                xuid,
                serde_json::Value::Object(config).to_string(),
                now_iso()
            ],
        )?;
        transaction.commit()?;

        Ok(())
    }

    pub fn save_xbox_live_client_id(
        connection: &mut Connection,
        client_id: &str,
    ) -> rusqlite::Result<()> {
        let client_id = normalize_xbox_live_client_id(client_id)
            .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
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
            "microsoftClientId".to_string(),
            serde_json::Value::String(client_id.clone()),
        );

        transaction.execute(
            r#"
            INSERT INTO provider_account_configs (
              provider_id,
              config_json,
              updated_at
            )
            VALUES ('xbox', ?1, ?2)
            ON CONFLICT(provider_id) DO UPDATE SET
              config_json = excluded.config_json,
              updated_at = excluded.updated_at
            "#,
            params![serde_json::Value::Object(config).to_string(), now_iso()],
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
        achievements: Option<GameAchievementsDto>,
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
        cover_url: Option<String>,
        hero_url: Option<String>,
        fallback_url: Option<String>,
        source: Option<String>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct GameAchievementsDto {
        provider_id: String,
        app_id: String,
        total: usize,
        unlocked: usize,
        percentage: f64,
        fetched_at: String,
        items: Vec<GameAchievementDto>,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct GameAchievementDto {
        api_name: String,
        name: String,
        description: String,
        icon_url: Option<String>,
        locked_icon_url: Option<String>,
        hidden: bool,
        achieved: bool,
        unlock_time: Option<i64>,
    }

    pub fn list_library_entries(connection: &Connection) -> rusqlite::Result<Vec<LibraryEntryDto>> {
        let list_started_at = std::time::Instant::now();
        let mut statement = connection.prepare(
            r#"
      SELECT
        library_entries.id,
        library_entries.primary_platform_id,
        library_entries.install_status,
        library_entries.last_played_label,
        library_entries.is_archived,
        library_entries.is_favorite,
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
            let game_id: String = row.get(8)?;
            Ok(EntryRow {
                entry_id: row.get(0)?,
                primary_platform_id: row.get(1)?,
                install_status: row.get(2)?,
                last_played_label: row.get(3)?,
                is_archived: row.get::<_, i64>(4)? == 1,
                is_favorite: row.get::<_, i64>(5)? == 1,
                added_at: row.get(6)?,
                updated_at: row.get(7)?,
                game_id,
                title: row.get(9)?,
                sort_title: row.get(10)?,
                installed: row.get::<_, i64>(11)? == 1,
                playtime_total_minutes: row.get(12)?,
                accent_color: row.get(13)?,
                source_account_id: None,
            })
        })?;
        let query_ms = list_started_at.elapsed().as_millis();
        let hydrate_started_at = std::time::Instant::now();

        let entries = rows
            .map(|row| row.and_then(|entry| hydrate_entry(connection, entry)))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let steam_entries = entries
            .iter()
            .filter(|entry| entry.primary_platform_id == "steam")
            .count();
        let local_entries = entries
            .iter()
            .filter(|entry| entry.primary_platform_id == "local")
            .count();
        let achievement_entries = entries
            .iter()
            .filter(|entry| entry.game.achievements.is_some())
            .count();
        log::info!(
            "[library-boot] backend.storage.list_library_entries.complete entries={} steam_entries={} local_entries={} achievement_entries={} query_ms={} hydrate_ms={} total_ms={}",
            entries.len(),
            steam_entries,
            local_entries,
            achievement_entries,
            query_ms,
            hydrate_started_at.elapsed().as_millis(),
            list_started_at.elapsed().as_millis()
        );

        Ok(entries)
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
        library_entries.is_favorite,
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
            let game_id: String = row.get(8)?;
            Ok(EntryRow {
                entry_id: row.get(0)?,
                primary_platform_id: row.get(1)?,
                install_status: row.get(2)?,
                last_played_label: row.get(3)?,
                is_archived: row.get::<_, i64>(4)? == 1,
                is_favorite: row.get::<_, i64>(5)? == 1,
                added_at: row.get(6)?,
                updated_at: row.get(7)?,
                game_id,
                title: row.get(9)?,
                sort_title: row.get(10)?,
                installed: row.get::<_, i64>(11)? == 1,
                playtime_total_minutes: row.get(12)?,
                accent_color: row.get(13)?,
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
        is_favorite: bool,
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
          library_entries.is_favorite,
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
                        is_favorite: row.get::<_, i64>(5)? == 1,
                        added_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        game_id: row.get(8)?,
                        title: row.get(9)?,
                        sort_title: row.get(10)?,
                        installed: row.get::<_, i64>(11)? == 1,
                        playtime_total_minutes: row.get(12)?,
                        accent_color: row.get(13)?,
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

        let artwork = game_artwork_from_sources(connection, row.accent_color, &sources);
        let achievements = game_achievements_from_sources(connection, &sources)?;

        Ok(LibraryEntryDto {
            id: row.entry_id,
            primary_platform_id: row.primary_platform_id.clone(),
            install_status: row.install_status,
            last_played_label: row.last_played_label,
            is_archived: row.is_archived,
            is_favorite: row.is_favorite,
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
                artwork,
                achievements,
                genres,
                tags: Vec::new(),
                user_overrides: serde_json::json!({}),
            },
        })
    }

    fn game_artwork_from_sources(
        connection: &Connection,
        accent_color: Option<String>,
        sources: &[GameSourceDto],
    ) -> GameArtworkDto {
        sources
            .iter()
            .find_map(|source| steam_artwork_app_id(source).map(|app_id| (source, app_id)))
            .map(|(_, app_id)| GameArtworkDto {
                accent_color: accent_color.clone(),
                cover_url: Some(format!(
                    "https://cdn.akamai.steamstatic.com/steam/apps/{app_id}/library_600x900.jpg"
                )),
                hero_url: Some(format!(
                    "https://cdn.akamai.steamstatic.com/steam/apps/{app_id}/library_hero.jpg"
                )),
                fallback_url: read_steam_artwork_header_image(connection, app_id)
                    .ok()
                    .flatten()
                    .or_else(|| steam_web_api::cached_appdetails_header_image(app_id))
                    .or_else(|| {
                        Some(format!(
                            "https://cdn.akamai.steamstatic.com/steam/apps/{app_id}/header.jpg"
                        ))
                    }),
                source: Some("steam".to_string()),
            })
            .unwrap_or(GameArtworkDto {
                accent_color,
                cover_url: None,
                hero_url: None,
                fallback_url: None,
                source: None,
            })
    }

    fn steam_artwork_app_id(source: &GameSourceDto) -> Option<&str> {
        let app_id = source.external_id.trim();

        if source.platform_id == "steam"
            && !app_id.is_empty()
            && app_id.bytes().all(|byte| byte.is_ascii_digit())
        {
            Some(app_id)
        } else {
            None
        }
    }

    fn game_achievements_from_sources(
        connection: &Connection,
        sources: &[GameSourceDto],
    ) -> rusqlite::Result<Option<GameAchievementsDto>> {
        let Some(source) = sources.iter().find(|source| {
            source.platform_id == "steam"
                && steam_web_api::is_valid_steam_app_id(&source.external_id)
        }) else {
            return Ok(None);
        };
        let Some(steam_id64) = source
            .account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };

        read_steam_achievements(connection, steam_id64, &source.external_id)
    }

    fn read_steam_achievements(
        connection: &Connection,
        steam_id64: &str,
        app_id: &str,
    ) -> rusqlite::Result<Option<GameAchievementsDto>> {
        let player_cache = connection
            .query_row(
                r#"
                SELECT achievements_json, unlocked_count, total_count, fetched_at
                FROM steam_player_achievement_cache
                WHERE steam_id64 = ?1
                  AND app_id = ?2
                "#,
                params![steam_id64, app_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((player_json, unlocked_count, player_total_count, fetched_at)) = player_cache
        else {
            return Ok(None);
        };

        let schema_cache = connection
            .query_row(
                r#"
                SELECT schema_json, achievement_count
                FROM steam_achievement_schema_cache
                WHERE app_id = ?1
                "#,
                params![app_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let (schema_json, schema_count) = schema_cache.unwrap_or_else(|| ("[]".to_string(), 0));
        let schema_items = parse_steam_achievement_array(&schema_json);
        let player_items = parse_steam_achievement_array(&player_json);
        let player_by_api_name = player_items
            .iter()
            .filter_map(|achievement| {
                achievement_api_name(achievement)
                    .map(|api_name| (api_name.to_string(), achievement))
            })
            .collect::<std::collections::HashMap<_, _>>();
        let mut items = schema_items
            .iter()
            .filter_map(|achievement| {
                let api_name = achievement_api_name(achievement)?;
                let player_achievement = player_by_api_name.get(api_name);
                Some(build_steam_achievement_dto(
                    api_name,
                    Some(achievement),
                    player_achievement.copied(),
                ))
            })
            .collect::<Vec<_>>();

        if items.is_empty() {
            items = player_items
                .iter()
                .filter_map(|achievement| {
                    let api_name = achievement_api_name(achievement)?;
                    Some(build_steam_achievement_dto(
                        api_name,
                        None,
                        Some(achievement),
                    ))
                })
                .collect();
        }

        let total = usize::try_from(schema_count.max(player_total_count).max(items.len() as i64))
            .unwrap_or_default();
        let unlocked = usize::try_from(unlocked_count.max(0)).unwrap_or_default();
        let percentage = if total > 0 {
            ((unlocked as f64 / total as f64) * 1000.0).round() / 10.0
        } else {
            0.0
        };

        Ok(Some(GameAchievementsDto {
            provider_id: "steam".to_string(),
            app_id: app_id.to_string(),
            total,
            unlocked,
            percentage,
            fetched_at,
            items,
        }))
    }

    fn parse_steam_achievement_array(raw_json: &str) -> Vec<serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(raw_json)
            .ok()
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
    }

    fn build_steam_achievement_dto(
        api_name: &str,
        schema: Option<&serde_json::Value>,
        player: Option<&serde_json::Value>,
    ) -> GameAchievementDto {
        let name = read_string_field(schema, &["displayName", "display_name", "name"])
            .or_else(|| {
                read_string_field(player, &["displayName", "display_name", "name", "apiname"])
            })
            .unwrap_or_else(|| api_name.to_string());
        let description = read_string_field(schema, &["description", "desc"])
            .or_else(|| read_string_field(player, &["description", "desc"]))
            .unwrap_or_default();
        let hidden = read_bool_field(schema, &["hidden", "secret"]).unwrap_or(false);
        let achieved = player
            .and_then(|value| value.get("achieved"))
            .and_then(parse_i64_from_json)
            .unwrap_or_default()
            > 0;
        let unlock_time = player
            .and_then(|value| value.get("unlocktime"))
            .and_then(parse_i64_from_json)
            .filter(|value| *value > 0);

        GameAchievementDto {
            api_name: api_name.to_string(),
            name,
            description,
            icon_url: read_string_field(schema, &["icon"]),
            locked_icon_url: read_string_field(schema, &["icongray", "iconGray", "lockedIcon"]),
            hidden,
            achieved,
            unlock_time,
        }
    }

    fn achievement_api_name(value: &serde_json::Value) -> Option<&str> {
        value
            .get("apiname")
            .or_else(|| value.get("name"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn read_string_field(value: Option<&serde_json::Value>, fields: &[&str]) -> Option<String> {
        fields.iter().find_map(|field| {
            value?
                .get(*field)
                .and_then(|field_value| field_value.as_str())
                .map(str::trim)
                .filter(|field_value| !field_value.is_empty())
                .map(str::to_string)
        })
    }

    fn read_bool_field(value: Option<&serde_json::Value>, fields: &[&str]) -> Option<bool> {
        fields.iter().find_map(|field| {
            value?.get(*field).and_then(|field_value| {
                field_value.as_bool().or_else(|| {
                    field_value.as_i64().map(|raw| raw != 0).or_else(|| {
                        field_value
                            .as_str()
                            .map(|raw| matches!(raw.trim(), "1" | "true" | "TRUE"))
                    })
                })
            })
        })
    }

    fn parse_i64_from_json(value: &serde_json::Value) -> Option<i64> {
        value
            .as_i64()
            .or_else(|| {
                value
                    .as_u64()
                    .and_then(|raw| (raw <= i64::MAX as u64).then_some(raw as i64))
            })
            .or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<i64>().ok())
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

    fn ensure_favorite_column(connection: &Connection) -> rusqlite::Result<()> {
        if !table_has_column(connection, "library_entries", "is_favorite")? {
            connection.execute(
                "ALTER TABLE library_entries ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
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

    fn ensure_favorite_entries_index(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_library_entries_active_favorites ON library_entries(added_at DESC) WHERE is_archived = 0 AND is_favorite = 1",
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

    fn xbox_live_client_id_from_config_json(value: &str) -> Option<String> {
        let value: serde_json::Value = serde_json::from_str(value).ok()?;

        [
            "microsoftClientId",
            "microsoft_client_id",
            "clientId",
            "client_id",
        ]
        .into_iter()
        .find_map(|key| value.get(key).and_then(|value| value.as_str()))
        .and_then(normalize_xbox_live_client_id)
    }

    fn normalize_xbox_live_client_id(value: &str) -> Option<String> {
        let value = value.trim();
        if value.len() != 36 {
            return None;
        }

        let valid_format = value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                8 | 13 | 18 | 23 => character == '-',
                _ => character.is_ascii_hexdigit(),
            });

        if valid_format {
            Some(value.to_string())
        } else {
            None
        }
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
        let settings = get_library_settings(connection)?;
        let roots = collect_local_game_roots(&settings);
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

    pub fn list_steam_app_ids(connection: &Connection) -> rusqlite::Result<Vec<String>> {
        let now = now_iso();
        let mut statement = connection.prepare(
            r#"
            SELECT DISTINCT game_sources.external_id
            FROM game_sources
            LEFT JOIN steam_artwork_cache
              ON steam_artwork_cache.app_id = game_sources.external_id
            WHERE game_sources.platform_id = 'steam'
              AND steam_artwork_cache.header_image_url IS NULL
            ORDER BY game_sources.external_id
            "#,
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

        rows.filter_map(|row| match row {
            Ok(app_id) if steam_web_api::cached_appdetails_header_image(&app_id).is_none() => {
                match has_fresh_steam_enrichment_attempt(
                    connection,
                    None,
                    &app_id,
                    STEAM_ENRICHMENT_PHASE_ARTWORK,
                    &now,
                ) {
                    Ok(false) => Some(Ok(app_id)),
                    Ok(true) => None,
                    Err(error) => Some(Err(error)),
                }
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
    }

    pub fn read_steam_artwork_header_image(
        connection: &Connection,
        app_id: &str,
    ) -> rusqlite::Result<Option<String>> {
        connection
            .query_row(
                r#"
                SELECT header_image_url
                FROM steam_artwork_cache
                WHERE app_id = ?1
                "#,
                params![app_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn save_steam_artwork_header_image(
        connection: &Connection,
        app_id: &str,
        header_image_url: &str,
    ) -> rusqlite::Result<()> {
        let now = now_iso();
        connection.execute(
            r#"
            INSERT INTO steam_artwork_cache (
              app_id,
              header_image_url,
              fetched_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?3)
            ON CONFLICT(app_id) DO UPDATE SET
              header_image_url = excluded.header_image_url,
              fetched_at = excluded.fetched_at,
              updated_at = excluded.updated_at
            "#,
            params![app_id, header_image_url, now],
        )?;

        Ok(())
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SteamEnrichmentCandidate {
        pub app_id: String,
        pub needs_artwork: bool,
        pub needs_achievement_schema: bool,
        pub needs_player_achievements: bool,
    }

    #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SteamEnrichmentRetrySummaryDto {
        pub marked_games: usize,
        pub marked_attempts: usize,
        pub artwork: usize,
        pub achievement_schema: usize,
        pub player_achievements: usize,
    }

    pub const STEAM_ENRICHMENT_PHASE_ARTWORK: &str = "artwork";
    pub const STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA: &str = "achievement_schema";
    pub const STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS: &str = "player_achievements";
    pub const STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS: i64 = 30;
    pub const STEAM_ENRICHMENT_ACHIEVEMENT_RETRY_DAYS: i64 = 1;

    pub fn list_steam_enrichment_candidates(
        connection: &Connection,
        steam_id64: &str,
        app_ids: &[String],
        limit: usize,
        include_recent_attempts: bool,
    ) -> rusqlite::Result<Vec<SteamEnrichmentCandidate>> {
        let now = now_iso();
        let mut candidates = Vec::new();

        for app_id in app_ids
            .iter()
            .map(|app_id| app_id.trim())
            .filter(|app_id| steam_web_api::is_valid_steam_app_id(app_id))
        {
            let needs_artwork = read_steam_artwork_header_image(connection, app_id)?.is_none()
                && steam_web_api::cached_appdetails_header_image(app_id).is_none()
                && (include_recent_attempts
                    || !has_fresh_steam_enrichment_attempt(
                        connection,
                        None,
                        app_id,
                        STEAM_ENRICHMENT_PHASE_ARTWORK,
                        &now,
                    )?);
            let needs_achievement_schema =
                !has_fresh_steam_achievement_schema(connection, app_id, &now)?
                    && (include_recent_attempts
                        || !has_fresh_steam_enrichment_attempt(
                            connection,
                            None,
                            app_id,
                            STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA,
                            &now,
                        )?);
            let needs_player_achievements =
                !has_fresh_steam_player_achievements(connection, steam_id64, app_id, &now)?
                    && (include_recent_attempts
                        || !has_fresh_steam_enrichment_attempt(
                            connection,
                            Some(steam_id64),
                            app_id,
                            STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS,
                            &now,
                        )?);

            if needs_artwork || needs_achievement_schema || needs_player_achievements {
                candidates.push(SteamEnrichmentCandidate {
                    app_id: app_id.to_string(),
                    needs_artwork,
                    needs_achievement_schema,
                    needs_player_achievements,
                });
            }

            if candidates.len() >= limit {
                break;
            }
        }

        Ok(candidates)
    }

    pub fn get_steam_enrichment_retry_summary(
        connection: &Connection,
        steam_id64: &str,
    ) -> rusqlite::Result<SteamEnrichmentRetrySummaryDto> {
        let now = now_iso();
        let mut summary = SteamEnrichmentRetrySummaryDto::default();
        let mut marked_games = std::collections::HashSet::new();
        let mut statement = connection.prepare(
            r#"
            SELECT DISTINCT game_sources.external_id, steam_enrichment_attempt_cache.phase
            FROM steam_enrichment_attempt_cache
            INNER JOIN game_sources
              ON game_sources.platform_id = 'steam'
             AND game_sources.external_id = steam_enrichment_attempt_cache.app_id
            WHERE steam_enrichment_attempt_cache.expires_at > ?1
              AND (
                steam_enrichment_attempt_cache.steam_id64 = ''
                OR steam_enrichment_attempt_cache.steam_id64 = ?2
              )
            "#,
        )?;
        let rows = statement.query_map(params![now, steam_id64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (app_id, phase) = row?;
            marked_games.insert(app_id);
            summary.marked_attempts += 1;

            match phase.as_str() {
                STEAM_ENRICHMENT_PHASE_ARTWORK => summary.artwork += 1,
                STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA => summary.achievement_schema += 1,
                STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS => summary.player_achievements += 1,
                _ => {}
            }
        }

        summary.marked_games = marked_games.len();
        Ok(summary)
    }

    pub fn record_steam_enrichment_attempt(
        connection: &Connection,
        steam_id64: Option<&str>,
        app_id: &str,
        phase: &str,
        retry_after_days: i64,
        outcome: &str,
    ) -> rusqlite::Result<()> {
        let now = now_iso();
        let expires_at = iso_days_from_now(retry_after_days.max(1));
        connection.execute(
            r#"
            INSERT INTO steam_enrichment_attempt_cache (
              steam_id64,
              app_id,
              phase,
              outcome,
              attempted_at,
              expires_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5)
            ON CONFLICT(steam_id64, app_id, phase) DO UPDATE SET
              outcome = excluded.outcome,
              attempted_at = excluded.attempted_at,
              expires_at = excluded.expires_at,
              updated_at = excluded.updated_at
            "#,
            params![
                steam_id64.unwrap_or_default(),
                app_id,
                phase,
                outcome,
                now,
                expires_at
            ],
        )?;

        Ok(())
    }

    pub fn clear_steam_enrichment_attempt(
        connection: &Connection,
        steam_id64: Option<&str>,
        app_id: &str,
        phase: &str,
    ) -> rusqlite::Result<()> {
        connection.execute(
            r#"
            DELETE FROM steam_enrichment_attempt_cache
            WHERE steam_id64 = ?1
              AND app_id = ?2
              AND phase = ?3
            "#,
            params![steam_id64.unwrap_or_default(), app_id, phase],
        )?;

        Ok(())
    }

    pub fn save_steam_achievement_schema_cache(
        connection: &Connection,
        app_id: &str,
        schema_json: &str,
        achievement_count: usize,
    ) -> rusqlite::Result<()> {
        let now = now_iso();
        let expires_at = iso_days_from_now(30);
        connection.execute(
            r#"
            INSERT INTO steam_achievement_schema_cache (
              app_id,
              schema_json,
              achievement_count,
              fetched_at,
              expires_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?4)
            ON CONFLICT(app_id) DO UPDATE SET
              schema_json = excluded.schema_json,
              achievement_count = excluded.achievement_count,
              fetched_at = excluded.fetched_at,
              expires_at = excluded.expires_at,
              updated_at = excluded.updated_at
            "#,
            params![
                app_id,
                schema_json,
                achievement_count as i64,
                now,
                expires_at
            ],
        )?;

        Ok(())
    }

    pub fn save_steam_player_achievement_cache(
        connection: &Connection,
        steam_id64: &str,
        app_id: &str,
        achievements_json: &str,
        unlocked_count: usize,
        total_count: usize,
    ) -> rusqlite::Result<()> {
        let now = now_iso();
        let expires_at = iso_days_from_now(1);
        connection.execute(
            r#"
            INSERT INTO steam_player_achievement_cache (
              steam_id64,
              app_id,
              achievements_json,
              unlocked_count,
              total_count,
              fetched_at,
              expires_at,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?6)
            ON CONFLICT(steam_id64, app_id) DO UPDATE SET
              achievements_json = excluded.achievements_json,
              unlocked_count = excluded.unlocked_count,
              total_count = excluded.total_count,
              fetched_at = excluded.fetched_at,
              expires_at = excluded.expires_at,
              updated_at = excluded.updated_at
            "#,
            params![
                steam_id64,
                app_id,
                achievements_json,
                unlocked_count as i64,
                total_count as i64,
                now,
                expires_at
            ],
        )?;

        Ok(())
    }

    fn has_fresh_steam_achievement_schema(
        connection: &Connection,
        app_id: &str,
        now: &str,
    ) -> rusqlite::Result<bool> {
        let expires_at = connection
            .query_row(
                "SELECT expires_at FROM steam_achievement_schema_cache WHERE app_id = ?1",
                params![app_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        Ok(expires_at.is_some_and(|expires_at| expires_at.as_str() > now))
    }

    fn has_fresh_steam_player_achievements(
        connection: &Connection,
        steam_id64: &str,
        app_id: &str,
        now: &str,
    ) -> rusqlite::Result<bool> {
        let expires_at = connection
            .query_row(
                r#"
                SELECT expires_at
                FROM steam_player_achievement_cache
                WHERE steam_id64 = ?1
                  AND app_id = ?2
                "#,
                params![steam_id64, app_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        Ok(expires_at.is_some_and(|expires_at| expires_at.as_str() > now))
    }

    fn has_fresh_steam_enrichment_attempt(
        connection: &Connection,
        steam_id64: Option<&str>,
        app_id: &str,
        phase: &str,
        now: &str,
    ) -> rusqlite::Result<bool> {
        let expires_at = connection
            .query_row(
                r#"
                SELECT expires_at
                FROM steam_enrichment_attempt_cache
                WHERE steam_id64 = ?1
                  AND app_id = ?2
                  AND phase = ?3
                "#,
                params![steam_id64.unwrap_or_default(), app_id, phase],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        Ok(expires_at.is_some_and(|expires_at| expires_at.as_str() > now))
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

    fn collect_local_game_roots(settings: &LibrarySettingsDto) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        let excluded_roots = normalize_local_scan_root_list(&settings.local_scan_excluded_roots);
        let configured_roots = normalize_local_scan_root_list(&settings.local_scan_roots);

        match settings.local_scan_mode.as_str() {
            "selected_only" => {
                for root in configured_roots {
                    push_unique_path(&mut roots, root);
                }
            }
            "automatic_plus_extra" => {
                for root in collect_automatic_local_game_roots() {
                    push_unique_path(&mut roots, root);
                }
                for root in configured_roots {
                    push_unique_path(&mut roots, root);
                }
            }
            _ => {
                for root in collect_automatic_local_game_roots() {
                    push_unique_path(&mut roots, root);
                }
            }
        }

        roots
            .into_iter()
            .filter(|root| root.exists())
            .filter(|root| !is_path_excluded(root, &excluded_roots))
            .collect()
    }

    fn collect_automatic_local_game_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();

        if let Some(raw_roots) = std::env::var_os("BIBLIOTECA_JOGOS_LOCAL_ROOTS") {
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

        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            let user_profile = PathBuf::from(user_profile);
            push_unique_path(&mut roots, user_profile.join("Games"));
            push_unique_path(&mut roots, user_profile.join("Desktop").join("Games"));
            push_unique_path(&mut roots, user_profile.join("Documents").join("Games"));
            push_unique_path(
                &mut roots,
                user_profile.join("AppData").join("Local").join("osu!"),
            );
        }

        if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
            let program_files = PathBuf::from(program_files);
            push_unique_path(&mut roots, program_files.join("GOG Games"));
            push_unique_path(&mut roots, program_files.join("Epic Games"));
            push_unique_path(&mut roots, program_files.join("EA Games"));
            push_unique_path(&mut roots, program_files.join("Ubisoft"));
            push_unique_path(&mut roots, program_files.join("Battle.net"));
        }

        if let Some(program_files_x86) = std::env::var_os("PROGRAMFILES(X86)") {
            let program_files_x86 = PathBuf::from(program_files_x86);
            push_unique_path(&mut roots, program_files_x86.join("GOG Games"));
            push_unique_path(&mut roots, program_files_x86.join("Epic Games"));
            push_unique_path(&mut roots, program_files_x86.join("Battle.net"));
        }

        if let Some(public) = std::env::var_os("PUBLIC") {
            push_unique_path(&mut roots, PathBuf::from(public).join("Games"));
        }

        for root in automatic_drive_game_roots() {
            push_unique_path(&mut roots, root);
        }

        roots
    }

    fn automatic_drive_game_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();

        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            let path = PathBuf::from(root);
            if !path.exists() {
                continue;
            }

            for child in [
                "Games",
                "Jogos",
                "GOG Games",
                "Epic Games",
                "EA Games",
                "Ubisoft",
                "Battle.net",
            ] {
                let candidate = path.join(child);
                if candidate.exists() {
                    roots.push(candidate);
                }
            }
        }

        roots
    }

    fn normalize_local_scan_root_list(values: &[String]) -> Vec<PathBuf> {
        values
            .iter()
            .filter_map(|value| normalize_local_scan_root(value))
            .collect::<Vec<_>>()
    }

    fn is_path_excluded(path: &Path, exclusions: &[PathBuf]) -> bool {
        exclusions
            .iter()
            .any(|exclusion| path.starts_with(exclusion))
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
            if !is_ignored_local_candidate_directory(root)
                && !is_helper_directory(root)
                && has_direct_executable(root)
            {
                directories.push(root.to_path_buf());
            }

            if let Ok(children) = fs::read_dir(root) {
                for child in children.flatten().take(256) {
                    let path = child.path();
                    if path.is_dir()
                        && !is_ignored_local_candidate_directory(&path)
                        && !is_helper_directory(&path)
                    {
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
        if is_ignored_local_candidate_directory(candidate_dir) || is_helper_directory(candidate_dir)
        {
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
        let normalized_stem = normalize_name(&stem);

        if is_battlenet_launcher_component(&normalized_stem) {
            return true;
        }

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
            "blizzardbrowser",
            "blizzarderror",
            "blizzardupdateagent",
        ]
        .iter()
        .any(|keyword| stem.contains(keyword) || normalized_stem.contains(keyword))
    }

    fn is_helper_directory(path: &Path) -> bool {
        let normalized_components = path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .map(normalize_name)
            .collect::<Vec<_>>();

        let normalized_file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(normalize_name)
            .unwrap_or_default();

        if is_battlenet_launcher_component(&normalized_file_name) {
            return true;
        }

        if normalized_components.iter().any(|component| {
            matches!(
                component.as_str(),
                "staging" | "build" | "helper" | "helpers" | "agent"
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
            "blizzardbrowser",
            "blizzarderror",
            "blizzardupdateagent",
        ]
        .iter()
        .any(|keyword| normalized_path.contains(keyword))
    }

    fn is_battlenet_launcher_component(normalized_name: &str) -> bool {
        normalized_name == "battlenet"
            || normalized_name == "battlenetlauncher"
            || normalized_name
                .strip_prefix("battlenet")
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
                })
    }

    fn is_ignored_local_candidate_directory(path: &Path) -> bool {
        let normalized_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(normalize_name)
            .unwrap_or_default();

        matches!(
            normalized_name.as_str(),
            "programfiles"
                | "programfilesx86"
                | "programdata"
                | "windows"
                | "users"
                | "documentsandsettings"
                | "systemvolumeinformation"
                | "recyclebin"
                | "python"
                | "python312"
                | "python311"
                | "python310"
                | "drivers"
                | "driver"
                | "xboxgames"
                | "dell"
                | "nvidia"
                | "amd"
                | "intel"
                | "battlenet"
                | "battlenetlauncher"
                | "blizzardbrowser"
                | "blizzarderror"
                | "blizzardupdateagent"
                | "perflogs"
        )
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
            || matches!(
                normalized_title.as_str(),
                "programfiles"
                    | "programfilesx86"
                    | "programdata"
                    | "windows"
                    | "users"
                    | "documentsandsettings"
                    | "systemvolumeinformation"
                    | "recyclebin"
                    | "python"
                    | "python312"
                    | "python311"
                    | "python310"
                    | "drivers"
                    | "driver"
                    | "xboxgames"
                    | "dell"
                    | "nvidia"
                    | "amd"
                    | "intel"
                    | "battlenet"
                    | "battlenetlauncher"
                    | "blizzardbrowser"
                    | "blizzarderror"
                    | "blizzardupdateagent"
                    | "perflogs"
            )
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
              library_entries.is_favorite,
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
                        is_favorite: row.get::<_, i64>(6)? == 1,
                        added_at: row.get(7)?,
                        updated_at: row.get(8)?,
                        game_id: row.get(9)?,
                        title: row.get(10)?,
                        sort_title: row.get(11)?,
                        installed: row.get::<_, i64>(12)? == 1,
                        playtime_total_minutes: row.get(13)?,
                        accent_color: row.get(14)?,
                        source_account_id: row.get(15)?,
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
              library_entries.is_favorite,
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
                        is_favorite: row.get::<_, i64>(6)? == 1,
                        added_at: row.get(7)?,
                        updated_at: row.get(8)?,
                        game_id: row.get(9)?,
                        title: row.get(10)?,
                        sort_title: row.get(11)?,
                        installed: row.get::<_, i64>(12)? == 1,
                        playtime_total_minutes: row.get(13)?,
                        accent_color: row.get(14)?,
                        source_account_id: row.get(15)?,
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
                    OR lower(target) LIKE '%python%'
                    OR lower(target) LIKE '%drivers%'
                    OR lower(target) LIKE '%xboxgames%'
                    OR lower(target) LIKE '%dell%'
                    OR lower(target) LIKE '%battle.net%'
                    OR lower(target) LIKE '%battlenet%'
                    OR lower(target) LIKE '%blizzardbrowser%'
                    OR lower(target) LIKE '%blizzarderror%'
                    OR lower(target) LIKE '%blizzardupdateagent%'
                    OR lower(label) LIKE '%directx%'
                    OR lower(label) LIKE '%dxsetup%'
                    OR lower(label) LIKE '%epiconlineservices%'
                    OR lower(label) LIKE '%installer%'
                    OR lower(label) LIKE '%battle.net%'
                    OR lower(replace(label, ' ', '')) LIKE '%battlenet%'
                    OR lower(label) LIKE '%blizzard browser%'
                    OR lower(label) LIKE '%blizzard error%'
                    OR lower(replace(label, ' ', '')) LIKE '%blizzardupdateagent%'
                    OR lower(replace(replace(label, ' ', ''), '(', '')) LIKE 'programfiles%'
                    OR lower(label) IN (
                      'programdata',
                      'windows',
                      'users',
                      'documents and settings',
                      'system volume information',
                      '$recycle.bin',
                      'python',
                      'python312',
                      'python311',
                      'python310',
                      'drivers',
                      'driver',
                      'xboxgames',
                      'xbox games',
                      'dell',
                      'nvidia',
                      'amd',
                      'intel',
                      'battlenet',
                      'battle.net',
                      'battle.net launcher',
                      'blizzard browser',
                      'blizzard error',
                      'blizzard update agent',
                      'perflogs'
                    )
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

    pub fn set_library_entry_favorite(
        connection: &mut Connection,
        entry_id: &str,
        is_favorite: bool,
    ) -> rusqlite::Result<()> {
        let updated_at = now_iso();
        let affected = connection.execute(
            r#"
            UPDATE library_entries
            SET is_favorite = ?2,
                updated_at = ?3
            WHERE id = ?1
            "#,
            params![entry_id, is_favorite as i64, updated_at],
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

    fn iso_days_from_now(days: i64) -> String {
        (Utc::now() + chrono::Duration::days(days)).to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    pub(crate) fn timestamp_millis() -> u128 {
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
            assert_eq!(
                entries[0].game.artwork.cover_url.as_deref(),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/413150/library_600x900.jpg")
            );
            assert_eq!(
                entries[0].game.artwork.hero_url.as_deref(),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/413150/library_hero.jpg")
            );
            assert_eq!(entries[0].game.artwork.source.as_deref(), Some("steam"));
            assert_eq!(
                serde_json::to_value(&entries[0].game.artwork)
                    .expect("serialize artwork")
                    .get("coverUrl")
                    .and_then(|value| value.as_str()),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/413150/library_600x900.jpg")
            );
            assert_eq!(
                serde_json::to_value(&entries[0].game.artwork)
                    .expect("serialize artwork")
                    .get("fallbackUrl")
                    .and_then(|value| value.as_str()),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/413150/header.jpg")
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
        fn save_library_settings_persists_local_scan_configuration() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            let selected_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-selected-root-{}",
                timestamp_millis()
            ));
            let excluded_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-excluded-root-{}",
                timestamp_millis()
            ));
            std::fs::create_dir_all(&selected_root).expect("create selected root");
            std::fs::create_dir_all(&excluded_root).expect("create excluded root");

            let dto = save_library_settings(
                &mut connection,
                LibrarySettingsInput {
                    preferred_store_id: "Xbox".to_string(),
                    local_scan_mode: "selected_only".to_string(),
                    local_scan_roots: vec![selected_root.to_string_lossy().to_string()],
                    local_scan_excluded_roots: vec![excluded_root.to_string_lossy().to_string()],
                    microsoft_client_id: "00000000-1111-2222-3333-444444444444".to_string(),
                },
            )
            .expect("save library settings");
            let loaded = get_library_settings(&connection).expect("load library settings");

            assert_eq!(dto.preferred_store_id, "xbox");
            assert_eq!(dto.local_scan_mode, "selected_only");
            assert_eq!(dto.local_scan_roots.len(), 1);
            assert_eq!(dto.local_scan_excluded_roots.len(), 1);
            assert_eq!(loaded, dto);

            let _ = std::fs::remove_dir_all(selected_root);
            let _ = std::fs::remove_dir_all(excluded_root);
        }

        #[test]
        fn sync_local_games_respects_selected_only_and_exclusions() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            let selected_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-selected-scan-root-{}",
                timestamp_millis()
            ));
            let excluded_root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-excluded-scan-root-{}",
                timestamp_millis()
            ));
            let selected_game_dir = selected_root.join("Nightfall");
            let excluded_game_dir = excluded_root.join("IgnoredGame");
            let selected_exe = selected_game_dir.join("Nightfall.exe");
            let excluded_exe = excluded_game_dir.join("IgnoredGame.exe");

            std::fs::create_dir_all(&selected_game_dir).expect("create selected game dir");
            std::fs::create_dir_all(&excluded_game_dir).expect("create excluded game dir");
            std::fs::write(&selected_exe, b"fake exe").expect("create selected exe");
            std::fs::write(&excluded_exe, b"fake exe").expect("create excluded exe");

            save_library_settings(
                &mut connection,
                LibrarySettingsInput {
                    preferred_store_id: "steam".to_string(),
                    local_scan_mode: "selected_only".to_string(),
                    local_scan_roots: vec![
                        selected_root.to_string_lossy().to_string(),
                        excluded_root.to_string_lossy().to_string(),
                    ],
                    local_scan_excluded_roots: vec![excluded_root.to_string_lossy().to_string()],
                    microsoft_client_id: "00000000-1111-2222-3333-444444444444".to_string(),
                },
            )
            .expect("save selected-only scan settings");

            let summary = sync_local_games(&mut connection).expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Nightfall");

            let _ = std::fs::remove_file(selected_exe);
            let _ = std::fs::remove_file(excluded_exe);
            let _ = std::fs::remove_dir_all(selected_root);
            let _ = std::fs::remove_dir_all(excluded_root);
        }

        #[test]
        fn artwork_falls_back_without_urls_for_non_steam_games() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-artwork-fallback-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_seeded_database(&path);

            add_manual_game(
                &mut connection,
                ManualGameInput {
                    title: "Sem Arte Remota".to_string(),
                    genre: Some("Teste".to_string()),
                    install_status: "not_installed".to_string(),
                    launch_target: None,
                },
            )
            .expect("add manual game");
            let entries = list_library_entries(&connection).expect("list entries");
            let manual_entry = entries
                .iter()
                .find(|entry| entry.game.title == "Sem Arte Remota")
                .expect("manual entry");
            let local_entry = entries
                .iter()
                .find(|entry| entry.primary_platform_id == "local")
                .expect("local entry");

            assert!(manual_entry.game.artwork.cover_url.is_none());
            assert!(manual_entry.game.artwork.hero_url.is_none());
            assert!(manual_entry.game.artwork.source.is_none());
            assert!(local_entry.game.artwork.cover_url.is_none());
            assert!(local_entry.game.artwork.hero_url.is_none());
            assert!(local_entry.game.artwork.source.is_none());
            assert_eq!(
                serde_json::to_value(&manual_entry.game.artwork)
                    .expect("serialize artwork")
                    .get("coverUrl"),
                Some(&serde_json::Value::Null)
            );

            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn steam_artwork_uses_library_hero_for_hero_url() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let app_id = "987654321";
            let header_image =
                "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/987654321/header.jpg";
            steam_web_api::cache_appdetails_header_image(app_id, header_image.to_string());
            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: app_id.to_string(),
                title: "Cached Header".to_string(),
                playtime_forever: None,
            }];

            sync_steam_account_games(&mut connection, "76561198000000000", &remote_games)
                .expect("sync remote steam account");
            let entries = list_library_entries(&connection).expect("list entries");

            assert_eq!(entries.len(), 1);
            assert_eq!(
                entries[0].game.artwork.cover_url.as_deref(),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/987654321/library_600x900.jpg")
            );
            assert_eq!(
                entries[0].game.artwork.hero_url.as_deref(),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/987654321/library_hero.jpg")
            );
            assert_eq!(
                entries[0].game.artwork.fallback_url.as_deref(),
                Some(header_image)
            );
            assert_eq!(entries[0].game.artwork.source.as_deref(), Some("steam"));
        }

        #[test]
        fn steam_artwork_uses_library_hero_before_persisted_header_cache() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let app_id = "3925760";
            let header_image = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/3925760/7b56098b0204dd2957d2437f932baf5dae91e353/header.jpg?t=1761941592";
            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: app_id.to_string(),
                title: "Unfair Flips".to_string(),
                playtime_forever: None,
            }];

            save_steam_artwork_header_image(&connection, app_id, header_image)
                .expect("persist steam artwork cache");
            sync_steam_account_games(&mut connection, "76561198000000000", &remote_games)
                .expect("sync remote steam account");
            let entries = list_library_entries(&connection).expect("list entries");

            assert_eq!(entries.len(), 1);
            assert_eq!(
                entries[0].game.artwork.cover_url.as_deref(),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/3925760/library_600x900.jpg")
            );
            assert_eq!(
                entries[0].game.artwork.hero_url.as_deref(),
                Some("https://cdn.akamai.steamstatic.com/steam/apps/3925760/library_hero.jpg")
            );
            assert_eq!(
                entries[0].game.artwork.fallback_url.as_deref(),
                Some(header_image)
            );
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
        fn save_xbox_live_client_config_persists_client_id_in_xbox_config_json() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            save_xbox_live_client_id(&mut connection, "00000000-1111-2222-3333-444444444444")
                .expect("save xbox client id");

            assert_eq!(
                read_xbox_live_client_id(&connection).expect("read xbox client id"),
                Some("00000000-1111-2222-3333-444444444444".to_string())
            );

            let config_json: String = connection
                .query_row(
                    "SELECT config_json FROM provider_account_configs WHERE provider_id = 'xbox'",
                    [],
                    |row| row.get(0),
                )
                .expect("read xbox config json");

            assert!(config_json
                .contains("\"microsoftClientId\":\"00000000-1111-2222-3333-444444444444\""));
        }

        #[test]
        fn save_xbox_library_roots_accepts_xboxgames_path_and_preserves_account_config() {
            let extra_library = std::env::temp_dir().join(format!(
                "biblioteca-jogos-xboxgames-input-{}",
                timestamp_millis()
            ));
            let extra_xbox_games = extra_library.join("XboxGames");

            std::fs::create_dir_all(&extra_xbox_games).expect("create xboxgames");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-xboxgames-input-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            save_verified_xbox_account_config(&mut connection, "2533274791234567")
                .expect("save xbox account");
            let dto = save_xbox_library_roots(
                &mut connection,
                XboxLibraryRootsInput {
                    roots: vec![extra_xbox_games.to_string_lossy().to_string()],
                },
            )
            .expect("save xboxgames path");

            assert_eq!(dto.roots.len(), 1);
            assert_eq!(
                read_xbox_account_config(&connection).expect("read xbox account"),
                Some("2533274791234567".to_string())
            );
            assert_eq!(
                read_xbox_library_roots(&connection).expect("read xbox roots"),
                dto.roots
            );

            let _ = std::fs::remove_dir_all(extra_library);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn steam_enrichment_candidates_skip_fresh_cache() {
            let connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let app_id = "413150";
            let steam_id = "76561198000000000";
            save_steam_artwork_header_image(
                &connection,
                app_id,
                "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/413150/header.jpg",
            )
            .expect("save artwork");
            save_steam_achievement_schema_cache(&connection, app_id, "[]", 0).expect("save schema");
            save_steam_player_achievement_cache(&connection, steam_id, app_id, "[]", 0, 0)
                .expect("save player achievements");

            let candidates = list_steam_enrichment_candidates(
                &connection,
                steam_id,
                &[app_id.to_string()],
                50,
                false,
            )
            .expect("list candidates");

            assert!(candidates.is_empty());
        }

        #[test]
        fn steam_player_achievement_cache_is_scoped_by_account_and_preserves_rows() {
            let connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let app_id = "413150";
            let first_steam_id = "76561198000000000";
            let second_steam_id = "76561198000000001";

            save_steam_artwork_header_image(
                &connection,
                app_id,
                "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/413150/header.jpg",
            )
            .expect("save artwork");
            save_steam_achievement_schema_cache(&connection, app_id, r#"[{"name":"FIRST"}]"#, 1)
                .expect("save schema");
            save_steam_player_achievement_cache(
                &connection,
                first_steam_id,
                app_id,
                r#"[{"apiname":"FIRST","achieved":1}]"#,
                1,
                1,
            )
            .expect("save first account achievements");

            let first_candidates = list_steam_enrichment_candidates(
                &connection,
                first_steam_id,
                &[app_id.to_string()],
                50,
                false,
            )
            .expect("list first candidates");
            let second_candidates = list_steam_enrichment_candidates(
                &connection,
                second_steam_id,
                &[app_id.to_string()],
                50,
                false,
            )
            .expect("list second candidates");

            assert!(first_candidates.is_empty());
            assert_eq!(second_candidates.len(), 1);
            assert!(!second_candidates[0].needs_artwork);
            assert!(!second_candidates[0].needs_achievement_schema);
            assert!(second_candidates[0].needs_player_achievements);

            save_steam_player_achievement_cache(
                &connection,
                second_steam_id,
                app_id,
                r#"[{"apiname":"FIRST","achieved":0}]"#,
                0,
                1,
            )
            .expect("save second account achievements");

            let mut statement = connection
                .prepare(
                    r#"
                    SELECT steam_id64, unlocked_count
                    FROM steam_player_achievement_cache
                    WHERE app_id = ?1
                    ORDER BY steam_id64
                    "#,
                )
                .expect("prepare cache query");
            let rows = statement
                .query_map(params![app_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .expect("query cache")
                .collect::<rusqlite::Result<Vec<_>>>()
                .expect("collect cache rows");

            assert_eq!(
                rows,
                vec![
                    (first_steam_id.to_string(), 1),
                    (second_steam_id.to_string(), 0)
                ]
            );
        }

        #[test]
        fn list_library_entries_includes_detailed_steam_achievements() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let steam_id = "76561198000000000";
            let app_id = "413150";
            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: app_id.to_string(),
                title: "Stardew Valley".to_string(),
                playtime_forever: Some(321),
            }];

            sync_steam_account_games(&mut connection, steam_id, &remote_games)
                .expect("sync remote steam account");
            save_steam_achievement_schema_cache(
                &connection,
                app_id,
                r#"[
                    {
                      "name":"FIRST_WIN",
                      "displayName":"Primeira vitoria",
                      "description":"Ganhe pela primeira vez.",
                      "icon":"https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/413150/first.jpg",
                      "icongray":"https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/413150/first-gray.jpg",
                      "hidden":0
                    },
                    {
                      "name":"SECRET_ROOM",
                      "displayName":"Sala secreta",
                      "description":"Encontre a sala escondida.",
                      "icon":"https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/413150/secret.jpg",
                      "icongray":"https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/413150/secret-gray.jpg",
                      "hidden":1
                    }
                  ]"#,
                2,
            )
            .expect("save schema");
            save_steam_player_achievement_cache(
                &connection,
                steam_id,
                app_id,
                r#"[
                    {"apiname":"FIRST_WIN","achieved":1,"unlocktime":1710000000},
                    {"apiname":"SECRET_ROOM","achieved":0,"unlocktime":0}
                  ]"#,
                1,
                2,
            )
            .expect("save player achievements");

            let entries = list_library_entries(&connection).expect("list entries");
            let achievements = entries[0]
                .game
                .achievements
                .as_ref()
                .expect("steam achievements dto");

            assert_eq!(achievements.provider_id, "steam");
            assert_eq!(achievements.app_id, app_id);
            assert_eq!(achievements.total, 2);
            assert_eq!(achievements.unlocked, 1);
            assert_eq!(achievements.percentage, 50.0);
            assert_eq!(achievements.items.len(), 2);
            assert_eq!(achievements.items[0].api_name, "FIRST_WIN");
            assert_eq!(achievements.items[0].name, "Primeira vitoria");
            assert!(achievements.items[0].achieved);
            assert_eq!(achievements.items[0].unlock_time, Some(1710000000));
            assert_eq!(achievements.items[1].api_name, "SECRET_ROOM");
            assert_eq!(achievements.items[1].name, "Sala secreta");
            assert_eq!(
                achievements.items[1].description,
                "Encontre a sala escondida."
            );
            assert!(achievements.items[1].hidden);
            assert!(!achievements.items[1].achieved);
        }

        #[test]
        fn list_library_entries_preserves_player_achievement_description_when_schema_is_minimal() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let steam_id = "76561198000000000";
            let app_id = "413150";
            let remote_games = vec![steam_web_api::RemoteSteamGame {
                app_id: app_id.to_string(),
                title: "Stardew Valley".to_string(),
                playtime_forever: Some(321),
            }];

            sync_steam_account_games(&mut connection, steam_id, &remote_games)
                .expect("sync remote steam account");
            save_steam_achievement_schema_cache(
                &connection,
                app_id,
                r#"[{"name":"SECRET_ROOM","displayName":"Sala secreta","hidden":1}]"#,
                1,
            )
            .expect("save schema");
            save_steam_player_achievement_cache(
                &connection,
                steam_id,
                app_id,
                r#"[{"apiname":"SECRET_ROOM","achieved":0,"description":"Descricao vinda do player payload."}]"#,
                0,
                1,
            )
            .expect("save player achievements");

            let entries = list_library_entries(&connection).expect("list entries");
            let achievement = entries[0]
                .game
                .achievements
                .as_ref()
                .expect("steam achievements dto")
                .items
                .first()
                .expect("achievement item");

            assert_eq!(achievement.api_name, "SECRET_ROOM");
            assert_eq!(achievement.name, "Sala secreta");
            assert_eq!(
                achievement.description,
                "Descricao vinda do player payload."
            );
            assert!(achievement.hidden);
            assert!(!achievement.achieved);
        }

        #[test]
        fn steam_enrichment_candidates_include_missing_cache() {
            let connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");

            let candidates = list_steam_enrichment_candidates(
                &connection,
                "76561198000000000",
                &["413150".to_string()],
                50,
                false,
            )
            .expect("list candidates");

            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].app_id, "413150");
            assert!(candidates[0].needs_artwork);
            assert!(candidates[0].needs_achievement_schema);
            assert!(candidates[0].needs_player_achievements);
        }

        #[test]
        fn steam_enrichment_candidates_skip_recent_negative_attempts() {
            let connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let app_id = "413150";
            let steam_id = "76561198000000000";

            record_steam_enrichment_attempt(
                &connection,
                None,
                app_id,
                STEAM_ENRICHMENT_PHASE_ARTWORK,
                STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS,
                "not_available",
            )
            .expect("record artwork attempt");
            record_steam_enrichment_attempt(
                &connection,
                None,
                app_id,
                STEAM_ENRICHMENT_PHASE_ACHIEVEMENT_SCHEMA,
                STEAM_ENRICHMENT_ACHIEVEMENT_RETRY_DAYS,
                "steam_web_api_auth_required",
            )
            .expect("record schema attempt");
            record_steam_enrichment_attempt(
                &connection,
                Some(steam_id),
                app_id,
                STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS,
                STEAM_ENRICHMENT_ACHIEVEMENT_RETRY_DAYS,
                "steam_web_api_auth_required",
            )
            .expect("record player achievements attempt");

            let candidates = list_steam_enrichment_candidates(
                &connection,
                steam_id,
                &[app_id.to_string()],
                50,
                false,
            )
            .expect("list candidates");

            assert!(candidates.is_empty());
        }

        #[test]
        fn steam_enrichment_candidates_can_retry_recent_negative_attempts() {
            let connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let app_id = "413150";
            let steam_id = "76561198000000000";

            record_steam_enrichment_attempt(
                &connection,
                None,
                app_id,
                STEAM_ENRICHMENT_PHASE_ARTWORK,
                STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS,
                "not_available",
            )
            .expect("record artwork attempt");

            let candidates = list_steam_enrichment_candidates(
                &connection,
                steam_id,
                &[app_id.to_string()],
                50,
                true,
            )
            .expect("list candidates");

            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].app_id, app_id);
            assert!(candidates[0].needs_artwork);
        }

        #[test]
        fn steam_enrichment_retry_summary_counts_marked_games() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let steam_id = "76561198000000000";
            let candidate = SteamGameCandidate {
                app_id: "413150".to_string(),
                title: "Stardew Valley".to_string(),
                install_path: Some("C:/Steam/steamapps/common/Stardew Valley".to_string()),
                source_id: "source-steam-413150".to_string(),
                game_id: "game-steam-stardew-valley-413150".to_string(),
                entry_id: "entry-steam-stardew-valley-413150".to_string(),
                launch_id: "launch-steam-413150".to_string(),
                accent_color: "#2563eb",
            };
            let transaction = connection.transaction().expect("begin transaction");
            insert_steam_entry(&transaction, &candidate).expect("insert steam entry");
            transaction.commit().expect("commit steam entry");

            record_steam_enrichment_attempt(
                &connection,
                None,
                &candidate.app_id,
                STEAM_ENRICHMENT_PHASE_ARTWORK,
                STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS,
                "not_available",
            )
            .expect("record artwork attempt");
            record_steam_enrichment_attempt(
                &connection,
                Some(steam_id),
                &candidate.app_id,
                STEAM_ENRICHMENT_PHASE_PLAYER_ACHIEVEMENTS,
                STEAM_ENRICHMENT_ACHIEVEMENT_RETRY_DAYS,
                "steam_web_api_auth_required",
            )
            .expect("record player achievements attempt");

            let summary =
                get_steam_enrichment_retry_summary(&connection, steam_id).expect("retry summary");

            assert_eq!(summary.marked_games, 1);
            assert_eq!(summary.marked_attempts, 2);
            assert_eq!(summary.artwork, 1);
            assert_eq!(summary.player_achievements, 1);
        }

        #[test]
        fn list_steam_app_ids_skips_recent_artwork_negative_attempts() {
            let mut connection = Connection::open_in_memory().expect("open in-memory database");
            migrate(&connection).expect("apply migration");
            let candidate = SteamGameCandidate {
                app_id: "413150".to_string(),
                title: "Stardew Valley".to_string(),
                install_path: Some("C:/Steam/steamapps/common/Stardew Valley".to_string()),
                source_id: "source-steam-413150".to_string(),
                game_id: "game-steam-stardew-valley-413150".to_string(),
                entry_id: "entry-steam-stardew-valley-413150".to_string(),
                launch_id: "launch-steam-413150".to_string(),
                accent_color: "#2563eb",
            };
            let transaction = connection.transaction().expect("begin transaction");
            insert_steam_entry(&transaction, &candidate).expect("insert steam entry");
            transaction.commit().expect("commit steam entry");

            record_steam_enrichment_attempt(
                &connection,
                None,
                &candidate.app_id,
                STEAM_ENRICHMENT_PHASE_ARTWORK,
                STEAM_ENRICHMENT_ARTWORK_RETRY_DAYS,
                "not_available",
            )
            .expect("record artwork attempt");

            let app_ids = list_steam_app_ids(&connection).expect("list app ids");

            assert!(app_ids.is_empty());
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
        fn sync_local_games_skips_system_candidate_directories() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-system-root-{}",
                timestamp_millis()
            ));
            let program_files_dir = root.join("Program Files");
            let nested_tool_dir = program_files_dir.join("Vendor").join("Tool");
            let nested_tool_executable = nested_tool_dir.join("Tool.exe");
            let python_dir = root.join("Python312");
            let python_executable = python_dir.join("python.exe");
            let drivers_dir = root.join("Drivers");
            let driver_executable = drivers_dir.join("driver.exe");
            let xbox_games_dir = root.join("XboxGames");
            let xbox_executable = xbox_games_dir.join("GamingServices.exe");
            let dell_dir = root.join("Dell");
            let dell_executable = dell_dir.join("SupportAssist.exe");
            let game_dir = root.join("Nightfall");
            let game_executable = game_dir.join("Nightfall.exe");

            std::fs::create_dir_all(&nested_tool_dir).expect("create nested system tool dir");
            std::fs::write(&nested_tool_executable, b"fake exe").expect("create tool exe");
            std::fs::create_dir_all(&python_dir).expect("create python dir");
            std::fs::write(&python_executable, b"fake exe").expect("create python exe");
            std::fs::create_dir_all(&drivers_dir).expect("create drivers dir");
            std::fs::write(&driver_executable, b"fake exe").expect("create driver exe");
            std::fs::create_dir_all(&xbox_games_dir).expect("create xboxgames dir");
            std::fs::write(&xbox_executable, b"fake exe").expect("create xboxgames exe");
            std::fs::create_dir_all(&dell_dir).expect("create dell dir");
            std::fs::write(&dell_executable, b"fake exe").expect("create dell exe");
            std::fs::create_dir_all(&game_dir).expect("create game dir");
            std::fs::write(&game_executable, b"fake exe").expect("create game exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-system-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 1);
            assert_eq!(summary.inserted, 1);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].game.title, "Nightfall");

            let _ = std::fs::remove_dir_all(root);
            let _ = std::fs::remove_file(path);
        }

        #[test]
        fn sync_local_games_archives_previously_imported_system_directories() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-archive-system-root-{}",
                timestamp_millis()
            ));
            std::fs::create_dir_all(&root).expect("create empty root");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-archive-system-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");
            let candidate = LocalGameCandidate {
                source_external_id: normalize_path_string(Path::new(
                    "C:\\Program Files\\Vendor\\Tool\\Tool.exe",
                )),
                title: "Program Files".to_string(),
                launch_target: "C:\\Program Files\\Vendor\\Tool\\Tool.exe".to_string(),
                source_id: "source-local-program-files".to_string(),
                game_id: "game-local-program-files".to_string(),
                entry_id: "entry-local-program-files".to_string(),
                launch_id: "launch-local-program-files".to_string(),
                accent_color: "#123456",
            };

            {
                let transaction = connection.transaction().expect("start transaction");
                insert_local_entry(&transaction, &candidate).expect("insert system entry");
                transaction.commit().expect("commit system entry");
            }

            let summary = sync_local_games_from_roots(&mut connection, std::slice::from_ref(&root))
                .expect("sync local games");
            let entries = list_library_entries(&connection).expect("list local entries");

            assert_eq!(summary.discovered, 0);
            assert_eq!(summary.archived, 1);
            assert!(entries.is_empty());

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
        fn sync_local_games_skips_battlenet_launcher_components() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-battlenet-root-{}",
                timestamp_millis()
            ));
            let battlenet_root = root.join("Battle.net");
            let versioned_dir = battlenet_root.join("Battle.net.14542");
            let agent_dir = battlenet_root.join("Agent");
            let browser_dir = battlenet_root.join("BlizzardBrowser");
            let game_dir = battlenet_root.join("Skybound");
            let root_exe = battlenet_root.join("Battle.net.exe");
            let versioned_exe = versioned_dir.join("Battle.net Launcher.exe");
            let agent_exe = agent_dir.join("Agent.exe");
            let browser_exe = browser_dir.join("BlizzardBrowser.exe");
            let game_exe = game_dir.join("Skybound.exe");

            std::fs::create_dir_all(&versioned_dir).expect("create battle.net version dir");
            std::fs::create_dir_all(&agent_dir).expect("create battle.net agent dir");
            std::fs::create_dir_all(&browser_dir).expect("create battle.net browser dir");
            std::fs::create_dir_all(&game_dir).expect("create game dir");
            std::fs::write(&root_exe, b"fake exe").expect("create battle.net exe");
            std::fs::write(&versioned_exe, b"fake exe").expect("create launcher exe");
            std::fs::write(&agent_exe, b"fake exe").expect("create agent exe");
            std::fs::write(&browser_exe, b"fake exe").expect("create browser exe");
            std::fs::write(&game_exe, b"fake exe").expect("create game exe");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-battlenet-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");

            let summary =
                sync_local_games_from_roots(&mut connection, std::slice::from_ref(&battlenet_root))
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
        fn sync_local_games_archives_previously_imported_battlenet_launcher_components() {
            let root = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-archive-battlenet-root-{}",
                timestamp_millis()
            ));
            std::fs::create_dir_all(&root).expect("create empty root");

            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-local-archive-battlenet-db-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_database(&path).expect("open empty database");
            let launcher_target = "C:\\Program Files (x86)\\Battle.net\\Battle.net.exe";
            let helper_candidate = LocalGameCandidate {
                source_external_id: normalize_path_string(Path::new(launcher_target)),
                title: "Battle.net".to_string(),
                launch_target: launcher_target.to_string(),
                source_id: "source-local-battlenet-test".to_string(),
                game_id: "game-local-battlenet-test".to_string(),
                entry_id: "entry-local-battlenet-test".to_string(),
                launch_id: "launch-local-battlenet-test".to_string(),
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
            assert!(
                table_has_column(&connection, "library_entries", "is_favorite")
                    .expect("check favorite column")
            );
            assert!(entries.iter().all(|entry| !entry.is_favorite));

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
        fn set_library_entry_favorite_is_reflected_in_listing() {
            let path = std::env::temp_dir().join(format!(
                "biblioteca-jogos-favorite-{}.sqlite3",
                timestamp_millis()
            ));
            let mut connection = open_seeded_database(&path);

            set_library_entry_favorite(&mut connection, "entry-manual-silksong", true)
                .expect("favorite entry");
            let favorited_entries =
                list_library_entries(&connection).expect("list favorited state");
            let favorited_entry = favorited_entries
                .iter()
                .find(|entry| entry.id == "entry-manual-silksong")
                .expect("favorited entry appears in listing");
            let serialized_entry =
                serde_json::to_value(favorited_entry).expect("serialize favorited entry");

            assert!(favorited_entry.is_favorite);
            assert_eq!(serialized_entry["isFavorite"], serde_json::json!(true));

            set_library_entry_favorite(&mut connection, "entry-manual-silksong", false)
                .expect("unfavorite entry");
            let unfavorited_entries =
                list_library_entries(&connection).expect("list unfavorited state");
            let unfavorited_entry = unfavorited_entries
                .iter()
                .find(|entry| entry.id == "entry-manual-silksong")
                .expect("unfavorited entry appears in listing");

            assert!(!unfavorited_entry.is_favorite);

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
