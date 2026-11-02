use crate::{models::Node, system_proxy::current_proxy_url};
use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value as JsonValue;
use serde_yaml_ng::Value;
use std::time::Duration;
use tokio::task::JoinSet;
use url::Url;

pub async fn fetch_nodes(url: &str) -> Result<Vec<Node>> {
    let mut builder = reqwest::Client::builder()
        .user_agent("mihomo/v1.19.33")
        .timeout(Duration::from_secs(45));
    if let Some(proxy_url) = current_proxy_url() {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
    }
    let text = builder
        .build()?
        .get(url)
        .send()
        .await
        .context("下载订阅失败")?
        .error_for_status()
        .context("订阅服务器返回错误")?
        .text()
        .await?;
    parse_subscription(&text)
}

pub fn parse_subscription(text: &str) -> Result<Vec<Node>> {
    if let Ok(config) = serde_yaml_ng::from_str::<Value>(text)
        && let Some(proxies) = config.get("proxies").and_then(Value::as_sequence)
    {
        let nodes = proxies
            .iter()
            .filter_map(parse_yaml_node)
            .collect::<Vec<_>>();
        if !nodes.is_empty() {
            return Ok(nodes);
        }
    }

    let decoded = STANDARD
        .decode(text.trim())
        .unwrap_or_else(|_| text.trim().as_bytes().to_vec());
    let text = String::from_utf8(decoded).context("订阅不是有效的 Base64 或 YAML")?;
    let nodes = text
        .lines()
        .filter_map(|line| parse_proxy_link(line.trim()).ok())
        .collect::<Vec<_>>();
    if nodes.is_empty() {
        return Err(anyhow!("订阅中没有支持的 VLESS / VMess / Hysteria2 节点"));
    }
    Ok(nodes)
}

fn parse_yaml_node(value: &Value) -> Option<Node> {
    let map = value.as_mapping()?;
    let protocol = map.get("type")?.as_str()?.to_lowercase();
    if !matches!(protocol.as_str(), "vless" | "vmess" | "hysteria2") {
        return None;
    }
    let ws = map.get("ws-opts").and_then(Value::as_mapping);
    let host = map
        .get("servername")
        .or_else(|| map.get("sni"))
        .and_then(Value::as_str)
        .or_else(|| {
            ws.and_then(|opts| opts.get("headers"))
                .and_then(Value::as_mapping)
                .and_then(|headers| headers.get("Host"))
                .and_then(Value::as_str)
        })
        .unwrap_or_default()
        .to_owned();
    let uuid = ["uuid", "password", "auth"]
        .iter()
        .find_map(|key| map.get(*key).and_then(Value::as_str))?
        .to_owned();
    let tls = if protocol == "vmess" {
        map.get("tls").and_then(Value::as_bool).unwrap_or(false)
    } else {
        true
    };
    let insecure = map
        .get("skip-cert-verify")
        .or_else(|| map.get("insecure"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(Node {
        name: map.get("name")?.as_str()?.to_owned(),
        server: map.get("server")?.as_str()?.to_owned(),
        port: map.get("port")?.as_u64()? as u16,
        protocol,
        uuid,
        host,
        path: ws
            .and_then(|opts| opts.get("path"))
            .and_then(Value::as_str)
            .unwrap_or("/")
            .to_owned(),
        fingerprint: map
            .get("client-fingerprint")
            .and_then(Value::as_str)
            .unwrap_or("chrome")
            .to_owned(),
        tls,
        insecure,
        delay: None,
    })
}

fn parse_proxy_link(line: &str) -> Result<Node> {
    let url = Url::parse(line)?;
    match url.scheme() {
        "vless" | "hysteria2" => {
            let query = url
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            let host = query
                .get("sni")
                .or_else(|| query.get("host"))
                .map(|value| value.to_string())
                .unwrap_or_default();
            let insecure = query
                .get("insecure")
                .map(|value| value == "1" || value == "true")
                .unwrap_or(false);
            Ok(Node {
                name: url
                    .fragment()
                    .map(|fragment| {
                        url::form_urlencoded::parse(fragment.as_bytes())
                            .next()
                            .map(|(name, _)| name.into_owned())
                            .unwrap_or_else(|| fragment.to_owned())
                    })
                    .unwrap_or_else(|| host.clone()),
                server: url.host_str().context("节点缺少地址")?.to_owned(),
                port: url.port().unwrap_or(443),
                protocol: url.scheme().into(),
                uuid: url.password().unwrap_or(url.username()).to_owned(),
                host,
                path: query
                    .get("path")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "/".into()),
                fingerprint: query
                    .get("fp")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "chrome".into()),
                tls: true,
                insecure,
                delay: None,
            })
        }
        "vmess" => parse_vmess_link(line),
        _ => Err(anyhow!("unsupported protocol")),
    }
}

fn parse_vmess_link(line: &str) -> Result<Node> {
    let encoded = line.strip_prefix("vmess://").context("不是 VMess 链接")?;
    let decoded = STANDARD.decode(encoded).context("VMess 链接 Base64 无效")?;
    let value: JsonValue = serde_json::from_slice(&decoded).context("VMess 链接 JSON 无效")?;
    let string = |key: &str| {
        value
            .get(key)
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let port = value
        .get("port")
        .and_then(JsonValue::as_u64)
        .or_else(|| string("port").parse::<u64>().ok())
        .unwrap_or(443) as u16;
    let tls = matches!(string("tls").as_str(), "tls" | "true" | "1");
    let host = match string("host") {
        value if !value.is_empty() => value,
        _ => string("sni"),
    };
    Ok(Node {
        name: string("ps"),
        server: string("add"),
        port,
        protocol: "vmess".into(),
        uuid: string("id"),
        host,
        path: match string("path") {
            value if !value.is_empty() => value,
            _ => "/".into(),
        },
        fingerprint: "chrome".into(),
        tls,
        insecure: false,
        delay: None,
    })
}

pub async fn fetch_many(items: Vec<(String, String)>) -> Vec<(String, Result<Vec<Node>>)> {
    let mut tasks = JoinSet::new();
    for (id, url) in items {
        tasks.spawn(async move { (id.clone(), fetch_nodes(&url).await) });
    }
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Ok(item) = result {
            results.push(item);
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::parse_subscription;

    #[test]
    fn parses_vless_yaml() {
        let yaml = r#"
proxies:
  - name: "CF官方优选8"
    server: 104.18.38.26
    port: 2087
    type: vless
    uuid: 34c48ec3-488e-46ce-8295-904a4ed9f037
    tls: true
    servername: example.com
    client-fingerprint: chrome
    network: ws
    ws-opts:
      path: /
      headers:
        Host: example.com
"#;
        let nodes = parse_subscription(yaml).expect("yaml should parse");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].port, 2087);
        assert_eq!(nodes[0].host, "example.com");
        assert_eq!(nodes[0].protocol, "vless");
    }

    #[test]
    fn parses_vless_link() {
        let link = "vless://34c48ec3-488e-46ce-8295-904a4ed9f037@example.com:2087?security=tls&type=ws&host=example.com&fp=chrome&sni=example.com&path=%2F&encryption=none#edgetunnel";
        let nodes = parse_subscription(link).expect("link should parse");
        assert_eq!(nodes[0].name, "edgetunnel");
        assert_eq!(nodes[0].path, "/");
    }

    #[test]
    fn parses_hysteria2_link() {
        let nodes = parse_subscription(
            "hysteria2://password@example.com:18385?insecure=1&sni=edge.example.com#%E6%96%B0%E5%8A%A0%E5%9D%A1-1",
        )
        .expect("hysteria2 should parse");
        assert_eq!(nodes[0].name, "新加坡-1");
        assert_eq!(nodes[0].protocol, "hysteria2");
        assert_eq!(nodes[0].uuid, "password");
        assert_eq!(nodes[0].host, "edge.example.com");
        assert!(nodes[0].insecure);
    }

    #[test]
    fn parses_vmess_link_without_tls() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let json = r#"{"ps":"HK VMess","add":"example.com","port":"12901","id":"test-uuid","host":"cdn.example.com","path":"/","tls":""}"#;
        let link = format!("vmess://{}", STANDARD.encode(json));
        let nodes = parse_subscription(&link).expect("vmess should parse");
        assert_eq!(nodes[0].protocol, "vmess");
        assert_eq!(nodes[0].port, 12901);
        assert_eq!(nodes[0].host, "cdn.example.com");
        assert!(!nodes[0].tls);
    }
}
