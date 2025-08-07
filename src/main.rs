use esp_idf_svc::hal::i2s::{config, I2sDriver};

const SAMPLE_RATE: u32 = 16000;

static WAV_DATA: &[u8] = include_bytes!("../assets/hello.wav");

fn player_wav() {
    let peripherals = esp_idf_svc::hal::prelude::Peripherals::take().unwrap();

    let i2s_config = config::StdConfig::new(
        config::Config::default().auto_clear(true),
        config::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        config::StdSlotConfig::philips_slot_default(
            config::DataBitWidth::Bits16,
            config::SlotMode::Mono,
        ),
        config::StdGpioConfig::default(),
    );

    let bclk = peripherals.pins.gpio15;
    let dout = peripherals.pins.gpio7;
    let lrclk = peripherals.pins.gpio16;
    let mclk: Option<esp_idf_svc::hal::gpio::AnyIOPin> = None;

    let mut tx_driver =
        I2sDriver::new_std_tx(peripherals.i2s1, &i2s_config, bclk, dout, mclk, lrclk).unwrap();

    tx_driver.tx_enable().unwrap();

    tx_driver.write_all(WAV_DATA, 1000).unwrap();
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");
    player_wav();
}
