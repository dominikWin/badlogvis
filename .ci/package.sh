#!/usr/bin/env bash

mkdir -p out/badlogvis-$TARGET
cp target/release/badlogvis out/badlogvis-$TARGET/
cp .ci/install.sh out/badlogvis-$TARGET/

if [[ -n "$ARM" ]]; then
  mkdir -p out/badlogvis-$ARM
  cp target/$ARM/release/badlogvis out/badlogvis-$ARM/
  cp .ci/install.sh out/badlogvis-$ARM/
fi

cd out/
tar -czvf badlogvis-$TARGET.tar.gz badlogvis-$TARGET/

if [[ -n "$ARM" ]]; then
  tar -czvf badlogvis-$ARM.tar.gz badlogvis-$ARM/
fi

