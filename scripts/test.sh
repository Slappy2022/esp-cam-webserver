#/bin/bash
set -eux -o pipefail

readonly BASE_DIR="$(
  cd -P "$(dirname "${BASH_SOURCE[0]}")/.."
  pwd
)"

main() {
  source "${BASE_DIR}/../wifi_config.sh" || source "${BASE_DIR}/wifi_config.sh"
  find . | grep -v /target | grep -v "/\." | entr -ds \
    'cargo +esp build --release --target xtensa-esp32-espidf'
}

main $@
