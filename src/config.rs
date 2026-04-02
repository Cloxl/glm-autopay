use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

// --- 运行时配置（每个用户一份）---

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub name: String,
    pub auth_token: String,
    pub cookie: String,
    pub customer_id: String,
    pub bigmodel_organization: String,
    pub bigmodel_project: String,
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

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s.to_uppercase().as_str() {
            "ALI" | "ALIPAY" => Ok(PayType::Ali),
            "WE_CHAT" | "WECHAT" => Ok(PayType::WeChat),
            _ => Err(AppError::InvalidPayType(s.to_string())),
        }
    }
}

// --- config.json 磁盘格式 ---

#[derive(Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_rps")]
    pub rps: usize,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    pub users: Vec<UserConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserConfig {
    pub name: String,
    pub auth_token: String,
    pub cookie: String,
    pub customer_id: String,
    pub product_id: String,
    pub pay_type: String,
    #[serde(default = "default_org")]
    pub bigmodel_organization: String,
    #[serde(default = "default_project")]
    pub bigmodel_project: String,
}

fn default_rps() -> usize { 30 }
fn default_timeout() -> u64 { 5 }
fn default_org() -> String { "org-583867d3f6994298908000F995870b6a".into() }
fn default_project() -> String { "proj_2fFEA9d9c35B4408A1C72206C32a0354".into() }

impl GlobalConfig {
    pub fn load() -> AppResult<Self> {
        let path = "config.json";
        let content = std::fs::read_to_string(path)
            .map_err(|_| AppError::ConfigNotFound(path.into()))?;
        let config: GlobalConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 为每个用户生成独立的 AppConfig
    pub fn into_app_configs(self) -> AppResult<Vec<AppConfig>> {
        let mut configs = Vec::with_capacity(self.users.len());
        for u in self.users {
            let pay_type = PayType::from_str(&u.pay_type)?;
            configs.push(AppConfig {
                name: u.name,
                auth_token: u.auth_token,
                cookie: u.cookie,
                customer_id: u.customer_id,
                bigmodel_organization: u.bigmodel_organization,
                bigmodel_project: u.bigmodel_project,
                base_url: "https://www.bigmodel.cn/api/biz/pay".into(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36".into(),
                product_id: u.product_id,
                pay_type,
                invitation_code: String::new(),
                rps: self.rps,
                timeout_secs: self.timeout_secs,
            });
        }
        Ok(configs)
    }
}
