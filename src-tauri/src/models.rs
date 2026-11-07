use serde::{Deserialize, Serialize};

fn default_tls() -> bool {
    true
}

pub fn default_proxy_port() -> u16 {
    10808
}

pub fn default_update_interval() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: String,
    pub uuid: String,
    pub host: String,
    pub path: String,
    pub fingerprint: String,
    #[serde(default = "default_tls")]
    pub tls: bool,
    #[serde(default)]
    pub insecure: bool,
    #[serde(default)]
    pub alpn: Vec<String>,
    #[serde(default)]
    pub delay: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistedState {
    pub subscriptions: Vec<Subscription>,
    pub current_subscription: Option<String>,
    pub current_node: Option<String>,
    pub auto_select: bool,
    pub proxy_enabled: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub silent_start: bool,
    #[serde(default)]
    pub tun_enabled: bool,
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_update_interval")]
    pub update_interval_min: u64,
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default)]
    pub last_auto_update: Option<String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            subscriptions: Vec::new(),
            current_subscription: None,
            current_node: None,
            auto_select: true,
            proxy_enabled: false,
            auto_start: false,
            silent_start: false,
            tun_enabled: false,
            proxy_port: default_proxy_port(),
            update_interval_min: default_update_interval(),
            auto_update: true,
            last_auto_update: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicState {
    pub subscriptions: Vec<Subscription>,
    pub current_subscription: Option<String>,
    pub current_node: Option<String>,
    pub auto_select: bool,
    pub proxy_enabled: bool,
    pub auto_start: bool,
    pub silent_start: bool,
    pub tun_enabled: bool,
    pub proxy_port: u16,
    pub update_interval_min: u64,
    pub auto_update: bool,
    pub update: Option<crate::update::UpdateInfo>,
    pub running: bool,
    pub last_error: Option<String>,
}

impl PublicState {
    pub fn from_parts(
        persisted: PersistedState,
        running: bool,
        last_error: Option<String>,
        update: Option<crate::update::UpdateInfo>,
    ) -> Self {
        Self {
            subscriptions: persisted.subscriptions,
            current_subscription: persisted.current_subscription,
            current_node: persisted.current_node,
            auto_select: persisted.auto_select,
            proxy_enabled: persisted.proxy_enabled,
            auto_start: persisted.auto_start,
            silent_start: persisted.silent_start,
            tun_enabled: persisted.tun_enabled,
            proxy_port: persisted.proxy_port,
            update_interval_min: persisted.update_interval_min,
            auto_update: persisted.auto_update,
            update,
            running,
            last_error,
        }
    }
}

pub fn is_subscription_info_node_name(name: &str) -> bool {
    ["剩余流量", "距离下次重置剩余", "套餐到期"]
        .iter()
        .any(|keyword| name.contains(keyword))
}

pub fn is_auto_selectable_node(node: &Node) -> bool {
    !is_mainland_china_node_name(&node.name) && !is_subscription_info_node_name(&node.name)
}

pub fn default_node_name(subscription: &Subscription) -> Option<String> {
    subscription
        .nodes
        .iter()
        .find(|node| is_auto_selectable_node(node))
        .map(|node| node.name.clone())
}

pub fn is_mainland_china_node_name(name: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "中国",
        "国内",
        "大陆",
        "电信",
        "联通",
        "移动",
        "广电",
        "铁通",
        "北京",
        "上海",
        "广州",
        "深圳",
        "杭州",
        "南京",
        "苏州",
        "成都",
        "重庆",
        "武汉",
        "西安",
        "天津",
        "郑州",
        "长沙",
        "合肥",
        "济南",
        "青岛",
        "福州",
        "厦门",
        "宁波",
        "无锡",
        "东莞",
        "佛山",
        "珠海",
        "沈阳",
        "大连",
        "哈尔滨",
        "长春",
        "石家庄",
        "太原",
        "南昌",
        "昆明",
        "贵阳",
        "南宁",
        "兰州",
        "银川",
        "西宁",
        "乌鲁木齐",
        "拉萨",
        "海口",
    ];
    let normalized = name.to_lowercase();
    let cloudflare_optimized = (normalized.starts_with("cf") || normalized.contains("cloudflare"))
        && normalized.contains("优选");
    if cloudflare_optimized {
        return false;
    }
    let location_name = normalized.replace("国内官网", "").replace("中国官网", "");
    location_name.contains("china")
        || location_name.contains("mainland")
        || location_name.contains("🇨🇳")
        || location_name
            .split(|character: char| !character.is_alphanumeric())
            .any(|token| token == "cn")
        || KEYWORDS
            .iter()
            .any(|keyword| location_name.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::{Node, Subscription, default_node_name, is_mainland_china_node_name};

    fn node(name: &str) -> Node {
        Node {
            name: name.into(),
            server: "example.com".into(),
            port: 443,
            protocol: "vless".into(),
            uuid: "test".into(),
            host: "example.com".into(),
            path: "/".into(),
            fingerprint: "chrome".into(),
            tls: true,
            insecure: false,
            alpn: Vec::new(),
            delay: None,
        }
    }

    #[test]
    fn default_node_skips_information_and_mainland_entries() {
        let subscription = Subscription {
            id: "test".into(),
            name: "test".into(),
            url: "https://example.invalid".into(),
            nodes: vec![
                node("剩余流量：42.86 GB"),
                node("距离下次重置剩余：11 天"),
                node("套餐到期：2026-11-15"),
                node("上海 01"),
                node("🇭🇰 HK香港-1-国内官网：example.com"),
            ],
        };
        assert_eq!(
            default_node_name(&subscription).as_deref(),
            Some("🇭🇰 HK香港-1-国内官网：example.com")
        );
    }

    #[test]
    fn mainland_filter_preserves_other_regions_and_cn2() {
        assert!(is_mainland_china_node_name("上海 01"));
        assert!(is_mainland_china_node_name("电信上海"));
        assert!(is_mainland_china_node_name("CF上海节点"));
        assert!(!is_mainland_china_node_name("CF电信优选8"));
        assert!(!is_mainland_china_node_name("CF官方优选2"));
        assert!(!is_mainland_china_node_name("香港 01"));
        assert!(!is_mainland_china_node_name(
            "🇭🇰 HK香港-1-国内官网：ktm01.com"
        ));
        assert!(!is_mainland_china_node_name(
            "🇸🇬 SG新加坡-01-国内官网：ktm.ooo"
        ));
        assert!(!is_mainland_china_node_name("美国 CN2 GIA"));
    }
}
