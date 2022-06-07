#!/usr/bin/bash

set -e

export DEBIAN_FRONTEND=noninteractive
apt-get update && apt-get install -y appstream curl file libfuse2 libgtk-4-dev librsvg2-dev libudev-dev python3
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init
sh ./rustup-init -y
source $HOME/.cargo/env
./build.py --release


