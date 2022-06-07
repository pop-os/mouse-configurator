#!/bin/bash

podman run -v ..:/mnt -w /mnt/appimage --privileged ubuntu:hirsute bash ./install-and-build.sh
