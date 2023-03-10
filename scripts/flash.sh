#/bin/bash
set -eux -o pipefail

readonly BASE_DIR="$(
  cd -P "$(dirname "${BASH_SOURCE[0]}")/.."
  pwd
)"

main() {
  source "${BASE_DIR}/../wifi_config.sh" || source "${BASE_DIR}/wifi_config.sh"
  cargo +esp espflash --release --target xtensa-esp32-espidf \
    --speed 115200 \
    --monitor /dev/ttyUSB0
}

main $@
