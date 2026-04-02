use crate::client::ApiClient;
use crate::config::{AppConfig, GlobalConfig, PayType, UserConfig};
use crate::error::{AppError, AppResult};
use crate::model::{ApiResponse, BatchPreviewData, ProductInfo};

use std::io::{self, Write};

/// 篡改猴脚本导出的 JSON 结构
#[derive(serde::Deserialize)]
struct TampermonkeyExport {
    headers: TmHeaders,
    cookies: String,
}

#[derive(serde::Deserialize)]
struct TmHeaders {
    authorization: String,
    #[serde(rename = "bigmodel-organization")]
    bigmodel_organization: Option<String>,
    #[serde(rename = "bigmodel-project")]
    bigmodel_project: Option<String>,
}

/// 从 JWT 的 payload 中解析 customer_id 和 username
fn parse_jwt_payload(token: &str) -> AppResult<(String, String)> {
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return Err(AppError::Setup("invalid JWT token".into()));
    }
    let payload = parts[1];
    let padded = match payload.len() % 4 {
        2 => format!("{payload}=="),
        3 => format!("{payload}="),
        _ => payload.to_string(),
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(padded.replace('-', "+").replace('_', "/"))
        .map_err(|e| AppError::Setup(format!("base64 decode JWT: {e}")))?;
    let json: serde_json::Value =
        serde_json::from_slice(&decoded).map_err(|e| AppError::Setup(format!("parse JWT: {e}")))?;
    let customer_id = json["customer_id"]
        .as_str()
        .ok_or_else(|| AppError::Setup("customer_id not found in JWT".into()))?
        .to_string();
    let username = json["username"].as_str().unwrap_or("unknown").to_string();
    Ok((customer_id, username))
}

/// 读一行用户输入
fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    buf.trim().to_string()
}

/// 根据价格和周期推断中文套餐名
fn guess_product_name(p: &ProductInfo) -> &'static str {
    let monthly = p.monthly_pay_amount.unwrap_or(0.0);
    let total = p.pay_amount.unwrap_or(0.0);

    // 判断周期：月付 total≈monthly, 季付 total≈3*monthly, 年付 total≈12*monthly
    let months = if monthly > 0.0 {
        (total / monthly).round() as i32
    } else {
        1
    };

    let tier = if monthly < 60.0 {
        "Pro"
    } else if monthly < 300.0 {
        "Pro+"
    } else {
        "Team"
    };

    match (tier, months) {
        ("Pro", 1) => "Pro 月付",
        ("Pro", 3) => "Pro 连续包季",
        ("Pro", 12) => "Pro 连续包年",
        ("Pro+", 1) => "Pro+ 月付",
        ("Pro+", 3) => "Pro+ 连续包季",
        ("Pro+", 12) => "Pro+ 连续包年",
        ("Team", 1) => "Team 月付",
        ("Team", 3) => "Team 连续包季",
        ("Team", 12) => "Team 连续包年",
        _ => "未知套餐",
    }
}

/// 拉取并展示套餐列表，返回用户选中的 (product_id, pay_type)
async fn select_product_and_pay(user: &UserConfig) -> AppResult<(String, String)> {
    println!("\n  为 {} 获取套餐列表...", user.name);
    let tmp_config = AppConfig {
        auth_token: user.auth_token.clone(),
        cookie: user.cookie.clone(),
        customer_id: user.customer_id.clone(),
        bigmodel_organization: user.bigmodel_organization.clone(),
        bigmodel_project: user.bigmodel_project.clone(),
        base_url: "https://www.bigmodel.cn/api/biz/pay".into(),
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36".into(),
        product_id: String::new(),
        pay_type: PayType::Ali,
        invitation_code: String::new(),
        rps: 1,
        timeout_secs: 5,
        name: user.name.clone(),
    };
    let client = ApiClient::new(tmp_config)?;

    let url = "https://www.bigmodel.cn/api/biz/pay/batch-preview";
    let body = serde_json::json!({"invitationCode": ""});
    let resp: ApiResponse<BatchPreviewData> = client.post_json(url, &body).await?;

    let products: Vec<ProductInfo> = resp
        .data
        .and_then(|d| d.product_list)
        .unwrap_or_default();

    if products.is_empty() {
        return Err(AppError::Setup("no products returned".into()));
    }

    println!("\n  {:<4} {:<16} {:<18} {:>8} {:>8} {}", "#", "套餐", "Product ID", "总价", "月均", "状态");
    println!("  {}", "-".repeat(76));
    for (i, p) in products.iter().enumerate() {
        let status = if p.sold_out.unwrap_or(false) {
            "已售罄"
        } else {
            "可购买"
        };
        let name = guess_product_name(p);
        println!(
            "  {:<4} {:<16} {:<18} {:>8.2} {:>8.2} {}",
            i + 1,
            name,
            p.product_id,
            p.pay_amount.unwrap_or(0.0),
            p.monthly_pay_amount.unwrap_or(0.0),
            status,
        );
    }

    let choice: usize = loop {
        let input = read_line(&format!("\n  为 {} 选择套餐编号: ", user.name));
        match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= products.len() => break n,
            _ => println!("  无效输入, 请输入 1-{}", products.len()),
        }
    };
    let selected = &products[choice - 1];
    let name = guess_product_name(selected);
    println!(
        "  -> {} 选择了: {} ({}) ￥{}",
        user.name,
        name,
        selected.product_id,
        selected.pay_amount.unwrap_or(0.0)
    );

    // 选择支付方式
    let pay_type = loop {
        let input = read_line(&format!("  {} 支付方式 (1=支付宝, 2=微信) [1]: ", user.name));
        match input.as_str() {
            "" | "1" => break "ALI".to_string(),
            "2" => break "WE_CHAT".to_string(),
            _ => println!("  无效输入, 请输入 1 或 2"),
        }
    };

    Ok((selected.product_id.clone(), pay_type))
}

/// 交互引导：收集所有用户，各自选择套餐，写入 config.json
pub async fn run_setup() -> AppResult<()> {
    println!("========== GLM AutoPay 配置向导 ==========\n");

    // --- 收集用户 ---
    let mut users: Vec<UserConfig> = Vec::new();
    loop {
        println!(
            "--- 用户 #{} (粘贴篡改猴 JSON, 输入 q 完成添加) ---",
            users.len() + 1
        );
        let raw = read_line("> ");

        if raw.eq_ignore_ascii_case("q") {
            if users.is_empty() {
                println!("至少需要一个用户!");
                continue;
            }
            break;
        }

        if raw.is_empty() {
            if users.is_empty() {
                continue;
            }
            break;
        }

        let export: TampermonkeyExport = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("解析失败: {e}  (请确保粘贴完整的 JSON)");
                continue;
            }
        };

        let (customer_id, username) = parse_jwt_payload(&export.headers.authorization)?;
        let org = export
            .headers
            .bigmodel_organization
            .unwrap_or_else(|| "org-583867d3f6994298908000F995870b6a".into());
        let project = export
            .headers
            .bigmodel_project
            .unwrap_or_else(|| "proj_2fFEA9d9c35B4408A1C72206C32a0354".into());

        println!("  -> 用户: {username} | 客户ID: {customer_id}");

        // 立即为该用户选择套餐和支付方式
        let mut user_cfg = UserConfig {
            name: username,
            auth_token: export.headers.authorization,
            cookie: export.cookies,
            customer_id,
            product_id: String::new(),
            pay_type: String::new(),
            bigmodel_organization: org,
            bigmodel_project: project,
        };

        let (product_id, pay_type) = select_product_and_pay(&user_cfg).await?;
        user_cfg.product_id = product_id;
        user_cfg.pay_type = pay_type;

        users.push(user_cfg);
        println!("\n  已添加! (共 {} 个用户)\n", users.len());
    }

    // --- RPS ---
    let rps_input = read_line("\n每秒请求数 (rps) [30]: ");
    let rps: usize = rps_input.parse().unwrap_or(30);

    // --- 写入 config.json ---
    let config = GlobalConfig {
        rps,
        timeout_secs: 5,
        users,
    };

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::Setup(format!("serialize config: {e}")))?;
    std::fs::write("config.json", &json)
        .map_err(|e| AppError::Setup(format!("write config.json: {e}")))?;

    println!("\n配置已保存到 config.json ({} 个用户)", config.users.len());
    for u in &config.users {
        let pay = if u.pay_type == "WE_CHAT" { "微信" } else { "支付宝" };
        println!("  {} -> {} ({})", u.name, u.product_id, pay);
    }
    println!("RPS: {}", config.rps);
    println!("\n配置完成! 直接运行即可开始抢购。");

    Ok(())
}
