use crate::{
    models::{Node, PublicState},
    subscription::{fetch_many, fetch_nodes},
    system_proxy,
    xray::RuntimeState,
};
use anyhow::{Result, anyhow};
use std::{sync::Arc, time::{Duration, Instant}};
use tokio::sync::Mutex;

pub type Shared = Arc<Mutex<RuntimeState>>;

async fn finish(shared: &Shared, result: Result<()>) -> Result<PublicState, String> {
    let mut state = shared.lock().await;
    state.last_error = result.as_ref().err().map(|error| format!("{error:#}"));
    if let Err(error) = state.save() {
        state.last_error = Some(format!("{error:#}"));
    }
    Ok(state.public())
}

#[tauri::command]
pub async fn get_state(shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    Ok(shared.lock().await.public())
}

#[tauri::command]
pub async fn add_subscription(name: String, url: String, shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    let result = async {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("请输入有效的订阅地址"));
        }
        let nodes = fetch_nodes(&url).await?;
        let mut state = shared.lock().await;
        let id = uuid::Uuid::new_v4().simple().to_string();
        let display_name = if name.trim().is_empty() { "未命名订阅".into() } else { name.trim().into() };
        state.persisted.subscriptions.push(crate::models::Subscription { id: id.clone(), name: display_name, url, nodes });
        if state.persisted.current_subscription.is_none() {
            state.persisted.current_subscription = Some(id);
            state.persisted.current_node = state.persisted.subscriptions.first().and_then(|item| item.nodes.first()).map(|node| node.name.clone());
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn delete_subscription(id: String, shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        state.persisted.subscriptions.retain(|item| item.id != id);
        if state.persisted.current_subscription.as_deref() == Some(id.as_str()) {
            state.persisted.current_subscription = state.persisted.subscriptions.first().map(|item| item.id.clone());
            state.persisted.current_node = state.persisted.subscriptions.first().and_then(|item| item.nodes.first()).map(|node| node.name.clone());
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
pub async fn select_subscription(id: String, shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        let first_node = state.persisted.subscriptions.iter().find(|item| item.id == id).and_then(|item| item.nodes.first()).map(|node| node.name.clone());
        state.persisted.current_subscription = Some(id);
        state.persisted.current_node = first_node;
        if state.persisted.proxy_enabled {
            state.restart()?;
        }
        Ok(())
    }
    .await;
    let output = finish(&shared, result).await;
    if shared.lock().await.persisted.auto_select {
        return refresh_best_node_impl(&shared).await;
    }
    output
}

async fn update_subscription_impl(id: &str, shared: &Shared) -> Result<PublicState, String> {
    let result = async {
        let url = {
            let state = shared.lock().await;
            state.persisted.subscriptions.iter().find(|item| item.id == id).map(|item| item.url.clone()).ok_or_else(|| anyhow!("订阅不存在"))?
        };
        let nodes = fetch_nodes(&url).await?;
        let mut state = shared.lock().await;
        let is_current = state.persisted.current_subscription.as_deref() == Some(id);
        let current_name = state.persisted.current_node.clone();
        let first_node = {
            let subscription = state.persisted.subscriptions.iter_mut().find(|item| item.id == id).ok_or_else(|| anyhow!("订阅不存在"))?;
            subscription.nodes = nodes;
            subscription.nodes.first().map(|node| node.name.clone())
        };
        if is_current {
            if current_name.as_ref().is_some_and(|name| !state.persisted.subscriptions.iter().find(|item| item.id == id).is_some_and(|subscription| subscription.nodes.iter().any(|node| &node.name == name))) {
                state.persisted.current_node = first_node;
            }
            if state.persisted.proxy_enabled {
                state.restart()?;
            }
        }
        Ok(())
    }
    .await;
    let output = finish(&shared, result).await;
    let should_refresh = {
        let state = shared.lock().await;
        state.persisted.auto_select && state.persisted.current_subscription.as_deref() == Some(id)
    };
    if should_refresh {
        return refresh_best_node_impl(&shared).await;
    }
    output
}

#[tauri::command]
pub async fn update_subscription(id: String, shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    update_subscription_impl(&id, &shared).await
}

#[tauri::command]
pub async fn update_all_subscriptions(shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    let result = async {
        let items = {
            let state = shared.lock().await;
            state.persisted.subscriptions.iter().map(|item| (item.id.clone(), item.url.clone())).collect::<Vec<_>>()
        };
        let results = fetch_many(items).await;
        let mut errors = Vec::new();
        {
            let mut state = shared.lock().await;
            for (id, result) in results {
                match result {
                    Ok(nodes) => {
                        if let Some(subscription) = state.persisted.subscriptions.iter_mut().find(|item| item.id == id) {
                            subscription.nodes = nodes;
                        }
                    }
                    Err(error) => errors.push(format!("{id}: {error:#}")),
                }
            }
            if state.persisted.proxy_enabled {
                state.restart()?;
            }
        }
        if !errors.is_empty() {
            return Err(anyhow!(errors.join("；")));
        }
        Ok(())
    }
    .await;
    let output = finish(&shared, result).await;
    if shared.lock().await.persisted.auto_select {
        return refresh_best_node_impl(&shared).await;
    }
    output
}

#[tauri::command]
pub async fn select_node(name: String, shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        if state.persisted.auto_select {
            return Err(anyhow!("自动选择开启时不能手动选择节点"));
        }
        let exists = state.persisted.subscriptions.iter().any(|item| {
            Some(&item.id) == state.persisted.current_subscription.as_ref() && item.nodes.iter().any(|node| node.name == name)
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
pub async fn set_auto_select(enabled: bool, shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
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
pub async fn set_proxy_enabled(enabled: bool, shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    let result = async {
        let mut state = shared.lock().await;
        if enabled {
            state.restart()?;
            system_proxy::enable()?;
        } else {
            state.stop();
            system_proxy::disable()?;
        }
        state.persisted.proxy_enabled = enabled;
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
async fn refresh_best_node_impl(shared: &Shared) -> Result<PublicState, String> {
    let candidates = {
        let state = shared.lock().await;
        let Some(subscription) = state.persisted.subscriptions.iter().find(|item| Some(&item.id) == state.persisted.current_subscription.as_ref()) else {
            return finish(&shared, Err(anyhow!("请先添加并选择订阅"))).await;
        };
        if !state.persisted.auto_select {
            return Ok(state.public());
        }
        subscription.nodes.iter().filter(|node| !crate::models::is_mainland_china_node_name(&node.name)).cloned().collect::<Vec<_>>()
    };
    if candidates.is_empty() {
        return finish(&shared, Err(anyhow!("当前订阅没有可自动选择的境外节点，可关闭自动选择手动连接"))).await;
    }

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all("http://127.0.0.1:10808").expect("local proxy"))
        .timeout(Duration::from_secs(4))
        .build()
        .expect("HTTP client");
    let mut best: Option<(String, u32)> = None;
    for candidate in candidates {
        let delay = measure_node(&shared, &client, candidate).await;
        if let Some(delay) = delay
            && best.as_ref().is_none_or(|(_, best_delay)| delay < *best_delay)
        {
            best = Some((shared.lock().await.persisted.current_node.clone().unwrap_or_default(), delay));
        }
    }
    let result = async {
        let Some((best_name, _)) = best else {
            return Err(anyhow!("所有境外节点测速均失败，保留当前节点"));
        };
        let mut state = shared.lock().await;
        state.persisted.current_node = Some(best_name);
        if state.running || state.persisted.proxy_enabled {
            state.restart()?;
        } else {
            state.stop();
        }
        Ok(())
    }
    .await;
    finish(&shared, result).await
}

#[tauri::command]
pub async fn refresh_best_node(shared: tauri::State<'_, Shared>) -> Result<PublicState, String> {
    refresh_best_node_impl(&shared).await
}

async fn measure_node(shared: &Shared, client: &reqwest::Client, node: Node) -> Option<u32> {
    {
        let mut state = shared.lock().await;
        state.persisted.current_node = Some(node.name.clone());
        if state.restart().is_err() {
            return None;
        }
    }
    let started = Instant::now();
    let result = client.get("http://cp.cloudflare.com/generate_204").send().await;
    let delay = result.ok().filter(|response| response.status().is_success()).map(|_| started.elapsed().as_millis() as u32);
    let mut state = shared.lock().await;
    let current_id = state.persisted.current_subscription.clone();
    if let Some(subscription) = state.persisted.subscriptions.iter_mut().find(|item| Some(&item.id) == current_id.as_ref())
        && let Some(current) = subscription.nodes.iter_mut().find(|item| item.name == node.name)
    {
        current.delay = delay;
    }
    delay
}

pub fn spawn_background(shared: Shared) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15 * 60)).await;
            let current = shared.lock().await.persisted.current_subscription.clone();
            if let Some(id) = current {
                let _ = update_subscription_impl(&id, &shared).await;
            }
        }
    });
}
