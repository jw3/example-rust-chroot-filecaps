#!/usr/bin/env bash

if [[ -z "$1" ]]; then
  echo "usage: build-and-test.sh <chroot-dir>"
  exit 1
fi

# cleanup
rm -f ./example-rust-chroot-filecaps
rm -rf .local
mkdir -p .local

# build
cargo build --release 2> /dev/null
if [ $? -ne 0 ]; then echo "cargo build failed"; exit 1; fi

# prep the executable
mv target/release/example-rust-chroot-filecaps .

# test no caps and fail
#sudo setcap '' example-rust-chroot-filecaps
#getcap example-rust-chroot-filecaps
#./example-rust-chroot-filecaps "$1"

# run the test with caps
sudo setcap 'cap_sys_chroot=ep' example-rust-chroot-filecaps
getcap example-rust-chroot-filecaps
./example-rust-chroot-filecaps "$1"
