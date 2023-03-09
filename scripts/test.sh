#/bin/bash
set -eux -o pipefail

main() {
  find . | grep -v /target | grep -v "/\." | entr -ds \
    'cargo +esp build --release --target xtensa-esp32-espidf'
}

main $@
