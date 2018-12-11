use std::fs::File;
use std::io::{Read, Write};
use std::str::FromStr;

mod kbd;

use clap::{App, Arg, SubCommand};
use strum::IntoEnumIterator;

enum Mode {
    Nothing,
    Brightness(u8),
    Preset {
        brightness: u8,
        preset: kbd::Preset,
        color: kbd::Color,
        speed: u8,
    },
    CustomSwitch {
        brightness: u8,
        slot: u8,
    },
    CustomSet {
        brightness: u8,
        slot: u8,
        config: String,
    },
    CustomGet {
        slot: u8,
        config: String,
    },
}

fn main() -> Result<(), libusb::Error> {
    // get all supported presets and colors
    let preset_strs: Vec<String> = kbd::Preset::iter().map(|x| x.to_string()).collect();
    let preset_strs: Vec<&str> = preset_strs.iter().map(|x| x.as_str()).collect();

    let color_strs: Vec<String> = kbd::Color::iter().map(|x| x.to_string()).collect();
    let mut color_strs: Vec<&str> = color_strs.iter().map(|x| x.as_str()).collect();
    color_strs.push("rand");
    color_strs.push("cycle");

    // use clap for arg parsing + validation
    #[rustfmt::skip]
    let app_m = App::new("fusion-kbd-controller")
        .version("0.1")
        .about("Control Fusion RGB Keyboard on Gigabyte Aero 15X")
        .arg(Arg::with_name("brightness")
            .global(true)
            .takes_value(true)
            .short("b")
            .long("brightness")
            .validator(|bstr| {
                let bval = bstr.parse::<u8>();
                if bval.is_err() || bval.unwrap() > 50 {
                    return Err("brightness must be a number from 0 - 50!".to_string())
                }
                Ok(())
            })
            .help("keyboard brightness (0 - 50)"))
        .subcommand(SubCommand::with_name("preset")
            .about("Work with Preset lighting profiles")
            .arg(Arg::with_name("preset")
                .required(true)
                .possible_values(&preset_strs)
                .case_insensitive(true)
                .index(1))
            .arg(Arg::with_name("color")
                .possible_values(&color_strs)
                .case_insensitive(true)
                .index(2))
            .arg(Arg::with_name("speed")
                .takes_value(true)
                .short("s")
                .long("speed")
                .validator(|sstr| {
                    let sval = sstr.parse::<u8>();
                    if sval.is_err() || sval.unwrap() > 10 {
                        return Err("speed must be a number from 0 - 10!".to_string())
                    }
                    Ok(())
                })
                .help("effect speed (0 - 10)")))
        .subcommand(SubCommand::with_name("custom")
            .about("Work with Custom lighting profiles")
            .arg(Arg::with_name("slot")
                .required(true)
                .index(1)
                .validator(|sstr| {
                    let sval = sstr.parse::<u8>();
                    if sval.is_err() || sval.unwrap() > 4 {
                        return Err("speed must be a number from 0 - 4!".to_string())
                    }
                    Ok(())
                })
                .help("Custom slot (0 - 4)"))
            .arg(Arg::with_name("set")
                .conflicts_with("get")
                .takes_value(true)
                .value_name("FILE")
                .long("set")
                .help("Upload new RGB Configuration to selected slot (binary)"))
            .arg(Arg::with_name("get")
                .conflicts_with("set")
                .takes_value(true)
                .value_name("FILE")
                .long("get")
                .help("Download RGB Configuration from selected slot (binary)")))
        .get_matches();

    // handle args

    let brightness = match app_m.value_of("brightness") {
        Some(bstr) => Some(bstr.parse::<u8>().unwrap()),
        None => None,
    };

    let mode: Mode = match app_m.subcommand() {
        ("preset", Some(preset_m)) => {
            let preset = kbd::Preset::from_str(preset_m.value_of("preset").unwrap()).unwrap();

            let speed = match preset_m.value_of("speed") {
                Some(sstr) => sstr.parse::<u8>().unwrap(),
                None => 5,
            };

            if !preset_m.is_present("color")
                && preset != kbd::Preset::Wave
                && preset != kbd::Preset::Neon
            {
                eprintln!("Error: Color must be specified for preset `{}`", preset);
                return Err(libusb::Error::Other);
            }

            let color = match preset_m.value_of("color") {
                Some(cstr) => kbd::Color::from_str(cstr).unwrap(),
                None => kbd::Color::Rand,
            };

            let brightness = brightness.unwrap_or(0x50 / 3);

            Mode::Preset {
                brightness,
                preset,
                color,
                speed,
            }
        }
        ("custom", Some(custom_m)) => {
            let slot = custom_m.value_of("slot").unwrap().parse::<u8>().unwrap();
            let brightness = brightness.unwrap_or(0x50 / 3);

            if let Some(cfg) = custom_m.value_of("set") {
                Mode::CustomSet {
                    brightness,
                    slot,
                    config: cfg.to_string(),
                }
            } else if let Some(cfg) = custom_m.value_of("get") {
                Mode::CustomGet {
                    slot,
                    config: cfg.to_string(),
                }
            } else {
                Mode::CustomSwitch { brightness, slot }
            }
        }
        ("", None) => match brightness {
            Some(brightness) => Mode::Brightness(brightness),
            None => Mode::Nothing,
        },
        _ => unimplemented!(), // this will never happen
    };

    // actually do the interesting stuff

    // set-up libusb devices, aquire handle to keyboard
    let context = libusb::Context::new()?;
    let kbd = kbd::FusionKBD::new(&context)?;

    match mode {
        Mode::Nothing => {}
        Mode::Brightness(_) => {
            println!("TODO: read current config, and write-back same config with updated brightness");
            unimplemented!();
        }
        Mode::Preset {
            brightness,
            preset,
            color,
            speed,
        } => {
            kbd.set_preset(preset, speed, brightness, color)?;
        }
        Mode::CustomSwitch { brightness, slot } => {
            kbd.set_custom(slot, brightness)?;
        }
        Mode::CustomSet {
            brightness,
            slot,
            config,
        } => {
            let mut data = [0; 512];
            let mut f = match File::open(&config) {
                Ok(file) => file,
                Err(_) => {
                    println!("couldn't open '{}'", config);
                    return Err(libusb::Error::Other);
                }
            };
            f.read_exact(&mut data).unwrap();

            kbd.upload_custom(slot, &data)?;
            kbd.set_custom(slot, brightness)?;
        }
        Mode::CustomGet { slot, config } => {
            let mut data: [u8; 512] = [0; 512];

            kbd.download_custom(slot, &mut data)?;

            let mut f = match File::create(&config) {
                Ok(file) => file,
                Err(_) => {
                    println!("couldn't open '{}'", config);
                    return Err(libusb::Error::Other);
                }
            };
            f.write(&data).unwrap();
        }
    }

    Ok(())
}
