use esp_idf_svc::http::client::EspHttpConnection;

mod mqtt;
mod network;

const URL: &str = "https://httpbin.org/get";

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = esp_idf_svc::hal::prelude::Peripherals::take().unwrap();
    let sysloop = esp_idf_svc::eventloop::EspSystemEventLoop::take().unwrap();

    // load SSID and password from environment variables on build time
    const SSID: Option<&str> = option_env!("SSID");
    const PASSWORD: Option<&str> = option_env!("PASSWORD");

    let _wifi = network::wifi(
        SSID.unwrap_or(""),
        PASSWORD.unwrap_or_default(),
        peripherals.modem,
        sysloop,
    )
    .unwrap();

    https_get(URL).unwrap();

    mqtt::run().unwrap();
}

/// HTTPS GET,证书校验走 ESP x509 certificate bundle(参考 vibekeys_firmware 的做法)
fn https_get(url: &str) -> anyhow::Result<()> {
    #[inline]
    unsafe extern "C" fn wrap_esp_crt_bundle_attach(conf: *mut ::core::ffi::c_void) -> i32 {
        esp_idf_svc::sys::esp_crt_bundle_attach(conf)
    }

    let config = esp_idf_svc::http::client::Configuration {
        crt_bundle_attach: Some(wrap_esp_crt_bundle_attach),
        ..Default::default()
    };
    let conn = EspHttpConnection::new(&config)?;
    let mut client = embedded_svc::http::client::Client::wrap(conn);

    let request = client.get(url)?;
    let mut response = request.submit()?;
    log::info!("GET {url} -> {}", response.status());

    let mut body = String::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = response.read(&mut buf)?;
        if n == 0 {
            break;
        }
        body.push_str(std::str::from_utf8(&buf[..n])?);
    }

    println!("{body}");
    Ok(())
}
