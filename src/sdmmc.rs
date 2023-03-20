use embedded_svc::http::server::{Connection, Handler, HandlerError, Response};
use esp_idf_hal_ext::sdmmc::Sdmmc;
use std::sync::{Arc, Mutex};

pub struct SdmmcHandler {
    sdcard: Arc<Mutex<Sdmmc>>,
}

impl SdmmcHandler {
    pub fn new(sdcard: Arc<Mutex<Sdmmc>>) -> Self {
        Self { sdcard }
    }
}

impl<C: Connection> Handler<C> for SdmmcHandler {
    fn handle(&self, connection: &mut C) -> Result<(), HandlerError> {
        use embedded_svc::http::Query;
        use embedded_svc::io::Write;
        let uri = connection.uri().to_string();

        let path = uri.split("?").nth(1).unwrap_or("");

        connection.initiate_response(200, Some("OK"), &[])?;
        let mut resp = Response::wrap(connection);
        if let Ok(sdcard) = self.sdcard.lock() {
            let dir_written = (|| -> Result<bool, HandlerError> {
                let dir = match sdcard.open_directory(path) {
                    Some(dir) => dir,
                    None => {
                        return Ok(false);
                    }
                };
                for entry in dir.ls() {
                    match entry.name() {
                        Ok(name) => resp.write_all(format!("{name}\n").as_bytes())?,
                        Err(e) => println!("{e:?}"),
                    }
                }
                Ok(true)
            })()?;
            let file_written = (|| -> Result<bool, HandlerError> {
                let file = match sdcard.open_file(path, "r") {
                    Some(f) => f,
                    None => return Ok(false),
                };
                let data = file.read_vec();
                resp.write_all(&data)?;
                Ok(true)
            })()?;
            if !dir_written && !file_written {
                resp.write_all(b"File/dir not found\n")?;
            }
        } else {
            resp.write_all(b"Couldn't access sdcard\n")?;
        }

        Ok(())
    }
}
