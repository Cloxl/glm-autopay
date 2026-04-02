use crate::error::{AppError, AppResult};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub auth_token: String,
    pub cookie: String,
    pub customer_id: String,
    pub base_url: String,
    pub user_agent: String,
    pub product_id: String,
    pub pay_type: PayType,
    pub invitation_code: String,
    pub rps: usize,
    pub timeout_secs: u64,
}

#[derive(Clone, Debug)]
pub enum PayType {
    Ali,
    WeChat,
}

impl PayType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PayType::Ali => "ALI",
            PayType::WeChat => "WE_CHAT",
        }
    }

    fn from_str(s: &str) -> AppResult<Self> {
        match s.to_uppercase().as_str() {
            "ALI" | "ALIPAY" => Ok(PayType::Ali),
            "WE_CHAT" | "WECHAT" => Ok(PayType::WeChat),
            _ => Err(AppError::InvalidPayType(s.to_string())),
        }
    }
}

#[derive(Deserialize)]
struct RawConfig {
    auth_token: String,
    cookie: String,
    customer_id: String,
    #[serde(default = "default_base_url")]
    base_url: String,
    #[serde(default = "default_user_agent")]
    user_agent: String,
    #[serde(default = "default_product_id")]
    product_id: String,
    #[serde(default = "default_pay_type")]
    pay_type: String,
    #[serde(default)]
    invitation_code: String,
    #[serde(default = "default_rps")]
    rps: usize,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
}

fn default_base_url() -> String { "https://www.bigmodel.cn/api/biz/pay".into() }
fn default_user_agent() -> String { "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36".into() }
fn default_product_id() -> String { "product-02434c".into() }
fn default_pay_type() -> String { "ALI".into() }
fn default_rps() -> usize { 30 }
fn default_timeout() -> u64 { 5 }

impl AppConfig {
    pub fn load() -> AppResult<Self> {
        let path = "config.json";
        let content = std::fs::read_to_string(path)
            .map_err(|_| AppError::ConfigNotFound(path.into()))?;
        let raw: RawConfig = serde_json::from_str(&content)?;

        Ok(Self {
            auth_token: raw.auth_token,
            cookie: raw.cookie,
            customer_id: raw.customer_id,
            base_url: raw.base_url,
            user_agent: raw.user_agent,
            product_id: raw.product_id,
            pay_type: PayType::from_str(&raw.pay_type)?,
            invitation_code: raw.invitation_code,
            rps: raw.rps,
            timeout_secs: raw.timeout_secs,
        })
    }
}
