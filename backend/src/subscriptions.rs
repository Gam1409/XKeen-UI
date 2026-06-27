use crate::configs::validate_core_files;
use crate::controller;
use crate::logger::{log, ts};
use crate::types::*;
use axum::extract::{Path as AxumPath, Query, State};
use axum::response::{IntoResponse, Json};
use base64::Engine;
use chrono::Utc;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::process::Command;
use yaml_rust2::YamlLoader;

const STORE_VERSION: u8 = 1;
const LOCK_MAX_AGE: Duration = Duration::from_secs(30 * 60);
const MAX_LOG_BYTES: u64 = 256 * 1024;
const MAX_SUBSCRIPTION_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub url: String,
    pub core: String,
    pub mode: String,
    pub format: String,
    pub output_tag: String,
    pub output_dir: String,
    pub output_file: String,
    pub auto_restart: bool,
    pub single_proxy: bool,
    pub reality_fingerprint: String,
    pub dialer_proxies: Vec<String>,
    pub update_interval: String,
    pub timeout_sec: u64,
    pub allow_insecure_url: bool,
    pub provider_name: String,
    pub provider_path: String,
    pub provider_group: String,
    pub provider_group_type: String,
    pub provider_health_check: bool,
    pub provider_health_check_url: String,
    pub provider_health_check_interval: u64,
    pub native_include: String,
    pub native_exclude: String,
    pub last_update_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_status: String,
    pub last_error: String,
    pub last_node_count: usize,
    pub last_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Default for Subscription {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            enabled: true,
            url: String::new(),
            core: "xray".into(),
            mode: "watcher".into(),
            format: "auto".into(),
            output_tag: String::new(),
            output_dir: XRAY_CONF.into(),
            output_file: String::new(),
            auto_restart: true,
            single_proxy: false,
            reality_fingerprint: String::new(),
            dialer_proxies: Vec::new(),
            update_interval: "0 */6 * * *".into(),
            timeout_sec: 20,
            allow_insecure_url: false,
            provider_name: String::new(),
            provider_path: String::new(),
            provider_group: String::new(),
            provider_group_type: "select".into(),
            provider_health_check: true,
            provider_health_check_url: "https://www.gstatic.com/generate_204".into(),
            provider_health_check_interval: 300,
            native_include: String::new(),
            native_exclude: String::new(),
            last_update_at: None,
            last_success_at: None,
            last_status: "never".into(),
            last_error: String::new(),
            last_node_count: 0,
            last_hash: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct SubscriptionStore {
    version: u8,
    subscriptions: Vec<Subscription>,
}

impl Default for SubscriptionStore {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            subscriptions: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SubscriptionInput {
    id: Option<String>,
    name: String,
    url: Option<String>,
    enabled: Option<bool>,
    core: Option<String>,
    mode: Option<String>,
    format: Option<String>,
    output_tag: Option<String>,
    output_dir: Option<String>,
    auto_restart: Option<bool>,
    single_proxy: Option<bool>,
    reality_fingerprint: Option<String>,
    dialer_proxies: Option<Vec<String>>,
    update_interval: Option<String>,
    timeout_sec: Option<u64>,
    allow_insecure_url: Option<bool>,
    provider_name: Option<String>,
    provider_path: Option<String>,
    provider_group: Option<String>,
    provider_group_type: Option<String>,
    provider_health_check: Option<bool>,
    provider_health_check_url: Option<String>,
    provider_health_check_interval: Option<u64>,
    native_include: Option<String>,
    native_exclude: Option<String>,
}

impl Default for SubscriptionInput {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            url: None,
            enabled: None,
            core: None,
            mode: None,
            format: None,
            output_tag: None,
            output_dir: None,
            auto_restart: None,
            single_proxy: None,
            reality_fingerprint: None,
            dialer_proxies: None,
            update_interval: None,
            timeout_sec: None,
            allow_insecure_url: None,
            provider_name: None,
            provider_path: None,
            provider_group: None,
            provider_group_type: None,
            provider_health_check: None,
            provider_health_check_url: None,
            provider_health_check_interval: None,
            native_include: None,
            native_exclude: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionView {
    id: String,
    name: String,
    enabled: bool,
    url_masked: String,
    core: String,
    mode: String,
    format: String,
    output_tag: String,
    output_dir: String,
    output_file: String,
    auto_restart: bool,
    single_proxy: bool,
    reality_fingerprint: String,
    dialer_proxies: Vec<String>,
    update_interval: String,
    timeout_sec: u64,
    allow_insecure_url: bool,
    provider_name: String,
    provider_path: String,
    provider_group: String,
    provider_group_type: String,
    provider_health_check: bool,
    provider_health_check_url: String,
    provider_health_check_interval: u64,
    native_include: String,
    native_exclude: String,
    last_update_at: Option<String>,
    last_success_at: Option<String>,
    last_status: String,
    last_error: String,
    last_node_count: usize,
    last_hash: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DeleteOptions {
    delete_output_file: bool,
    remove_cron: bool,
    backup_before_delete: bool,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateOptions {
    dry_run: bool,
    force: bool,
    no_restart: bool,
    with_backup: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ScheduleReq {
    update_interval: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateResult {
    id: String,
    status: String,
    node_count: usize,
    output_file: String,
    dry_run: bool,
    restarted: bool,
    message: String,
}

#[derive(Serialize)]
struct ListData {
    items: Vec<SubscriptionView>,
}

#[derive(Serialize)]
struct ItemData {
    item: SubscriptionView,
}

#[derive(Serialize)]
struct LogData {
    lines: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeNodePreview {
    name: String,
    protocol: String,
    server: String,
    port: u16,
    warning: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewData {
    items: Vec<NativeNodePreview>,
    total: usize,
    warnings: Vec<String>,
}

struct NativeParseResult {
    outbounds: Vec<Value>,
    preview: Vec<NativeNodePreview>,
    warnings: Vec<String>,
}

struct FileLock {
    path: PathBuf,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub async fn list_subscriptions(State(state): State<AppState>) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    match read_store(&settings) {
        Ok(store) => ok(Some(ListData {
            items: store.subscriptions.iter().map(to_view).collect(),
        })),
        Err(e) => err(e),
    }
}

pub async fn get_subscription(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    match read_store(&settings).and_then(|store| {
        store
            .subscriptions
            .iter()
            .find(|item| item.id == id)
            .map(|item| ItemData { item: to_view(item) })
            .ok_or_else(|| "Подписка не найдена".to_string())
    }) {
        Ok(data) => ok(Some(data)),
        Err(e) => err(e),
    }
}

pub async fn create_subscription(
    State(state): State<AppState>, Json(input): Json<SubscriptionInput>,
) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    match mutate_store(&settings, |store| {
        let mut sub = build_subscription(&settings, input, None)?;
        if store.subscriptions.iter().any(|item| item.id == sub.id) {
            return Err("Подписка с таким id уже существует".into());
        }
        ensure_unique_output(store, &sub, None)?;
        sub.created_at = now();
        sub.updated_at = sub.created_at.clone();
        store.subscriptions.push(sub.clone());
        Ok(ItemData { item: to_view(&sub) })
    }) {
        Ok(data) => ok(Some(data)),
        Err(e) => err(e),
    }
}

pub async fn update_subscription(
    State(state): State<AppState>, AxumPath(id): AxumPath<String>, Json(input): Json<SubscriptionInput>,
) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    match mutate_store(&settings, |store| {
        let pos = store
            .subscriptions
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| "Подписка не найдена".to_string())?;
        let current = store.subscriptions[pos].clone();
        let mut next = build_subscription(&settings, input, Some(&current))?;
        next.id = current.id;
        next.created_at = current.created_at;
        next.updated_at = now();
        if next.url != current.url {
            next.last_hash.clear();
            next.last_status = "warning".into();
            next.last_error = "URL изменён, требуется проверка".into();
        }
        ensure_unique_output(store, &next, Some(pos))?;
        store.subscriptions[pos] = next.clone();
        Ok(ItemData { item: to_view(&next) })
    }) {
        Ok(data) => ok(Some(data)),
        Err(e) => err(e),
    }
}

pub async fn delete_subscription(
    State(state): State<AppState>, AxumPath(id): AxumPath<String>, Json(options): Json<DeleteOptions>,
) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let result = mutate_store(&settings, |store| {
        let pos = store
            .subscriptions
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| "Подписка не найдена".to_string())?;
        Ok(store.subscriptions.remove(pos))
    });

    match result {
        Ok(sub) => {
            if options.backup_before_delete {
                if sub.core == "mihomo" {
                    let _ = backup_file(
                        &settings,
                        &sub.id,
                        Path::new(&settings.mihomo_config_path),
                        "config.yaml",
                    );
                } else {
                    let _ = backup_output(&settings, &sub);
                }
            }
            if options.delete_output_file {
                if sub.core == "mihomo" {
                    let _ = remove_mihomo_subscription_block(&settings, &sub);
                } else if !sub.output_file.is_empty() {
                    let _ = fs::remove_file(&sub.output_file);
                }
            }
            if options.remove_cron {
                let _ = remove_cron_block(&sub.id);
            }
            ok::<()>(None)
        }
        Err(e) => err(e),
    }
}

pub async fn rollback_update(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let sub = match find_subscription(&settings, &id) {
        Ok(sub) => sub,
        Err(e) => return err(e),
    };
    match rollback_subscription(&settings, Some(&state), &sub).await {
        Ok(result) => ok(Some(result)),
        Err(e) => err(e),
    }
}

pub async fn check_subscription(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let sub = match find_subscription(&settings, &id) {
        Ok(sub) => sub,
        Err(e) => return err(e),
    };
    match run_subscription_update(
        &settings,
        Some(&state),
        sub,
        UpdateOptions {
            dry_run: true,
            no_restart: true,
            ..Default::default()
        },
    )
    .await
    {
        Ok(result) => ok(Some(result)),
        Err(e) => err(e),
    }
}

pub async fn preview_subscription(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let sub = match find_subscription(&settings, &id) {
        Ok(sub) => sub,
        Err(e) => return err(e),
    };
    if sub.core != "xray" || sub.mode != "native" {
        return err("Предпросмотр доступен только для подписок xray/native".into());
    }
    match fetch_and_parse_native(&state, &sub).await {
        Ok(parsed) => ok(Some(PreviewData {
            total: parsed.preview.len(),
            items: parsed.preview.into_iter().take(100).collect(),
            warnings: parsed.warnings,
        })),
        Err(e) => err(e),
    }
}

pub async fn update_now(
    State(state): State<AppState>, AxumPath(id): AxumPath<String>, Json(options): Json<UpdateOptions>,
) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let sub = match find_subscription(&settings, &id) {
        Ok(sub) => sub,
        Err(e) => return err(e),
    };
    match run_subscription_update(&settings, Some(&state), sub, options).await {
        Ok(result) => ok(Some(result)),
        Err(e) => err(e),
    }
}

pub async fn update_all(State(state): State<AppState>, Json(options): Json<UpdateOptions>) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let store = match read_store(&settings) {
        Ok(store) => store,
        Err(e) => return err(e),
    };
    let mut results = Vec::new();
    for sub in store.subscriptions.into_iter().filter(|item| item.enabled) {
        let id = sub.id.clone();
        let mut item_options = UpdateOptions {
            no_restart: true,
            ..options.clone()
        };
        item_options.dry_run = options.dry_run;
        match run_subscription_update(&settings, Some(&state), sub, item_options).await {
            Ok(result) => results.push(result),
            Err(error) => results.push(UpdateResult {
                id,
                status: "error".into(),
                node_count: 0,
                output_file: String::new(),
                dry_run: options.dry_run,
                restarted: false,
                message: error,
            }),
        }
    }
    if !options.dry_run && !options.no_restart && results.iter().any(|r| r.status == "ok") {
        let _ = restart_xkeen(Some(&state)).await;
    }
    ok(Some(serde_json::json!({ "items": results })))
}

pub async fn get_subscription_log(
    State(state): State<AppState>, AxumPath(id): AxumPath<String>, Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let tail = params
        .get("tail")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(200)
        .min(1000);
    let path = log_path(&settings, &id);
    let lines = fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(tail)
        .map(str::to_string)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    ok(Some(LogData { lines }))
}

pub async fn set_schedule(
    State(state): State<AppState>, AxumPath(id): AxumPath<String>, Json(req): Json<ScheduleReq>,
) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    let interval = match req.update_interval {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return err("Расписание не указано".into()),
    };
    let sub = match find_subscription(&settings, &id) {
        Ok(sub) => sub,
        Err(e) => return err(e),
    };
    if let Err(e) = validate_cron(&interval) {
        return err(e);
    }
    if let Err(e) = upsert_cron_block(&sub.id, &interval, &settings) {
        return err(e);
    }
    let _ = mutate_store(&settings, |store| {
        if let Some(item) = store.subscriptions.iter_mut().find(|item| item.id == sub.id) {
            item.update_interval = interval.clone();
            item.updated_at = now();
        }
        Ok(())
    });
    ok::<()>(None)
}

pub async fn delete_schedule(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let settings = state.settings.read().unwrap().subscriptions.clone();
    if let Err(e) = remove_cron_block(&id) {
        return err(e);
    }
    let _ = mutate_store(&settings, |store| {
        if let Some(item) = store.subscriptions.iter_mut().find(|item| item.id == id) {
            item.updated_at = now();
        }
        Ok(())
    });
    ok::<()>(None)
}

pub async fn get_schedules() -> impl IntoResponse {
    let lines = read_crontab().unwrap_or_default();
    ok(Some(serde_json::json!({ "crontab": lines })))
}

pub async fn run_cli_update(target: Option<String>, all: bool, dry_run: bool, no_restart: bool) -> Result<(), String> {
    let settings = SubscriptionSettings::default();
    let store = read_store(&settings)?;
    let targets = if all {
        store
            .subscriptions
            .into_iter()
            .filter(|item| item.enabled)
            .collect::<Vec<_>>()
    } else {
        let id = target.ok_or_else(|| "Укажите id подписки или --all".to_string())?;
        vec![
            store
                .subscriptions
                .into_iter()
                .find(|item| item.id == id)
                .ok_or_else(|| "Подписка не найдена".to_string())?,
        ]
    };

    let mut any_updated = false;
    for sub in targets {
        let result = run_subscription_update(
            &settings,
            None,
            sub,
            UpdateOptions {
                dry_run,
                no_restart: true,
                with_backup: Some(true),
                force: false,
            },
        )
        .await?;
        any_updated |= result.status == "ok";
    }
    if any_updated && !dry_run && !no_restart {
        restart_xkeen(None).await?;
    }
    Ok(())
}

async fn run_subscription_update(
    settings: &SubscriptionSettings, state: Option<&AppState>, sub: Subscription, options: UpdateOptions,
) -> Result<UpdateResult, String> {
    let _lock = acquire_lock(settings)?;
    validate_subscription(&sub)?;
    if sub.core == "mihomo" && sub.mode == "provider" {
        return run_mihomo_provider_update(settings, state, sub, options).await;
    }
    if sub.core == "xray" && sub.mode == "native" {
        return run_native_update(settings, state, sub, options).await;
    }
    if sub.core != "xray" || sub.mode != "watcher" {
        return Err("В MVP поддерживается только Xray/Watcher".into());
    }
    if !Path::new(&settings.watcher_binary_path).exists() {
        let msg = format!("Утилита не найдена: {}", settings.watcher_binary_path);
        append_log(settings, &sub.id, "error", &msg);
        update_status(settings, &sub.id, "error", &msg, 0, None)?;
        return Err(msg);
    }

    let temp_dir = std::env::temp_dir().join(format!(
        "xkeen-sub-{}-{}",
        sub.id,
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| e.to_string())?;
    let args = watcher_args(&sub, &temp_dir, true);
    append_log(
        settings,
        &sub.id,
        "info",
        &format!(
            "Запуск watcher: {}",
            command_preview(&settings.watcher_binary_path, &args)
        ),
    );
    let output = tokio::time::timeout(
        Duration::from_secs(sub.timeout_sec),
        Command::new(&settings.watcher_binary_path).args(&args).output(),
    )
    .await
    .map_err(|_| format!("Watcher превысил таймаут {} сек", sub.timeout_sec))
    .and_then(|result| result.map_err(|e| e.to_string()));
    let output = match output {
        Ok(output) => output,
        Err(e) => {
            let _ = tokio::fs::remove_dir_all(&temp_dir).await;
            append_log(settings, &sub.id, "error", &e);
            update_status(settings, &sub.id, "error", &e, 0, None)?;
            return Err(e);
        }
    };
    let stdout = sanitize_log(&String::from_utf8_lossy(&output.stdout), &sub.url);
    let stderr = sanitize_log(&String::from_utf8_lossy(&output.stderr), &sub.url);
    if !stdout.trim().is_empty() {
        append_log(settings, &sub.id, "info", stdout.trim());
    }
    if !stderr.trim().is_empty() {
        append_log(settings, &sub.id, "warn", stderr.trim());
    }
    if !output.status.success() {
        let msg = format!("Watcher завершился с кодом {}", output.status);
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        append_log(settings, &sub.id, "error", &msg);
        update_status(settings, &sub.id, "error", &msg, 0, None)?;
        return Err(msg);
    }

    let generated = temp_dir.join(watcher_output_file_name(&sub));
    let generated_content = tokio::fs::read_to_string(&generated)
        .await
        .map_err(|e| format!("Watcher не создал output-файл {}: {}", generated.display(), e))?;
    let node_count = count_outbounds(&generated_content);
    if node_count == 0 {
        let msg = "В подписке не найдено поддерживаемых узлов".to_string();
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        append_log(settings, &sub.id, "warn", &msg);
        update_status(settings, &sub.id, "warning", &msg, 0, None)?;
        return Err(msg);
    }
    let hash = format!("{:x}", md5::compute(generated_content.as_bytes()));
    if !options.force && !sub.last_hash.is_empty() && sub.last_hash == hash {
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        let msg = "Изменений нет".to_string();
        update_status(settings, &sub.id, "ok", "", node_count, Some(hash))?;
        append_log(settings, &sub.id, "info", &msg);
        return Ok(UpdateResult {
            id: sub.id,
            status: "ok".into(),
            node_count,
            output_file: sub.output_file,
            dry_run: options.dry_run,
            restarted: false,
            message: msg,
        });
    }

    if let Err(e) = validate_generated_config(settings, &sub, &generated_content).await {
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        return Err(e);
    }
    if options.dry_run {
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        let msg = format!("Проверка успешна: найдено {} узлов", node_count);
        append_log(settings, &sub.id, "info", &msg);
        update_status(settings, &sub.id, "ok", "", node_count, Some(hash))?;
        return Ok(UpdateResult {
            id: sub.id,
            status: "ok".into(),
            node_count,
            output_file: sub.output_file,
            dry_run: true,
            restarted: false,
            message: msg,
        });
    }

    if options.with_backup.unwrap_or(true) {
        backup_output(settings, &sub)?;
    }
    atomic_write(&sub.output_file, &generated_content).await?;
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    let mut restarted = false;
    if sub.auto_restart && !options.no_restart {
        restart_xkeen(state).await?;
        restarted = true;
    }
    let msg = format!("Подписка обновлена: найдено {} узлов", node_count);
    append_log(settings, &sub.id, "info", &msg);
    update_status(settings, &sub.id, "ok", "", node_count, Some(hash))?;
    Ok(UpdateResult {
        id: sub.id,
        status: "ok".into(),
        node_count,
        output_file: sub.output_file,
        dry_run: false,
        restarted,
        message: msg,
    })
}

async fn run_mihomo_provider_update(
    settings: &SubscriptionSettings, state: Option<&AppState>, sub: Subscription, options: UpdateOptions,
) -> Result<UpdateResult, String> {
    if !Path::new(&settings.mihomo_binary_path).exists() && !command_exists("mihomo") {
        let msg = format!("Бинарный файл Mihomo не найден: {}", settings.mihomo_binary_path);
        append_log(settings, &sub.id, "error", &msg);
        update_status(settings, &sub.id, "error", &msg, 0, None)?;
        return Err(msg);
    }

    let config_path = Path::new(&settings.mihomo_config_path);
    let original = tokio::fs::read_to_string(config_path)
        .await
        .unwrap_or_else(|_| "proxy-providers: {}\nproxy-groups: []\n".into());
    let next = build_mihomo_provider_config(&original, &sub)?;
    validate_core_files("mihomo", vec![(settings.mihomo_config_path.clone(), next.clone())])
        .await
        .map_err(|e| {
            let err = sanitize_log(&e, &sub.url);
            append_log(settings, &sub.id, "error", &err);
            err
        })?;

    let hash = format!("{:x}", md5::compute(next.as_bytes()));
    if !options.force && !sub.last_hash.is_empty() && sub.last_hash == hash {
        let msg = "Изменений нет".to_string();
        append_log(settings, &sub.id, "info", &msg);
        update_status(settings, &sub.id, "ok", "", sub.last_node_count, Some(hash))?;
        return Ok(UpdateResult {
            id: sub.id,
            status: "ok".into(),
            node_count: sub.last_node_count,
            output_file: settings.mihomo_config_path.clone(),
            dry_run: options.dry_run,
            restarted: false,
            message: msg,
        });
    }

    if options.dry_run {
        let msg = "Проверка Mihomo provider выполнена".to_string();
        append_log(settings, &sub.id, "info", &msg);
        update_status(settings, &sub.id, "ok", "", sub.last_node_count, Some(hash))?;
        return Ok(UpdateResult {
            id: sub.id,
            status: "ok".into(),
            node_count: sub.last_node_count,
            output_file: settings.mihomo_config_path.clone(),
            dry_run: true,
            restarted: false,
            message: msg,
        });
    }

    if options.with_backup.unwrap_or(true) {
        backup_file(settings, &sub.id, config_path, "config.yaml")?;
    }
    atomic_write(&settings.mihomo_config_path, &next).await?;

    let mut restarted = false;
    if sub.auto_restart && !options.no_restart {
        restart_xkeen(state).await?;
        restarted = true;
    }
    let msg = format!("Mihomo provider обновлён: {}", sub.provider_name);
    append_log(settings, &sub.id, "info", &msg);
    update_status(settings, &sub.id, "ok", "", sub.last_node_count, Some(hash))?;
    Ok(UpdateResult {
        id: sub.id,
        status: "ok".into(),
        node_count: sub.last_node_count,
        output_file: settings.mihomo_config_path.clone(),
        dry_run: false,
        restarted,
        message: msg,
    })
}

async fn run_native_update(
    settings: &SubscriptionSettings, state: Option<&AppState>, sub: Subscription, options: UpdateOptions,
) -> Result<UpdateResult, String> {
    let state = state.ok_or_else(|| "Native-обновление требует состояние приложения".to_string())?;
    let parsed = fetch_and_parse_native(state, &sub).await?;
    if parsed.outbounds.is_empty() {
        let msg = "Поддерживаемые native-узлы не найдены".to_string();
        append_log(settings, &sub.id, "warn", &msg);
        update_status(settings, &sub.id, "warning", &msg, 0, None)?;
        return Err(msg);
    }
    let generated_content = serde_json::to_string_pretty(&serde_json::json!({ "outbounds": parsed.outbounds }))
        .map_err(|e| e.to_string())?;
    let node_count = count_outbounds(&generated_content);
    let hash = format!("{:x}", md5::compute(generated_content.as_bytes()));

    if !options.force && !sub.last_hash.is_empty() && sub.last_hash == hash {
        let msg = "Изменений нет".to_string();
        append_log(settings, &sub.id, "info", &msg);
        update_status(settings, &sub.id, "ok", "", node_count, Some(hash))?;
        return Ok(UpdateResult {
            id: sub.id,
            status: "ok".into(),
            node_count,
            output_file: sub.output_file,
            dry_run: options.dry_run,
            restarted: false,
            message: msg,
        });
    }

    validate_generated_config(settings, &sub, &generated_content).await?;
    if options.dry_run {
        let msg = format!("Native-проверка выполнена: {} узлов", node_count);
        append_log(settings, &sub.id, "info", &msg);
        update_status(settings, &sub.id, "ok", "", node_count, Some(hash))?;
        return Ok(UpdateResult {
            id: sub.id,
            status: "ok".into(),
            node_count,
            output_file: sub.output_file,
            dry_run: true,
            restarted: false,
            message: msg,
        });
    }

    if options.with_backup.unwrap_or(true) {
        backup_output(settings, &sub)?;
    }
    atomic_write(&sub.output_file, &generated_content).await?;
    let mut restarted = false;
    if sub.auto_restart && !options.no_restart {
        restart_xkeen(Some(state)).await?;
        restarted = true;
    }
    let msg = format!("Native-подписка обновлена: {} узлов", node_count);
    append_log(settings, &sub.id, "info", &msg);
    for warning in parsed.warnings.iter().take(20) {
        append_log(settings, &sub.id, "warn", warning);
    }
    update_status(settings, &sub.id, "ok", "", node_count, Some(hash))?;
    Ok(UpdateResult {
        id: sub.id,
        status: "ok".into(),
        node_count,
        output_file: sub.output_file,
        dry_run: false,
        restarted,
        message: msg,
    })
}

fn read_store(settings: &SubscriptionSettings) -> Result<SubscriptionStore, String> {
    let path = Path::new(&settings.store_path);
    if !path.exists() {
        return Ok(SubscriptionStore::default());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut store: SubscriptionStore = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    for sub in &mut store.subscriptions {
        normalize_subscription(settings, sub);
    }
    Ok(store)
}

fn write_store(settings: &SubscriptionSettings, store: &SubscriptionStore) -> Result<(), String> {
    let path = Path::new(&settings.store_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn mutate_store<T>(
    settings: &SubscriptionSettings, f: impl FnOnce(&mut SubscriptionStore) -> Result<T, String>,
) -> Result<T, String> {
    let mut store = read_store(settings)?;
    let result = f(&mut store)?;
    write_store(settings, &store)?;
    Ok(result)
}

fn build_subscription(
    settings: &SubscriptionSettings, input: SubscriptionInput, current: Option<&Subscription>,
) -> Result<Subscription, String> {
    let mut sub = current.cloned().unwrap_or_default();
    if let Some(id) = input.id {
        sub.id = id.trim().to_string();
    } else if sub.id.is_empty() {
        sub.id = slugify(&input.name);
    }
    if !input.name.trim().is_empty() {
        sub.name = input.name.trim().to_string();
    }
    if let Some(url) = input.url {
        sub.url = url.trim().to_string();
    }
    if let Some(enabled) = input.enabled {
        sub.enabled = enabled;
    }
    if let Some(core) = input.core {
        sub.core = core;
    }
    if let Some(mode) = input.mode {
        sub.mode = mode;
    }
    if let Some(format) = input.format {
        sub.format = format;
    }
    if let Some(output_tag) = input.output_tag {
        sub.output_tag = output_tag.trim().to_string();
    } else if sub.output_tag.is_empty() {
        sub.output_tag = format!("sub-{}", sub.id);
    }
    if let Some(output_dir) = input.output_dir {
        sub.output_dir = output_dir.trim().to_string();
    }
    if sub.output_dir.is_empty() {
        sub.output_dir = settings.xray_config_dir.clone();
    }
    if let Some(auto_restart) = input.auto_restart {
        sub.auto_restart = auto_restart;
    }
    if let Some(single_proxy) = input.single_proxy {
        sub.single_proxy = single_proxy;
    }
    if let Some(reality_fingerprint) = input.reality_fingerprint {
        sub.reality_fingerprint = reality_fingerprint.trim().to_string();
    }
    if let Some(dialer_proxies) = input.dialer_proxies {
        sub.dialer_proxies = dialer_proxies;
    }
    if let Some(update_interval) = input.update_interval {
        sub.update_interval = update_interval.trim().to_string();
    }
    if let Some(timeout_sec) = input.timeout_sec {
        sub.timeout_sec = timeout_sec.clamp(1, 300);
    }
    if let Some(allow_insecure_url) = input.allow_insecure_url {
        sub.allow_insecure_url = allow_insecure_url;
    }
    if let Some(provider_name) = input.provider_name {
        sub.provider_name = provider_name.trim().to_string();
    }
    if let Some(provider_path) = input.provider_path {
        sub.provider_path = provider_path.trim().to_string();
    }
    if let Some(provider_group) = input.provider_group {
        sub.provider_group = provider_group.trim().to_string();
    }
    if let Some(provider_group_type) = input.provider_group_type {
        sub.provider_group_type = provider_group_type.trim().to_string();
    }
    if let Some(provider_health_check) = input.provider_health_check {
        sub.provider_health_check = provider_health_check;
    }
    if let Some(provider_health_check_url) = input.provider_health_check_url {
        sub.provider_health_check_url = provider_health_check_url.trim().to_string();
    }
    if let Some(provider_health_check_interval) = input.provider_health_check_interval {
        sub.provider_health_check_interval = provider_health_check_interval.clamp(30, 86400);
    }
    if let Some(native_include) = input.native_include {
        sub.native_include = native_include.trim().to_string();
    }
    if let Some(native_exclude) = input.native_exclude {
        sub.native_exclude = native_exclude.trim().to_string();
    }
    normalize_subscription(settings, &mut sub);
    validate_subscription(&sub)?;
    Ok(sub)
}

fn normalize_subscription(settings: &SubscriptionSettings, sub: &mut Subscription) {
    if sub.core.is_empty() {
        sub.core = "xray".into();
    }
    if sub.mode.is_empty() {
        sub.mode = if sub.core == "mihomo" {
            "provider".into()
        } else {
            "watcher".into()
        };
    }
    if sub.core == "mihomo" {
        sub.output_dir = Path::new(&settings.mihomo_config_path)
            .parent()
            .unwrap_or_else(|| Path::new(MIHOMO_CONF))
            .to_string_lossy()
            .to_string();
    } else if sub.output_dir.is_empty() {
        sub.output_dir = settings.xray_config_dir.clone();
    }
    if sub.core == "mihomo" {
        sub.output_file = settings.mihomo_config_path.clone();
    } else {
        sub.output_file = Path::new(&sub.output_dir)
            .join(output_file_name(sub))
            .to_string_lossy()
            .to_string();
    }
    if sub.provider_name.is_empty() {
        sub.provider_name = sub.output_tag.clone();
    }
    if sub.provider_path.is_empty() {
        sub.provider_path = format!("./providers/{}.yaml", sub.provider_name);
    }
    if sub.provider_group_type.is_empty() {
        sub.provider_group_type = "select".into();
    }
    if sub.provider_health_check_url.is_empty() {
        sub.provider_health_check_url = "https://www.gstatic.com/generate_204".into();
    }
    if sub.provider_health_check_interval == 0 {
        sub.provider_health_check_interval = 300;
    }
}

fn validate_subscription(sub: &Subscription) -> Result<(), String> {
    validate_ident("id", &sub.id, r"^[a-zA-Z0-9_-]{1,64}$")?;
    validate_ident("outputTag", &sub.output_tag, r"^[a-zA-Z0-9_.-]{1,64}$")?;
    if sub.name.trim().is_empty() {
        return Err("Название подписки не может быть пустым".into());
    }
    if sub.core != "xray" && sub.core != "mihomo" {
        return Err("В MVP поддерживается только ядро Xray".into());
    }
    if !((sub.core == "xray" && (sub.mode == "watcher" || sub.mode == "native"))
        || (sub.core == "mihomo" && sub.mode == "provider"))
    {
        return Err("В MVP поддерживается только режим Watcher".into());
    }
    if sub.core == "mihomo" {
        validate_ident("providerName", &sub.provider_name, r"^[a-zA-Z0-9_.-]{1,64}$")?;
        if sub.provider_path.contains("..") || sub.provider_path.trim().is_empty() {
            return Err("Некорректный путь provider".into());
        }
        if !["select", "url-test", "fallback", "load-balance"].contains(&sub.provider_group_type.as_str()) {
            return Err("Некорректный тип группы provider".into());
        }
        if sub.provider_health_check {
            validate_url(&sub.provider_health_check_url, false)?;
        }
    }
    validate_url(&sub.url, sub.allow_insecure_url)?;
    Ok(())
}

fn validate_ident(label: &str, value: &str, pattern: &str) -> Result<(), String> {
    Regex::new(pattern)
        .map_err(|e| e.to_string())?
        .is_match(value)
        .then_some(())
        .ok_or_else(|| format!("Некорректное поле {label}"))
}

fn validate_url(value: &str, allow_insecure: bool) -> Result<(), String> {
    if value.len() > 4096 {
        return Err("URL слишком длинный".into());
    }
    let url = reqwest::Url::parse(value).map_err(|_| "Некорректный URL".to_string())?;
    if url.scheme() != "https" && !(allow_insecure && url.scheme() == "http") {
        return Err("По умолчанию разрешены только HTTPS URL".into());
    }
    if url.host_str().unwrap_or("").is_empty() {
        return Err("URL должен содержать host".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL с userinfo запрещён".into());
    }
    let host = url.host_str().unwrap_or("");
    if is_local_host(host) {
        return Err("Локальные адреса запрещены".into());
    }
    Ok(())
}

fn is_local_host(host: &str) -> bool {
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".local")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host == "::1"
        || host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|v| v.parse::<u8>().ok())
                .is_some_and(|v| (16..=31).contains(&v))
}

fn ensure_unique_output(
    store: &SubscriptionStore, sub: &Subscription, ignore_pos: Option<usize>,
) -> Result<(), String> {
    for (idx, other) in store.subscriptions.iter().enumerate() {
        if Some(idx) == ignore_pos {
            continue;
        }
        if other.output_tag == sub.output_tag {
            return Err("outputTag уже используется другой подпиской".into());
        }
        if other.core == "mihomo" && sub.core == "mihomo" && other.provider_name == sub.provider_name {
            return Err("providerName уже используется другой подпиской".into());
        }
        if other.core == "mihomo"
            && sub.core == "mihomo"
            && !sub.provider_group.is_empty()
            && other.provider_group == sub.provider_group
        {
            return Err("providerGroup уже используется другой подпиской".into());
        }
        if other.core != "mihomo" && sub.core != "mihomo" && other.output_file == sub.output_file {
            return Err("outputFile уже используется другой подпиской".into());
        }
    }
    Ok(())
}

fn find_subscription(settings: &SubscriptionSettings, id: &str) -> Result<Subscription, String> {
    read_store(settings)?
        .subscriptions
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "Подписка не найдена".to_string())
}

fn output_file_name(sub: &Subscription) -> String {
    format!("04_outbounds.sub.{}.json", sub.id)
}

fn watcher_output_file_name(sub: &Subscription) -> String {
    format!("04_outbounds.{}.json", sub.output_tag)
}

fn watcher_args(sub: &Subscription, output_dir: &Path, no_restart: bool) -> Vec<String> {
    let mut args = Vec::new();
    if no_restart {
        args.push("--no-restart".into());
    }
    if sub.single_proxy {
        args.push("--single-proxy".into());
    }
    if !sub.reality_fingerprint.is_empty() {
        args.push("--reality-fingerprint".into());
        args.push(sub.reality_fingerprint.clone());
    }
    if !sub.dialer_proxies.is_empty() {
        args.push("--dialer-proxies".into());
        args.push(sub.dialer_proxies.join(","));
    }
    args.push("--output-dir".into());
    args.push(output_dir.to_string_lossy().to_string());
    args.push(format!("{}={}", sub.output_tag, sub.url));
    args
}

async fn fetch_and_parse_native(state: &AppState, sub: &Subscription) -> Result<NativeParseResult, String> {
    let response = state
        .http_client
        .get(&sub.url)
        .timeout(Duration::from_secs(sub.timeout_sec))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Запрос подписки завершился ошибкой: {}", response.status()));
    }
    if response.content_length().is_some_and(|len| len > MAX_SUBSCRIPTION_BYTES) {
        return Err("Ответ подписки слишком большой".into());
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_SUBSCRIPTION_BYTES {
        return Err("Ответ подписки слишком большой".into());
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    parse_native_subscription(sub, &text)
}

fn parse_native_subscription(sub: &Subscription, content: &str) -> Result<NativeParseResult, String> {
    if let Ok(json) = serde_json::from_str::<Value>(content) {
        if let Some(items) = json.get("outbounds").and_then(|v| v.as_array()) {
            let mut preview = Vec::new();
            let mut outbounds = Vec::new();
            for item in items {
                let tag = item
                    .get("tag")
                    .and_then(|v| v.as_str())
                    .unwrap_or("outbound")
                    .to_string();
                if !native_name_allowed(sub, &tag) {
                    continue;
                }
                preview.push(NativeNodePreview {
                    name: tag,
                    protocol: item
                        .get("protocol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("xray-json")
                        .to_string(),
                    server: String::new(),
                    port: 0,
                    warning: String::new(),
                });
                outbounds.push(item.clone());
            }
            return Ok(NativeParseResult {
                outbounds,
                preview,
                warnings: Vec::new(),
            });
        }
    }

    let decoded = decode_subscription_content(content);
    let mut outbounds = Vec::new();
    let mut preview = Vec::new();
    let mut warnings = Vec::new();
    let mut tags = HashSet::new();

    for raw in decoded.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match parse_native_link(raw, sub, &mut tags) {
            Ok(Some((outbound, item_warnings, item_preview))) => {
                warnings.extend(item_warnings);
                preview.push(item_preview);
                outbounds.push(outbound);
            }
            Ok(None) => {}
            Err(e) => warnings.push(format!("{}: {}", mask_url_arg(raw), e)),
        }
    }

    Ok(NativeParseResult {
        outbounds,
        preview,
        warnings,
    })
}

fn decode_subscription_content(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.contains("://") {
        return content.to_string();
    }
    // Many subscription endpoints return one base64 blob containing newline
    // separated share links. Try the common padded and unpadded alphabets, but
    // fall back to the original text so plain subscriptions still work.
    for engine in [
        &base64::engine::general_purpose::STANDARD,
        &base64::engine::general_purpose::STANDARD_NO_PAD,
        &base64::engine::general_purpose::URL_SAFE,
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
    ] {
        if let Ok(bytes) = engine.decode(trimmed.as_bytes()) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                if decoded.contains("://") {
                    return decoded;
                }
            }
        }
    }
    content.to_string()
}

fn parse_native_link(
    raw: &str, sub: &Subscription, tags: &mut HashSet<String>,
) -> Result<Option<(Value, Vec<String>, NativeNodePreview)>, String> {
    if raw.starts_with("vmess://") {
        return parse_vmess_link(raw, sub, tags);
    }
    let url = reqwest::Url::parse(raw).map_err(|_| "Некорректный URL".to_string())?;
    let scheme = url.scheme().to_ascii_lowercase();
    let result = match scheme.as_str() {
        "vless" => parse_vless_link(&url, sub, tags),
        "trojan" => parse_trojan_link(&url, sub, tags),
        "ss" => parse_ss_link(&url, sub, tags),
        "socks" | "socks5" | "http" => parse_simple_proxy_link(&url, sub, tags),
        "hysteria2" | "hy2" => parse_hysteria2_link(&url, sub, tags),
        // These schemes are valid in Mihomo or have non-uniform share-link
        // conventions, but this native path emits Xray outbound JSON. Keeping
        // them as warnings is safer than generating a config Xray cannot test.
        "ssr" | "tuic" | "snell" | "anytls" | "mieru" | "sudoku" | "hysteria" | "wireguard" | "tailscale" | "ssh"
        | "masque" | "trust" | "trust-tunnel" | "openvpn" => Ok(None),
        _ => Err(format!("Неподдерживаемая схема {scheme}")),
    }?;
    if result.is_none() {
        return Err(format!(
            "Схема {scheme} не поддерживается генерацией Xray native JSON"
        ));
    }
    Ok(result)
}

fn parse_vless_link(
    url: &reqwest::Url, sub: &Subscription, tags: &mut HashSet<String>,
) -> Result<Option<(Value, Vec<String>, NativeNodePreview)>, String> {
    let name = unique_tag(link_name(url, "vless"), tags);
    if !native_name_allowed(sub, &name) {
        return Ok(None);
    }
    let host = host(url)?;
    let port = url.port().unwrap_or(443);
    let q = query_map(url);
    let user = compact_json(serde_json::json!({
        "id": url.username(),
        "encryption": q.get("encryption").map(String::as_str).unwrap_or("none"),
        "flow": q.get("flow").cloned().unwrap_or_default(),
    }));
    let outbound = with_stream_settings(
        serde_json::json!({
            "tag": name,
            "protocol": "vless",
            "settings": { "vnext": [{ "address": host, "port": port, "users": [user] }] }
        }),
        &q,
    );
    Ok(Some(preview_result(outbound, Vec::new(), "vless", url, port)))
}

fn parse_trojan_link(
    url: &reqwest::Url, sub: &Subscription, tags: &mut HashSet<String>,
) -> Result<Option<(Value, Vec<String>, NativeNodePreview)>, String> {
    let name = unique_tag(link_name(url, "trojan"), tags);
    if !native_name_allowed(sub, &name) {
        return Ok(None);
    }
    let host = host(url)?;
    let port = url.port().unwrap_or(443);
    let q = query_map(url);
    let outbound = with_stream_settings(
        serde_json::json!({
            "tag": name,
            "protocol": "trojan",
            "settings": { "servers": [{ "address": host, "port": port, "password": url.username() }] }
        }),
        &q,
    );
    Ok(Some(preview_result(outbound, Vec::new(), "trojan", url, port)))
}

fn parse_ss_link(
    url: &reqwest::Url, sub: &Subscription, tags: &mut HashSet<String>,
) -> Result<Option<(Value, Vec<String>, NativeNodePreview)>, String> {
    let name = unique_tag(link_name(url, "ss"), tags);
    if !native_name_allowed(sub, &name) {
        return Ok(None);
    }
    let host = host(url)?;
    let port = url.port().ok_or_else(|| "Порт не указан".to_string())?;
    let user = decode_userinfo(url.username());
    let (method, password) = user
        .split_once(':')
        .map(|(method, password)| (method.to_string(), password.to_string()))
        .ok_or_else(|| "Некорректный userinfo Shadowsocks".to_string())?;
    let mut warnings = Vec::new();
    let q = query_map(url);
    if q.contains_key("plugin") {
        warnings.push("Параметр plugin из SIP002 не переносится в Xray outbound".into());
    }
    let outbound = serde_json::json!({
        "tag": name,
        "protocol": "shadowsocks",
        "settings": { "servers": [{ "address": host, "port": port, "method": method, "password": password }] }
    });
    Ok(Some(preview_result(outbound, warnings, "shadowsocks", url, port)))
}

fn parse_simple_proxy_link(
    url: &reqwest::Url, sub: &Subscription, tags: &mut HashSet<String>,
) -> Result<Option<(Value, Vec<String>, NativeNodePreview)>, String> {
    let protocol = if url.scheme() == "http" { "http" } else { "socks" };
    let name = unique_tag(link_name(url, protocol), tags);
    if !native_name_allowed(sub, &name) {
        return Ok(None);
    }
    let host = host(url)?;
    let port = url.port().ok_or_else(|| "Порт не указан".to_string())?;
    let mut server = serde_json::json!({ "address": host, "port": port });
    if !url.username().is_empty() {
        server["users"] = serde_json::json!([{ "user": decode_userinfo(url.username()), "pass": decode_userinfo(url.password().unwrap_or("")) }]);
    }
    let outbound = serde_json::json!({
        "tag": name,
        "protocol": protocol,
        "settings": { "servers": [server] }
    });
    Ok(Some(preview_result(outbound, Vec::new(), protocol, url, port)))
}

fn parse_hysteria2_link(
    url: &reqwest::Url, sub: &Subscription, tags: &mut HashSet<String>,
) -> Result<Option<(Value, Vec<String>, NativeNodePreview)>, String> {
    let name = unique_tag(link_name(url, "hysteria2"), tags);
    if !native_name_allowed(sub, &name) {
        return Ok(None);
    }
    let host = host(url)?;
    let port = url.port().unwrap_or(443);
    let q = query_map(url);
    let mut server = serde_json::json!({
        "address": host,
        "port": port,
        "password": decode_userinfo(url.username()),
    });
    if let Some(sni) = q.get("sni") {
        server["serverName"] = serde_json::json!(sni);
    }
    if q.get("insecure").is_some_and(|v| v == "1" || v == "true") {
        server["insecure"] = serde_json::json!(true);
    }
    if let Some(pin) = q.get("pinSHA256") {
        server["pinSHA256"] = serde_json::json!(pin);
    }
    if let Some(alpn) = q.get("alpn") {
        server["alpn"] = serde_json::json!(alpn.split(',').collect::<Vec<_>>());
    }
    if let Some(obfs) = q.get("obfs") {
        server["obfs"] = serde_json::json!(obfs);
    }
    if let Some(obfs_password) = q.get("obfs-password").or_else(|| q.get("obfs_password")) {
        server["obfsPassword"] = serde_json::json!(obfs_password);
    }
    if let Some(mport) = q.get("mport") {
        server["mport"] = serde_json::json!(mport);
    }
    let mut warnings = Vec::new();
    for unsupported in ["auth", "stun", "lport", "fast-open"] {
        if q.contains_key(unsupported) {
            warnings.push(format!(
                "Параметр Hysteria2 {unsupported} может требовать Mihomo-специфичный конфиг"
            ));
        }
    }
    let outbound = serde_json::json!({
        "tag": name,
        "protocol": "hysteria",
        "settings": {
            "servers": [server],
            "version": 2
        }
    });
    Ok(Some(preview_result(outbound, warnings, "hysteria2", url, port)))
}

fn parse_vmess_link(
    raw: &str, sub: &Subscription, tags: &mut HashSet<String>,
) -> Result<Option<(Value, Vec<String>, NativeNodePreview)>, String> {
    let encoded = raw.trim_start_matches("vmess://");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(encoded))
        .map_err(|_| "Некорректный VMess base64".to_string())?;
    let data: Value = serde_json::from_slice(&bytes).map_err(|_| "Некорректный VMess JSON".to_string())?;
    let name = unique_tag(
        data.get("ps").and_then(|v| v.as_str()).unwrap_or("vmess").to_string(),
        tags,
    );
    if !native_name_allowed(sub, &name) {
        return Ok(None);
    }
    let address = data
        .get("add")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Адрес не указан".to_string())?;
    let port = data
        .get("port")
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<u16>().ok())
                .or_else(|| v.as_u64().map(|n| n as u16))
        })
        .ok_or_else(|| "Порт не указан".to_string())?;
    let user = compact_json(serde_json::json!({
        "id": data.get("id").and_then(|v| v.as_str()).unwrap_or(""),
        "alterId": data.get("aid").and_then(|v| v.as_str().and_then(|s| s.parse::<u16>().ok())).unwrap_or(0),
        "security": data.get("scy").or_else(|| data.get("security")).and_then(|v| v.as_str()).unwrap_or("auto"),
    }));
    let mut outbound = serde_json::json!({
        "tag": name,
        "protocol": "vmess",
        "settings": { "vnext": [{ "address": address, "port": port, "users": [user] }] }
    });
    let q = vmess_query_map(&data);
    outbound = with_stream_settings(outbound, &q);
    let fake_url = reqwest::Url::parse(&format!("vmess://{}:{port}", address)).map_err(|e| e.to_string())?;
    Ok(Some(preview_result(outbound, Vec::new(), "vmess", &fake_url, port)))
}

fn preview_result(
    outbound: Value, warnings: Vec<String>, protocol: &str, url: &reqwest::Url, port: u16,
) -> (Value, Vec<String>, NativeNodePreview) {
    let name = outbound
        .get("tag")
        .and_then(|v| v.as_str())
        .unwrap_or(protocol)
        .to_string();
    let warning = warnings.join("; ");
    (
        outbound,
        warnings,
        NativeNodePreview {
            name,
            protocol: protocol.into(),
            server: url.host_str().unwrap_or("").to_string(),
            port,
            warning,
        },
    )
}

fn with_stream_settings(mut outbound: Value, q: &HashMap<String, String>) -> Value {
    let network = q
        .get("type")
        .or_else(|| q.get("net"))
        .map(|v| normalize_transport(v))
        .unwrap_or_else(|| "tcp".into());
    let security = q.get("security").or_else(|| q.get("tls")).cloned().unwrap_or_default();
    let mut stream = serde_json::json!({ "network": network });
    if security == "tls" || security == "reality" {
        stream["security"] = serde_json::json!(security);
    }
    if security == "tls" {
        let mut tls = serde_json::Map::new();
        if let Some(sni) = q.get("sni").or_else(|| q.get("peer")) {
            tls.insert("serverName".into(), serde_json::json!(sni));
        }
        if let Some(fp) = q.get("fp") {
            tls.insert("fingerprint".into(), serde_json::json!(fp));
        }
        if let Some(alpn) = q.get("alpn") {
            tls.insert("alpn".into(), serde_json::json!(alpn.split(',').collect::<Vec<_>>()));
        }
        stream["tlsSettings"] = Value::Object(tls);
    }
    if security == "reality" {
        let mut reality = serde_json::Map::new();
        for (from, to) in [
            ("sni", "serverName"),
            ("fp", "fingerprint"),
            ("pbk", "publicKey"),
            ("sid", "shortId"),
            ("spx", "spiderX"),
            ("spiderX", "spiderX"),
        ] {
            if let Some(value) = q.get(from) {
                reality.insert(to.into(), serde_json::json!(value));
            }
        }
        stream["realitySettings"] = Value::Object(reality);
    }
    match network.as_str() {
        "ws" => {
            stream["wsSettings"] = serde_json::json!({
                "path": q.get("path").cloned().unwrap_or_default(),
                "headers": q.get("host").map(|host| serde_json::json!({ "Host": host })).unwrap_or_else(|| serde_json::json!({})),
            });
        }
        "grpc" => {
            stream["grpcSettings"] = serde_json::json!({ "serviceName": q.get("serviceName").or_else(|| q.get("path")).cloned().unwrap_or_default() });
        }
        "xhttp" => {
            stream["xhttpSettings"] = serde_json::json!({ "path": q.get("path").cloned().unwrap_or_default(), "host": q.get("host").cloned().unwrap_or_default() });
        }
        _ => {}
    }
    outbound["streamSettings"] = stream;
    outbound
}

fn normalize_transport(value: &str) -> String {
    match value {
        "httpupgrade" => "httpupgrade".into(),
        "splithttp" | "xhttp" => "xhttp".into(),
        "kcp" | "mkcp" => "kcp".into(),
        "http" | "h2" => "http".into(),
        "quic" => "quic".into(),
        "grpc" => "grpc".into(),
        "ws" | "websocket" => "ws".into(),
        _ => "tcp".into(),
    }
}

fn query_map(url: &reqwest::Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn vmess_query_map(data: &Value) -> HashMap<String, String> {
    let mut q = HashMap::new();
    for (from, to) in [
        ("net", "type"),
        ("type", "headerType"),
        ("host", "host"),
        ("path", "path"),
        ("tls", "security"),
        ("sni", "sni"),
        ("alpn", "alpn"),
        ("fp", "fp"),
    ] {
        if let Some(value) = data.get(from).and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
            q.insert(to.into(), value.into());
        }
    }
    q
}

fn host(url: &reqwest::Url) -> Result<String, String> {
    url.host_str()
        .map(str::to_string)
        .ok_or_else(|| "Host не указан".to_string())
}

fn link_name(url: &reqwest::Url, fallback: &str) -> String {
    url.fragment()
        .and_then(|v| urlencoding::decode(v).ok().map(|v| v.into_owned()))
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| fallback.into())
}

fn unique_tag(name: String, tags: &mut HashSet<String>) -> String {
    let base = name.trim().chars().take(64).collect::<String>();
    let base = if base.is_empty() { "node".into() } else { base };
    if tags.insert(base.clone()) {
        return base;
    }
    for idx in 2..10000 {
        let candidate = format!("{base}-{idx}");
        if tags.insert(candidate.clone()) {
            return candidate;
        }
    }
    base
}

fn native_name_allowed(sub: &Subscription, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let include = sub.native_include.trim().to_ascii_lowercase();
    let exclude = sub.native_exclude.trim().to_ascii_lowercase();
    (include.is_empty() || lower.contains(&include)) && (exclude.is_empty() || !lower.contains(&exclude))
}

fn decode_userinfo(value: &str) -> String {
    let decoded = urlencoding::decode(value)
        .map(|v| v.into_owned())
        .unwrap_or_else(|_| value.into());
    if decoded.contains(':') {
        return decoded;
    }
    base64::engine::general_purpose::STANDARD
        .decode(decoded.as_bytes())
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(decoded.as_bytes()))
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or(decoded)
}

fn compact_json(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.retain(|_, v| !v.as_str().is_some_and(|s| s.is_empty()));
    }
    value
}

fn build_mihomo_provider_config(content: &str, sub: &Subscription) -> Result<String, String> {
    let without_block = remove_managed_block_from_text(content, &sub.id);
    if yaml_section_has_key(&without_block, "proxy-providers", &sub.provider_name) {
        return Err("proxy-provider уже существует вне управляемого блока XKeen-UI".into());
    }
    if !sub.provider_group.is_empty() && yaml_group_exists(&without_block, &sub.provider_group) {
        return Err("proxy-group уже существует вне управляемого блока XKeen-UI".into());
    }

    let provider_block = mihomo_provider_block(sub);
    let mut next = insert_yaml_section_block(&without_block, "proxy-providers", &provider_block);
    if !sub.provider_group.is_empty() {
        let group_block = mihomo_group_block(sub);
        next = insert_yaml_section_block(&next, "proxy-groups", &group_block);
    }
    YamlLoader::load_from_str(&next).map_err(|e| e.to_string())?;
    Ok(next)
}

// Mihomo config is intentionally edited with small tagged blocks instead of
// round-tripping the whole YAML tree; this keeps user formatting and comments
// outside XKeen-UI-owned subscription snippets intact.
fn mihomo_provider_block(sub: &Subscription) -> String {
    let mut lines = vec![
        format!("  # XKeen-UI subscription {} BEGIN", sub.id),
        format!("  {}:", sub.provider_name),
        "    type: http".into(),
        format!("    url: {}", yaml_quote(&sub.url)),
        "    interval: 3600".into(),
        format!("    path: {}", yaml_quote(&sub.provider_path)),
    ];
    if sub.provider_health_check {
        lines.extend([
            "    health-check:".into(),
            "      enable: true".into(),
            format!("      url: {}", yaml_quote(&sub.provider_health_check_url)),
            format!("      interval: {}", sub.provider_health_check_interval),
        ]);
    }
    lines.push(format!("  # XKeen-UI subscription {} END", sub.id));
    lines.join("\n")
}

fn mihomo_group_block(sub: &Subscription) -> String {
    [
        format!("  # XKeen-UI subscription {} BEGIN", sub.id),
        format!("  - name: {}", yaml_quote(&sub.provider_group)),
        format!("    type: {}", sub.provider_group_type),
        "    use:".into(),
        format!("      - {}", yaml_quote(&sub.provider_name)),
        format!("  # XKeen-UI subscription {} END", sub.id),
    ]
    .join("\n")
}

fn yaml_quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn yaml_section_has_key(content: &str, section: &str, key: &str) -> bool {
    YamlLoader::load_from_str(content)
        .ok()
        .and_then(|docs| docs.into_iter().next())
        .map(|doc| !doc[section][key].is_badvalue())
        .unwrap_or(false)
}

fn yaml_group_exists(content: &str, group: &str) -> bool {
    YamlLoader::load_from_str(content)
        .ok()
        .and_then(|docs| docs.into_iter().next())
        .and_then(|doc| {
            doc["proxy-groups"].as_vec().map(|items| {
                items
                    .iter()
                    .any(|item| item["name"].as_str().is_some_and(|name| name == group))
            })
        })
        .unwrap_or(false)
}

fn insert_yaml_section_block(content: &str, section: &str, block: &str) -> String {
    let header = format!("{section}:");
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let Some(index) = lines.iter().position(|line| {
        let trimmed = line.trim_end();
        trimmed == header || trimmed == format!("{section}: {{}}") || trimmed == format!("{section}: []")
    }) else {
        let mut next = content.trim_end().to_string();
        if !next.is_empty() {
            next.push('\n');
        }
        next.push_str(&header);
        next.push('\n');
        next.push_str(block);
        next.push('\n');
        return next;
    };

    if lines[index].trim_end().ends_with("{}") || lines[index].trim_end().ends_with("[]") {
        lines[index] = header;
    }
    lines.insert(index + 1, block.to_string());
    let mut next = lines.join("\n");
    next.push('\n');
    next
}

fn managed_begin(id: &str) -> String {
    format!("# XKeen-UI subscription {id} BEGIN")
}

fn managed_end(id: &str) -> String {
    format!("# XKeen-UI subscription {id} END")
}

fn remove_managed_block_from_text(content: &str, id: &str) -> String {
    let begin = managed_begin(id);
    let end = managed_end(id);
    let mut skipping = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim() == begin {
            skipping = true;
            continue;
        }
        if line.trim() == end {
            skipping = false;
            continue;
        }
        if !skipping {
            lines.push(line);
        }
    }
    let mut next = lines.join("\n");
    if content.ends_with('\n') {
        next.push('\n');
    }
    next
}

fn command_preview(binary: &str, args: &[String]) -> String {
    std::iter::once(binary.to_string())
        .chain(args.iter().map(|arg| {
            if arg.contains("://") {
                mask_url_arg(arg)
            } else {
                arg.clone()
            }
        }))
        .collect::<Vec<_>>()
        .join(" ")
}

fn mask_url_arg(value: &str) -> String {
    if let Some((tag, url)) = value.split_once('=') {
        format!("{}={}", tag, mask_url(url))
    } else {
        mask_url(value)
    }
}

fn to_view(sub: &Subscription) -> SubscriptionView {
    SubscriptionView {
        id: sub.id.clone(),
        name: sub.name.clone(),
        enabled: sub.enabled,
        url_masked: mask_url(&sub.url),
        core: sub.core.clone(),
        mode: sub.mode.clone(),
        format: sub.format.clone(),
        output_tag: sub.output_tag.clone(),
        output_dir: sub.output_dir.clone(),
        output_file: sub.output_file.clone(),
        auto_restart: sub.auto_restart,
        single_proxy: sub.single_proxy,
        reality_fingerprint: sub.reality_fingerprint.clone(),
        dialer_proxies: sub.dialer_proxies.clone(),
        update_interval: sub.update_interval.clone(),
        timeout_sec: sub.timeout_sec,
        allow_insecure_url: sub.allow_insecure_url,
        provider_name: sub.provider_name.clone(),
        provider_path: sub.provider_path.clone(),
        provider_group: sub.provider_group.clone(),
        provider_group_type: sub.provider_group_type.clone(),
        provider_health_check: sub.provider_health_check,
        provider_health_check_url: sub.provider_health_check_url.clone(),
        provider_health_check_interval: sub.provider_health_check_interval,
        native_include: sub.native_include.clone(),
        native_exclude: sub.native_exclude.clone(),
        last_update_at: sub.last_update_at.clone(),
        last_success_at: sub.last_success_at.clone(),
        last_status: sub.last_status.clone(),
        last_error: sub.last_error.clone(),
        last_node_count: sub.last_node_count,
        last_hash: sub.last_hash.clone(),
        created_at: sub.created_at.clone(),
        updated_at: sub.updated_at.clone(),
    }
}

fn mask_url(value: &str) -> String {
    let Ok(url) = reqwest::Url::parse(value) else {
        return "****".into();
    };
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("");
    let path = url.path().trim_matches('/');
    let path_hint = path
        .split('/')
        .next()
        .filter(|v| !v.is_empty())
        .map(|v| format!("/{:.4}", v))
        .unwrap_or_default();
    format!("{scheme}://{host}{path_hint}...****")
}

fn sanitize_log(value: &str, url: &str) -> String {
    value.replace(url, &mask_url(url))
}

fn slugify(value: &str) -> String {
    let mut result = value
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == '-' || c == '_' || c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>();
    while result.contains("--") {
        result = result.replace("--", "-");
    }
    result = result.trim_matches('-').to_string();
    if result.is_empty() {
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    } else {
        result.chars().take(64).collect()
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn count_outbounds(content: &str) -> usize {
    serde_json::from_str::<Value>(content)
        .ok()
        .and_then(|v| v.get("outbounds").and_then(|v| v.as_array()).map(Vec::len))
        .unwrap_or(0)
}

async fn validate_generated_config(
    settings: &SubscriptionSettings, sub: &Subscription, content: &str,
) -> Result<(), String> {
    let mut files = Vec::new();
    let entries = fs::read_dir(&settings.xray_config_dir).map_err(|e| e.to_string())?;
    let mut found_target = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        if path_str == sub.output_file {
            found_target = true;
            files.push((path_str, content.to_string()));
        } else {
            files.push((path_str, fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    if !found_target {
        files.push((sub.output_file.clone(), content.to_string()));
    }
    validate_core_files("xray", files).await.map_err(|e| {
        append_log(settings, &sub.id, "error", &sanitize_log(&e, &sub.url));
        e
    })
}

async fn atomic_write(path: &str, content: &str) -> Result<(), String> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, content).await.map_err(|e| e.to_string())?;
    tokio::fs::rename(&tmp, path).await.map_err(|e| e.to_string())
}

fn backup_output(settings: &SubscriptionSettings, sub: &Subscription) -> Result<(), String> {
    if sub.output_file.is_empty() || !Path::new(&sub.output_file).exists() {
        return Ok(());
    }
    let dir = Path::new(&settings.backup_dir).join(&sub.id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("{}_{}", Utc::now().format("%Y-%m-%d_%H-%M-%S"), output_file_name(sub));
    fs::copy(&sub.output_file, dir.join(name)).map_err(|e| e.to_string())?;
    Ok(())
}

fn backup_file(settings: &SubscriptionSettings, id: &str, path: &Path, display_name: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let dir = Path::new(&settings.backup_dir).join(id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("{}_{}", Utc::now().format("%Y-%m-%d_%H-%M-%S"), display_name);
    fs::copy(path, dir.join(name)).map_err(|e| e.to_string())?;
    Ok(())
}

async fn rollback_subscription(
    settings: &SubscriptionSettings, state: Option<&AppState>, sub: &Subscription,
) -> Result<UpdateResult, String> {
    let backup = latest_backup(settings, sub)?;
    let target = if sub.core == "mihomo" {
        settings.mihomo_config_path.clone()
    } else {
        sub.output_file.clone()
    };
    let content = tokio::fs::read_to_string(&backup).await.map_err(|e| e.to_string())?;
    if sub.core == "mihomo" {
        validate_core_files("mihomo", vec![(settings.mihomo_config_path.clone(), content.clone())]).await?;
    } else {
        validate_generated_config(settings, sub, &content).await?;
    }
    atomic_write(&target, &content).await?;
    let mut restarted = false;
    if sub.auto_restart {
        restart_xkeen(state).await?;
        restarted = true;
    }
    let hash = format!("{:x}", md5::compute(content.as_bytes()));
    let node_count = if sub.core == "xray" {
        count_outbounds(&content)
    } else {
        sub.last_node_count
    };
    update_status(settings, &sub.id, "ok", "", node_count, Some(hash))?;
    append_log(
        settings,
        &sub.id,
        "info",
        &format!("Rollback restored {}", backup.display()),
    );
    Ok(UpdateResult {
        id: sub.id.clone(),
        status: "ok".into(),
        node_count,
        output_file: target,
        dry_run: false,
        restarted,
        message: "Rollback completed".into(),
    })
}

fn latest_backup(settings: &SubscriptionSettings, sub: &Subscription) -> Result<PathBuf, String> {
    let dir = Path::new(&settings.backup_dir).join(&sub.id);
    let mut entries = fs::read_dir(&dir)
        .map_err(|_| "No backups found".to_string())?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    entries.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    entries.pop().ok_or_else(|| "No backups found".into())
}

fn remove_mihomo_subscription_block(settings: &SubscriptionSettings, sub: &Subscription) -> Result<(), String> {
    let path = Path::new(&settings.mihomo_config_path);
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let next = remove_managed_block_from_text(&content, &sub.id);
    backup_file(settings, &sub.id, path, "config.yaml")?;
    fs::write(path, next).map_err(|e| e.to_string())
}

fn update_status(
    settings: &SubscriptionSettings, id: &str, status: &str, error: &str, node_count: usize, hash: Option<String>,
) -> Result<(), String> {
    mutate_store(settings, |store| {
        let Some(sub) = store.subscriptions.iter_mut().find(|item| item.id == id) else {
            return Ok(());
        };
        let at = now();
        sub.last_update_at = Some(at.clone());
        sub.last_status = status.into();
        sub.last_error = sanitize_log(error, &sub.url);
        sub.last_node_count = node_count;
        if status == "ok" {
            sub.last_success_at = Some(at);
        }
        if let Some(hash) = hash {
            sub.last_hash = hash;
        }
        sub.updated_at = now();
        Ok(())
    })
}

fn append_log(settings: &SubscriptionSettings, id: &str, level: &str, message: &str) {
    let _ = fs::create_dir_all(&settings.log_dir);
    let path = log_path(settings, id);
    rotate_log(&path);
    let line = serde_json::json!({
        "ts": ts(),
        "level": level,
        "subscriptionId": id,
        "message": message,
    });
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", line);
    }
}

fn log_path(settings: &SubscriptionSettings, id: &str) -> PathBuf {
    Path::new(&settings.log_dir).join(format!("{id}.log"))
}

fn rotate_log(path: &Path) {
    if fs::metadata(path).map(|m| m.len()).unwrap_or(0) <= MAX_LOG_BYTES {
        return;
    }
    let _ = fs::rename(path, path.with_extension("log.1"));
}

fn acquire_lock(settings: &SubscriptionSettings) -> Result<FileLock, String> {
    let path = PathBuf::from(&settings.lock_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if let Ok(metadata) = fs::metadata(&path) {
        let fresh = metadata
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age < LOCK_MAX_AGE);
        if fresh {
            return Err("Обновление подписок уже выполняется".into());
        }
        let _ = fs::remove_file(&path);
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    Ok(FileLock { path })
}

fn command_exists(name: &str) -> bool {
    std::process::Command::new(name)
        .arg("-v")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

async fn restart_xkeen(state: Option<&AppState>) -> Result<(), String> {
    if let Some(state) = state {
        return controller::run_init_command(state, &["restart", "on"]).await;
    }
    let init = controller::find_init_file(false).ok_or_else(|| "Не найден init файл XKeen".to_string())?;
    Command::new(init)
        .args(["restart", "on"])
        .status()
        .await
        .map_err(|e| e.to_string())
        .and_then(|status| status.success().then_some(()).ok_or_else(|| status.to_string()))
}

fn read_crontab() -> Result<String, String> {
    let output = std::process::Command::new("crontab")
        .arg("-l")
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Ok(String::new())
    }
}

fn write_crontab(content: &str) -> Result<(), String> {
    let path = std::env::temp_dir().join(format!("xkeen-ui-cron-{}", std::process::id()));
    fs::write(&path, content).map_err(|e| e.to_string())?;
    let status = std::process::Command::new("crontab")
        .arg(&path)
        .status()
        .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(path);
    status
        .success()
        .then_some(())
        .ok_or_else(|| "Не удалось обновить crontab".into())
}

fn upsert_cron_block(id: &str, interval: &str, settings: &SubscriptionSettings) -> Result<(), String> {
    let current = remove_cron_block_from_text(&read_crontab()?, id);
    let log_file = Path::new(&settings.log_dir)
        .join(format!("{id}.cron.log"))
        .to_string_lossy()
        .to_string();
    let block = format!(
        "# XKeen-UI subscription {id} BEGIN\n{interval} /opt/sbin/xkeen-ui subscription-update {id} >{log_file} 2>&1\n# XKeen-UI subscription {id} END\n"
    );
    write_crontab(&format!("{}\n{}", current.trim_end(), block))
}

fn remove_cron_block(id: &str) -> Result<(), String> {
    let next = remove_cron_block_from_text(&read_crontab()?, id);
    write_crontab(&next)
}

fn remove_cron_block_from_text(content: &str, id: &str) -> String {
    let begin = format!("# XKeen-UI subscription {id} BEGIN");
    let end = format!("# XKeen-UI subscription {id} END");
    let mut skipping = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim() == begin {
            skipping = true;
            continue;
        }
        if line.trim() == end {
            skipping = false;
            continue;
        }
        if !skipping {
            lines.push(line);
        }
    }
    lines.join("\n")
}

fn validate_cron(value: &str) -> Result<(), String> {
    let parts = value.split_whitespace().count();
    if parts != 5 {
        return Err("Cron-расписание должно содержать 5 полей".into());
    }
    Ok(())
}

fn ok<T: Serialize>(data: Option<T>) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        success: true,
        error: None,
        data,
    })
}

fn err<T: Serialize>(message: String) -> Json<ApiResponse<T>> {
    log("ERROR", message.clone());
    Json(ApiResponse {
        success: false,
        error: Some(message),
        data: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_url() {
        assert_eq!(
            mask_url("https://example.com/sub/token?secret=1"),
            "https://example.com/sub...****"
        );
    }

    #[test]
    fn validates_identifiers() {
        assert!(validate_ident("id", "main-vps_1", r"^[a-zA-Z0-9_-]{1,64}$").is_ok());
        assert!(validate_ident("id", "bad/id", r"^[a-zA-Z0-9_-]{1,64}$").is_err());
    }

    #[test]
    fn removes_cron_block() {
        let input = "a\n# XKeen-UI subscription main BEGIN\nx\n# XKeen-UI subscription main END\nb";
        assert_eq!(remove_cron_block_from_text(input, "main"), "a\nb");
    }

    #[test]
    fn builds_watcher_args_without_shell() {
        let sub = Subscription {
            id: "main".into(),
            output_tag: "sub-main".into(),
            url: "https://example.com/sub".into(),
            single_proxy: true,
            ..Default::default()
        };
        let args = watcher_args(&sub, Path::new("/tmp/out"), true);
        assert!(args.contains(&"--no-restart".into()));
        assert!(args.contains(&"--single-proxy".into()));
        assert!(args.contains(&"sub-main=https://example.com/sub".into()));
    }
}
