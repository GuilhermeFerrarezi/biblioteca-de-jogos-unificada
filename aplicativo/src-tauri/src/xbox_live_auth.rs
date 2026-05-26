use crate::{security, storage};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
#[cfg(not(test))]
use std::sync::{mpsc, Arc, Mutex};
#[cfg(not(test))]
use std::time::Duration;
#[cfg(not(test))]
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use url::form_urlencoded;
use url::Url;

const MICROSOFT_AUTHORIZE_ENDPOINT: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const MICROSOFT_TOKEN_ENDPOINT: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_USER_AUTH_ENDPOINT: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XBOX_XSTS_AUTH_ENDPOINT: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const XBOX_PRESENCE_ENDPOINT: &str = "https://userpresence.xboxlive.com/users/me";
const XBOX_TITLE_HISTORY_ENDPOINT: &str =
    "https://achievements.xboxlive.com/users/xuid({xuid})/history/titles";
const XBOX_LIVE_LOGIN_EVENT: &str = "xbox-live-login-complete";
const XBOX_LIVE_SCOPE: &str = "xboxlive.signin xboxlive.offline_access";
const XBOX_LIVE_REDIRECT_URI: &str = "https://login.microsoftonline.com/common/oauth2/nativeclient";
const XBOX_LIVE_LOGIN_WINDOW_LABEL: &str = "xbox-live-login";
const XBOX_TITLE_HISTORY_PAGE_SIZE: usize = 100;
const XBOX_TITLE_HISTORY_MAX_PAGES: usize = 20;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XboxLiveLoginStartDto {
    pending: bool,
    provider_id: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct XboxLiveLoginCompleteDto {
    success: bool,
    provider_id: &'static str,
    xuid: Option<String>,
    gamertag: Option<String>,
    message: String,
    code: Option<&'static str>,
    phase: Option<&'static str>,
    recoverable: Option<bool>,
    details_sanitized: Option<String>,
}

#[derive(Debug, Clone)]
struct XboxLiveLoginError {
    code: &'static str,
    phase: &'static str,
    recoverable: bool,
    message: String,
    details_sanitized: Option<String>,
}

impl XboxLiveLoginError {
    fn new(
        code: &'static str,
        phase: &'static str,
        recoverable: bool,
        message: impl Into<String>,
        details_sanitized: Option<String>,
    ) -> Self {
        Self {
            code,
            phase,
            recoverable,
            message: message.into(),
            details_sanitized,
        }
    }

    fn timeout(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_login_timeout",
            "callback_wait",
            true,
            message,
            None,
        )
    }

    fn callback(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_login_callback_failed",
            "callback",
            true,
            message,
            None,
        )
    }

    fn token_exchange(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_token_exchange_failed",
            "token_exchange",
            true,
            message,
            None,
        )
    }

    fn xbox_user_auth(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_user_auth_failed",
            "xbox_user_auth",
            true,
            message,
            None,
        )
    }

    fn xsts_exchange(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_xsts_exchange_failed",
            "xsts_exchange",
            true,
            message,
            None,
        )
    }

    fn identity_fetch(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_identity_fetch_failed",
            "identity_fetch",
            true,
            message,
            None,
        )
    }

    fn persist_session(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_persist_session_failed",
            "persist_session",
            true,
            message,
            None,
        )
    }

    fn missing_configuration(message: impl Into<String>) -> Self {
        Self::new(
            "xbox_live_configuration_missing",
            "preflight",
            true,
            message,
            None,
        )
    }

    fn to_complete_dto(&self) -> XboxLiveLoginCompleteDto {
        XboxLiveLoginCompleteDto {
            success: false,
            provider_id: "xbox",
            xuid: None,
            gamertag: None,
            message: self.message.clone(),
            code: Some(self.code),
            phase: Some(self.phase),
            recoverable: Some(self.recoverable),
            details_sanitized: self.details_sanitized.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XboxAchievementHistoryRecord {
    pub title_id: String,
    pub service_config_id: Option<String>,
    pub name: String,
    pub title_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XboxLiveAuthError {
    CallbackUnavailable,
    ClientIdUnavailable,
    RefreshTokenUnavailable,
    SessionUnavailable { operation: &'static str },
    RequestFailed { operation: &'static str },
}

impl fmt::Display for XboxLiveAuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            XboxLiveAuthError::CallbackUnavailable => {
                "Nao foi possivel preparar a janela de login Xbox Live."
            }
            XboxLiveAuthError::ClientIdUnavailable => {
                "Configure o Client ID do app Microsoft antes de conectar o Xbox Live."
            }
            XboxLiveAuthError::RefreshTokenUnavailable => {
                "Nao foi possivel ler a sessao Xbox Live salva."
            }
            XboxLiveAuthError::SessionUnavailable { operation } => {
                return write!(
                    formatter,
                    "Nao foi possivel concluir {operation} do Xbox Live."
                );
            }
            XboxLiveAuthError::RequestFailed { operation } => {
                return write!(formatter, "Nao foi possivel concluir {operation}.");
            }
        };

        formatter.write_str(message)
    }
}

#[cfg(not(test))]
pub fn start_login(
    app: AppHandle,
    connection: Arc<Mutex<rusqlite::Connection>>,
    auth_vault: Arc<security::AuthVault>,
) -> Result<XboxLiveLoginStartDto, XboxLiveAuthError> {
    let client_id = {
        let connection = connection
            .lock()
            .map_err(|_| XboxLiveAuthError::ClientIdUnavailable)?;
        storage::read_xbox_live_client_id(&connection)
            .map_err(|_| XboxLiveAuthError::ClientIdUnavailable)?
            .ok_or(XboxLiveAuthError::ClientIdUnavailable)?
    };
    let state = generate_state();
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let login_url = build_login_url(&client_id, &state, &code_challenge)?;
    let (authorization_code_tx, authorization_code_rx) = mpsc::channel::<Result<String, String>>();
    let worker_app = app.clone();
    let worker_connection = connection.clone();
    let worker_auth_vault = auth_vault.clone();
    let worker_client_id = client_id.clone();

    std::thread::spawn(move || {
        let result = authorization_code_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| {
                XboxLiveLoginError::timeout(
                    "Tempo expirado aguardando o retorno do login Xbox Live.",
                )
            })
            .and_then(|authorization_code| authorization_code.map_err(XboxLiveLoginError::callback))
            .and_then(|authorization_code| {
                redeem_authorization_code(
                    &authorization_code,
                    XBOX_LIVE_REDIRECT_URI,
                    &code_verifier,
                    &worker_client_id,
                )
                .map_err(XboxLiveLoginError::token_exchange)
            })
            .and_then(|microsoft_tokens| {
                exchange_for_xbox_tokens(&microsoft_tokens.access_token)
                    .map(|session| (microsoft_tokens, session))
                    .map_err(XboxLiveLoginError::xbox_user_auth)
            })
            .and_then(|(microsoft_tokens, xbox_session)| {
                fetch_xbox_identity(&xbox_session.user_hash, &xbox_session.token)
                    .map(|identity| (microsoft_tokens, xbox_session, identity))
                    .map_err(XboxLiveLoginError::identity_fetch)
            })
            .and_then(|(microsoft_tokens, xbox_session, identity)| {
                let xuid = identity.xuid.or_else(|| xbox_session.xuid).ok_or_else(|| {
                    XboxLiveLoginError::identity_fetch("Xbox Live nao retornou um XUID valido.")
                })?;
                {
                    let mut connection = worker_connection.lock().map_err(|_| {
                        XboxLiveLoginError::persist_session(
                            "Nao foi possivel salvar a conta Xbox conectada.",
                        )
                    })?;
                    storage::save_verified_xbox_account_config(&mut connection, &xuid).map_err(
                        |_| {
                            XboxLiveLoginError::persist_session(
                                "Nao foi possivel salvar a conta Xbox conectada.",
                            )
                        },
                    )?;
                }
                if let Some(refresh_token) = microsoft_tokens.refresh_token {
                    let _ = worker_auth_vault.save_xbox_live_refresh_token(
                        security::XboxLiveRefreshTokenInput { refresh_token },
                    );
                }
                Ok((xuid, identity.gamertag))
            });

        let event = match result {
            Ok((xuid, gamertag)) => XboxLiveLoginCompleteDto {
                success: true,
                provider_id: "xbox",
                xuid: Some(xuid),
                gamertag,
                message: "Xbox Live conectado neste dispositivo.".to_string(),
                code: None,
                phase: None,
                recoverable: None,
                details_sanitized: None,
            },
            Err(error) => error.to_complete_dto(),
        };

        let _ = worker_app.emit(XBOX_LIVE_LOGIN_EVENT, event);
    });

    let window_app = app.clone();
    let authorization_code_tx_for_window = authorization_code_tx.clone();
    let login_state = state.clone();
    std::thread::spawn(move || {
        let window_label = XBOX_LIVE_LOGIN_WINDOW_LABEL.to_string();
        let navigation_app = window_app.clone();
        let navigation_label = window_label.clone();
        let login_result =
            WebviewWindowBuilder::new(&window_app, window_label, WebviewUrl::External(login_url))
                .title("Conectar Xbox Live")
                .center()
                .on_navigation(move |url| {
                    if !is_xbox_live_callback_url(url) {
                        return true;
                    }

                    let result = verify_callback(url, &login_state, XBOX_LIVE_REDIRECT_URI);
                    if let Some(window) = navigation_app.get_webview_window(&navigation_label) {
                        let _ = window.close();
                    }
                    let _ = authorization_code_tx_for_window.send(result);
                    false
                })
                .build();

        if let Err(error) = login_result {
            let _ = authorization_code_tx.send(Err(format!(
                "Nao foi possivel abrir a janela de login Xbox Live: {error}"
            )));
        }
    });

    Ok(XboxLiveLoginStartDto {
        pending: true,
        provider_id: "xbox",
    })
}

pub fn fetch_xbox_achievement_title_history(
    connection: &rusqlite::Connection,
    auth_vault: &security::AuthVault,
    xuid: &str,
) -> Result<Vec<XboxAchievementHistoryRecord>, XboxLiveAuthError> {
    let tokens = load_xbox_live_tokens(connection, auth_vault)?;
    let xsts = exchange_for_xbox_tokens(&tokens.access_token).map_err(|_| {
        XboxLiveAuthError::SessionUnavailable {
            operation: "conclusao da sessao Xbox Live",
        }
    })?;

    fetch_title_history_pages(&xsts.user_hash, &xsts.token, xuid).map_err(|_| {
        XboxLiveAuthError::RequestFailed {
            operation: "consulta do historico de titulos Xbox",
        }
    })
}

fn redeem_authorization_code(
    authorization_code: &str,
    redirect_uri: &str,
    code_verifier: &str,
    client_id: &str,
) -> Result<MicrosoftTokenResponse, String> {
    let client = Client::new();
    let response = client
        .post(MICROSOFT_TOKEN_ENDPOINT)
        .form(&authorization_code_token_form(
            authorization_code,
            redirect_uri,
            code_verifier,
            client_id,
        ))
        .send()
        .map_err(|_| "Nao foi possivel trocar o codigo de autorizacao do Xbox Live.".to_string())?;

    if !response.status().is_success() {
        return Err(response_failure_message(
            "concluir o login Xbox Live",
            response,
        ));
    }

    response.json::<MicrosoftTokenResponse>().map_err(|_| {
        "Nao foi possivel interpretar a resposta da troca do codigo Xbox Live.".to_string()
    })
}

fn load_xbox_live_tokens(
    connection: &rusqlite::Connection,
    auth_vault: &security::AuthVault,
) -> Result<MicrosoftTokenResponse, XboxLiveAuthError> {
    let refresh_token = auth_vault
        .xbox_live_refresh_token()
        .map_err(|_| XboxLiveAuthError::RefreshTokenUnavailable)?
        .ok_or(XboxLiveAuthError::RefreshTokenUnavailable)?;

    let client_id = storage::read_xbox_live_client_id(connection)
        .map_err(|_| XboxLiveAuthError::ClientIdUnavailable)?
        .ok_or(XboxLiveAuthError::ClientIdUnavailable)?;

    refresh_microsoft_access_token(&refresh_token, &client_id, auth_vault)
}

fn refresh_microsoft_access_token(
    refresh_token: &str,
    client_id: &str,
    auth_vault: &security::AuthVault,
) -> Result<MicrosoftTokenResponse, XboxLiveAuthError> {
    let client = Client::new();
    let response = client
        .post(MICROSOFT_TOKEN_ENDPOINT)
        .form(&refresh_token_form(refresh_token, client_id))
        .send()
        .map_err(|_| XboxLiveAuthError::SessionUnavailable {
            operation: "atualizacao da sessao Xbox Live",
        })?;

    if !response.status().is_success() {
        return Err(XboxLiveAuthError::SessionUnavailable {
            operation: "atualizacao da sessao Xbox Live",
        });
    }

    let response = response.json::<MicrosoftTokenResponse>().map_err(|_| {
        XboxLiveAuthError::SessionUnavailable {
            operation: "leitura da sessao Xbox Live renovada",
        }
    })?;

    if let Some(next_refresh_token) = response.refresh_token.as_deref() {
        let _ = auth_vault.save_xbox_live_refresh_token(security::XboxLiveRefreshTokenInput {
            refresh_token: next_refresh_token.to_string(),
        });
    }

    Ok(response)
}

fn exchange_for_xbox_tokens(access_token: &str) -> Result<XboxSession, String> {
    let client = Client::new();
    let user_token_response = client
        .post(XBOX_USER_AUTH_ENDPOINT)
        .header("x-xbl-contract-version", "1")
        .json(&XboxUserAuthRequest {
            properties: XboxUserAuthProperties {
                auth_method: "RPS",
                site_name: "user.auth.xboxlive.com",
                rps_ticket: format!("d={access_token}"),
            },
            relying_party: "http://auth.xboxlive.com",
            token_type: "JWT",
        })
        .send()
        .map_err(|_| "Nao foi possivel autenticar o usuario Xbox Live.".to_string())?;
    if !user_token_response.status().is_success() {
        return Err(response_failure_message(
            "autenticar o usuario Xbox Live",
            user_token_response,
        ));
    }
    let user_token_response = user_token_response
        .json::<XboxAuthResponse>()
        .map_err(|_| "Nao foi possivel interpretar o token de usuario Xbox Live.".to_string())?;

    let xsts_response = client
        .post(XBOX_XSTS_AUTH_ENDPOINT)
        .header("x-xbl-contract-version", "1")
        .json(&XboxXstsRequest {
            properties: XboxXstsProperties {
                sandbox_id: "RETAIL",
                user_tokens: vec![user_token_response.token.clone()],
            },
            relying_party: "http://xboxlive.com",
            token_type: "JWT",
        })
        .send()
        .map_err(|_| "Nao foi possivel concluir a autorizacao Xbox Live.".to_string())?;
    if !xsts_response.status().is_success() {
        return Err(response_failure_message(
            "concluir a autorizacao Xbox Live",
            xsts_response,
        ));
    }
    let xsts_response = xsts_response
        .json::<XboxAuthResponse>()
        .map_err(|_| "Nao foi possivel interpretar o token Xbox Live.".to_string())?;

    let user_hash = xsts_response
        .display_claims
        .xui
        .first()
        .and_then(|claim| claim.uhs.clone())
        .ok_or_else(|| "Xbox Live nao retornou o user hash da sessao.".to_string())?;

    let xuid = xsts_response
        .display_claims
        .xui
        .first()
        .and_then(|claim| claim.xuid.clone());

    Ok(XboxSession {
        user_hash,
        token: xsts_response.token,
        xuid,
    })
}

fn fetch_xbox_identity(user_hash: &str, xsts_token: &str) -> Result<XboxIdentity, String> {
    let client = Client::new();
    let response = client
        .get(XBOX_PRESENCE_ENDPOINT)
        .header(
            "Authorization",
            format!("XBL3.0 x={user_hash};{xsts_token}"),
        )
        .send()
        .map_err(|_| "Nao foi possivel consultar a identidade Xbox Live.".to_string())?;
    if !response.status().is_success() {
        return Err(response_failure_message(
            "consultar a identidade Xbox Live",
            response,
        ));
    }
    let value = response
        .json::<serde_json::Value>()
        .map_err(|_| "Nao foi possivel interpretar a identidade Xbox Live.".to_string())?;

    Ok(XboxIdentity {
        xuid: extract_xuid(&value),
        gamertag: extract_gamertag(&value),
    })
}

fn fetch_title_history_pages(
    user_hash: &str,
    xsts_token: &str,
    xuid: &str,
) -> Result<Vec<XboxAchievementHistoryRecord>, String> {
    let client = Client::new();

    collect_title_history_pages(|continuation_token| {
        fetch_title_history_page(&client, user_hash, xsts_token, xuid, continuation_token)
    })
}

fn fetch_title_history_page(
    client: &Client,
    user_hash: &str,
    xsts_token: &str,
    xuid: &str,
    continuation_token: Option<&str>,
) -> Result<XboxTitleHistoryPage, String> {
    let endpoint = XBOX_TITLE_HISTORY_ENDPOINT.replace("{xuid}", xuid);
    let mut request = client
        .get(endpoint)
        .query(&[("maxItems", XBOX_TITLE_HISTORY_PAGE_SIZE.to_string())])
        .header(
            "Authorization",
            format!("XBL3.0 x={user_hash};{xsts_token}"),
        )
        .header("x-xbl-contract-version", "2");

    if let Some(continuation_token) = continuation_token {
        request = request.query(&[("continuationToken", continuation_token)]);
    }

    let response = request
        .send()
        .map_err(|_| "Nao foi possivel consultar o historico de titulos Xbox.".to_string())?;
    if !response.status().is_success() {
        return Err(response_failure_message(
            "consultar o historico de titulos Xbox",
            response,
        ));
    }
    let payload = response
        .json::<serde_json::Value>()
        .map_err(|_| "Nao foi possivel interpretar o historico de titulos Xbox.".to_string())?;

    Ok(parse_title_history_page(&payload))
}

fn collect_title_history_pages<F>(
    mut fetch_page: F,
) -> Result<Vec<XboxAchievementHistoryRecord>, String>
where
    F: FnMut(Option<&str>) -> Result<XboxTitleHistoryPage, String>,
{
    let mut records = Vec::new();
    let mut seen_title_ids = HashSet::new();
    let mut continuation_token: Option<String> = None;

    for _ in 0..XBOX_TITLE_HISTORY_MAX_PAGES {
        let page = fetch_page(continuation_token.as_deref())?;

        for record in page.records {
            if seen_title_ids.insert(record.title_id.clone()) {
                records.push(record);
            }
        }

        match page.continuation_token {
            Some(next_token) => continuation_token = Some(next_token),
            None => return Ok(records),
        }
    }

    Ok(records)
}

fn parse_title_history_page(payload: &serde_json::Value) -> XboxTitleHistoryPage {
    XboxTitleHistoryPage {
        records: parse_title_history_records(payload),
        continuation_token: parse_continuation_token(payload),
    }
}

fn parse_title_history_records(payload: &serde_json::Value) -> Vec<XboxAchievementHistoryRecord> {
    let items = match payload {
        serde_json::Value::Array(values) => values.as_slice(),
        serde_json::Value::Object(map) => map
            .get("titles")
            .or_else(|| map.get("items"))
            .and_then(|value| value.as_array())
            .map(|values| values.as_slice())
            .unwrap_or(&[]),
        _ => &[],
    };

    items
        .iter()
        .filter_map(parse_title_history_record)
        .collect()
}

fn parse_title_history_record(value: &serde_json::Value) -> Option<XboxAchievementHistoryRecord> {
    let title_id = extract_string_from_value(value, &["titleId"])?;
    let name = extract_string_from_value(value, &["name"])?;

    Some(XboxAchievementHistoryRecord {
        title_id,
        service_config_id: extract_string_from_value(value, &["serviceConfigId"]),
        name,
        title_type: extract_string_from_value(value, &["titleType"]),
    })
}

fn parse_continuation_token(payload: &serde_json::Value) -> Option<String> {
    const TOKEN_KEYS: &[&str] = &[
        "continuationToken",
        "continuation_token",
        "nextContinuationToken",
        "next_continuation_token",
    ];
    const TOKEN_CONTAINERS: &[&str] = &["pagingInfo", "paging", "pagination", "pageInfo", "links"];

    for key in TOKEN_KEYS {
        if let Some(token) = direct_string_field(payload, key) {
            return Some(token);
        }
    }

    if let serde_json::Value::Object(map) = payload {
        for container_key in TOKEN_CONTAINERS {
            if let Some(container) = map.get(*container_key) {
                for key in TOKEN_KEYS {
                    if let Some(token) = direct_string_field(container, key) {
                        return Some(token);
                    }
                }

                if let Some(next) = container.get("next") {
                    for key in TOKEN_KEYS {
                        if let Some(token) = direct_string_field(next, key) {
                            return Some(token);
                        }
                    }
                }
            }
        }
    }

    None
}

fn build_login_url(
    client_id: &str,
    state: &str,
    code_challenge: &str,
) -> Result<Url, XboxLiveAuthError> {
    let mut url = Url::parse(MICROSOFT_AUTHORIZE_ENDPOINT)
        .map_err(|_| XboxLiveAuthError::CallbackUnavailable)?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", XBOX_LIVE_REDIRECT_URI)
        .append_pair("scope", XBOX_LIVE_SCOPE)
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");

    Ok(url)
}

fn authorization_code_token_form<'a>(
    authorization_code: &'a str,
    redirect_uri: &'a str,
    code_verifier: &'a str,
    client_id: &'a str,
) -> [(&'a str, &'a str); 6] {
    [
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", authorization_code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
        ("scope", XBOX_LIVE_SCOPE),
    ]
}

fn refresh_token_form<'a>(refresh_token: &'a str, client_id: &'a str) -> [(&'a str, &'a str); 4] {
    [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("scope", XBOX_LIVE_SCOPE),
    ]
}

fn verify_callback(
    callback_url: &Url,
    expected_state: &str,
    expected_redirect_uri: &str,
) -> Result<String, String> {
    if !callback_url.as_str().starts_with(expected_redirect_uri) {
        return Err("Resposta do login Xbox Live nao corresponde a sessao iniciada.".to_string());
    }
    if callback_value(callback_url, "state").as_deref() != Some(expected_state) {
        return Err("Resposta do login Xbox Live rejeitada.".to_string());
    }
    if callback_value(callback_url, "code").is_none() {
        return Err("Login Xbox Live cancelado ou incompleto.".to_string());
    }

    callback_value(callback_url, "code")
        .ok_or_else(|| "Resposta do login Xbox Live nao trouxe um codigo valido.".to_string())
}

fn is_xbox_live_callback_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("login.microsoftonline.com")
        && url.path() == "/common/oauth2/nativeclient"
}

fn callback_value(url: &Url, key: &str) -> Option<String> {
    query_value(url.query(), key).or_else(|| query_value(url.fragment(), key))
}

fn response_failure_message(operation: &str, response: reqwest::blocking::Response) -> String {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    let body = body.trim();

    if body.is_empty() {
        format!("Nao foi possivel {operation}. HTTP {status}.")
    } else {
        format!("Nao foi possivel {operation}. HTTP {status}: {body}")
    }
}

fn query_value(query: Option<&str>, key: &str) -> Option<String> {
    query
        .into_iter()
        .flat_map(|value| form_urlencoded::parse(value.as_bytes()))
        .find(|(candidate_key, _)| candidate_key == key)
        .map(|(_, value)| value.into_owned())
}

fn _collect_callback_values(url: &Url) -> HashMap<String, String> {
    let mut values = HashMap::new();

    for (key, value) in url.query_pairs() {
        values.insert(key.into_owned(), value.into_owned());
    }
    if let Some(fragment) = url.fragment() {
        for (key, value) in form_urlencoded::parse(fragment.as_bytes()) {
            values.insert(key.into_owned(), value.into_owned());
        }
    }

    values
}

fn generate_state() -> String {
    encode_random_bytes(24)
}

fn generate_code_verifier() -> String {
    let verifier = encode_random_bytes(64);
    verifier.trim_end_matches('=').to_string()
}

fn generate_code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn encode_random_bytes(length: usize) -> String {
    let mut bytes = vec![0_u8; length];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn extract_xuid(value: &serde_json::Value) -> Option<String> {
    extract_string_from_value(value, &["xuid", "xid", "userId", "id"]).and_then(|candidate| {
        let trimmed = candidate.trim();
        if trimmed.len() == 17 && trimmed.chars().all(|character| character.is_ascii_digit()) {
            Some(trimmed.to_string())
        } else {
            None
        }
    })
}

fn extract_gamertag(value: &serde_json::Value) -> Option<String> {
    extract_string_from_value(value, &["gamertag", "gt", "displayName", "userName"])
}

fn extract_string_from_value(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in keys {
                if let Some(entry) = map.get(*key) {
                    match entry {
                        serde_json::Value::String(string_value) => {
                            let trimmed = string_value.trim();
                            if !trimmed.is_empty() {
                                return Some(trimmed.to_string());
                            }
                        }
                        serde_json::Value::Number(number_value) => {
                            return Some(number_value.to_string());
                        }
                        _ => {}
                    }
                }
            }

            for nested_value in map.values() {
                if let Some(candidate) = extract_string_from_value(nested_value, keys) {
                    return Some(candidate);
                }
            }
            None
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|nested_value| extract_string_from_value(nested_value, keys)),
        _ => None,
    }
}

fn direct_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    let entry = value.as_object()?.get(key)?;

    match entry {
        serde_json::Value::String(string_value) => {
            let trimmed = string_value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Number(number_value) => Some(number_value.to_string()),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct MicrosoftTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxAuthResponse {
    token: String,
    display_claims: XboxDisplayClaims,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxDisplayClaims {
    #[serde(rename = "xui")]
    xui: Vec<XboxUserClaim>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XboxUserClaim {
    uhs: Option<String>,
    #[serde(alias = "xid", alias = "userId", alias = "id")]
    xuid: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XboxUserAuthRequest<'a> {
    properties: XboxUserAuthProperties<'a>,
    relying_party: &'a str,
    token_type: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XboxUserAuthProperties<'a> {
    auth_method: &'a str,
    site_name: &'a str,
    rps_ticket: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XboxXstsRequest<'a> {
    properties: XboxXstsProperties<'a>,
    relying_party: &'a str,
    token_type: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XboxXstsProperties<'a> {
    sandbox_id: &'a str,
    user_tokens: Vec<String>,
}

struct XboxSession {
    user_hash: String,
    token: String,
    xuid: Option<String>,
}

struct XboxIdentity {
    xuid: Option<String>,
    gamertag: Option<String>,
}

struct XboxTitleHistoryPage {
    records: Vec<XboxAchievementHistoryRecord>,
    continuation_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_login_url_uses_configured_client_id() {
        let url = build_login_url("00000000-1111-2222-3333-444444444444", "state", "challenge")
            .expect("build login url");

        let client_id = url
            .query_pairs()
            .find(|(key, _)| key == "client_id")
            .map(|(_, value)| value.into_owned());

        assert_eq!(
            client_id.as_deref(),
            Some("00000000-1111-2222-3333-444444444444")
        );
    }

    #[test]
    fn authorization_code_token_form_omits_client_secret() {
        let form = authorization_code_token_form("code", "redirect", "verifier", "client-id");

        assert_eq!(
            form.iter()
                .find(|(key, _)| *key == "client_id")
                .map(|(_, value)| *value),
            Some("client-id")
        );
        assert_eq!(
            form.iter()
                .find(|(key, _)| *key == "grant_type")
                .map(|(_, value)| *value),
            Some("authorization_code")
        );
        assert!(form.iter().all(|(key, _)| *key != "client_secret"));
    }

    #[test]
    fn refresh_token_form_omits_client_secret() {
        let form = refresh_token_form("refresh-token", "client-id");

        assert_eq!(
            form.iter()
                .find(|(key, _)| *key == "client_id")
                .map(|(_, value)| *value),
            Some("client-id")
        );
        assert_eq!(
            form.iter()
                .find(|(key, _)| *key == "grant_type")
                .map(|(_, value)| *value),
            Some("refresh_token")
        );
        assert!(form.iter().all(|(key, _)| *key != "client_secret"));
    }

    #[test]
    fn login_error_complete_dto_exposes_structured_fields() {
        let dto = XboxLiveLoginError::new(
            "xbox_live_token_exchange_failed",
            "token_exchange",
            true,
            "Nao foi possivel trocar o codigo de autorizacao do Xbox Live.",
            Some("HTTP 400".to_string()),
        )
        .to_complete_dto();

        let value = serde_json::to_value(dto).expect("serialize dto");

        assert_eq!(value["success"], false);
        assert_eq!(value["providerId"], "xbox");
        assert_eq!(
            value["message"],
            "Nao foi possivel trocar o codigo de autorizacao do Xbox Live."
        );
        assert_eq!(value["code"], "xbox_live_token_exchange_failed");
        assert_eq!(value["phase"], "token_exchange");
        assert_eq!(value["recoverable"], true);
        assert_eq!(value["detailsSanitized"], "HTTP 400");
    }

    #[test]
    fn xbox_auth_response_accepts_xid_as_xuid_alias() {
        let response = serde_json::from_str::<XboxAuthResponse>(
            r#"{
                "Token": "token-value",
                "DisplayClaims": {
                    "xui": [
                        {
                            "uhs": "user-hash",
                            "xid": "2533274791234567"
                        }
                    ]
                }
            }"#,
        )
        .expect("deserialize xbox auth response");

        assert_eq!(
            response
                .display_claims
                .xui
                .first()
                .and_then(|claim| claim.xuid.as_deref()),
            Some("2533274791234567")
        );
    }

    #[test]
    fn parse_title_history_page_reads_records_and_continuation_token() {
        let payload = serde_json::json!({
            "titles": [
                {
                    "titleId": "100",
                    "serviceConfigId": "scid-100",
                    "name": "First Game",
                    "titleType": "Game"
                },
                {
                    "titleId": 200,
                    "name": "Second Game"
                }
            ],
            "pagingInfo": {
                "continuationToken": "next-page"
            }
        });

        let page = parse_title_history_page(&payload);

        assert_eq!(page.continuation_token.as_deref(), Some("next-page"));
        assert_eq!(page.records.len(), 2);
        assert_eq!(page.records[0].title_id, "100");
        assert_eq!(
            page.records[0].service_config_id.as_deref(),
            Some("scid-100")
        );
        assert_eq!(page.records[1].title_id, "200");
    }

    #[test]
    fn parse_continuation_token_accepts_common_response_locations() {
        let top_level = serde_json::json!({ "continuationToken": "top-level" });
        let snake_case = serde_json::json!({
            "paging": {
                "continuation_token": "snake-case"
            }
        });
        let links_next = serde_json::json!({
            "links": {
                "next": {
                    "nextContinuationToken": "linked-next"
                }
            }
        });

        assert_eq!(
            parse_continuation_token(&top_level).as_deref(),
            Some("top-level")
        );
        assert_eq!(
            parse_continuation_token(&snake_case).as_deref(),
            Some("snake-case")
        );
        assert_eq!(
            parse_continuation_token(&links_next).as_deref(),
            Some("linked-next")
        );
    }

    #[test]
    fn parse_continuation_token_ignores_title_record_fields() {
        let payload = serde_json::json!({
            "titles": [
                {
                    "titleId": "100",
                    "name": "First Game",
                    "continuationToken": "not-page-metadata"
                }
            ]
        });

        assert_eq!(parse_continuation_token(&payload), None);
    }

    #[test]
    fn collect_title_history_pages_uses_tokens_and_dedupes_title_ids() {
        let pages = vec![
            XboxTitleHistoryPage {
                records: vec![
                    title_history_record("100", "First Game"),
                    title_history_record("200", "Second Game"),
                ],
                continuation_token: Some("token-2".to_string()),
            },
            XboxTitleHistoryPage {
                records: vec![
                    title_history_record("100", "First Game Duplicate"),
                    title_history_record("300", "Third Game"),
                ],
                continuation_token: None,
            },
        ];
        let mut calls = Vec::new();
        let mut next_page = pages.into_iter();

        let records = collect_title_history_pages(|continuation_token| {
            calls.push(continuation_token.map(str::to_string));
            next_page
                .next()
                .ok_or_else(|| "unexpected call".to_string())
        })
        .expect("collect pages");

        assert_eq!(calls, vec![None, Some("token-2".to_string())]);
        assert_eq!(
            records
                .iter()
                .map(|record| record.title_id.as_str())
                .collect::<Vec<_>>(),
            vec!["100", "200", "300"]
        );
    }

    #[test]
    fn collect_title_history_pages_stops_at_safe_page_limit() {
        let mut calls = 0;

        let records = collect_title_history_pages(|_| {
            calls += 1;
            Ok(XboxTitleHistoryPage {
                records: vec![title_history_record(&calls.to_string(), "Game")],
                continuation_token: Some(format!("token-{}", calls + 1)),
            })
        })
        .expect("collect pages");

        assert_eq!(calls, XBOX_TITLE_HISTORY_MAX_PAGES);
        assert_eq!(records.len(), XBOX_TITLE_HISTORY_MAX_PAGES);
    }

    fn title_history_record(title_id: &str, name: &str) -> XboxAchievementHistoryRecord {
        XboxAchievementHistoryRecord {
            title_id: title_id.to_string(),
            service_config_id: None,
            name: name.to_string(),
            title_type: None,
        }
    }
}
