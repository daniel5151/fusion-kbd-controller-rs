# fusion-kbd-controller-rs

This project is a tiny userspace binary that allows you to configure the RGB
Fusion keyboard of the Gigabyte AERO 15X using libusb.

At the moment, you can:
- switch between the built-in presets
- upload custom configurations!

Time permitting, more functionality will be RE'd and added to the tool.

On my radar:
- update brightness without re-sending entire config
  - read current brightness
- read current config

It is based off [martin31821's `fusion-kbd-controller`](https://github.com/martin31821/fusion-kbd-controller)

## Install

A standard `cargo install` should do the trick!

## Usage

cfg files are currently raw binary corresponding to the USB payload sent to the
keyboard. Check out the OG project to get an idea of the data format!

TODO: write a user-friendly tool to generate configs

Root privileges are required, since the tool has to temporarily unbinds the USB
device from the kernel module.

## Current state

It seems to work, but more testing and functionality work is required.

## Disclaimer

It's possible to brick your keyboard when sending bogus values over the wire!
While it seems to work fine for me, use this softawre at your own risk!

(that said, a "bricked" keyboard can usually be fixed with a reboot. Unless you
really mess it up, in which case, gg)
