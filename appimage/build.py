#!/usr/bin/env python3

import argparse
import glob
import os
import shutil
import subprocess
import sys
from urllib.request import urlopen

# Handle commandline arguments
parser = argparse.ArgumentParser()
parser.add_argument('--release', action='store_true')
args = parser.parse_args()

# Appimage packaging
# XXX name
PKG = "mouse-configurator"
APPID = "org.pop_os.mouseconfigurator"
ARCH = "x86_64"

# Executables to install
TARGET_DIR = "../target/" + ('release' if args.release else 'debug')
ICON = f"../data/{APPID}.svg"

# Remove previous build
for i in glob.glob(f"{PKG}*.AppImage"):
    os.remove(i)
if os.path.exists(f"{PKG}.AppDir"):
    shutil.rmtree(f"{PKG}.AppDir")
if os.path.exists(PKG):
    os.remove(PKG)

# Build the application
cmd = ["cargo", "build", "--features", "appimage"]
if args.release:
    cmd.append('--release')
subprocess.check_call(cmd)

# Copy executable
subprocess.check_call([f"strip", '-o', PKG, f"{TARGET_DIR}/{PKG}"])

# Download linuxdeploy
LINUXDEPLOY = f"linuxdeploy-{ARCH}.AppImage"
LINUXDEPLOY_URL = f"https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/{LINUXDEPLOY}"
if not os.path.exists(LINUXDEPLOY):
    with urlopen(LINUXDEPLOY_URL) as u:
        with open(LINUXDEPLOY, 'wb') as f:
            f.write(u.read())
    os.chmod(LINUXDEPLOY, os.stat(LINUXDEPLOY).st_mode | 0o111)

# Copy appdata
os.makedirs(f"{PKG}.AppDir/usr/share/metainfo")
shutil.copy(f"../data/{APPID}.appdata.xml", f"{PKG}.AppDir/usr/share/metainfo")

# Build appimage
subprocess.check_call([f"./{LINUXDEPLOY}",
                       f"--appdir={PKG}.AppDir",
                       f"--executable={PKG}",
                       f"--desktop-file=../data/{APPID}.desktop",
                       f"--icon-file={ICON}",
                        "--plugin", "gtk",
                        "--output", "appimage"])
shutil.move(glob.glob(f"HP_Mouse_Configurator-*-{ARCH}.AppImage")[0], f"{PKG}-{ARCH}.AppImage")
