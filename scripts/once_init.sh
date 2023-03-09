#/bin/bash
set -eux -o pipefail

readonly BASE_DIR="$(
  cd -P "$(dirname "${BASH_SOURCE[0]}")/.."
  pwd
)"

readonly COMPONENTS_DIR="${BASE_DIR}/.embuild/espressif/esp-idf/release-v4.4/components/"

main() {
  mkdir -p "${COMPONENTS_DIR}"
  cd "${COMPONENTS_DIR}"
  git clone https://github.com/Slappy2022/esp32-camera
}

main $@
