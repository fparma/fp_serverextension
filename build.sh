#!/bin/bash

build_windows() {
    if [ "$1" == "--release" ]; then
        cargo build --release
        cp ./target/release/fpa_extension.dll ./beacon_x64.dll
    else
        cargo build
        cp ./target/debug/fpa_extension.dll ./beacon_x64.dll
    fi
};

build_linux() {
    if [ "$1" == "--release" ]; then
        PKG_CONFIG_ALLOW_CROSS=1 cargo build --release
        cp ./target/release/libfpa_extension_x64.so ./fpa_extension_x64.so
    else
        PKG_CONFIG_ALLOW_CROSS=1 cargo build
        cp ./target/debug/libfpa_extension_x64.so ./fpa_extension_x64.so
    fi
};

if [[ "$OSTYPE" == "msys" ]]; then
    build_windows "$1"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    build_linux "$1"
else
    echo "OS Type is not supported by this utility."
fi
