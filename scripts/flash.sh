#/bin/bash
set -eux -o pipefail

main() {
  cargo +esp espflash --release --target xtensa-esp32-espidf \
    --example sdmmc \
    --speed 115200 \
    --monitor /dev/ttyUSB0
}

main $@
