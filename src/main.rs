use esp_idf_hal::cam::Cam;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_sys::*;

fn main() -> Result<(), anyhow::Error> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let peripherals = Peripherals::take().unwrap();

    let sysloop = EspSystemEventLoop::take()?;

    let camera_ram_location = match unsafe { esp_idf_sys::esp_spiram_is_initialized() } {
        true => {
            log::info!("SPI ram is initialized");
            camera_fb_location_t_CAMERA_FB_IN_PSRAM
        }
        false => {
            log::info!("SPI ram is not initialized");
            camera_fb_location_t_CAMERA_FB_IN_DRAM
        }
    };

    let cam = loop {
        match Cam::new(camera_config_t {
            pin_pwdn: 32,
            pin_reset: -1,
            pin_xclk: 0,
            pin_sccb_sda: 26,
            pin_sccb_scl: 27,
            pin_d7: 35,
            pin_d6: 34,
            pin_d5: 39,
            pin_d4: 36,
            pin_d3: 21,
            pin_d2: 19,
            pin_d1: 18,
            pin_d0: 5,
            pin_vsync: 25,
            pin_href: 23,
            pin_pclk: 22,
            xclk_freq_hz: 20_000_000,
            ledc_timer: ledc_timer_t_LEDC_TIMER_0,
            ledc_channel: ledc_channel_t_LEDC_CHANNEL_0,
            pixel_format: pixformat_t_PIXFORMAT_JPEG,
            //pixel_format: pixformat_t_PIXFORMAT_RGB565,
            //frame_size: framesize_t_FRAMESIZE_UXGA,
            frame_size: framesize_t_FRAMESIZE_HD,
            //frame_size: framesize_t_FRAMESIZE_240X240,
            //frame_size: framesize_t_FRAMESIZE_QQVGA,
            //frame_size: framesize_t_FRAMESIZE_96X96,
            jpeg_quality: 5,
            fb_count: 3,
            fb_location: camera_ram_location,
            //grab_mode: camera_grab_mode_t_CAMERA_GRAB_WHEN_EMPTY,
            grab_mode: camera_grab_mode_t_CAMERA_GRAB_LATEST,
            ..camera_config_t::default()
        }) {
            Ok(cam) => break cam,
            Err(e) => log::error!("Error initializing cam: {e}"),
        }
        FreeRtos::delay_ms(1000);
    };
    let mut _wifi = esp_cam_webserver::init_wifi(peripherals.modem, sysloop.clone())?;

    let flash = esp_idf_hal::gpio::PinDriver::output(peripherals.pins.gpio4)?;
    let _http = esp_cam_webserver::http_server(cam, Some(flash))?;
    //let _http = esp_cam_webserver::http_server(cam, None)?;
    let mut led = PinDriver::output(peripherals.pins.gpio33)?;
    loop {
        led.toggle()?;
        FreeRtos::delay_ms(1000);
    }
}
