#!/bin/bash

# Build script for hello_esp32 (单二进制,A/B OTA)
# Usage: ./build.sh [all|ota|bin]
#
# all - 一次 cargo build,同时产出下面两种镜像
# ota - OTA 镜像(仅 app,在 ota_1;运行中的 app 在 ota_0,OTA 会写 ota_1)
# bin - 合并工厂镜像(16MB,app 在 ota_0,首启槽;esptool/espflash 直接烧)

set -e

MODE="${1:-all}"
BIN=target/xtensa-esp32s3-espidf/release/hello_esp32

case "$MODE" in
    all|ota|bin)
        echo "Building release binary..."
        cargo build --bin hello_esp32 --release
        ;;
esac

case "$MODE" in
    all|ota)
        echo "Building OTA image (ota_1)..."
        espflash save-image --chip esp32s3 --flash-size 16mb --partition-table partitions.csv --target-app-partition ota_1 "$BIN" ./hello_esp32_ota.bin
        ;;
esac

case "$MODE" in
    all|bin)
        echo "Building merged factory image (ota_0, 16mb)..."
        espflash save-image --chip esp32s3 --merge --flash-size 16mb --partition-table partitions.csv --target-app-partition ota_0 "$BIN" ./hello_esp32.bin
        ;;
esac

echo "Done:"
ls -lh hello_esp32_ota.bin hello_esp32.bin 2>/dev/null | awk '{print "  " $9 "  " $5}'
