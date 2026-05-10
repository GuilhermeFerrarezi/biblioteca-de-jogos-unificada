use tauri::Manager;

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

            app.manage(AppState {
                connection: std::sync::Mutex::new(connection),
            });

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
            commands::list_manual_games,
            commands::add_manual_game,
            commands::launch_library_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct AppState {
    connection: std::sync::Mutex<rusqlite::Connection>,
}

mod commands {
    use super::{launcher, storage, AppState};
    use tauri::State;

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
                  AND library_entries.primary_platform_id = 'manual'
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
    use chrono::{SecondsFormat, Utc};
    use rusqlite::{params, Connection, OptionalExtension};
    use serde::{Deserialize, Serialize};
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn open_database(path: &Path) -> rusqlite::Result<Connection> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&connection)?;
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
        added_at: String,
        updated_at: String,
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

    pub fn list_manual_games(connection: &Connection) -> rusqlite::Result<Vec<LibraryEntryDto>> {
        let mut statement = connection.prepare(
            r#"
      SELECT
        library_entries.id,
        library_entries.primary_platform_id,
        library_entries.install_status,
        library_entries.last_played_label,
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
      ORDER BY library_entries.added_at DESC
      "#,
        )?;

        let rows = statement.query_map([], |row| {
            let game_id: String = row.get(6)?;
            Ok(EntryRow {
                entry_id: row.get(0)?,
                primary_platform_id: row.get(1)?,
                install_status: row.get(2)?,
                last_played_label: row.get(3)?,
                added_at: row.get(4)?,
                updated_at: row.get(5)?,
                game_id,
                title: row.get(7)?,
                sort_title: row.get(8)?,
                installed: row.get::<_, i64>(9)? == 1,
                playtime_total_minutes: row.get(10)?,
                accent_color: row.get(11)?,
            })
        })?;

        rows.map(|row| row.and_then(|entry| hydrate_entry(connection, entry)))
            .collect()
    }

    pub fn add_manual_game(
        connection: &mut Connection,
        input: ManualGameInput,
    ) -> rusqlite::Result<LibraryEntryDto> {
        let title = input.title.trim();
        if title.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "title is required".to_string(),
            ));
        }

        let install_status = match input.install_status.as_str() {
            "installed" => "installed",
            _ => "not_installed",
        };
        let installed = install_status == "installed";
        let genre = input
            .genre
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Sem genero");
        let launch_target = input.launch_target.unwrap_or_default().trim().to_string();
        let launch_kind = launch_action_kind(&launch_target);
        let launch_label = if launch_target.is_empty() {
            "Sem acao configurada".to_string()
        } else {
            launch_target.clone()
        };
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
      params![game_id, title, title, installed as i64, accent_color, now],
    )?;
        transaction.execute(
            r#"
      INSERT INTO library_entries (
        id, game_id, primary_platform_id, install_status, last_played_label, added_at, updated_at
      ) VALUES (?1, ?2, 'manual', ?3, 'Nunca', ?4, ?4)
      "#,
            params![entry_id, game_id, install_status, now],
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
            params![launch_id, game_id, launch_kind, launch_label, launch_target],
        )?;
        transaction.execute(
            "INSERT INTO game_genres (game_id, genre, position) VALUES (?1, ?2, 0)",
            params![game_id, genre],
        )?;
        transaction.commit()?;

        let row = find_entry(connection, &entry_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        hydrate_entry(connection, row)
    }

    struct EntryRow {
        entry_id: String,
        primary_platform_id: String,
        install_status: String,
        last_played_label: String,
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
                        added_at: row.get(4)?,
                        updated_at: row.get(5)?,
                        game_id: row.get(6)?,
                        title: row.get(7)?,
                        sort_title: row.get(8)?,
                        installed: row.get::<_, i64>(9)? == 1,
                        playtime_total_minutes: row.get(10)?,
                        accent_color: row.get(11)?,
                    })
                },
            )
            .optional()
    }

    fn hydrate_entry(connection: &Connection, row: EntryRow) -> rusqlite::Result<LibraryEntryDto> {
        let sources = list_sources(connection, &row.game_id)?;
        let launch_actions = list_launch_actions(connection, &row.game_id)?;
        let genres = list_genres(connection, &row.game_id)?;

        Ok(LibraryEntryDto {
            id: row.entry_id,
            primary_platform_id: row.primary_platform_id.clone(),
            install_status: row.install_status,
            last_played_label: row.last_played_label,
            added_at: row.added_at,
            updated_at: row.updated_at,
            game: GameDto {
                internal_id: row.game_id,
                title: row.title,
                sort_title: row.sort_title,
                platforms: vec![row.primary_platform_id],
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
    }
}
