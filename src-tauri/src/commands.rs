use crate::{
    models::{Node, PublicState},
    subscription::{fetch_many, fetch_nodes},
    system_proxy,
    xray::RuntimeState,
};
use anyhow::{Result, anyhow};
use std::{
    collections::VecDeque,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, task::JoinSet};

pub type Shared = Arc<Mutex<RuntimeState>>;

const PROBE_CONCURRENCY: usize = 32;
const BACKGROUND_FULL_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
const CONNECTIVITY_INTERVAL: Duration = Duration::from_secs(20);
const FULL_DELAY_INTERVAL: Duration = Duration::from_secs(120);
const RECOVERY_COOLDOWN: Duration = Duration::from_secs(300);
const RECOVERY_GRACE: Duration = Duration::from_secs(60);
const FAILURE_LIMIT: u32 = 2;
static REFRESH_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
static SUBSCRIPTION_UPDATE_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

async fn finish_with(
    shared: &Shared,
    result: Result<()>,
    update_banner: bool,
) -> Result<PublicState, String> {
    if let Err(error) = &result {
        crate::logging::error(&format!("操作失败：{error:#}"));
    }
    let mut state = shared.lock().await;
    if update_banner {
        state.last_error = result
            .as_ref()
            .err()
            .map(|error| crate::logging::redact_urls(&format!("{error:#}")));
    }
    if let Err(error) = state.save() {
        crate::logging::error(&format!("保存配置失败：{error:#}"));
        state.last_error = Some(format!("{error:#}"));
    }
    let public = state.public();
    crate::xray::emit_state(&public);
    Ok(public)
}

async fn finish(shared: &Shared, result: Result<()>) -> Result<PublicState, String> {
    finish_with(shared, result, true).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RefreshAfterUpdate {
    Always,
    RecoveryOnly,
}

fn should_refresh_after_update(
    failed: bool,
    quiet: bool,
    auto_select: bool,
    policy: RefreshAfterUpdate,
    recovery_required: bool,
) -> bool {
    auto_select
        && match policy {
            RefreshAfterUpdate::Always => !failed || quiet,
            RefreshAfterUpdate::RecoveryOnly => recovery_required,
        }
}

fn node_changed_after_refresh(previous: &Option<Node>, current: Option<Node>) -> bool {
    match (previous, current) {
        (Some(previous), Some(current)) => !previous.has_same_connection(&current),
        (None, None) => false,
        _ => true,
    }
}

fn has_valid_current_node(state: &RuntimeState) -> bool {
    let Some(subscription) = state
        .persisted
        .subscriptions
        .iter()
        .find(|item| Some(&item.id) == state.persisted.current_subscription.as_ref())
    else {
        return false;
    };
    let Some(name) = state.persisted.current_node.as_ref() else {
        return false;
    };
    subscription.nodes.iter().any(|node| {
        &node.name == name
            && !crate::models::is_subscription_info_node_name(&node.name)
            && (!state.persisted.auto_select
                || !crate::models::is_mainland_china_node_name(&node.name))
    })
}

fn repair_current_node(state: &mut RuntimeState) {
    if has_valid_current_node(state) {
        return;
    }
    let default = state
        .persisted
        .subscriptions
        .iter()
        .find(|item| Some(&item.id) == state.persisted.current_subscription.as_ref())
        .and_then(crate::models::default_node_name);
    state.persisted.current_node = default;
}

#[tauri::command]
pub async fn get_state(shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    Ok(shared.lock().await.public())
}

#[tauri::command]
pub async fn add_subscription(
    name: String,
    url: String,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("请输入有效的订阅地址"));
        }
        let nodes = fetch_nodes(&url).await?;
        let mut state = shared.lock().await;
        let id = uuid::Uuid::new_v4().simple().to_string();
        let display_name = if name.trim().is_empty() {
            "未命名订阅".into()
        } else {
            name.trim().into()
        };
        state
            .persisted
            .subscriptions
            .push(crate::models::Subscription {
                id: id.clone(),
                name: display_name,
                url,
                nodes,
            });
        if state.persisted.current_subscription.is_none() {
            state.persisted.current_subscription = Some(id);
            state.persisted.current_node = state
                .persisted
                .subscriptions
                .first()
                .and_then(crate::models::default_node_name);
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn delete_subscription(
    id: String,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        state.persisted.subscriptions.retain(|item| item.id != id);
        if state.persisted.current_subscription.as_deref() == Some(id.as_str()) {
            state.persisted.current_subscription = state
                .persisted
                .subscriptions
                .first()
                .map(|item| item.id.clone());
            state.persisted.current_node = state
                .persisted
                .subscriptions
                .first()
                .and_then(crate::models::default_node_name);
            if state.persisted.proxy_enabled {
                state.restart()?;
            }
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn select_subscription(
    id: String,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        let first_node = state
            .persisted
            .subscriptions
            .iter()
            .find(|item| item.id == id)
            .and_then(crate::models::default_node_name);
        state.persisted.current_subscription = Some(id);
        state.persisted.current_node = first_node;
        if state.persisted.proxy_enabled {
            state.restart()?;
        }
        Ok(())
    }
    .await;
    let output = finish(&shared, result).await;
    let succeeded = output
        .as_ref()
        .is_ok_and(|state| state.last_error.is_none());
    if succeeded && shared.lock().await.persisted.auto_select {
        return refresh_best_node_impl(&shared).await;
    }
    output
}

async fn update_subscription_impl(id: &str, shared: &Shared) -> Result<PublicState, String> {
    let update_lock = SUBSCRIPTION_UPDATE_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone();
    let _update_guard = update_lock.lock().await;
    let result = async {
        let url = {
            let state = shared.lock().await;
            state
                .persisted
                .subscriptions
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.url.clone())
                .ok_or_else(|| anyhow!("订阅不存在"))?
        };
        let nodes = fetch_nodes(&url).await?;
        let mut state = shared.lock().await;
        let previous_node = state.current_node().ok();
        let is_current = state.persisted.current_subscription.as_deref() == Some(id);
        {
            let subscription = state
                .persisted
                .subscriptions
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or_else(|| anyhow!("订阅不存在"))?;
            subscription.nodes = nodes;
        }
        if is_current {
            repair_current_node(&mut state);
            let node_changed =
                node_changed_after_refresh(&previous_node, state.current_node().ok());
            if node_changed && (state.running || state.persisted.proxy_enabled) {
                state.restart()?;
            }
        }
        Ok(())
    }
    .await;
    let output = finish(&shared, result).await;
    let should_refresh = output
        .as_ref()
        .is_ok_and(|state| state.last_error.is_none())
        && {
            let state = shared.lock().await;
            state.persisted.auto_select
                && state.persisted.current_subscription.as_deref() == Some(id)
        };
    if should_refresh {
        return refresh_best_node_impl(&shared).await;
    }
    output
}

#[tauri::command]
pub async fn update_subscription(
    id: String,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    update_subscription_impl(&id, &shared).await
}

async fn update_all_subscriptions_impl(
    shared: &Shared,
    quiet: bool,
) -> Result<PublicState, String> {
    update_all_subscriptions_impl_with_policy(shared, quiet, RefreshAfterUpdate::Always).await
}

async fn update_all_subscriptions_impl_with_policy(
    shared: &Shared,
    quiet: bool,
    refresh_policy: RefreshAfterUpdate,
) -> Result<PublicState, String> {
    let update_lock = SUBSCRIPTION_UPDATE_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone();
    let _update_guard = update_lock.lock().await;
    let mut restart_after_refresh = false;
    let mut recovery_required = false;
    let result = async {
        let items = {
            let state = shared.lock().await;
            state
                .persisted
                .subscriptions
                .iter()
                .map(|item| (item.id.clone(), item.url.clone()))
                .collect::<Vec<_>>()
        };
        let results = fetch_many(items).await;
        let mut errors = Vec::new();
        {
            let mut state = shared.lock().await;
            let current_was_valid = has_valid_current_node(&state);
            let previous_node = state.current_node().ok();
            for (id, result) in results {
                match result {
                    Ok(nodes) => {
                        if let Some(subscription) = state
                            .persisted
                            .subscriptions
                            .iter_mut()
                            .find(|item| item.id == id)
                        {
                            subscription.nodes = nodes;
                        }
                    }
                    Err(error) => {
                        let name = state
                            .persisted
                            .subscriptions
                            .iter()
                            .find(|item| item.id == id)
                            .map(|item| item.name.clone())
                            .unwrap_or(id);
                        errors.push(format!("{name}：{error:#}（已保留原节点）"));
                    }
                }
            }
            repair_current_node(&mut state);
            let node_changed =
                node_changed_after_refresh(&previous_node, state.current_node().ok());
            recovery_required = !current_was_valid || node_changed;
            if node_changed && (state.running || state.persisted.proxy_enabled) {
                if quiet && state.persisted.auto_select {
                    restart_after_refresh = true;
                } else {
                    state.restart()?;
                }
            }
        }
        if !errors.is_empty() {
            return Err(anyhow!(errors.join("；")));
        }
        crate::logging::info("全部订阅更新完成，当前节点未变化时不重启核心");
        Ok(())
    }
    .await;
    let failed = result.is_err();
    let output = finish_with(&shared, result, !quiet).await;
    let auto_select = shared.lock().await.persisted.auto_select;
    if should_refresh_after_update(
        failed,
        quiet,
        auto_select,
        refresh_policy,
        recovery_required,
    ) {
        return refresh_best_node_impl_with_restart(&shared, restart_after_refresh).await;
    }
    if quiet && auto_select && !recovery_required {
        crate::logging::info("后台订阅刷新未触发全量选优：当前节点仍有效且连接配置未变化");
    }
    output
}

#[tauri::command]
pub async fn update_all_subscriptions(
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    update_all_subscriptions_impl(&shared, false).await
}

#[tauri::command]
pub async fn select_node(
    name: String,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        if state.persisted.auto_select {
            return Err(anyhow!("自动选择开启时不能手动选择节点"));
        }
        if crate::models::is_subscription_info_node_name(&name) {
            return Err(anyhow!("该条目是订阅信息，不是可连接节点"));
        }
        let exists = state.persisted.subscriptions.iter().any(|item| {
            Some(&item.id) == state.persisted.current_subscription.as_ref()
                && item.nodes.iter().any(|node| node.name == name)
        });
        if !exists {
            return Err(anyhow!("节点不存在"));
        }
        state.persisted.current_node = Some(name);
        if state.running || state.persisted.proxy_enabled {
            state.restart()?;
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn set_auto_select(
    enabled: bool,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    {
        let mut state = shared.lock().await;
        state.persisted.auto_select = enabled;
        let _ = state.save();
    }
    if enabled {
        refresh_best_node_impl(&shared).await
    } else {
        Ok(shared.lock().await.public())
    }
}

#[tauri::command]
pub async fn set_proxy_enabled(
    enabled: bool,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        let port = state.persisted.proxy_port;
        if enabled {
            state.restart()?;
            if !state.persisted.tun_enabled
                && let Err(error) = system_proxy::enable(port)
            {
                let _ = state.stop();
                return Err(error);
            }
        } else {
            let stop_result = state.stop();
            let proxy_result = system_proxy::disable(port);
            stop_result?;
            proxy_result?;
        }
        state.persisted.proxy_enabled = enabled;
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn set_auto_start(
    enabled: bool,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        let previous = (state.persisted.auto_start, state.persisted.silent_start);
        state.persisted.auto_start = enabled;
        if !enabled {
            state.persisted.silent_start = false;
        }
        if let Err(error) = crate::startup::configure(enabled, state.persisted.silent_start) {
            state.persisted.auto_start = previous.0;
            state.persisted.silent_start = previous.1;
            return Err(error);
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn set_silent_start(
    enabled: bool,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        let previous = (state.persisted.auto_start, state.persisted.silent_start);
        if enabled {
            state.persisted.auto_start = true;
        }
        state.persisted.silent_start = enabled;
        if let Err(error) = crate::startup::configure(state.persisted.auto_start, enabled) {
            state.persisted.auto_start = previous.0;
            state.persisted.silent_start = previous.1;
            return Err(error);
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn set_tun_enabled(
    enabled: bool,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        state.persisted.tun_enabled = enabled;
        let port = state.persisted.proxy_port;
        if enabled {
            state.persisted.proxy_enabled = true;
            state.restart()?;
        } else {
            state.restart()?;
            if state.persisted.proxy_enabled {
                system_proxy::enable(port)?;
            }
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn set_proxy_port(
    port: u16,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let requested = validate_proxy_port(port)?;
        let mut state = shared.lock().await;
        let previous = state.persisted.proxy_port;
        if requested == previous {
            return Ok(());
        }
        let system_proxy_on = state.persisted.proxy_enabled && !state.persisted.tun_enabled;
        if system_proxy_on {
            system_proxy::disable(previous)?;
        }
        state.persisted.proxy_port = requested;
        if state.persisted.proxy_enabled || state.persisted.tun_enabled {
            if let Err(error) = state.restart() {
                state.persisted.proxy_port = previous;
                let _ = state.restart();
                if system_proxy_on {
                    let _ = system_proxy::enable(previous);
                }
                return Err(error);
            }
            if system_proxy_on {
                system_proxy::enable(requested)?;
            }
        }
        crate::logging::info(&format!("代理端口已切换：{previous} -> {requested}"));
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn set_update_interval(
    minutes: u64,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        state.persisted.update_interval_min = clamp_update_interval(minutes);
        crate::logging::info(&format!(
            "订阅自动更新间隔已设为 {} 分钟",
            state.persisted.update_interval_min
        ));
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn set_auto_update(
    enabled: bool,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        state.persisted.auto_update = enabled;
        crate::logging::info(&format!(
            "自动更新已{}",
            if enabled { "开启" } else { "关闭" }
        ));
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn check_update(shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    let outcome = crate::update::check().await;
    let mut state = shared.lock().await;
    match outcome {
        Ok(info) => {
            state.update = Some(info);
            state.last_error = None;
        }
        Err(error) => {
            let message = crate::logging::redact_urls(&format!("{error:#}"));
            crate::logging::error(&format!("检查更新失败：{message}"));
            state.last_error = Some(format!("检查更新失败：{message}"));
        }
    }
    let public = state.public();
    crate::xray::emit_state(&public);
    Ok(public)
}

#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    shared: tauri::State<'_, Shared>,
) -> Result<PublicState, String> {
    let (url, latest) = {
        let state = shared.lock().await;
        match state.update.as_ref() {
            Some(info) if info.available => (info.url.clone(), info.latest.clone()),
            Some(_) => return Err("当前已经是最新版本".into()),
            None => return Err("请先检查更新".into()),
        }
    };
    let setup = crate::update::download(&url)
        .await
        .map_err(|error| format!("下载更新包失败：{error:#}"))?;
    crate::update::launch_installer(&setup, &latest)
        .map_err(|error| format!("启动更新程序失败：{error:#}"))?;
    let public = shared.lock().await.public();
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(400)).await;
        handle.exit(0);
    });
    Ok(public)
}

pub fn spawn_update_checker(shared: Shared) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(8)).await;
        loop {
            let (auto_update, last_attempt) = {
                let state = shared.lock().await;
                (
                    state.persisted.auto_update,
                    state.persisted.last_auto_update.clone(),
                )
            };
            if auto_update && let Ok(info) = crate::update::check().await {
                let should_install = info.available
                    && crate::update::auto_install_allowed(&info.latest, last_attempt.as_deref());
                {
                    let mut state = shared.lock().await;
                    state.update = Some(info.clone());
                    if should_install {
                        state.persisted.last_auto_update = Some(info.latest.clone());
                        let _ = state.save();
                    }
                    let public = state.public();
                    crate::xray::emit_state(&public);
                }
                if should_install && crate::update::mark_pending(&info.latest).is_ok() {
                    if let Ok(setup) = crate::update::download(&info.url).await
                        && crate::update::launch_installer(&setup, &info.latest).is_ok()
                    {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        crate::xray::quit_app();
                        return;
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

fn validate_proxy_port(port: u16) -> Result<u16> {
    if !(1024..=65535).contains(&port) {
        return Err(anyhow!("端口需要在 1024-65535 之间"));
    }
    if matches!(port, 10820 | 10821) {
        return Err(anyhow!("10820/10821 保留给测速核心，请换一个端口"));
    }
    Ok(port)
}

fn clamp_update_interval(minutes: u64) -> u64 {
    minutes.clamp(1, 24 * 60)
}

#[tauri::command]
async fn refresh_best_node_impl(shared: &Shared) -> Result<PublicState, String> {
    refresh_best_node_impl_with_restart(shared, false).await
}

async fn refresh_best_node_impl_with_restart(
    shared: &Shared,
    restart_after_refresh: bool,
) -> Result<PublicState, String> {
    let refresh_lock = REFRESH_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone();
    let _refresh_guard = refresh_lock.lock().await;
    let (subscription_id, subscription_name, candidates) = {
        let state = shared.lock().await;
        let Some(subscription) = state
            .persisted
            .subscriptions
            .iter()
            .find(|item| Some(&item.id) == state.persisted.current_subscription.as_ref())
        else {
            return finish(&shared, Err(anyhow!("请先添加并选择订阅"))).await;
        };
        if !state.persisted.auto_select {
            return Ok(state.public());
        }
        (
            subscription.id.clone(),
            subscription.name.clone(),
            subscription
                .nodes
                .iter()
                .filter(|node| crate::models::is_auto_selectable_node(node))
                .cloned()
                .collect::<Vec<_>>(),
        )
    };
    if candidates.is_empty() {
        return finish(
            &shared,
            Err(anyhow!(
                "当前订阅没有可自动选择的境外节点，可关闭自动选择手动连接"
            )),
        )
        .await;
    }

    let candidate_count = candidates.len();
    crate::logging::info(&format!(
        "开始自动选优：订阅={subscription_name}，候选节点={candidate_count}"
    ));
    let probe = match crate::xray::start_probe_group(&candidates) {
        Ok(probe) => probe,
        Err(error) => return finish(&shared, Err(error)).await,
    };
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(4500))
        .build()
    {
        Ok(client) => client,
        Err(error) => return finish(&shared, Err(error.into())).await,
    };
    let mut pending = VecDeque::from(candidates.into_iter().enumerate().collect::<Vec<_>>());
    let mut tasks = JoinSet::new();
    let mut best: Option<(String, u32)> = None;
    let mut measured_count = 0;
    let mut logged_failures = 0;
    while !pending.is_empty() || !tasks.is_empty() {
        while tasks.len() < PROBE_CONCURRENCY && !pending.is_empty() {
            let (index, node) = pending.pop_front().expect("candidate exists");
            tasks.spawn(measure_node(
                shared.clone(),
                subscription_id.clone(),
                index,
                node,
                client.clone(),
            ));
        }
        if let Some(result) = tasks.join_next().await {
            match result {
                Ok((name, Some(delay), _)) => {
                    measured_count += 1;
                    if best
                        .as_ref()
                        .is_none_or(|(_, best_delay)| delay < *best_delay)
                    {
                        best = Some((name, delay));
                    }
                }
                Ok((name, None, reason)) => {
                    if logged_failures < 3 {
                        crate::logging::error(&format!(
                            "节点测速失败：{name}，{}",
                            reason.unwrap_or_else(|| "两次探测均未返回延迟".into())
                        ));
                        logged_failures += 1;
                    }
                }
                Err(error) => crate::logging::error(&format!("测速任务异常：{error}")),
            }
        }
    }
    drop(probe);

    let result = async {
        let Some((best_name, best_delay)) = best else {
            return Err(anyhow!(
                "{candidate_count} 个境外节点测速均失败，保留当前节点"
            ));
        };
        let mut state = shared.lock().await;
        if !state.persisted.auto_select
            || state.persisted.current_subscription.as_ref() != Some(&subscription_id)
        {
            crate::logging::info("忽略已过期订阅的自动选优结果");
            return Ok(());
        }
        let best_still_exists = state
            .persisted
            .subscriptions
            .iter()
            .find(|item| item.id == subscription_id)
            .is_some_and(|subscription| {
                subscription
                    .nodes
                    .iter()
                    .any(|node| node.name == best_name && crate::models::is_auto_selectable_node(node))
            });
        if !best_still_exists {
            crate::logging::info("最佳节点已在订阅刷新后移除，忽略过期结果");
            return Ok(());
        }
        let changed = state.persisted.current_node.as_deref() != Some(best_name.as_str());
        state.persisted.current_node = Some(best_name.clone());
        if (changed || restart_after_refresh) && (state.running || state.persisted.proxy_enabled) {
            state.restart()?;
        }
        crate::logging::info(&format!(
            "自动选优完成：订阅={subscription_name}，最佳节点={best_name}，延迟={best_delay}ms，测速成功={measured_count}/{candidate_count}"
        ));
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn refresh_best_node(shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    refresh_best_node_impl(&shared).await
}

async fn measure_node(
    shared: Shared,
    subscription_id: String,
    index: usize,
    node: Node,
    client: reqwest::Client,
) -> (String, Option<u32>, Option<String>) {
    let name = node.name.clone();
    let (delay, failure) = async {
        let mut attempts = JoinSet::new();
        for _ in 0..2 {
            let client = client.clone();
            attempts.spawn(async move {
                let response = client
                    .get(format!(
                        "http://127.0.0.1:10821/proxies/EX1-PROBE-{index}/delay"
                    ))
                    .query(&[
                        ("url", "http://cp.cloudflare.com/generate_204"),
                        ("timeout", "4000"),
                    ])
                    .send()
                    .await
                    .map_err(|error| error.to_string())?;
                let value = response
                    .json::<serde_json::Value>()
                    .await
                    .map_err(|error| error.to_string())?;
                value
                    .get("delay")
                    .and_then(serde_json::Value::as_u64)
                    .map(|delay| delay as u32)
                    .ok_or_else(|| {
                        value
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("测速核心未返回延迟")
                            .to_owned()
                    })
            });
        }
        let mut measured = None;
        let mut failure = None;
        while let Some(result) = attempts.join_next().await {
            match result {
                Ok(Ok(delay)) => {
                    measured = Some(delay);
                    break;
                }
                Ok(Err(error)) => failure = Some(error),
                Err(error) => failure = Some(error.to_string()),
            }
        }
        (measured, failure)
    }
    .await;
    let mut state = shared.lock().await;
    if state.persisted.current_subscription.as_ref() == Some(&subscription_id)
        && let Some(subscription) = state
            .persisted
            .subscriptions
            .iter_mut()
            .find(|item| item.id == subscription_id)
        && let Some(current) = subscription.nodes.iter_mut().find(|item| item.name == name)
    {
        current.delay = delay;
    }
    (name, delay, failure)
}

fn full_refresh_due(last_refresh: Instant, now: Instant) -> bool {
    now.saturating_duration_since(last_refresh) >= BACKGROUND_FULL_REFRESH_INTERVAL
}

pub fn spawn_background(shared: Shared) {
    tokio::spawn(async move {
        if shared.lock().await.persisted.auto_select {
            let _ = refresh_best_node_impl(&shared).await;
        }
        let mut last_full_refresh = Instant::now();
        loop {
            let minutes = shared
                .lock()
                .await
                .persisted
                .update_interval_min
                .clamp(1, 24 * 60);
            let until_full_refresh =
                BACKGROUND_FULL_REFRESH_INTERVAL.saturating_sub(last_full_refresh.elapsed());
            tokio::time::sleep(Duration::from_secs(minutes * 60).min(until_full_refresh)).await;
            let now = Instant::now();
            let full_refresh = full_refresh_due(last_full_refresh, now);
            let _ = update_all_subscriptions_impl_with_policy(
                &shared,
                true,
                RefreshAfterUpdate::RecoveryOnly,
            )
            .await;
            if full_refresh {
                if shared.lock().await.persisted.auto_select {
                    crate::logging::info("开始每 15 分钟一次的后台全量选优");
                    let _ = refresh_best_node_impl(&shared).await;
                }
                last_full_refresh = now;
            }
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn spawn_core_watchdog(shared: Shared) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let recovered = {
                let mut state = shared.lock().await;
                if !state.core_exited() {
                    None
                } else {
                    crate::logging::error("代理核心已退出，正在自动恢复");
                    state.running = false;
                    if let Err(error) = state.restart() {
                        crate::logging::error(&format!("代理核心自动恢复失败：{error:#}"));
                        state.last_error = Some(format!("代理核心自动恢复失败：{error:#}"));
                    }
                    Some(state.public())
                }
            };
            if let Some(public) = recovered {
                crate::xray::emit_state(&public);
            }
        }
    });
}

#[cfg(not(target_os = "windows"))]
struct ConnectivityState {
    failures: u32,
    last_recovery: Option<std::time::Instant>,
    last_full_check: Option<std::time::Instant>,
}

#[cfg(not(target_os = "windows"))]
impl ConnectivityState {
    fn new() -> Self {
        Self {
            failures: 0,
            last_recovery: None,
            last_full_check: Some(std::time::Instant::now()),
        }
    }

    fn note_failure(&mut self, now: std::time::Instant) -> bool {
        self.failures = self.failures.saturating_add(1);
        if self.failures < FAILURE_LIMIT {
            return false;
        }
        if self
            .last_recovery
            .is_some_and(|last| now.duration_since(last) < RECOVERY_COOLDOWN)
        {
            return false;
        }
        self.last_recovery = Some(now);
        self.failures = 0;
        true
    }

    fn note_success(&mut self) {
        self.failures = 0;
    }

    fn take_full_check_due(&mut self, now: std::time::Instant) -> bool {
        if self
            .last_full_check
            .is_some_and(|last| now.duration_since(last) < FULL_DELAY_INTERVAL)
        {
            return false;
        }
        self.last_full_check = Some(now);
        true
    }

    fn mark_full_check(&mut self) {
        self.last_full_check = Some(std::time::Instant::now());
    }
}

#[cfg(not(target_os = "windows"))]
async fn proxy_is_healthy(port: u16) -> bool {
    let result = async {
        let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{port}"))?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .proxy(proxy)
            .timeout(Duration::from_secs(5))
            .build()?;
        let response = client
            .get("https://www.gstatic.com/generate_204")
            .send()
            .await?;
        Ok::<bool, reqwest::Error>(response.status() == reqwest::StatusCode::NO_CONTENT)
    }
    .await;
    result.unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn spawn_connectivity_watchdog(shared: Shared) {
    tokio::spawn(async move {
        let mut connectivity = ConnectivityState::new();
        loop {
            tokio::time::sleep(CONNECTIVITY_INTERVAL).await;
            let (enabled, running, port, auto_select) = {
                let state = shared.lock().await;
                (
                    state.persisted.proxy_enabled,
                    state.running,
                    state.persisted.proxy_port,
                    state.persisted.auto_select,
                )
            };
            if !enabled || !running {
                connectivity.note_success();
                continue;
            }
            if proxy_is_healthy(port).await {
                connectivity.note_success();
                if auto_select && connectivity.take_full_check_due(std::time::Instant::now()) {
                    crate::logging::info("开始每 2 分钟一次的独立节点选优");
                    let _ = refresh_best_node_impl(&shared).await;
                }
            } else if connectivity.note_failure(std::time::Instant::now()) {
                crate::logging::error("真实联网连续失败，立即刷新订阅并重新选优");
                let _ = update_all_subscriptions_impl(&shared, true).await;
                connectivity.mark_full_check();
                tokio::time::sleep(RECOVERY_GRACE).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        BACKGROUND_FULL_REFRESH_INTERVAL, RefreshAfterUpdate, clamp_update_interval,
        full_refresh_due, should_refresh_after_update, validate_proxy_port,
    };
    #[cfg(not(target_os = "windows"))]
    use super::{ConnectivityState, FULL_DELAY_INTERVAL, RECOVERY_COOLDOWN};

    #[test]
    fn manual_and_quiet_recovery_updates_keep_full_refresh_behavior() {
        assert!(should_refresh_after_update(
            true,
            true,
            true,
            RefreshAfterUpdate::Always,
            false
        ));
        assert!(!should_refresh_after_update(
            true,
            false,
            true,
            RefreshAfterUpdate::Always,
            false
        ));
        assert!(should_refresh_after_update(
            false,
            false,
            true,
            RefreshAfterUpdate::Always,
            false
        ));
    }

    #[test]
    fn background_update_with_valid_unchanged_node_skips_full_refresh() {
        assert!(!should_refresh_after_update(
            false,
            true,
            true,
            RefreshAfterUpdate::RecoveryOnly,
            false
        ));
        assert!(!should_refresh_after_update(
            true,
            true,
            true,
            RefreshAfterUpdate::RecoveryOnly,
            false
        ));
    }

    #[test]
    fn background_update_with_invalid_or_changed_node_recovers() {
        assert!(should_refresh_after_update(
            false,
            true,
            true,
            RefreshAfterUpdate::RecoveryOnly,
            true
        ));
        assert!(!should_refresh_after_update(
            false,
            true,
            false,
            RefreshAfterUpdate::RecoveryOnly,
            true
        ));
    }

    #[test]
    fn background_full_refresh_uses_a_fifteen_minute_schedule() {
        let now = std::time::Instant::now();
        assert_eq!(
            BACKGROUND_FULL_REFRESH_INTERVAL,
            std::time::Duration::from_secs(15 * 60)
        );
        assert!(!full_refresh_due(
            now,
            now + BACKGROUND_FULL_REFRESH_INTERVAL - std::time::Duration::from_secs(1)
        ));
        assert!(full_refresh_due(
            now,
            now + BACKGROUND_FULL_REFRESH_INTERVAL
        ));
    }

    #[test]
    fn proxy_port_rejects_privileged_and_zero_values() {
        assert!(validate_proxy_port(1023).is_err());
        assert!(validate_proxy_port(0).is_err());
        assert!(validate_proxy_port(10820).is_err());
        assert!(validate_proxy_port(10821).is_err());
        assert_eq!(validate_proxy_port(10808).unwrap(), 10808);
        assert_eq!(validate_proxy_port(65535).unwrap(), 65535);
    }

    #[test]
    fn update_interval_is_clamped_to_sane_range() {
        assert_eq!(clamp_update_interval(0), 1);
        assert_eq!(clamp_update_interval(5), 5);
        assert_eq!(clamp_update_interval(60), 60);
        assert_eq!(clamp_update_interval(10_000), 24 * 60);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn consecutive_outages_trigger_recovery_with_cooldown() {
        let mut state = ConnectivityState::new();
        let now = std::time::Instant::now();
        assert!(!state.note_failure(now));
        assert!(state.note_failure(now));
        assert!(!state.note_failure(now));
        assert!(state.note_failure(now + RECOVERY_COOLDOWN));
        state.note_success();
        assert!(!state.note_failure(now + RECOVERY_COOLDOWN));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn full_node_checks_use_an_independent_interval() {
        let mut state = ConnectivityState::new();
        let now = std::time::Instant::now();
        assert!(!state.take_full_check_due(now));
        assert!(state.take_full_check_due(now + FULL_DELAY_INTERVAL));
        assert!(!state.take_full_check_due(now + FULL_DELAY_INTERVAL));
    }

    #[test]
    fn subscription_delay_changes_do_not_restart_the_current_node() {
        let node = crate::models::Node {
            name: "node".into(),
            server: "example.com".into(),
            port: 443,
            protocol: "vless".into(),
            uuid: "id".into(),
            host: "example.com".into(),
            path: "/".into(),
            fingerprint: "chrome".into(),
            tls: true,
            insecure: false,
            alpn: Vec::new(),
            delay: None,
        };
        let mut refreshed = node.clone();
        refreshed.delay = Some(250);
        assert!(!super::node_changed_after_refresh(
            &Some(node.clone()),
            Some(refreshed.clone())
        ));
        refreshed.server = "changed.example.com".into();
        assert!(super::node_changed_after_refresh(
            &Some(node.clone()),
            Some(refreshed)
        ));
        assert!(super::node_changed_after_refresh(&Some(node.clone()), None));
        assert!(!super::node_changed_after_refresh(&None, None));
    }
}
