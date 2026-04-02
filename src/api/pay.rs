use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use image::Luma;
use qrcode::render::unicode::Dense1x2;
use qrcode::QrCode;

use crate::client::ApiClient;
use crate::config::AppConfig;
use crate::error::AppResult;
use crate::model::{ApiResponse, CreateSignData, CreateSignRequest, PreviewData, PreviewRequest};

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
    println!("[{now}] #{attempt_id} | {status} | bizId: {biz_id:?} | {cost_ms}ms");

    if let Some(id) = biz_id {
        if !sold_out && !success.swap(true, Ordering::SeqCst) {
            return Ok(Some(id.to_string()));
        }
    }

    Ok(None)
}

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
            println!("\n{}", "=".repeat(50));
            render_qrcode_terminal(&sign_url);
            println!("pay url: {sign_url}");
            save_qrcode_image(&sign_url);
            println!("{}", "=".repeat(50));
            return Ok(());
        }
    }

    eprintln!("create-sign failed: code={:?} msg={:?}", resp.code, resp.msg);
    Ok(())
}

fn render_qrcode_terminal(url: &str) {
    let code = match QrCode::new(url.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("qrcode encode error: {e}");
            return;
        }
    };

    let unicode_str = code
        .render::<Dense1x2>()
        .dark_color(Dense1x2::Dark)
        .light_color(Dense1x2::Light)
        .quiet_zone(true)
        .build();
    println!("\n{unicode_str}\n");
}

fn save_qrcode_image(url: &str) {
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
    let filename = format!("pay_{ts}.png");
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
