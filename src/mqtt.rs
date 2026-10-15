use core::time::Duration;

use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration, QoS};

const MQTT_URL: &str = "mqtts://broker.emqx.io:8883";
const MQTT_TOPIC: &str = "hello-esp32/demo";

/// MQTT over TLS:证书校验走 ESP x509 certificate bundle(和 https_get 一致)
pub fn run() -> anyhow::Result<()> {
    // load broker/user info from environment variables on build time
    const URL: Option<&str> = option_env!("MQTT_URL");
    const USERNAME: Option<&str> = option_env!("MQTT_USERNAME");
    const PASSWORD: Option<&str> = option_env!("MQTT_PASSWORD");

    const CLIENT_ID: &str = "hello-esp32";
    let url = URL.unwrap_or(MQTT_URL);
    let username = USERNAME.filter(|s| !s.is_empty());
    let password = PASSWORD.filter(|s| !s.is_empty());

    #[inline]
    unsafe extern "C" fn wrap_esp_crt_bundle_attach(conf: *mut ::core::ffi::c_void) -> i32 {
        esp_idf_svc::sys::esp_crt_bundle_attach(conf)
    }

    let config = MqttClientConfiguration {
        client_id: Some(CLIENT_ID),
        crt_bundle_attach: Some(wrap_esp_crt_bundle_attach),
        username,
        password,
        // mbedTLS 握手 + 收发都在 esp-mqtt 自己的任务里,默认太小
        task_stack: 32768,
        ..Default::default()
    };

    let (mut client, mut connection) = EspMqttClient::new(url, &config)?;
    log::info!("MQTT connected to {url}");

    std::thread::scope(|s| -> anyhow::Result<()> {
        // 必须立刻开始泵连接,否则下面的 subscribe/publish 都收不到响应
        std::thread::Builder::new()
            .stack_size(6144)
            .spawn_scoped(s, move || {
                log::info!("MQTT listening for messages");
                while let Ok(event) = connection.next() {
                    log::info!("[MQTT] {}", event.payload());
                }
                log::info!("MQTT connection closed");
            })
            .unwrap();

        client.subscribe(MQTT_TOPIC, QoS::AtMostOnce)?;
        log::info!("MQTT subscribed to \"{MQTT_TOPIC}\"");

        let mut i = 0;
        loop {
            let payload = format!("hello esp32 over mqtts #{i}");
            client.enqueue(MQTT_TOPIC, QoS::AtMostOnce, false, payload.as_bytes())?;
            log::info!("MQTT published \"{payload}\"");
            i += 1;

            std::thread::sleep(Duration::from_secs(5));
        }
    })
}
