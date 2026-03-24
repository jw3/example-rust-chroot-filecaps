#!/usr/bin/env bash

if [[ -z "$1" ]]; then
  echo "usage: build-and-test.sh <chroot-dir>"
  exit 1
fi

# cleanup
rm ./example-rust-chroot-filecaps
rm -rf .local
mkdir -p .local

#mkdir -p .local/lib64 .local/lib/x86_64-linux-gnu

#cp --parents /lib64/ld-linux-x86-64.so.2 .local
#cp --parents /lib/x86_64-linux-gnu/libc.so.6 .local
#cp --parents /lib/x86_64-linux-gnu/libtinfo.so.6 .local
#cp --parents /lib/x86_64-linux-gnu/libgcc_s.so.1 .local
#cp --parents /lib/x86_64-linux-gnu/libselinux.so.1 .local
#cp --parents  /lib/x86_64-linux-gnu/libpcre2-8.so.0 .local

# build
cargo build 2> /dev/null
if [ $? -ne 0 ]; then echo "cargo build failed"; exit 1; fi

# prep the executable
mv target/debug/example-rust-chroot-filecaps .
sudo setcap 'cap_sys_chroot=ep' example-rust-chroot-filecaps

# debug for sanity check
id; getcap example-rust-chroot-filecaps

# run the test
./example-rust-chroot-filecaps "$1"
