#!/bin/bash
if [[ -n "$WIN" ]]; then
  sudo apt-get install gcc-mingw-w64 &&
  printf "[target.$WIN]\nlinker = \"x86_64-w64-mingw32-gcc\"" >> ~/.cargo/config &&
  cargo build --release --target=$WIN &&
  mv target/$WIN/release/badlogvis.exe target/release/badlogvis-$WIN.exe
fi
