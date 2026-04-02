use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use aes::Aes128;
use base64::Engine;
use cipher::{BlockEncryptMut, KeyInit};
use ecb::Encryptor;
use image::Luma;
use qrcode::QrCode;

use crate::client::ApiClient;
use crate::config::AppConfig;
use crate::error::AppResult;
use crate::model::{
    ApiResponse, CreateSignData, CreateSignRequest, PayMiddleInfo, PreviewData, PreviewRequest,
};

pub async fn poll_preview(
    client: &ApiClient,
    config: &AppConfig,
    attempt_id: usize,
    success: &Arc<AtomicBool>,
) -> AppResult<Option<String>> {
    if success.load(Ordering::Relaxed) {
        return Ok(None);
    }

    let refer = generate_refer_param();
    let url = format!("{}/preview?refer__1090={}", config.base_url, refer);
    let payload = PreviewRequest {
        product_id: &config.product_id,
        invitation_code: &config.invitation_code,
    };

    let start = Instant::now();
    let resp: ApiResponse<PreviewData> = client.post_json(&url, &payload).await?;
    let cost_ms = start.elapsed().as_millis();

    let data = resp.data.as_ref();
    let biz_id = data.and_then(|d| d.biz_id.as_deref());
    let sold_out = data.and_then(|d| d.sold_out).unwrap_or(true);

    let now = chrono::Local::now().format("%H:%M:%S%.3f");
    let status = if sold_out { "sold out" } else { "in stock" };
    println!("[{now}] {} #{attempt_id} | {status} | bizId: {biz_id:?} | {cost_ms}ms", config.name);

    if let Some(id) = biz_id {
        if !sold_out && !success.swap(true, Ordering::SeqCst) {
            return Ok(Some(id.to_string()));
        }
    }

    Ok(None)
}

#[allow(dead_code)]
pub async fn create_sign(client: &ApiClient, config: &AppConfig, biz_id: &str) -> AppResult<()> {
    let url = format!("{}/create-sign", config.base_url);
    let payload = CreateSignRequest {
        pay_type: config.pay_type.as_str(),
        product_id: &config.product_id,
        customer_id: &config.customer_id,
        biz_id,
        invitation_code: &config.invitation_code,
    };

    let resp: ApiResponse<CreateSignData> = client.post_json(&url, &payload).await?;

    if resp.code == Some(200) {
        if let Some(sign_url) = resp.data.and_then(|d| d.sign) {
            println!("pay url: {sign_url}");
            save_qrcode_image(&sign_url, &config.name);
            return Ok(());
        }
    }

    eprintln!("create-sign failed: code={:?} msg={:?}", resp.code, resp.msg);
    Ok(())
}

/// 生成 pay-middle-page URL 并保存二维码图片
pub fn pay_middle(config: &AppConfig, biz_id: &str) {
    let pay_type = match config.pay_type.as_str() {
        "ALI" => "alipay",
        "WE_CHAT" => "wechat",
        other => other,
    };

    let info = PayMiddleInfo {
        product_id: config.product_id.clone(),
        product_name: String::new(),
        amount: String::new(),
        customer_id: config.customer_id.clone(),
        customer_name: String::new(),
        old_product_id: String::new(),
        agreement_no: String::new(),
        is_subscribe: false,
        biz_id: biz_id.to_string(),
        pay_type: pay_type.to_string(),
        user_state: "NORMAL".to_string(),
        ic: config.invitation_code.clone(),
    };

    let json = serde_json::to_string(&info).expect("serialize PayMiddleInfo");
    let encrypted = aes_ecb_encrypt(&json);
    let encoded = urlencoding::encode(&encrypted);
    let url = format!("https://www.bigmodel.cn/pay-middle-page?info={encoded}");

    println!("\n{}", "=".repeat(50));
    println!("[{}] pay middle page ready!", config.name);
    save_qrcode_image(&url, &config.name);
    println!("{}", "=".repeat(50));
}

/// AES-128-ECB + PKCS7 加密，输出 Base64
fn aes_ecb_encrypt(plaintext: &str) -> String {
    const KEY: &[u8; 16] = b"zhiPuAi123456789";
    let mut data = plaintext.as_bytes().to_vec();

    // PKCS7 padding
    let pad_len = 16 - (data.len() % 16);
    data.extend(std::iter::repeat(pad_len as u8).take(pad_len));

    let enc = Encryptor::<Aes128>::new(KEY.into());
    for chunk in data.chunks_exact_mut(16) {
        enc.clone().encrypt_block_mut(chunk.into());
    }

    base64::engine::general_purpose::STANDARD.encode(&data)
}

fn save_qrcode_image(url: &str, username: &str) {
    let code = match QrCode::new(url.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("qrcode encode error: {e}");
            return;
        }
    };

    let img = code.render::<Luma<u8>>().min_dimensions(300, 300).build();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("pay_{username}_{ts}.png");
    match img.save(&filename) {
        Ok(()) => {
            let abs = std::path::Path::new(&filename)
                .canonicalize()
                .map(|p| p.display().to_string())
                .unwrap_or(filename);
            println!("qrcode saved: {abs}");
        }
        Err(e) => eprintln!("save qrcode error: {e}"),
    }
}

fn generate_refer_param() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let random_part: u64 = rand::random();
    let input = format!("{ts}{random_part:x}");
    let digest = md5::compute(input.as_bytes());
    format!("{digest:x}")[..30].to_string()
}
