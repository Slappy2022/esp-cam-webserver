export RUST_ESP32_STD_DEMO_WIFI_SSID=
export RUST_ESP32_STD_DEMO_WIFI_PASS=

if [ -z "${RUST_ESP32_STD_DEMO_WIFI_SSID}" ] || [ -z "${RUST_ESP32_STD_DEMO_WIFI_PASS}" ]; then
  echo "Fill in the SSID and password in wifi_config.sh"
  exit 1
fi
