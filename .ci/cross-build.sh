#!/bin/bash
if [[ -n "$WIN" ]]; then
  rustup target add $WIN &&
  printf "[target.$WIN]\nlinker = \"x86_64-w64-mingw32-gcc\"" >> ~/.cargo/config &&
  cargo build --release --target=$WIN &&
  mv target/$WIN/release/badlogvis.exe target/$WIN/release/badlogvis-$WIN.exe
fi

if [[ -n "$ARM" ]]; then
  rustup target add $ARM &&
  SDKROOT=$(xcrun -sdk macosx11.0 --show-sdk-path) \
  MACOSX_DEPLOYMENT_TARGET=$(xcrun -sdk macosx11.0 --show-sdk-platform-version) \
  cargo build --target=$ARM &&
  mv target/$ARM/release/badlogvis target/$ARM/release/badlogvis-$ARM
fi
