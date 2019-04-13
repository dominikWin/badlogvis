#!/bin/bash
if [[ -n "$WIN" ]]; then
  rustup target add $WIN &&
  printf "[target.$WIN]\nlinker = \"x86_64-w64-mingw32-gcc\"" >> ~/.cargo/config &&
  cargo build --release --target=$WIN &&
  mv target/$WIN/release/badlogvis.exe target/$WIN/release/badlogvis-$WIN.exe
fi
