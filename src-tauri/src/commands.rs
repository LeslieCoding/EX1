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
    time::Duration,
};
use tauri::Manager;
use tokio::{sync::Mutex, task::JoinSet};

pub type Shared = Arc<Mutex<RuntimeState>>;

const PROBE_CONCURRENCY: usize = 32;
static REFRESH_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

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

fn should_refresh_after_update(failed: bool, quiet: bool, auto_select: bool) -> bool {
    auto_select && (!failed || quiet)
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
            if state.persisted.proxy_enabled {
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
            if state.persisted.proxy_enabled {
                state.restart()?;
            }
        }
        if !errors.is_empty() {
            return Err(anyhow!(errors.join("；")));
        }
        crate::logging::info("全部订阅更新完成");
        Ok(())
    }
    .await;
    let failed = result.is_err();
    let output = finish_with(&shared, result, !quiet).await;
    let auto_select = shared.lock().await.persisted.auto_select;
    if should_refresh_after_update(failed, quiet, auto_select) {
        return refresh_best_node_impl(&shared).await;
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
            if !state.persisted.tun_enabled {
                system_proxy::enable(port)?;
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
        crate::logging::info(&format!("自动更新已{}", if enabled { "开启" } else { "关闭" }));
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
            if auto_update
                && let Ok(info) = crate::update::check().await
            {
                let pending = crate::update::pending_version();
                let should_install = info.available
                    && pending.as_deref() != Some(info.latest.as_str())
                    && last_attempt.as_deref() != Some(info.latest.as_str());
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
                if should_install
                    && let Ok(setup) = crate::update::download(&info.url).await
                    && crate::update::launch_installer(&setup, &info.latest).is_ok()
                {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    crate::xray::quit_app();
                    return;
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
        if changed && (state.running || state.persisted.proxy_enabled) {
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

pub fn spawn_background(shared: Shared) {
    tokio::spawn(async move {
        if shared.lock().await.persisted.auto_select {
            let _ = refresh_best_node_impl(&shared).await;
        }
        loop {
            let minutes = shared
                .lock()
                .await
                .persisted
                .update_interval_min
                .clamp(1, 24 * 60);
            tokio::time::sleep(Duration::from_secs(minutes * 60)).await;
            let _ = update_all_subscriptions_impl(&shared, true).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{clamp_update_interval, should_refresh_after_update, validate_proxy_port};

    #[test]
    fn background_partial_failure_still_refreshes_current_subscription() {
        assert!(should_refresh_after_update(true, true, true));
        assert!(!should_refresh_after_update(true, false, true));
        assert!(should_refresh_after_update(false, false, true));
        assert!(!should_refresh_after_update(false, true, false));
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
}
