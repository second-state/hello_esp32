use embedded_graphics::prelude::*;
use embedded_graphics::{
    draw_target::DrawTarget,
    framebuffer::{buffer_size, Framebuffer},
    pixelcolor::{raw::LittleEndian, Rgb565},
    prelude::Size,
    primitives::Rectangle,
    Drawable,
};
use embedded_text::TextBox;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::sys::EspError;
use u8g2_fonts::U8g2TextStyle;

const DISPLAY_WIDTH: usize = 240;
const DISPLAY_HEIGHT: usize = 240;
pub type ColorFormat = Rgb565;

static mut ESP_LCD_PANEL_HANDLE: esp_idf_svc::sys::esp_lcd_panel_handle_t = std::ptr::null_mut();

fn init_spi_rs(
    spi: esp_idf_svc::hal::spi::SPI3,
    sclk: AnyIOPin,
    sdo: AnyIOPin,
    sdi: Option<AnyIOPin>,
) -> esp_idf_svc::hal::spi::SpiDriver<'static> {
    let config = esp_idf_svc::hal::spi::SpiDriverConfig::new().dma(
        esp_idf_svc::hal::spi::Dma::Auto(DISPLAY_WIDTH * DISPLAY_HEIGHT),
    );

    let driver = esp_idf_svc::hal::spi::SpiDriver::new(spi, sclk, sdo, sdi, &config).unwrap();
    driver
}

fn init_spi() -> Result<(), EspError> {
    use esp_idf_svc::sys::*;
    const GPIO_NUM_NC: i32 = -1;
    const DISPLAY_MOSI_PIN: i32 = 47;
    const DISPLAY_CLK_PIN: i32 = 21;
    let mut buscfg = spi_bus_config_t::default();
    buscfg.__bindgen_anon_1.mosi_io_num = DISPLAY_MOSI_PIN;
    buscfg.__bindgen_anon_2.miso_io_num = GPIO_NUM_NC;
    buscfg.sclk_io_num = DISPLAY_CLK_PIN;
    buscfg.__bindgen_anon_3.quadwp_io_num = GPIO_NUM_NC;
    buscfg.__bindgen_anon_4.quadhd_io_num = GPIO_NUM_NC;
    buscfg.max_transfer_sz = (DISPLAY_WIDTH * DISPLAY_HEIGHT * std::mem::size_of::<u16>()) as i32;
    esp!(unsafe {
        spi_bus_initialize(
            spi_host_device_t_SPI3_HOST,
            &buscfg,
            spi_common_dma_t_SPI_DMA_CH_AUTO,
        )
    })
}

fn init_lcd() -> Result<(), EspError> {
    use esp_idf_svc::sys::*;
    const DISPLAY_CS_PIN: i32 = 41;
    const DISPLAY_DC_PIN: i32 = 40;
    ::log::info!("Install panel IO");
    let mut panel_io: esp_lcd_panel_io_handle_t = std::ptr::null_mut();
    let mut io_config = esp_lcd_panel_io_spi_config_t::default();
    io_config.cs_gpio_num = DISPLAY_CS_PIN;
    io_config.dc_gpio_num = DISPLAY_DC_PIN;
    io_config.spi_mode = 3;
    io_config.pclk_hz = 40 * 1000 * 1000;
    io_config.trans_queue_depth = 10;
    io_config.lcd_cmd_bits = 8;
    io_config.lcd_param_bits = 8;
    esp!(unsafe {
        esp_lcd_new_panel_io_spi(spi_host_device_t_SPI3_HOST as _, &io_config, &mut panel_io)
    })?;

    ::log::info!("Install LCD driver");
    const DISPLAY_RST_PIN: i32 = 45;
    let mut panel_config = esp_lcd_panel_dev_config_t::default();
    let mut panel: esp_lcd_panel_handle_t = std::ptr::null_mut();

    panel_config.reset_gpio_num = DISPLAY_RST_PIN;
    panel_config.data_endian = lcd_rgb_data_endian_t_LCD_RGB_DATA_ENDIAN_LITTLE;
    panel_config.__bindgen_anon_1.rgb_ele_order = lcd_rgb_element_order_t_LCD_RGB_ELEMENT_ORDER_RGB;
    panel_config.bits_per_pixel = 16;

    esp!(unsafe { esp_lcd_new_panel_st7789(panel_io, &panel_config, &mut panel) })?;
    unsafe { ESP_LCD_PANEL_HANDLE = panel };

    const DISPLAY_MIRROR_X: bool = false;
    const DISPLAY_MIRROR_Y: bool = false;
    const DISPLAY_SWAP_XY: bool = false;
    const DISPLAY_INVERT_COLOR: bool = true;

    ::log::info!("Reset LCD panel");
    unsafe {
        esp!(esp_lcd_panel_reset(panel))?;
        esp!(esp_lcd_panel_init(panel))?;
        esp!(esp_lcd_panel_invert_color(panel, DISPLAY_INVERT_COLOR))?;
        esp!(esp_lcd_panel_swap_xy(panel, DISPLAY_SWAP_XY))?;
        esp!(esp_lcd_panel_mirror(
            panel,
            DISPLAY_MIRROR_X,
            DISPLAY_MIRROR_Y
        ))?;
        esp!(esp_lcd_panel_disp_on_off(panel, true))?; /* 启动屏幕 */
    }

    Ok(())
}

#[inline(always)]
fn get_esp_lcd_panel_handle() -> esp_idf_svc::sys::esp_lcd_panel_handle_t {
    unsafe { ESP_LCD_PANEL_HANDLE }
}

pub fn flush_display(color_data: &[u8], x_start: i32, y_start: i32, x_end: i32, y_end: i32) -> i32 {
    unsafe {
        let e = esp_idf_svc::sys::esp_lcd_panel_draw_bitmap(
            get_esp_lcd_panel_handle(),
            x_start,
            y_start,
            x_end,
            y_end,
            color_data.as_ptr().cast(),
        );
        if e != 0 {
            log::warn!("flush_display error: {}", e);
        }
        e
    }
}

pub fn init_ui() -> Result<(), EspError> {
    init_spi()?;
    init_lcd()?;
    Ok(())
}

pub fn init_ui_rs(
    spi: esp_idf_svc::hal::spi::SPI3,
    sclk: AnyIOPin,
    sdo: AnyIOPin,
    sdi: Option<AnyIOPin>,
) -> Result<esp_idf_svc::hal::spi::SpiDriver<'static>, EspError> {
    let driver = init_spi_rs(spi, sclk, sdo, sdi);
    init_lcd()?;
    Ok(driver)
}

pub fn hello_lcd() -> Result<(), EspError> {
    let mut display = Box::new(Framebuffer::<
        ColorFormat,
        _,
        LittleEndian,
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        { buffer_size::<ColorFormat>(DISPLAY_WIDTH, DISPLAY_HEIGHT) },
    >::new());
    display.clear(ColorFormat::WHITE).unwrap();

    let text_area = Rectangle::new(
        display.bounding_box().top_left + Point { x: 0, y: 32 },
        Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32 - 32),
    );

    let text = "Hello, ESP32!\n 请按下k0开始录音";

    let textbox_style = embedded_text::style::TextBoxStyleBuilder::new()
        .height_mode(embedded_text::style::HeightMode::FitToText)
        .alignment(embedded_text::alignment::HorizontalAlignment::Center)
        .line_height(embedded_graphics::text::LineHeight::Percent(120))
        .paragraph_spacing(16)
        .build();
    let text_box = TextBox::with_textbox_style(
        &text,
        text_area,
        U8g2TextStyle::new(
            u8g2_fonts::fonts::u8g2_font_wqy12_t_gb2312a,
            ColorFormat::BLACK,
        ),
        textbox_style,
    );
    text_box.draw(display.as_mut()).unwrap();

    flush_display(
        display.data(),
        0,
        0,
        DISPLAY_WIDTH as _,
        DISPLAY_HEIGHT as _,
    );

    Ok(())
}
