use serde::{Deserialize, Serialize};

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
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            subscriptions: Vec::new(),
            current_subscription: None,
            current_node: None,
            auto_select: true,
            proxy_enabled: false,
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
    pub running: bool,
    pub last_error: Option<String>,
}

impl PublicState {
    pub fn from_parts(persisted: PersistedState, running: bool, last_error: Option<String>) -> Self {
        Self {
            subscriptions: persisted.subscriptions,
            current_subscription: persisted.current_subscription,
            current_node: persisted.current_node,
            auto_select: persisted.auto_select,
            proxy_enabled: persisted.proxy_enabled,
            running,
            last_error,
        }
    }
}

pub fn is_mainland_china_node_name(name: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "中国", "国内", "大陆", "电信", "联通", "移动", "广电", "铁通", "北京", "上海", "广州", "深圳",
        "杭州", "南京", "苏州", "成都", "重庆", "武汉", "西安", "天津", "郑州", "长沙", "合肥", "济南",
        "青岛", "福州", "厦门", "宁波", "无锡", "东莞", "佛山", "珠海", "沈阳", "大连", "哈尔滨", "长春",
        "石家庄", "太原", "南昌", "昆明", "贵阳", "南宁", "兰州", "银川", "西宁", "乌鲁木齐", "拉萨", "海口",
    ];
    let normalized = name.to_lowercase();
    normalized.contains("china")
        || normalized.contains("mainland")
        || normalized.contains("🇨🇳")
        || normalized.split(|character: char| !character.is_alphanumeric()).any(|token| token == "cn")
        || KEYWORDS.iter().any(|keyword| normalized.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::is_mainland_china_node_name;

    #[test]
    fn mainland_filter_preserves_other_regions_and_cn2() {
        assert!(is_mainland_china_node_name("CF电信优选8"));
        assert!(is_mainland_china_node_name("上海 01"));
        assert!(!is_mainland_china_node_name("香港 01"));
        assert!(!is_mainland_china_node_name("美国 CN2 GIA"));
    }
}
