mod api;
mod client;
mod config;
mod error;
mod model;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use client::ApiClient;
use config::AppConfig;
use error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    init_console();

    let config = AppConfig::load()?;
    let client = ApiClient::new(config.clone())?;

    println!(
        "target: {} | pay: {} | rps: {}",
        config.product_id,
        config.pay_type.as_str(),
        config.rps,
    );

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
                    Ok(Some(biz_id)) => {
                        if let Err(e) = api::pay::create_sign(&client, &config, &biz_id).await {
                            eprintln!("create-sign error: {e}");
                        }
                    }
                    Err(e) => {
                        eprintln!("preview error: {e}");
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
    use std::os::windows::ffi::OsStrExt;
    unsafe {
        // SetConsoleOutputCP(65001) - UTF-8
        windows_sys::Win32::System::Console::SetConsoleOutputCP(65001);
        // Enable virtual terminal processing for ANSI escape codes
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
