use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripheral,
    wifi::{AuthMethod, BlockingWifi, EspWifi},
};
use futures_util::SinkExt;

pub fn wifi(
    ssid: &str,
    pass: &str,
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> anyhow::Result<Box<EspWifi<'static>>> {
    let mut auth_method = AuthMethod::WPA2Personal;
    if ssid.is_empty() {
        anyhow::bail!("Missing WiFi name")
    }
    if pass.is_empty() {
        auth_method = AuthMethod::None;
        log::info!("Wifi password is empty");
    }
    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), None)?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    wifi.set_configuration(&esp_idf_svc::wifi::Configuration::Client(
        esp_idf_svc::wifi::ClientConfiguration {
            ssid: ssid
                .try_into()
                .expect("Could not parse the given SSID into WiFi config"),
            password: pass
                .try_into()
                .expect("Could not parse the given password into WiFi config"),
            auth_method,
            ..Default::default()
        },
    ))?;

    wifi.start()?;

    log::info!("Connecting wifi...");

    wifi.connect()?;

    log::info!("Waiting for DHCP lease...");

    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    log::info!("Wifi DHCP info: {:?}", ip_info);

    Ok(Box::new(esp_wifi))
}

pub async fn http_get(url: &str) -> anyhow::Result<String> {
    Ok(reqwest::get(url).await?.text().await?)
}

pub async fn ws_task(url: &str) -> anyhow::Result<()> {
    use futures_util::StreamExt;

    let (mut ws_stream, _) = tokio_websockets::ClientBuilder::new()
        .uri(url)?
        .connect()
        .await?;
    log::info!("WebSocket connected to {}", url);

    let mut i = 0;

    while let Some(message) = ws_stream.next().await {
        match message {
            Ok(msg) => {
                if let Some(text) = msg.as_text() {
                    log::info!("Received text message: {}", text);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    ws_stream
                        .send(tokio_websockets::Message::text(format!(
                            "Hello Websocket Server {}",
                            i
                        )))
                        .await?;
                    i += 1;
                    if i > 10 {
                        log::info!("Closing WebSocket connection after 10 messages.");
                        ws_stream.close().await?;
                        break;
                    }
                } else {
                    log::warn!("Received non-text message: {:?}", msg);
                }
            }
            Err(e) => {
                log::error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
