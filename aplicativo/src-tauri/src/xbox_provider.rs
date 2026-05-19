use crate::storage::SyncSummaryDto;
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Deserializer};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub(crate) struct XboxProviderError {
    code: &'static str,
    message: String,
    recoverable: bool,
    phase: &'static str,
    details_sanitized: Option<String>,
}

impl XboxProviderError {
    fn new(
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
            phase,
            details_sanitized,
        }
    }

    #[cfg_attr(target_os = "windows", allow(dead_code))]
    fn unsupported_platform() -> Self {
        Self::new(
            "xbox_provider_unsupported_platform",
            "A descoberta Xbox local esta disponivel apenas no Windows.",
            true,
            "discovery",
            Some("plataforma nao suportada".to_string()),
        )
    }

    pub(crate) fn into_provider_error(self) -> crate::ProviderErrorDto {
        crate::ProviderErrorDto::xbox(
            self.code,
            self.message,
            self.recoverable,
            self.phase,
            self.details_sanitized,
        )
    }
}

#[derive(Debug, Clone)]
struct XboxGameCandidate {
    app_id: String,
    title: String,
    package_family_name: String,
    install_location: Option<String>,
    launch_target: Option<String>,
    store_id: Option<String>,
    source_id: String,
    game_id: String,
    entry_id: String,
    launch_id: String,
    accent_color: &'static str,
}

#[derive(Debug, Clone)]
struct EntryRow {
    entry_id: String,
    install_status: String,
    is_archived: bool,
    game_id: String,
    title: String,
    sort_title: String,
    installed: bool,
    accent_color: Option<String>,
}

#[derive(Debug, Clone)]
struct XboxPrimaryActionRow {
    kind: String,
    label: String,
    target: String,
    arguments: Vec<String>,
    working_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxDiscoveryRecord {
    app_id: String,
    title: String,
    package_family_name: String,
    package_name: Option<String>,
    package_full_name: Option<String>,
    install_location: Option<String>,
    launch_target: Option<String>,
    store_id: Option<String>,
    has_microsoft_game_config: bool,
    is_framework: bool,
    non_removable: bool,
    #[serde(default, deserialize_with = "deserialize_optional_signature_kind")]
    signature_kind: Option<String>,
}

fn deserialize_optional_signature_kind<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    let signature_kind = match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Number(value) => Some(value.to_string()),
        other => Some(other.to_string()),
    };

    Ok(signature_kind)
}

pub fn sync_xbox_games(connection: &mut Connection) -> Result<SyncSummaryDto, XboxProviderError> {
    let discovered_records = collect_xbox_discovery_records()?;
    let candidates = discover_xbox_game_candidates_from_records(&discovered_records);
    let summary = sync_xbox_games_from_candidates(connection, &candidates).map_err(|error| {
        XboxProviderError::new(
            "xbox_library_merge_failed",
            "Nao foi possivel aplicar a sincronizacao local do Xbox no banco local.",
            false,
            "merge",
            Some(error.to_string()),
        )
    })?;

    if let Err(error) = record_xbox_provider_metadata(connection, candidates.len(), &summary) {
        eprintln!("xbox provider metadata update failed: {error}");
    }

    Ok(summary)
}

fn sync_xbox_games_from_candidates(
    connection: &mut Connection,
    candidates: &[XboxGameCandidate],
) -> rusqlite::Result<SyncSummaryDto> {
    let existing_entries = list_xbox_entries_by_source(connection)?;
    let mut summary = SyncSummaryDto {
        discovered: candidates.len(),
        inserted: 0,
        updated: 0,
        archived: 0,
        unavailable: 0,
    };

    let transaction = connection.transaction()?;
    let discovered_app_ids = candidates
        .iter()
        .map(|candidate| candidate.app_id.clone())
        .collect::<HashSet<_>>();

    for candidate in candidates {
        if let Some(existing_row) = existing_entries.get(&candidate.app_id) {
            if update_xbox_entry(&transaction, existing_row, candidate)? {
                summary.updated += 1;
            }
        } else {
            insert_xbox_entry(&transaction, candidate)?;
            summary.inserted += 1;
        }
    }

    for (app_id, existing_row) in &existing_entries {
        if !existing_row.is_archived && is_rejected_persisted_xbox_entry(app_id, existing_row) {
            if archive_xbox_entry(&transaction, existing_row)? {
                summary.archived += 1;
            }
            continue;
        }

        if !existing_row.is_archived
            && !discovered_app_ids.contains(app_id)
            && mark_xbox_entry_unavailable(&transaction, existing_row)?
        {
            summary.unavailable += 1;
        }
    }

    transaction.commit()?;
    Ok(summary)
}

fn is_rejected_persisted_xbox_entry(app_id: &str, existing_row: &EntryRow) -> bool {
    let package_family_name = app_id.split('!').next().unwrap_or(app_id);

    is_known_non_game_xbox_record(&existing_row.title, None, package_family_name)
        || normalize_name(&existing_row.title).is_empty()
}

fn archive_xbox_entry(
    transaction: &rusqlite::Transaction<'_>,
    existing_row: &EntryRow,
) -> rusqlite::Result<bool> {
    if existing_row.is_archived {
        return Ok(false);
    }

    transaction.execute(
        r#"
        UPDATE library_entries
        SET is_archived = 1,
            install_status = 'not_installed',
            updated_at = ?2
        WHERE id = ?1
        "#,
        params![existing_row.entry_id, now_iso()],
    )?;
    transaction.execute(
        "UPDATE games SET installed = 0, updated_at = ?2 WHERE id = ?1",
        params![existing_row.game_id, now_iso()],
    )?;

    Ok(true)
}

fn discover_xbox_game_candidates_from_records(
    records: &[XboxDiscoveryRecord],
) -> Vec<XboxGameCandidate> {
    let mut candidates = HashMap::new();

    for record in records {
        if !should_keep_xbox_record(record) {
            continue;
        }

        let title = normalize_xbox_title(
            &record.title,
            record.package_name.as_deref(),
            &record.package_family_name,
        );
        let app_id = record.app_id.trim().to_string();
        if title.trim().is_empty() || app_id.is_empty() {
            continue;
        }

        let slug = create_slug(&title);
        let hash = stable_hash_hex(&app_id);
        let accent_color = deterministic_accent_color(&title);
        let package_key = format!("title:{}", normalize_name(&title));

        let candidate = XboxGameCandidate {
            app_id,
            title: title.clone(),
            package_family_name: record.package_family_name.clone(),
            install_location: record.install_location.clone(),
            launch_target: record.launch_target.clone(),
            store_id: record.store_id.clone(),
            source_id: format!("source-xbox-{slug}-{hash}"),
            game_id: format!("game-xbox-{slug}-{hash}"),
            entry_id: format!("entry-xbox-{slug}-{hash}"),
            launch_id: format!("launch-xbox-{hash}"),
            accent_color,
        };

        candidates
            .entry(package_key)
            .and_modify(|existing| {
                if should_replace_xbox_candidate(existing, &candidate) {
                    *existing = candidate.clone();
                }
            })
            .or_insert(candidate);
    }

    candidates.into_values().collect()
}

fn should_replace_xbox_candidate(
    existing: &XboxGameCandidate,
    candidate: &XboxGameCandidate,
) -> bool {
    let existing_has_registered_aumid = is_registered_xbox_app_id(&existing.app_id);
    let candidate_has_registered_aumid = is_registered_xbox_app_id(&candidate.app_id);

    if candidate_has_registered_aumid != existing_has_registered_aumid {
        return candidate_has_registered_aumid;
    }

    candidate.launch_target.is_some() && existing.launch_target.is_none()
}

fn is_registered_xbox_app_id(app_id: &str) -> bool {
    app_id
        .split('!')
        .next()
        .is_some_and(|family| family.contains('_'))
}

#[cfg(target_os = "windows")]
fn collect_xbox_discovery_records() -> Result<Vec<XboxDiscoveryRecord>, XboxProviderError> {
    let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$packageByFamily = @{}
try {
    foreach ($package in Get-AppxPackage) {
        try {
            if (-not $package.IsFramework -and $package.PackageFamilyName) {
                $packageByFamily[$package.PackageFamilyName] = $package
            }
        } catch {
        }
    }
} catch {
}
try {
    foreach ($app in Get-StartApps) {
        try {
            if (-not $app.AppID -or $app.AppID -notmatch '!') {
                continue
            }
            $family = $app.AppID.Split('!')[0].Trim()
            if (-not $family) {
                continue
            }

            $package = $packageByFamily[$family]
            $record = [PSCustomObject]@{
                AppId = $app.AppID
                Title = $app.Name
                PackageFamilyName = if ($package -and $package.PackageFamilyName) { $package.PackageFamilyName } else { $family }
                PackageFullName = if ($package) { $package.PackageFullName } else { $null }
                InstallLocation = if ($package) { $package.InstallLocation } else { $null }
                StoreId = $null
                HasMicrosoftGameConfig = [bool]($package -and $package.InstallLocation -and (Test-Path (Join-Path $package.InstallLocation 'MicrosoftGame.config')))
                IsFramework = [bool]($package -and $package.IsFramework)
                NonRemovable = [bool]($package -and $package.NonRemovable)
                SignatureKind = if ($package -and $package.SignatureKind) { "$($package.SignatureKind)" } else { $null }
                PackageName = if ($package -and $package.Name) { "$($package.Name)" } else { $null }
            }

            Write-Output ($record | ConvertTo-Json -Compress -Depth 6)
        } catch {
        }
    }
} catch {
}
"#;
    let output = run_powershell(script)?;
    let mut records = parse_discovery_output(&output)?;
    records.extend(collect_xbox_games_folder_records());
    Ok(records)
}

#[cfg(not(target_os = "windows"))]
fn collect_xbox_discovery_records() -> Result<Vec<XboxDiscoveryRecord>, XboxProviderError> {
    Err(XboxProviderError::unsupported_platform())
}

#[cfg(target_os = "windows")]
fn collect_xbox_games_folder_records() -> Vec<XboxDiscoveryRecord> {
    let mut records = Vec::new();

    for root in filesystem_roots() {
        let xbox_games_root = root.join("XboxGames");
        let Ok(entries) = fs::read_dir(&xbox_games_root) else {
            continue;
        };

        for entry in entries.flatten().take(512) {
            let game_dir = entry.path();
            if !game_dir.is_dir() {
                continue;
            }

            if let Some(record) = xbox_record_from_game_directory(&game_dir) {
                records.push(record);
            }
        }
    }

    records
}

#[cfg(target_os = "windows")]
fn filesystem_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for letter in b'A'..=b'Z' {
        let root = format!("{}:\\", letter as char);
        let path = PathBuf::from(root);
        if path.exists() {
            roots.push(path);
        }
    }

    roots
}

#[cfg(target_os = "windows")]
fn xbox_record_from_game_directory(game_dir: &Path) -> Option<XboxDiscoveryRecord> {
    let config_path = find_microsoft_game_config(game_dir)?;
    let contents = fs::read_to_string(&config_path).ok()?;

    if is_rejected_xbox_game_config(&contents, game_dir) {
        return None;
    }

    let identity_name = xml_tag_attribute(&contents, "Identity", "Name")
        .or_else(|| game_dir.file_name()?.to_str().map(str::to_string))?;
    let title = xml_tag_attribute(&contents, "ShellVisuals", "DefaultDisplayName")
        .filter(|value| !value.starts_with("ms-resource:"))
        .or_else(|| xml_tag_attribute(&contents, "Executable", "OverrideDisplayName"))
        .filter(|value| !value.starts_with("ms-resource:"))
        .or_else(|| game_dir.file_name()?.to_str().map(str::to_string))?;
    let executable_name = xml_tag_attribute(&contents, "Executable", "Name")?;
    let executable_id = xml_tag_attribute(&contents, "Executable", "Id")
        .or_else(|| {
            Path::new(&executable_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "App".to_string());
    Some(XboxDiscoveryRecord {
        app_id: format!("{identity_name}!{executable_id}"),
        title,
        package_family_name: identity_name.clone(),
        package_name: Some(identity_name.clone()),
        package_full_name: Some(identity_name),
        install_location: Some(game_dir.to_string_lossy().to_string()),
        launch_target: resolve_xbox_game_executable(&config_path, &executable_name)
            .map(|path| path.to_string_lossy().to_string()),
        store_id: xml_tag_text(&contents, "StoreId"),
        has_microsoft_game_config: true,
        is_framework: false,
        non_removable: false,
        signature_kind: Some("XboxGames".to_string()),
    })
}

#[cfg(target_os = "windows")]
fn resolve_xbox_game_executable(config_path: &Path, executable_name: &str) -> Option<PathBuf> {
    let config_dir = config_path.parent()?;
    for candidate in [
        config_dir.join(executable_name),
        config_dir
            .parent()
            .map(|parent| parent.join(executable_name))
            .unwrap_or_else(|| config_dir.join(executable_name)),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn find_microsoft_game_config(game_dir: &Path) -> Option<PathBuf> {
    for candidate in [
        game_dir.join("MicrosoftGame.config"),
        game_dir.join("MicrosoftGame.Config"),
        game_dir.join("Content").join("MicrosoftGame.config"),
        game_dir.join("Content").join("MicrosoftGame.Config"),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn is_rejected_xbox_game_config(contents: &str, game_dir: &Path) -> bool {
    let haystack = normalize_name(&format!(
        "{} {} {}",
        game_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default(),
        xml_tag_attribute(contents, "ShellVisuals", "DefaultDisplayName").unwrap_or_default(),
        xml_tag_attribute(contents, "Identity", "Name").unwrap_or_default()
    ));

    contents.contains("<TargetDeviceFamilyForDLC")
        || contents.contains("<AllowedProducts")
        || ["dlc", "stub", "tracker", "gamesave", "betaearlyaccess"]
            .iter()
            .any(|keyword| haystack.contains(keyword))
}

fn xml_tag_attribute(contents: &str, tag_name: &str, attribute_name: &str) -> Option<String> {
    let tag_index = find_xml_tag_index(contents, tag_name)?;
    let tag = contents[tag_index..].split('>').next()?;
    let attribute_start = format!("{attribute_name}=\"");
    let value_start = tag.find(&attribute_start)? + attribute_start.len();
    let value = tag[value_start..].split('"').next()?.trim();

    if value.is_empty() {
        None
    } else {
        Some(xml_unescape(value))
    }
}

fn find_xml_tag_index(contents: &str, tag_name: &str) -> Option<usize> {
    let tag_start = format!("<{tag_name}");
    let mut search_from = 0;

    while let Some(relative_index) = contents[search_from..].find(&tag_start) {
        let index = search_from + relative_index;
        let next_character = contents[index + tag_start.len()..].chars().next();

        if next_character.is_some_and(|character| {
            character.is_ascii_whitespace() || character == '>' || character == '/'
        }) {
            return Some(index);
        }

        search_from = index + tag_start.len();
    }

    None
}

fn xml_tag_text(contents: &str, tag_name: &str) -> Option<String> {
    let start_tag = format!("<{tag_name}>");
    let end_tag = format!("</{tag_name}>");
    let start = contents.find(&start_tag)? + start_tag.len();
    let value = contents[start..].split(&end_tag).next()?.trim();

    if value.is_empty() {
        None
    } else {
        Some(xml_unescape(value))
    }
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn run_powershell(script: &str) -> Result<String, XboxProviderError> {
    let mut commands = Vec::new();

    for variable in ["WINDIR", "SystemRoot"] {
        if let Some(root) = std::env::var_os(variable) {
            let powershell = PathBuf::from(&root)
                .join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe");
            if !powershell.as_os_str().is_empty() {
                commands.push(powershell);
            }
        }
    }

    commands.push(PathBuf::from("powershell.exe"));
    commands.push(PathBuf::from("pwsh.exe"));

    let mut last_error = None;

    for executable in commands {
        let output = Command::new(&executable)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                last_error = Some(if stderr.is_empty() {
                    format!(
                        "{} returned exit code {:?}",
                        executable.display(),
                        output.status.code()
                    )
                } else {
                    stderr
                });
            }
            Err(error) => {
                last_error = Some(format!("{}: {error}", executable.display()));
            }
        }
    }

    Err(XboxProviderError::new(
        "xbox_discovery_unavailable",
        "Nao foi possivel consultar o inventario local do Xbox no Windows.",
        true,
        "discovery",
        last_error,
    ))
}

fn parse_discovery_output(raw: &str) -> Result<Vec<XboxDiscoveryRecord>, XboxProviderError> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        return parse_discovery_value(value);
    }

    let mut output = Vec::new();

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        append_discovery_value(value, &mut output)?;
    }

    Ok(output)
}

fn parse_discovery_value(
    value: serde_json::Value,
) -> Result<Vec<XboxDiscoveryRecord>, XboxProviderError> {
    let mut output = Vec::new();

    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                append_discovery_value(item, &mut output)?;
            }
        }
        serde_json::Value::Object(_) => {
            append_discovery_value(value, &mut output)?;
        }
        serde_json::Value::Null => {}
        other => {
            return Err(XboxProviderError::new(
                "xbox_discovery_parse_failed",
                "O inventario local do Xbox retornou um formato inesperado.",
                true,
                "parse",
                Some(other.to_string()),
            ));
        }
    }

    Ok(output)
}

fn append_discovery_value(
    value: serde_json::Value,
    output: &mut Vec<XboxDiscoveryRecord>,
) -> Result<(), XboxProviderError> {
    if let Ok(record) = serde_json::from_value::<XboxDiscoveryRecord>(value) {
        output.push(record);
    }

    Ok(())
}
fn should_keep_xbox_record(record: &XboxDiscoveryRecord) -> bool {
    if record.app_id.trim().is_empty() {
        return false;
    }

    if !is_valid_xbox_game_app_id(&record.app_id) {
        return false;
    }

    if record.is_framework {
        return false;
    }

    if !record.has_microsoft_game_config {
        return false;
    }

    if record
        .signature_kind
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case("system"))
        .unwrap_or(false)
    {
        return false;
    }

    let title = normalize_xbox_title(
        &record.title,
        record.package_name.as_deref(),
        &record.package_family_name,
    );
    if title.trim().is_empty() {
        return false;
    }

    let haystack = normalize_name(&format!(
        "{} {} {} {}",
        title,
        record.package_name.clone().unwrap_or_default(),
        record.package_family_name,
        record.package_full_name.clone().unwrap_or_default()
    ));

    ![
        "gamingapp",
        "gamingservices",
        "gamingservicesui",
        "microsoftgamingapp",
        "xboxidentityprovider",
        "xboxgamingoverlay",
        "xboxgamebar",
        "xboxtcui",
        "tcui",
        "storepurchaseapp",
        "windowsstore",
        "desktopappinstaller",
        "appinstaller",
        "vclibs",
        "webview2",
        "shellexperiencehost",
        "microsoftstore",
    ]
    .iter()
    .any(|keyword| haystack.contains(keyword))
        && !(record.non_removable && haystack.contains("gamingservices"))
}

fn is_valid_xbox_game_app_id(app_id: &str) -> bool {
    let normalized = app_id.trim();
    if normalized.is_empty() {
        return false;
    }

    let lowered = normalized.to_ascii_lowercase();
    if lowered.contains('\\') || lowered.contains('/') || lowered.ends_with(".exe") {
        return false;
    }

    true
}

fn is_known_non_game_xbox_record(
    title: &str,
    package_name: Option<&str>,
    package_family_name: &str,
) -> bool {
    let normalized_title = normalize_name(title);
    let normalized_package_name = package_name.map(normalize_name).unwrap_or_default();
    let normalized_family = normalize_name(package_family_name);

    const TITLE_DENYLIST: &[&str] = &[
        "calculator",
        "calendar",
        "camera",
        "skype",
        "skype preview",
        "copilot",
        "clocks",
        "outlook",
        "outlook (new)",
        "intelligo neptune",
        "email",
        "feedback hub",
        "films and tv",
        "get help",
        "hyperx ngenuity",
        "intel graphics software",
        "intel graphics experience",
        "intel rapid storage technology application",
        "maps",
        "media player",
        "microsoft teams",
        "music",
        "news",
        "noticias",
        "maps",
        "mail",
        "microsoft store",
        "movie maker",
        "notes",
        "notepad",
        "phone link",
        "paint",
        "photos",
        "realtek audio console",
        "relógio",
        "relogio",
        "reprodutor multimídia",
        "reprodutor multimidia",
        "snipping tool",
        "sound recorder",
        "sticky notes",
        "segurança do windows",
        "seguranca do windows",
        "spotify",
        "teams",
        "terminal",
        "weather",
        "windows backup",
        "vincular ao celular",
        "your phone",
        "xbox game bar",
        "game bar",
        "settings",
        "supportassist",
        "dolby access",
        "nvidia control panel",
        "crunchyroll",
        "netflix",
        "mixed reality portal",
        "hello",
        "click to do",
        "acao com um clique",
        "aÃ§Ã£o com um clique",
        "assistencia rapida",
        "assistÃªncia rÃ¡pida",
        "introducao",
        "introduÃ§Ã£o",
        "microsoft 365 copilot",
        "microsoft clipchamp",
        "microsoft noticias",
        "microsoft notÃ­cias",
        "microsoft to do",
        "pagina inicial de desenvolvimento",
        "pÃ¡gina inicial de desenvolvimento",
        "power automate",
        "solitaire & casual games",
        "windows terminal",
    ];

    const FAMILY_DENYLIST: &[&str] = &[
        "microsoftwindowscalculator",
        "microsoftwindowscommunicationsapps",
        "microsoftwindowscamera",
        "microsoftbingweather",
        "microsoftwindowssoundrecorder",
        "microsoftwindowsfeedbackhub",
        "microsoftwindowsphotos",
        "microsoftwindowsstore",
        "microsoftscreen sketch",
        "microsoftwindowsnotepad",
        "microsoftwindowsmaps",
        "microsoftoutlookforwindows",
        "microsoftwindowsalarms",
        "microsoftwindowsterminal",
        "microsoftwindowscommunicationsapps",
        "microsoftwindows.client.cbs",
        "microsoftwindows.client.coreai",
        "microsoftcorporationii.quickassist",
        "microsoft.microsoftofficehub",
        "clipchamp.clipchamp",
        "microsoftbingnews",
        "microsofttodos",
        "microsoftwindows.devhome",
        "microsoftpowerautomatedesktop",
        "microsoftmicrosoftsolitairecollection",
        "microsoftyourphone",
        "microsoftsechealthui",
        "microsoftcopilot",
        "microsoftmixedrealityportal",
        "microsoftgethelp",
        "microsoft.microsoftstickynotes",
        "windows.immersivecontrolpanel",
        "microsoftxboxgamingoverlay",
        "microsoftxboxtcui",
        "microsoftgamingapp",
        "microsoftgamingservices",
        "microsoftxboxidentityprovider",
        "appup.intelgraphicsexperience",
        "appup.intelarcsoftware",
        "33c30b79.hyperxngenuity",
        "dell.supportassistforpcs",
        "dellinc.dellupdate",
        "dolbylaboratories.dolbyaccess",
        "15ef7777.crunchyroll",
        "4df9e0f8.netflix",
        "nvidiacorp.nvidiacontrolpanel",
        "spotifyab.spotifymusic",
        "realteksemiconductorcorp.realtekaudiocontrol",
    ];

    TITLE_DENYLIST.iter().any(|keyword| {
        let normalized_keyword = normalize_name(keyword);
        normalized_title == normalized_keyword || normalized_package_name == normalized_keyword
    }) || FAMILY_DENYLIST.iter().any(|keyword| {
        let normalized_keyword = normalize_name(keyword);
        normalized_family.contains(&normalized_keyword)
    })
}

fn normalize_xbox_title(
    title: &str,
    package_name: Option<&str>,
    package_family_name: &str,
) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("ms-resource:")
        || is_placeholder_xbox_title(trimmed)
    {
        if let Some(package_name) = package_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return package_name.to_string();
        }
        return package_family_name.trim().to_string();
    }

    trimmed.to_string()
}

fn is_placeholder_xbox_title(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "app" | "application" | "game" | "launcher" | "desktop app" | "xbox" | "store"
    )
}

fn list_xbox_entries_by_source(
    connection: &Connection,
) -> rusqlite::Result<HashMap<String, EntryRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
          game_sources.external_id,
          library_entries.id,
          library_entries.install_status,
          library_entries.is_archived,
          games.id,
          games.title,
          games.sort_title,
          games.installed,
          games.accent_color
        FROM library_entries
        JOIN games ON games.id = library_entries.game_id
        JOIN game_sources ON game_sources.game_id = library_entries.game_id
        WHERE library_entries.primary_platform_id = 'xbox'
          AND game_sources.platform_id = 'xbox'
        "#,
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                EntryRow {
                    entry_id: row.get(1)?,
                    install_status: row.get(2)?,
                    is_archived: row.get::<_, i64>(3)? == 1,
                    game_id: row.get(4)?,
                    title: row.get(5)?,
                    sort_title: row.get(6)?,
                    installed: row.get::<_, i64>(7)? == 1,
                    accent_color: row.get(8)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows.into_iter().collect())
}

fn insert_xbox_entry(
    transaction: &rusqlite::Transaction<'_>,
    candidate: &XboxGameCandidate,
) -> rusqlite::Result<()> {
    let _ = (&candidate.package_family_name, &candidate.install_location);
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
        ) VALUES (?1, ?2, 'xbox', 'installed', 'Nunca', 0, ?3, ?3)
        "#,
        params![candidate.entry_id, candidate.game_id, now],
    )?;
    transaction.execute(
        r#"
        INSERT INTO game_sources (id, game_id, platform_id, external_id)
        VALUES (?1, ?2, 'xbox', ?3)
        "#,
        params![candidate.source_id, candidate.game_id, candidate.app_id],
    )?;
    upsert_xbox_primary_action(transaction, candidate, true, &candidate.launch_id)?;
    transaction.execute(
        "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, 'Xbox', 0)",
        params![candidate.game_id],
    )?;

    Ok(())
}

fn update_xbox_entry(
    transaction: &rusqlite::Transaction<'_>,
    existing_row: &EntryRow,
    candidate: &XboxGameCandidate,
) -> rusqlite::Result<bool> {
    let current_action = find_xbox_primary_action(transaction, &existing_row.game_id)?;
    let needs_action_update = current_action
        .as_ref()
        .map(|action| {
            let expected_action = xbox_action_for_candidate(candidate, true);
            action.kind != expected_action.kind
                || action.label != expected_action.label
                || action.target != expected_action.target
                || action.arguments != expected_action.arguments
                || action.working_directory != expected_action.working_directory
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
        upsert_xbox_primary_action(transaction, candidate, true, &candidate.launch_id)?;
    }

    transaction.execute(
        "INSERT OR IGNORE INTO game_genres (game_id, genre, position) VALUES (?1, 'Xbox', 0)",
        params![existing_row.game_id],
    )?;

    Ok(true)
}

fn mark_xbox_entry_unavailable(
    transaction: &rusqlite::Transaction<'_>,
    existing_row: &EntryRow,
) -> rusqlite::Result<bool> {
    let current_action = find_xbox_primary_action(transaction, &existing_row.game_id)?;
    let expected_action = xbox_action_for_candidate(
        &XboxGameCandidate {
            app_id: String::new(),
            title: existing_row.title.clone(),
            package_family_name: String::new(),
            install_location: None,
            launch_target: None,
            store_id: None,
            source_id: String::new(),
            game_id: existing_row.game_id.clone(),
            entry_id: existing_row.entry_id.clone(),
            launch_id: String::new(),
            accent_color: "#2563eb",
        },
        false,
    );

    let needs_action_update = current_action
        .as_ref()
        .map(|action| {
            action.kind != expected_action.kind
                || action.label != expected_action.label
                || action.target != expected_action.target
                || action.arguments != expected_action.arguments
                || action.working_directory != expected_action.working_directory
        })
        .unwrap_or(true);
    let needs_entry_update =
        existing_row.installed || existing_row.install_status != "not_installed";

    if !needs_entry_update && !needs_action_update {
        return Ok(false);
    }

    let updated_at = now_iso();
    if needs_entry_update {
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
    }

    if needs_action_update {
        let fallback_launch_id = format!(
            "launch-xbox-store-{}",
            stable_hash_hex(&existing_row.game_id)
        );
        upsert_xbox_primary_action(
            transaction,
            &XboxGameCandidate {
                app_id: String::new(),
                title: existing_row.title.clone(),
                package_family_name: String::new(),
                install_location: None,
                launch_target: None,
                store_id: None,
                source_id: String::new(),
                game_id: existing_row.game_id.clone(),
                entry_id: existing_row.entry_id.clone(),
                launch_id: String::new(),
                accent_color: "#2563eb",
            },
            false,
            &fallback_launch_id,
        )?;
    }

    Ok(true)
}

fn find_xbox_primary_action(
    transaction: &rusqlite::Transaction<'_>,
    game_id: &str,
) -> rusqlite::Result<Option<XboxPrimaryActionRow>> {
    transaction
        .query_row(
            r#"
            SELECT kind, label, target, arguments_json, working_directory
            FROM launch_actions
            WHERE game_id = ?1
              AND platform_id = 'xbox'
              AND is_primary = 1
            ORDER BY id
            LIMIT 1
            "#,
            params![game_id],
            |row| {
                let arguments_json: Option<String> = row.get(3)?;
                let arguments = arguments_json
                    .and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
                    .unwrap_or_default();

                Ok(XboxPrimaryActionRow {
                    kind: row.get(0)?,
                    label: row.get(1)?,
                    target: row.get(2)?,
                    arguments,
                    working_directory: row.get(4)?,
                })
            },
        )
        .optional()
}

fn upsert_xbox_primary_action(
    transaction: &rusqlite::Transaction<'_>,
    candidate: &XboxGameCandidate,
    installed: bool,
    launch_id: &str,
) -> rusqlite::Result<()> {
    let action = xbox_action_for_candidate(candidate, installed);
    let arguments_json =
        serde_json::to_string(&action.arguments).unwrap_or_else(|_| "[]".to_string());
    let changed = transaction.execute(
        r#"
        UPDATE launch_actions
        SET kind = ?2,
            label = ?3,
            target = ?4,
            arguments_json = ?5,
            working_directory = ?6
        WHERE game_id = ?1
          AND platform_id = 'xbox'
          AND is_primary = 1
        "#,
        params![
            candidate.game_id,
            action.kind,
            action.label,
            action.target,
            arguments_json,
            action.working_directory,
        ],
    )?;

    if changed == 0 {
        transaction.execute(
            r#"
            INSERT INTO launch_actions (
              id, game_id, platform_id, kind, label, target, arguments_json, working_directory, is_primary
            ) VALUES (?1, ?2, 'xbox', ?3, ?4, ?5, ?6, ?7, 1)
            "#,
            params![
                launch_id,
                candidate.game_id,
                action.kind,
                action.label,
                action.target,
                arguments_json,
                action.working_directory,
            ],
        )?;
    }

    Ok(())
}

fn xbox_action_for_candidate(
    candidate: &XboxGameCandidate,
    installed: bool,
) -> XboxPrimaryActionRow {
    if installed {
        if is_registered_xbox_app_id(&candidate.app_id) {
            XboxPrimaryActionRow {
                kind: "executable".to_string(),
                label: "Jogar no Xbox".to_string(),
                target: windows_explorer_target().to_string_lossy().to_string(),
                arguments: vec![format!("shell:AppsFolder\\{}", candidate.app_id)],
                working_directory: None,
            }
        } else if let Some(target) = candidate.launch_target.as_deref() {
            XboxPrimaryActionRow {
                kind: "executable".to_string(),
                label: "Jogar no Xbox".to_string(),
                target: target.to_string(),
                arguments: Vec::new(),
                working_directory: Path::new(target)
                    .parent()
                    .map(|path| path.to_string_lossy().to_string()),
            }
        } else {
            XboxPrimaryActionRow {
                kind: "uri".to_string(),
                label: "Abrir Microsoft Store".to_string(),
                target: build_store_target(&candidate.title, candidate.store_id.as_deref()),
                arguments: Vec::new(),
                working_directory: None,
            }
        }
    } else {
        XboxPrimaryActionRow {
            kind: "uri".to_string(),
            label: "Abrir Microsoft Store".to_string(),
            target: build_store_target(&candidate.title, candidate.store_id.as_deref()),
            arguments: Vec::new(),
            working_directory: None,
        }
    }
}

fn build_store_target(title: &str, product_id: Option<&str>) -> String {
    if let Some(product_id) = product_id.map(str::trim).filter(|value| !value.is_empty()) {
        return format!("ms-windows-store://pdp/?ProductId={product_id}");
    }

    let query = url::form_urlencoded::byte_serialize(title.trim().as_bytes()).collect::<String>();
    format!("ms-windows-store://search/?query={query}")
}

fn windows_explorer_target() -> PathBuf {
    for variable in ["WINDIR", "SystemRoot"] {
        if let Some(value) = std::env::var_os(variable) {
            let path = PathBuf::from(value).join("explorer.exe");
            if !path.as_os_str().is_empty() {
                return path;
            }
        }
    }

    PathBuf::from(r"C:\Windows\explorer.exe")
}

fn record_xbox_provider_metadata(
    connection: &mut Connection,
    discovered_count: usize,
    summary: &SyncSummaryDto,
) -> rusqlite::Result<()> {
    let metadata = serde_json::json!({
        "providerId": "xbox",
        "mode": "experimental_local",
        "discoverySource": "windows_appx",
        "lastDiscoveryAt": now_iso(),
        "lastDiscoveryCount": discovered_count,
        "lastDiscoverySummary": {
            "discovered": summary.discovered,
            "inserted": summary.inserted,
            "updated": summary.updated,
            "archived": summary.archived,
            "unavailable": summary.unavailable,
        }
    });

    connection.execute(
        r#"
        INSERT INTO provider_account_configs (
          provider_id, config_json, updated_at
        ) VALUES ('xbox', ?1, ?2)
        ON CONFLICT(provider_id) DO UPDATE SET
          config_json = excluded.config_json,
          updated_at = excluded.updated_at
        "#,
        params![metadata.to_string(), now_iso()],
    )?;

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
        "jogo-xbox".to_string()
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

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn stable_hash_hex(value: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;

    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candidate() -> XboxGameCandidate {
        let title = "Halo Infinite";
        let app_id = "Microsoft.HaloInfinite_8wekyb3d8bbwe!App";
        let slug = create_slug(title);
        let hash = stable_hash_hex(app_id);

        XboxGameCandidate {
            app_id: app_id.to_string(),
            title: title.to_string(),
            package_family_name: "Microsoft.HaloInfinite_8wekyb3d8bbwe".to_string(),
            install_location: Some("C:\\Program Files\\WindowsApps\\Halo".to_string()),
            launch_target: None,
            store_id: Some("9MWPM2CQNLHN".to_string()),
            source_id: format!("source-xbox-{slug}-{hash}"),
            game_id: format!("game-xbox-{slug}-{hash}"),
            entry_id: format!("entry-xbox-{slug}-{hash}"),
            launch_id: format!("launch-xbox-{hash}"),
            accent_color: deterministic_accent_color(title),
        }
    }

    fn open_memory_database() -> Connection {
        let connection = Connection::open_in_memory().expect("open memory db");
        connection
            .execute_batch(
                r#"
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
                    is_archived INTEGER NOT NULL DEFAULT 0,
                    added_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE game_sources (
                    id TEXT PRIMARY KEY,
                    game_id TEXT NOT NULL,
                    platform_id TEXT NOT NULL,
                    external_id TEXT NOT NULL,
                    account_id TEXT
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

                CREATE TABLE game_genres (
                    game_id TEXT NOT NULL,
                    genre TEXT NOT NULL,
                    position INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY (game_id, genre)
                );

                CREATE TABLE provider_account_configs (
                    provider_id TEXT PRIMARY KEY,
                    account_id TEXT,
                    steam_id64 TEXT,
                    steam_web_api_key_configured INTEGER NOT NULL DEFAULT 0,
                    config_json TEXT,
                    updated_at TEXT NOT NULL
                );
                "#,
            )
            .expect("create schema");
        connection
    }

    #[test]
    fn builds_store_target_with_search_fallback() {
        assert_eq!(
            build_store_target("Halo Infinite", None),
            "ms-windows-store://search/?query=Halo+Infinite"
        );
    }

    #[test]
    fn builds_store_target_with_product_id_when_available() {
        assert_eq!(
            build_store_target("Halo Infinite", Some("9MWPM2CQNLHN")),
            "ms-windows-store://pdp/?ProductId=9MWPM2CQNLHN"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn reads_xbox_games_folder_record_from_microsoft_game_config() {
        let root = std::env::temp_dir().join(format!(
            "biblioteca-xbox-game-config-{}",
            std::process::id()
        ));
        let content = root.join("Content");
        let executable = content.join("Hollow Knight.exe");

        std::fs::create_dir_all(&content).expect("create content dir");
        std::fs::write(&executable, "").expect("write executable placeholder");
        std::fs::write(
            content.join("MicrosoftGame.config"),
            r#"<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="0">
  <Identity Name="TeamCherry.15373CD61C66B" Publisher="CN=test" Version="1.0.0.0" />
  <StoreId>9MW9469V91LM</StoreId>
  <ExecutableList>
    <Executable Name="Hollow Knight.exe" Id="Game" TargetDeviceFamily="PC" />
  </ExecutableList>
  <ShellVisuals DefaultDisplayName="Hollow Knight" PublisherDisplayName="Team Cherry" />
</Game>"#,
        )
        .expect("write config");

        let record = xbox_record_from_game_directory(&root).expect("read xbox game config");

        assert_eq!(record.title, "Hollow Knight");
        assert_eq!(record.app_id, "TeamCherry.15373CD61C66B!Game");
        assert_eq!(record.store_id.as_deref(), Some("9MW9469V91LM"));
        assert!(executable.is_file());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn reads_minecraft_launcher_without_executable_id() {
        let root = std::env::temp_dir().join(format!(
            "biblioteca-xbox-minecraft-config-{}",
            std::process::id()
        ));
        let content = root.join("Content");
        let executable = content.join("Minecraft.exe");

        std::fs::create_dir_all(&content).expect("create content dir");
        std::fs::write(&executable, "").expect("write executable placeholder");
        std::fs::write(
            content.join("MicrosoftGame.config"),
            r#"<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="1">
  <Identity Name="Microsoft.4297127D64EC6" Publisher="CN=Microsoft Corporation" Version="2.6.2.0" />
  <ExecutableList>
    <Executable Name="Minecraft.exe" TargetDeviceFamily="PC" IsDevOnly="false" />
  </ExecutableList>
  <ShellVisuals DefaultDisplayName="Minecraft Launcher" PublisherDisplayName="Microsoft Studios" />
  <StoreId>9PGW18NPBZV5</StoreId>
</Game>"#,
        )
        .expect("write config");

        let record = xbox_record_from_game_directory(&root).expect("read minecraft config");

        assert_eq!(record.title, "Minecraft Launcher");
        assert_eq!(record.app_id, "Microsoft.4297127D64EC6!Minecraft");
        assert_eq!(record.store_id.as_deref(), Some("9PGW18NPBZV5"));
        assert!(should_keep_xbox_record(&record));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn rejects_xbox_games_folder_dlc_stub_config() {
        let root =
            std::env::temp_dir().join(format!("biblioteca-xbox-dlc-config-{}", std::process::id()));
        let content = root.join("Content");

        std::fs::create_dir_all(&content).expect("create content dir");
        std::fs::write(
            content.join("MicrosoftGame.config"),
            r#"<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="1">
  <Identity Name="Example.DLCStub" Publisher="CN=test" Version="1.0.0.0" />
  <ShellVisuals DefaultDisplayName="DLC Game Stub" PublisherDisplayName="Publisher" />
  <TargetDeviceFamilyForDLC>PC</TargetDeviceFamilyForDLC>
  <AllowedProducts>
    <AllowedProduct>9TEST</AllowedProduct>
  </AllowedProducts>
</Game>"#,
        )
        .expect("write config");

        assert!(xbox_record_from_game_directory(&root).is_none());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn xbox_action_for_registered_candidate_uses_appsfolder_activation() {
        let candidate = sample_candidate();

        let action = xbox_action_for_candidate(&candidate, true);

        assert_eq!(action.kind, "executable");
        assert!(action.target.ends_with(r"\explorer.exe"));
        assert_eq!(
            action.arguments,
            vec!["shell:AppsFolder\\Microsoft.HaloInfinite_8wekyb3d8bbwe!App"]
        );
        assert_eq!(action.working_directory, None);
    }

    #[test]
    fn xbox_action_for_folder_only_candidate_uses_resolved_executable() {
        let mut candidate = sample_candidate();
        candidate.app_id = "Microsoft.HaloInfinite!App".to_string();
        candidate.launch_target =
            Some(r"C:\XboxGames\Halo Infinite\Content\HaloInfinite.exe".to_string());

        let action = xbox_action_for_candidate(&candidate, true);

        assert_eq!(action.kind, "executable");
        assert_eq!(
            action.target,
            r"C:\XboxGames\Halo Infinite\Content\HaloInfinite.exe"
        );
        assert_eq!(action.arguments, Vec::<String>::new());
        assert_eq!(
            action.working_directory.as_deref(),
            Some(r"C:\XboxGames\Halo Infinite\Content")
        );
    }

    #[test]
    fn dedupes_duplicate_discovery_records_by_app_id() {
        let records = vec![
            XboxDiscoveryRecord {
                app_id: "Microsoft.HaloInfinite_8wekyb3d8bbwe!App".to_string(),
                title: "Halo Infinite".to_string(),
                package_family_name: "Microsoft.HaloInfinite_8wekyb3d8bbwe".to_string(),
                package_name: Some("Halo Infinite".to_string()),
                package_full_name: Some(
                    "Microsoft.HaloInfinite_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
                ),
                install_location: Some("C:\\Games\\Halo".to_string()),
                launch_target: None,
                store_id: None,
                has_microsoft_game_config: true,
                is_framework: false,
                non_removable: false,
                signature_kind: Some("Store".to_string()),
            },
            XboxDiscoveryRecord {
                app_id: "Microsoft.HaloInfinite_8wekyb3d8bbwe!App".to_string(),
                title: "Halo Infinite".to_string(),
                package_family_name: "Microsoft.HaloInfinite_8wekyb3d8bbwe".to_string(),
                package_name: Some("Halo Infinite".to_string()),
                package_full_name: Some(
                    "Microsoft.HaloInfinite_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
                ),
                install_location: Some("C:\\Games\\Halo".to_string()),
                launch_target: None,
                store_id: None,
                has_microsoft_game_config: true,
                is_framework: false,
                non_removable: false,
                signature_kind: Some("Store".to_string()),
            },
        ];

        let candidates = discover_xbox_game_candidates_from_records(&records);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].app_id,
            "Microsoft.HaloInfinite_8wekyb3d8bbwe!App"
        );
    }

    #[test]
    fn prefers_registered_aumid_over_folder_only_record_for_same_store_game() {
        let records = vec![
            XboxDiscoveryRecord {
                app_id: "KeplerInteractive.Expedition33!AppExpedition33Shipping".to_string(),
                title: "Clair Obscur: Expedition 33".to_string(),
                package_family_name: "KeplerInteractive.Expedition33".to_string(),
                package_name: Some("KeplerInteractive.Expedition33".to_string()),
                package_full_name: Some("KeplerInteractive.Expedition33".to_string()),
                install_location: Some(r"E:\XboxGames\Clair Obscur- Expedition 33".to_string()),
                launch_target: Some(
                    r"E:\XboxGames\Clair Obscur- Expedition 33\Content\SandFall.exe".to_string(),
                ),
                store_id: Some("9NQZDJNV65BR".to_string()),
                has_microsoft_game_config: true,
                is_framework: false,
                non_removable: false,
                signature_kind: Some("XboxGames".to_string()),
            },
            XboxDiscoveryRecord {
                app_id: "KeplerInteractive.Expedition33_ymj30pw7xe604!AppExpedition33Shipping"
                    .to_string(),
                title: "Clair Obscur: Expedition 33".to_string(),
                package_family_name: "KeplerInteractive.Expedition33_ymj30pw7xe604".to_string(),
                package_name: Some("Clair Obscur: Expedition 33".to_string()),
                package_full_name: Some(
                    "KeplerInteractive.Expedition33_1.0.0.0_x64__ymj30pw7xe604".to_string(),
                ),
                install_location: Some(
                    r"C:\Program Files\WindowsApps\KeplerInteractive.Expedition33".to_string(),
                ),
                launch_target: None,
                store_id: Some("9NQZDJNV65BR".to_string()),
                has_microsoft_game_config: true,
                is_framework: false,
                non_removable: false,
                signature_kind: Some("Store".to_string()),
            },
        ];

        let candidates = discover_xbox_game_candidates_from_records(&records);

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].app_id,
            "KeplerInteractive.Expedition33_ymj30pw7xe604!AppExpedition33Shipping"
        );
        let action = xbox_action_for_candidate(&candidates[0], true);
        assert!(action.target.ends_with(r"\explorer.exe"));
        assert_eq!(
            action.arguments,
            vec![
                "shell:AppsFolder\\KeplerInteractive.Expedition33_ymj30pw7xe604!AppExpedition33Shipping"
            ]
        );
    }

    #[test]
    fn parse_discovery_output_skips_invalid_lines_without_failing() {
        let raw = concat!(
            r#"{"AppId":"Microsoft.HaloInfinite_8wekyb3d8bbwe!App","Title":"Halo Infinite","PackageFamilyName":"Microsoft.HaloInfinite_8wekyb3d8bbwe","PackageName":"Halo Infinite","PackageFullName":"Microsoft.HaloInfinite_1.0.0.0_x64__8wekyb3d8bbwe","InstallLocation":"C:\\Games\\Halo","HasMicrosoftGameConfig":true,"IsFramework":false,"NonRemovable":false,"SignatureKind":3}"#,
            "\n",
            "not-json",
            "\n",
            r#"{"AppId":"Microsoft.ForzaHorizon5_8wekyb3d8bbwe!App","Title":"Forza Horizon 5","PackageFamilyName":"Microsoft.ForzaHorizon5_8wekyb3d8bbwe","PackageName":null,"PackageFullName":null,"InstallLocation":null,"HasMicrosoftGameConfig":true,"IsFramework":false,"NonRemovable":false,"SignatureKind":"Store"}"#,
            "\n",
        );

        let records = parse_discovery_output(raw).expect("parse discovery output");
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].app_id,
            "Microsoft.HaloInfinite_8wekyb3d8bbwe!App"
        );
        assert_eq!(
            records[1].app_id,
            "Microsoft.ForzaHorizon5_8wekyb3d8bbwe!App"
        );
        assert_eq!(records[0].signature_kind.as_deref(), Some("3"));
        assert_eq!(records[1].signature_kind.as_deref(), Some("Store"));
    }

    #[test]
    fn normalize_xbox_title_prefers_package_name_for_placeholder_titles() {
        assert_eq!(
            normalize_xbox_title(
                "App",
                Some("Halo Infinite"),
                "Microsoft.HaloInfinite_8wekyb3d8bbwe"
            ),
            "Halo Infinite"
        );
    }

    #[test]
    fn should_keep_xbox_record_accepts_placeholder_title_with_package_name() {
        let record = XboxDiscoveryRecord {
            app_id: "Microsoft.HaloInfinite_8wekyb3d8bbwe!App".to_string(),
            title: "App".to_string(),
            package_family_name: "Microsoft.HaloInfinite_8wekyb3d8bbwe".to_string(),
            package_name: Some("Halo Infinite".to_string()),
            package_full_name: Some(
                "Microsoft.HaloInfinite_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
            ),
            install_location: Some("C:\\Games\\Halo".to_string()),
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: true,
            is_framework: false,
            non_removable: true,
            signature_kind: Some("Store".to_string()),
        };

        assert!(should_keep_xbox_record(&record));
    }

    #[test]
    fn should_keep_xbox_record_rejects_known_infrastructure_package() {
        let record = XboxDiscoveryRecord {
            app_id: "Microsoft.GamingServices_8wekyb3d8bbwe!App".to_string(),
            title: "Gaming Services".to_string(),
            package_family_name: "Microsoft.GamingServices_8wekyb3d8bbwe".to_string(),
            package_name: Some("Gaming Services".to_string()),
            package_full_name: Some(
                "Microsoft.GamingServices_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
            ),
            install_location: Some("C:\\Program Files\\WindowsApps\\GamingServices".to_string()),
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: false,
            is_framework: false,
            non_removable: true,
            signature_kind: Some("Store".to_string()),
        };

        assert!(!should_keep_xbox_record(&record));
    }

    #[test]
    fn should_keep_xbox_record_accepts_real_game_without_package_metadata() {
        let record = XboxDiscoveryRecord {
            app_id: "Microsoft.ForzaHorizon5_8wekyb3d8bbwe!App".to_string(),
            title: "Forza Horizon 5".to_string(),
            package_family_name: "Microsoft.ForzaHorizon5_8wekyb3d8bbwe".to_string(),
            package_name: None,
            package_full_name: None,
            install_location: None,
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: true,
            is_framework: false,
            non_removable: false,
            signature_kind: None,
        };

        assert!(should_keep_xbox_record(&record));
    }

    #[test]
    fn normalize_xbox_title_falls_back_to_package_family_for_ms_resource_title() {
        assert_eq!(
            normalize_xbox_title(
                "ms-resource:AppTitle",
                None,
                "Microsoft.XboxTCUI_8wekyb3d8bbwe"
            ),
            "Microsoft.XboxTCUI_8wekyb3d8bbwe"
        );
    }

    #[test]
    fn should_keep_xbox_record_rejects_tcui_helper_packages() {
        let record = XboxDiscoveryRecord {
            app_id: "Microsoft.XboxTCUI_8wekyb3d8bbwe!App".to_string(),
            title: "Xbox TCUI".to_string(),
            package_family_name: "Microsoft.XboxTCUI_8wekyb3d8bbwe".to_string(),
            package_name: Some("Xbox TCUI".to_string()),
            package_full_name: Some("Microsoft.XboxTCUI_1.0.0.0_x64__8wekyb3d8bbwe".to_string()),
            install_location: Some("C:\\Program Files\\WindowsApps\\XboxTCUI".to_string()),
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: false,
            is_framework: false,
            non_removable: true,
            signature_kind: Some("Store".to_string()),
        };

        assert!(!should_keep_xbox_record(&record));
    }

    #[test]
    fn should_keep_xbox_record_rejects_common_store_apps() {
        let record = XboxDiscoveryRecord {
            app_id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".to_string(),
            title: "Calculadora".to_string(),
            package_family_name: "Microsoft.WindowsCalculator_8wekyb3d8bbwe".to_string(),
            package_name: Some("Microsoft.WindowsCalculator".to_string()),
            package_full_name: Some(
                "Microsoft.WindowsCalculator_10.0.0.0_x64__8wekyb3d8bbwe".to_string(),
            ),
            install_location: Some("C:\\Program Files\\WindowsApps\\Calculator".to_string()),
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: false,
            is_framework: false,
            non_removable: false,
            signature_kind: Some("Store".to_string()),
        };

        assert!(!should_keep_xbox_record(&record));
    }

    #[test]
    fn should_keep_xbox_record_rejects_store_app_without_game_config() {
        let record = XboxDiscoveryRecord {
            app_id: "Microsoft.MicrosoftSolitaireCollection_8wekyb3d8bbwe!App".to_string(),
            title: "Solitaire & Casual Games".to_string(),
            package_family_name: "Microsoft.MicrosoftSolitaireCollection_8wekyb3d8bbwe".to_string(),
            package_name: Some("Microsoft.MicrosoftSolitaireCollection".to_string()),
            package_full_name: Some(
                "Microsoft.MicrosoftSolitaireCollection_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
            ),
            install_location: Some("C:\\Program Files\\WindowsApps\\Solitaire".to_string()),
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: false,
            is_framework: false,
            non_removable: false,
            signature_kind: Some("Store".to_string()),
        };

        assert!(!should_keep_xbox_record(&record));
    }

    #[test]
    fn should_keep_xbox_record_rejects_desktop_shortcut_app_ids() {
        let record = XboxDiscoveryRecord {
            app_id: r#"C:\Users\Guiti\AppData\Local\osu!\osu!.exe"#.to_string(),
            title: "osu!".to_string(),
            package_family_name: r#"C:\Users\Guiti\AppData\Local\osu"#.to_string(),
            package_name: None,
            package_full_name: None,
            install_location: None,
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: false,
            is_framework: false,
            non_removable: false,
            signature_kind: Some("Store".to_string()),
        };

        assert!(!should_keep_xbox_record(&record));
    }

    #[test]
    fn should_keep_xbox_record_rejects_non_game_package_app_ids() {
        let record = XboxDiscoveryRecord {
            app_id: "Microsoft.SkypeApp_kzf8qxf38zg5c!App".to_string(),
            title: "Skype".to_string(),
            package_family_name: "Microsoft.SkypeApp_kzf8qxf38zg5c".to_string(),
            package_name: Some("Skype".to_string()),
            package_full_name: Some("Microsoft.SkypeApp_1.0.0.0_x64__kzf8qxf38zg5c".to_string()),
            install_location: Some("C:\\Program Files\\WindowsApps\\Skype".to_string()),
            launch_target: None,
            store_id: None,
            has_microsoft_game_config: false,
            is_framework: false,
            non_removable: false,
            signature_kind: Some("Store".to_string()),
        };

        assert!(!should_keep_xbox_record(&record));
    }

    #[test]
    fn sync_xbox_games_imports_installed_candidate_once() {
        let mut connection = open_memory_database();
        let candidate = sample_candidate();

        let summary =
            sync_xbox_games_from_candidates(&mut connection, std::slice::from_ref(&candidate))
                .expect("sync xbox games");
        assert_eq!(summary.discovered, 1);
        assert_eq!(summary.inserted, 1);
        assert_eq!(summary.updated, 0);
        assert_eq!(summary.unavailable, 0);

        let entry = connection
            .query_row(
                r#"
                SELECT library_entries.primary_platform_id, library_entries.install_status, games.installed
                FROM library_entries
                JOIN games ON games.id = library_entries.game_id
                WHERE library_entries.id = ?1
                "#,
                params![candidate.entry_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)? == 1,
                    ))
                },
            )
            .expect("read xbox entry");
        assert_eq!(entry.0, "xbox");
        assert_eq!(entry.1, "installed");
        assert!(entry.2);

        let expected_target = windows_explorer_target().to_string_lossy().to_string();
        let action = connection
            .query_row(
                r#"
                SELECT kind, label, target, arguments_json
                FROM launch_actions
                WHERE game_id = ?1 AND platform_id = 'xbox' AND is_primary = 1
                "#,
                params![candidate.game_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("read xbox action");

        assert_eq!(action.0, "executable");
        assert_eq!(action.1, "Jogar no Xbox");
        assert_eq!(action.2, expected_target);
        assert_eq!(
            action.3,
            r#"["shell:AppsFolder\\Microsoft.HaloInfinite_8wekyb3d8bbwe!App"]"#
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn sync_xbox_games_with_live_windows_inventory() {
        let records = collect_xbox_discovery_records().expect("collect live xbox records");
        let candidates = discover_xbox_game_candidates_from_records(&records);
        let mut connection = open_memory_database();
        let summary = sync_xbox_games_from_candidates(&mut connection, &candidates)
            .expect("sync live xbox inventory");
        let entries = crate::storage::list_library_entries(&connection)
            .expect("list live xbox entries after sync");

        assert!(records.len() >= candidates.len());
        assert!(summary.discovered <= candidates.len());
        assert_eq!(entries.len(), candidates.len());
    }

    #[test]
    fn sync_xbox_games_roundtrips_without_duplicates_and_restores_install_state() {
        let mut connection = open_memory_database();
        let candidate = sample_candidate();

        let initial =
            sync_xbox_games_from_candidates(&mut connection, std::slice::from_ref(&candidate))
                .expect("initial sync");
        assert_eq!(initial.inserted, 1);

        let missing = sync_xbox_games_from_candidates(&mut connection, &[]).expect("missing sync");
        assert_eq!(missing.unavailable, 1);

        connection
            .execute(
                "UPDATE library_entries SET is_archived = 1 WHERE id = ?1",
                params![candidate.entry_id],
            )
            .expect("archive xbox entry");

        let restored =
            sync_xbox_games_from_candidates(&mut connection, std::slice::from_ref(&candidate))
                .expect("restored sync");
        assert_eq!(restored.inserted, 0);
        assert_eq!(restored.updated, 1);

        let counts = connection
            .query_row(
                r#"
                SELECT
                    (SELECT COUNT(*) FROM games WHERE id = ?1),
                    (SELECT COUNT(*) FROM library_entries WHERE id = ?2),
                    (SELECT COUNT(*) FROM game_sources WHERE game_id = ?3 AND platform_id = 'xbox'),
                    (SELECT COUNT(*) FROM launch_actions WHERE game_id = ?3 AND platform_id = 'xbox' AND is_primary = 1)
                "#,
                params![candidate.game_id, candidate.entry_id, candidate.game_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .expect("count xbox rows");

        assert_eq!(counts, (1, 1, 1, 1));

        let final_state = connection
            .query_row(
                r#"
                SELECT library_entries.is_archived, library_entries.install_status, games.installed
                FROM library_entries
                JOIN games ON games.id = library_entries.game_id
                WHERE library_entries.id = ?1
                "#,
                params![candidate.entry_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? == 1,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)? == 1,
                    ))
                },
            )
            .expect("final xbox state");

        assert_eq!(final_state.0, false);
        assert_eq!(final_state.1, "installed");
        assert!(final_state.2);

        let action = connection
            .query_row(
                r#"
                SELECT kind, label, target, arguments_json
                FROM launch_actions
                WHERE game_id = ?1 AND platform_id = 'xbox' AND is_primary = 1
                "#,
                params![candidate.game_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("final xbox action");

        assert_eq!(action.0, "executable");
        assert_eq!(action.1, "Jogar no Xbox");
        assert!(action.2.ends_with(r"\explorer.exe"));
        assert_eq!(
            action.3,
            r#"["shell:AppsFolder\\Microsoft.HaloInfinite_8wekyb3d8bbwe!App"]"#
        );
    }

    #[test]
    fn sync_xbox_games_marks_missing_entries_not_installed_and_switches_to_store_link() {
        let mut connection = open_memory_database();
        let candidate = sample_candidate();

        sync_xbox_games_from_candidates(&mut connection, std::slice::from_ref(&candidate))
            .expect("initial sync");

        let summary = sync_xbox_games_from_candidates(&mut connection, &[])
            .expect("resync with no installed apps");
        assert_eq!(summary.discovered, 0);
        assert_eq!(summary.inserted, 0);
        assert_eq!(summary.updated, 0);
        assert_eq!(summary.unavailable, 1);

        let entry = connection
            .query_row(
                r#"
                SELECT library_entries.install_status, games.installed
                FROM library_entries
                JOIN games ON games.id = library_entries.game_id
                WHERE library_entries.id = ?1
                "#,
                params![candidate.entry_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)),
            )
            .expect("read xbox entry after missing sync");

        assert_eq!(entry.0, "not_installed");
        assert!(!entry.1);

        let action = connection
            .query_row(
                r#"
                SELECT kind, label, target, arguments_json
                FROM launch_actions
                WHERE game_id = ?1 AND platform_id = 'xbox' AND is_primary = 1
                "#,
                params![candidate.game_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("read xbox store action");

        assert_eq!(action.0, "uri");
        assert_eq!(action.1, "Abrir Microsoft Store");
        assert!(action
            .2
            .starts_with("ms-windows-store://search/?query=Halo+Infinite"));
        assert_eq!(action.3, "[]");
    }

    #[test]
    fn sync_xbox_games_archives_rejected_existing_store_apps() {
        let mut connection = open_memory_database();
        let candidate = XboxGameCandidate {
            app_id: "Microsoft.MicrosoftSolitaireCollection_8wekyb3d8bbwe!App".to_string(),
            title: "Solitaire & Casual Games".to_string(),
            package_family_name: "Microsoft.MicrosoftSolitaireCollection_8wekyb3d8bbwe".to_string(),
            install_location: Some("C:\\Program Files\\WindowsApps\\Solitaire".to_string()),
            launch_target: None,
            store_id: None,
            source_id: "source-xbox-solitaire".to_string(),
            game_id: "game-xbox-solitaire".to_string(),
            entry_id: "entry-xbox-solitaire".to_string(),
            launch_id: "launch-xbox-solitaire".to_string(),
            accent_color: "#2563eb",
        };

        sync_xbox_games_from_candidates(&mut connection, std::slice::from_ref(&candidate))
            .expect("seed rejected app");
        let summary =
            sync_xbox_games_from_candidates(&mut connection, &[]).expect("archive rejected app");
        let state = connection
            .query_row(
                r#"
                SELECT is_archived, install_status
                FROM library_entries
                WHERE id = 'entry-xbox-solitaire'
                "#,
                [],
                |row| Ok((row.get::<_, i64>(0)? == 1, row.get::<_, String>(1)?)),
            )
            .expect("read archived rejected app");

        assert_eq!(summary.archived, 1);
        assert_eq!(summary.unavailable, 0);
        assert!(state.0);
        assert_eq!(state.1, "not_installed");
    }
}
