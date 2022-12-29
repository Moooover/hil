#!/bin/sh

SYSROOT=/build/root

export PKG_CONFIG_DIR=
export PKG_CONFIG_LIBDIR=${SYSROOT}/usr/lib/pkgconfig:${SYSROOT}/usr/share/pkgconfig
export PKG_CONFIG_SYSROOT_DIR=${SYSROOT}
export PKG_CONFIG_ALLOW_CROSS=1
# tell pkg-config where to find libudev.pc
export PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
# tell cargo to link with an aarch64 compatible linker
#export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc-8
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=/root/toolchain/gcc-linaro-7.3.1-2018.05-x86_64_aarch64-linux-gnu/bin/aarch64-linux-gnu-gcc

cargo build --release --target=aarch64-unknown-linux-gnu
