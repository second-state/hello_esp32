use esp_idf_svc::{hal::reset::restart, http::client::EspHttpConnection, ota::EspOta};

use embedded_svc::http::Method;

/// 从 OTA_URL(默认 GitHub release)下载固件写进对面分区,完成后重启。
///
/// 下载链路不稳时(代理/端口映射会把连接中途掐断),不丢已写数据——OTA 分区句柄保持
/// 打开,带 `Range: bytes=<已收>-` 重连续传(参考 vibekeys_firmware 的实测经验)。
pub fn run() -> anyhow::Result<()> {
    const DEFAULT_URL: &str =
        "https://github.com/second-state/hello_esp32/releases/latest/download/hello_esp32.bin";
    let url = option_env!("OTA_URL").unwrap_or(DEFAULT_URL);

    const MAX_ATTEMPTS: usize = 10;

    let mut ota = EspOta::new()?;
    ota.mark_running_slot_valid()?;
    // 尺寸未知:整分区擦写(续传轮拿到的 content-length 是剩余长度,不可靠,省去分支)
    let mut update = ota.initiate_update()?;

    let mut total: usize = 0;
    let mut full_len: Option<usize> = None; // 总长,首轮 200 响应里拿
    let mut next_report: u8 = 5;
    let mut buf = vec![0u8; 8192];
    let mut completed = false;

    for attempt in 1..=MAX_ATTEMPTS {
        // 已收 > 0 时带 Range 续传(Response 借用 client,连接建立只能内联在循环里)
        let range_from = if total == 0 { None } else { Some(total) };
        if attempt > 1 {
            log::warn!("OTA download retry #{attempt}, resuming at {total} bytes via Range");
        }
        let config = esp_idf_svc::http::client::Configuration {
            buffer_size: Some(16 * 1024),
            buffer_size_tx: Some(1024),
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            timeout: Some(std::time::Duration::from_secs(60)),
            ..Default::default()
        };
        let conn = EspHttpConnection::new(&config)?;
        let mut client = embedded_svc::http::client::Client::wrap(conn);
        // range/headers 必须活到 response 用完(header 切片借它们,生命周期跟 request 相同)
        let range = format!("bytes={total}-");
        let headers = [("Range", range.as_str())];
        let mut response = match range_from {
            Some(_) => {
                let request = client.request(Method::Get, url, &headers)?;
                request.submit()?
            }
            None => client.get(url)?.submit()?,
        };
        let status = response.status();
        log::info!("OTA download HTTP status: {status}");

        // 首轮要 200;续传轮服务器必须支持 Range(206),返回 200 说明它忽略了 Range,
        // 继续写会从头覆盖错位,直接报错
        if status != 200 && !(status == 206 && range_from.is_some()) {
            anyhow::bail!("OTA download failed: HTTP {status}");
        }
        if status == 200 {
            full_len = response
                .header("content-length")
                .and_then(|value| value.parse::<usize>().ok());
        }

        // true = 本轮正常读完(EOF);false = 中途掉线,需要续传
        let mut round_eof = true;
        loop {
            match response.read(&mut buf) {
                Ok(0) => break, // 本轮 EOF
                Ok(n) => {
                    update.write(&buf[..n])?;
                    total += n;
                    if let Some(len) = full_len {
                        if len > 0 {
                            let pct = ((total as u64 * 100) / len as u64) as u8;
                            if pct >= next_report {
                                log::info!("OTA progress {pct}% ({total}/{len} bytes)");
                                next_report = pct.saturating_add(5);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("OTA download connection lost at {total} bytes: {e:?}");
                    round_eof = false;
                    break;
                }
            }
        }

        // 完整读完(EOF 且达到总长,或服务器没给总长且 EOF)→ 收尾重启
        if round_eof && full_len.map_or(true, |len| total >= len) {
            completed = true;
            break;
        }

        drop(response);
        drop(client);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    if !completed {
        anyhow::bail!("OTA download incomplete after {MAX_ATTEMPTS} attempts: got {total} bytes");
    }
    update.complete()?;
    log::info!("OTA download complete: {total} bytes, restarting");
    restart();
}
