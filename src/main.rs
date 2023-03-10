use esp_cam_bindings::{FrameBufferLocation, FrameSize, GrabMode, InitConfig, PixelFormat};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{IOPin, InputPin, PinDriver};
use esp_idf_hal::prelude::FromValueType;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;

fn main() -> Result<(), anyhow::Error> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let peripherals = Peripherals::take().unwrap();
    InitConfig {
        pin_pwdn: peripherals.pins.gpio32.downgrade(),
        pin_reset: None,
        pin_xclk: peripherals.pins.gpio0.downgrade(),
        pin_sccb_sda: peripherals.pins.gpio26.downgrade(),
        pin_sccb_scl: peripherals.pins.gpio27.downgrade(),
        pin_d7: peripherals.pins.gpio35.downgrade_input(),
        pin_d6: peripherals.pins.gpio34.downgrade_input(),
        pin_d5: peripherals.pins.gpio39.downgrade_input(),
        pin_d4: peripherals.pins.gpio36.downgrade_input(),
        pin_d3: peripherals.pins.gpio21.downgrade_input(),
        pin_d2: peripherals.pins.gpio19.downgrade_input(),
        pin_d1: peripherals.pins.gpio18.downgrade_input(),
        pin_d0: peripherals.pins.gpio5.downgrade_input(),
        pin_vsync: peripherals.pins.gpio25.downgrade(),
        pin_href: peripherals.pins.gpio23.downgrade(),
        pin_pclk: peripherals.pins.gpio22.downgrade(),

        xclk_freq_hz: 16u32.MHz().into(),
        ledc_timer: peripherals.ledc.timer0,
        ledc_channel: peripherals.ledc.channel0,

        pixel_format: PixelFormat::Jpeg,
        frame_size: FrameSize::Hd,
        jpeg_quality: 5,

        fb_count: 5,
        fb_location: FrameBufferLocation::Psram,
        grab_mode: GrabMode::Latest,

        sccb_i2c_port: 0,
    }
    .init()?;

    let sysloop = EspSystemEventLoop::take()?;

    let mut _wifi = esp_cam_webserver::init_wifi(peripherals.modem, sysloop.clone())?;

    let flash = esp_idf_hal::gpio::PinDriver::output(peripherals.pins.gpio4)?;
    let _http = esp_cam_webserver::http_server(Some(flash))?;
    //let _http = esp_cam_webserver::http_server(None)?;
    let mut led = PinDriver::output(peripherals.pins.gpio33)?;
    loop {
        led.toggle()?;
        FreeRtos::delay_ms(1000);
    }
}
