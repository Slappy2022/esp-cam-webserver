mod sdmmc;

use anyhow::bail;
use embedded_svc::http::server::Method;
use embedded_svc::wifi::{AccessPointConfiguration, ClientConfiguration, Configuration, Wifi};
use esp_cam_bindings::Pic;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Gpio4, Output, PinDriver};
use esp_idf_hal_ext::sdmmc::Sdmmc;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::netif::{EspNetif, EspNetifWait};
use esp_idf_svc::wifi::{EspWifi, WifiWait};
use log::*;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
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
    sdcard: Arc<Mutex<Sdmmc>>,
) -> Result<esp_idf_svc::http::server::EspHttpServer, anyhow::Error> {
    *FLASH.lock().unwrap() = flash;
    use embedded_svc::io::Write;
    let mut server = EspHttpServer::new(&Default::default())?;

    server
        .fn_handler("/time", Method::Get, |req| {
            let unixtime = get_unixtime()?;
            req.into_ok_response()?
                .write_all(format!("{unixtime}").as_bytes())?;
            Ok(())
        })?
        .handler(
            "/sdcard",
            Method::Get,
            crate::sdmmc::SdmmcHandler::new(sdcard),
        )?
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

            Ok(())
        })?;

    Ok(server)
}

pub fn get_unixtime() -> anyhow::Result<u32> {
    use embedded_svc::http::client::*;
    use embedded_svc::io::Read;
    use esp_idf_svc::http::client::*;
    let url = "http://worldtimeapi.org/api/timezone/Etc/UTC";
    let mut client = Client::wrap(EspHttpConnection::new(&Configuration::default())?);

    let mut response = client.get(&url)?.submit()?;
    let mut body = [0u8; 1024];

    {
        let len = embedded_svc::utils::io::try_read_full(&mut response, &mut body)
            .map_err(|err| err.0)?;
        let body = core::str::from_utf8(&body[..len])?;

        for s in body.split(",") {
            let mut kv = s.split(":");
            let key = match kv.next() {
                Some(key) => key,
                None => continue,
            };
            if key != "\"unixtime\"" {
                continue;
            }
            let value = match kv.next() {
                Some(value) => value,
                None => continue,
            };
            if !kv.next().is_none() {
                continue;
            }
            let unixtime: u32 = match value.parse() {
                Ok(t) => t,
                Err(_) => continue,
            };
            return Ok(unixtime);
        }
    }

    // Complete the response
    while response.read(&mut body)? > 0 {}
    Err(anyhow::Error::msg("Error getting time"))
}
