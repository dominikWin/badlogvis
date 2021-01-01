#!/usr/bin/env bash

mkdir -p out/badlogvis-$TARGET
cp target/release/badlogvis out/badlogvis-$TARGET/
cp .ci/install.sh out/badlogvis-$TARGET/

mkdir -p out/badlogvis-$SECONDARY

if [[ "$SECONDARY" != "x86_64-pc-windows-gnu" ]]; then
  cp .ci/install.sh out/badlogvis-$SECONDARY/
  cp target/$SECONDARY/release/badlogvis out/badlogvis-$SECONDARY/
else
  cp target/$SECONDARY/release/badlogvis.exe out/badlogvis-$SECONDARY/
fi

cd out/
tar -czvf badlogvis-$TARGET.tar.gz badlogvis-$TARGET/

if [[ "$SECONDARY" != "x86_64-pc-windows-gnu" ]]; then
  tar -czvf badlogvis-$SECONDARY.tar.gz badlogvis-$SECONDARY/
else
  zip -r badlogvis-$SECONDARY.zip badlogvis-$SECONDARY/
fi

