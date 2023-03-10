use anyhow::bail;
use embedded_svc::http::server::Method;
use embedded_svc::wifi::{AccessPointConfiguration, ClientConfiguration, Configuration, Wifi};
use esp_cam_bindings::Pic;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Gpio4, Output, PinDriver};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::netif::{EspNetif, EspNetifWait};
use esp_idf_svc::wifi::{EspWifi, WifiWait};
use log::*;
use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::time::Duration;

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

// Flash struct can be twiddled on and off, but will also turn off when out of scope, so it won't
// stay on after errors.
pub struct Flash<'a> {
    pin: Option<&'a mut PinDriver<'static, Gpio4, Output>>,
}
impl<'a> Flash<'a> {
    pub fn on(&mut self) {
        if let Some(pin) = &mut self.pin {
            let _result = pin.set_high();
            FreeRtos::delay_ms(5);
        }
    }
    pub fn off(&mut self) {
        if let Some(pin) = &mut self.pin {
            let _result = pin.set_low();
        }
    }
}
impl<'a> Drop for Flash<'a> {
    fn drop(&mut self) {
        self.off();
    }
}

/*
fn rgb888_bmp_header(pic: &Pic, data_len: usize) -> Vec<u8> {
    let header_len = 54u32;
    let mut payload = Vec::<u8>::with_capacity(header_len as usize);

    // 2 bytes
    payload.extend_from_slice(b"BM");

    // 12 bytes
    payload.extend_from_slice(&(header_len + data_len as u32).to_le_bytes());
    payload.extend_from_slice(&[0x00; 4]); // Creators
    payload.extend_from_slice(&header_len.to_le_bytes());

    // 40 bytes
    payload.extend_from_slice(&40u32.to_le_bytes()); // Dip header length
    payload.extend_from_slice(&(pic.width() as u32).to_le_bytes());
    payload.extend_from_slice(&(pic.height() as u32).to_le_bytes());
    payload.extend_from_slice(&1u16.to_le_bytes()); // num planes
    payload.extend_from_slice(&24u16.to_le_bytes()); // bits per pixel
    payload.extend_from_slice(&0u32.to_le_bytes()); // compress type
    payload.extend_from_slice(&(pic.data().len() as u32).to_le_bytes()); // data size

    payload.extend_from_slice(&(pic.width() as i32).to_le_bytes());
    payload.extend_from_slice(&(pic.height() as i32).to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes()); // num colors
    payload.extend_from_slice(&0u32.to_le_bytes()); // num imp colors
    payload
}
fn raw_rgb565_to_rgb888_bmp(pic: &Pic) -> Vec<u8> {
    let header_len = 54u32;
    let data_len = pic.width() * pic.height() * 3;
    let mut payload = Vec::<u8>::with_capacity(header_len as usize + data_len);
    let header = rgb888_bmp_header(pic, data_len);
    payload.extend_from_slice(&header);

    for i in (0..pic.data().len()).step_by(2) {
        let color = u16::from_be_bytes([pic.data()[i], pic.data()[i + 1]]);
        let red = (color & 0xf800) >> 11;
        let green = (color & 0x07e0) >> 5;
        let blue = color & 0x001f;
        payload.extend_from_slice(&[(green as u8) << 2, (blue as u8) << 3, (red as u8) << 3]);
    }
    payload
}
*/

pub fn init_wifi(
    modem: impl esp_idf_hal::peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<Box<EspWifi<'static>>, anyhow::Error> {
    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), None)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    wifi.start()?;

    info!("Starting wifi...");

    if !WifiWait::new(&sysloop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started().unwrap())
    {
        bail!("Wifi did not start");
    }

    info!("Connecting wifi...");

    wifi.connect()?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sysloop)?.wait_with_timeout(
        Duration::from_secs(20),
        || {
            wifi.is_connected().unwrap()
                && wifi.sta_netif().get_ip_info().unwrap().ip != Ipv4Addr::new(0, 0, 0, 0)
        },
    ) {
        bail!("Wifi did not connect or did not receive a DHCP lease");
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(wifi)
}

static FLASH: Mutex<Option<PinDriver<Gpio4, Output>>> = Mutex::new(None);
pub fn http_server(
    flash: Option<PinDriver<'static, Gpio4, Output>>,
) -> Result<esp_idf_svc::http::server::EspHttpServer, anyhow::Error> {
    *FLASH.lock().unwrap() = flash;
    use embedded_svc::io::Write;
    let mut server = EspHttpServer::new(&Default::default())?;

    server
        .fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all("Hello, World!".as_bytes())?;
            Ok(())
        })?
        .fn_handler("/camera", Method::Get, |req| {
            let mut flash = FLASH.lock()?;
            let mut flash = Flash {
                pin: flash.as_mut(),
            };
            flash.on();
            let pic = Pic::new().ok_or("failed to take pic")?;
            let width = pic.width();
            let height = pic.height();
            let data = pic.data();
            let len = data.len();
            println!("{width}x{height} {len}");
            flash.off();

            let len_str = format!("{}", pic.data().len());
            let headers = &[
                ("Content-Type", "image/jpeg"),
                ("Content-Length", &len_str),
                // Comment to force multiline
            ];
            let mut resp = req.into_response(200, None, headers)?;
            resp.write_all(pic.data())?;

            /* RGB565
            let payload = raw_rgb565_to_rgb888_bmp(&pic);
            let len_str = format!("{}", payload.len());

            let headers = &[
                ("Content-Type", "image/bmp"),
                ("Content-Length", &len_str),
                // Comment to force multiline
            ];
            let mut resp = req.into_response(200, None, headers)?;
            resp.write_all(&payload)?;
            */
            Ok(())
        })?;

    Ok(server)
}
