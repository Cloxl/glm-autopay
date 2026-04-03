mod api;
mod client;
mod config;
mod error;
mod model;
mod setup;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use client::ApiClient;
use config::GlobalConfig;
use error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    init_console();

    // --setup: 交互式引导
    if std::env::args().any(|a| a == "--setup") {
        return setup::run_setup().await;
    }

    let global = GlobalConfig::load()?;
    let configs = global.into_app_configs()?;

    // --test: 每个用户生成一张测试二维码
    if std::env::args().any(|a| a == "--test") {
        println!("=== TEST MODE ===\n");
        for cfg in &configs {
            println!("generating test qrcode for: {}", cfg.name);
            let test_preview = model::PreviewData {
                product_id: Some(cfg.product_id.clone()),
                sold_out: Some(false),
                biz_id: Some("TEST_BIZ_ID_000000".to_string()),
                delay: None,
                effective_time: None,
                original_amount: None,
                pay_amount: Some(49.0),
                cash_amount: None,
                give_amount: None,
                third_party_amount: Some(49.0),
                refund_amount: None,
                residual_amount: None,
                renew_amount: None,
                has_first_time_subscription_promo: None,
                renew: Some(false),
                order_value_composition_feature_enabled: None,
                product_big_title: None,
                product_small_title: None,
                product_name: Some("Pro".to_string()),
                product_introduction: None,
                campaign_discount_details: None,
                refund_breakdown: None,
                last_subscription_summary: None,
            };
            api::pay::pay_middle(cfg, &test_preview);
            println!();
        }
        return Ok(());
    }

    // 每个用户独立并发轮询
    println!("rps: {}/user", configs[0].rps);
    for cfg in &configs {
        println!("  {} -> {} ({})", cfg.name, cfg.product_id, cfg.pay_type.as_str());
    }
    println!();

    let mut user_handles = Vec::new();
    for cfg in configs {
        user_handles.push(tokio::spawn(run_user(cfg)));
    }

    for h in user_handles {
        if let Err(e) = h.await.unwrap() {
            eprintln!("user task error: {e}");
        }
    }

    Ok(())
}

async fn run_user(config: config::AppConfig) -> AppResult<()> {
    let client = ApiClient::new(config.clone())?;
    let success = Arc::new(AtomicBool::new(false));
    let mut attempt: usize = 0;

    while !success.load(Ordering::Relaxed) {
        let loop_start = Instant::now();
        let mut handles = Vec::with_capacity(config.rps);

        for _ in 0..config.rps {
            attempt += 1;
            let client = client.clone();
            let config = config.clone();
            let success = success.clone();
            let id = attempt;

            handles.push(tokio::spawn(async move {
                match api::pay::poll_preview(&client, &config, id, &success).await {
                    Ok(Some(preview)) => {
                        if std::env::args().any(|a| a == "--mid") {
                            api::pay::pay_middle(&config, &preview);
                        } else {
                            let biz_id = preview.biz_id.as_deref().unwrap_or("");
                            if let Err(e) = api::pay::create_sign(&client, &config, biz_id).await {
                                eprintln!("[{}] create-sign error: {e}", config.name);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}] preview error: {e}", config.name);
                    }
                    _ => {}
                }
            }));
        }

        for h in handles {
            let _ = h.await;
        }

        let elapsed = loop_start.elapsed();
        let one_sec = std::time::Duration::from_secs(1);
        if elapsed < one_sec && !success.load(Ordering::Relaxed) {
            tokio::time::sleep(one_sec - elapsed).await;
        }
    }

    Ok(())
}

#[cfg(windows)]
fn init_console() {
    unsafe {
        windows_sys::Win32::System::Console::SetConsoleOutputCP(65001);
        let handle = windows_sys::Win32::System::Console::GetStdHandle(
            windows_sys::Win32::System::Console::STD_OUTPUT_HANDLE,
        );
        let mut mode: u32 = 0;
        windows_sys::Win32::System::Console::GetConsoleMode(handle, &mut mode);
        windows_sys::Win32::System::Console::SetConsoleMode(
            handle,
            mode | windows_sys::Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        );
    }
}

#[cfg(not(windows))]
fn init_console() {}
