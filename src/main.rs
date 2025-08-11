use esp_idf_svc::{
    hal::{
        gpio::AnyIOPin,
        i2s::{config, I2sDriver, I2S0, I2S1},
    },
    io::Read,
};

mod ui;

const SAMPLE_RATE: u32 = 16000;

static WAV_DATA: &[u8] = include_bytes!("../assets/hello.wav");

fn player_wav(
    i2s1: I2S1,
    bclk: AnyIOPin,
    dout: AnyIOPin,
    lrclk: AnyIOPin,
    mclk: Option<AnyIOPin>,
    data: Option<&[u8]>,
) {
    let i2s_config = config::StdConfig::new(
        config::Config::default().auto_clear(true),
        config::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        config::StdSlotConfig::philips_slot_default(
            config::DataBitWidth::Bits16,
            config::SlotMode::Mono,
        ),
        config::StdGpioConfig::default(),
    );

    let mut tx_driver = I2sDriver::new_std_tx(i2s1, &i2s_config, bclk, dout, mclk, lrclk).unwrap();

    tx_driver.tx_enable().unwrap();

    if let Some(data) = data {
        tx_driver.write_all(data, 1000).unwrap();
    } else {
        tx_driver.write_all(WAV_DATA, 1000).unwrap();
    }
}

fn record(
    i2s: I2S0,
    ws: AnyIOPin,
    sck: AnyIOPin,
    din: AnyIOPin,
    mclk: Option<AnyIOPin>,
) -> Vec<u8> {
    let i2s_config = config::StdConfig::new(
        config::Config::default().auto_clear(true),
        config::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        config::StdSlotConfig::philips_slot_default(
            config::DataBitWidth::Bits16,
            config::SlotMode::Mono,
        ),
        config::StdGpioConfig::default(),
    );

    let mut rx_driver = I2sDriver::new_std_rx(i2s, &i2s_config, sck, din, mclk, ws).unwrap();
    rx_driver.rx_enable().unwrap();

    let mut buffer = vec![0u8; 5 * SAMPLE_RATE as usize * 2]; // 5 seconds of audio at 16kHz, 16-bit mono
    rx_driver.read_exact(&mut buffer).unwrap();
    buffer
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = esp_idf_svc::hal::prelude::Peripherals::take().unwrap();

    log::info!("Hello, world!");

    // ui::init_ui().unwrap();
    let _spi_driver = ui::init_ui_rs(
        peripherals.spi3,
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio47.into(),
        None,
    )
    .unwrap();
    log::info!("UI initialized");
    ui::hello_lcd().unwrap();

    let sck = peripherals.pins.gpio5;
    let din = peripherals.pins.gpio6;
    let ws = peripherals.pins.gpio4;

    let dout = peripherals.pins.gpio7;
    let bclk = peripherals.pins.gpio15;
    let lrclk = peripherals.pins.gpio16;

    let mut button = esp_idf_svc::hal::gpio::PinDriver::input(peripherals.pins.gpio0).unwrap();
    button.set_pull(esp_idf_svc::hal::gpio::Pull::Up).unwrap();
    button
        .set_interrupt_type(esp_idf_svc::hal::gpio::InterruptType::PosEdge)
        .unwrap();

    log::info!("capacity of SPIRAM: {} KB", get_cap_spiram() / 1024); // it will show 8M if open CONFIG_SPIRAM in sdkconfig.default, else 0
    log::info!("capacity of internal RAM: {} KB", get_cap_internal() / 1024); // 363KB
    log::info!("stack high: {}", get_stack_high());

    // try malloc a large buffer to test memory
    // if not open CONFIG_SPIRAM, it will panic and restart
    let _large_buffer = Vec::<u8>::with_capacity(1024 * 1024); // 1MB

    log::info!("Waiting for button press...");
    esp_idf_svc::hal::task::block_on(button.wait_for_rising_edge()).unwrap();
    log::info!("Button pressed, starting recording...");

    let samples = record(peripherals.i2s0, ws.into(), sck.into(), din.into(), None);
    log::info!("Recording complete, length: {} bytes", samples.len());

    player_wav(
        peripherals.i2s1,
        bclk.into(),
        dout.into(),
        lrclk.into(),
        None,
        Some(&samples),
    );

    unsafe { esp_idf_svc::sys::esp_restart() }
}

pub fn get_stack_high() -> u32 {
    let stack_high =
        unsafe { esp_idf_svc::sys::uxTaskGetStackHighWaterMark2(std::ptr::null_mut()) };
    stack_high
}

pub fn get_cap_spiram() -> usize {
    unsafe {
        use esp_idf_svc::sys::{heap_caps_get_free_size, MALLOC_CAP_SPIRAM};
        heap_caps_get_free_size(MALLOC_CAP_SPIRAM)
    }
}

pub fn get_cap_internal() -> usize {
    unsafe {
        use esp_idf_svc::sys::{heap_caps_get_free_size, MALLOC_CAP_INTERNAL};
        heap_caps_get_free_size(MALLOC_CAP_INTERNAL)
    }
}
