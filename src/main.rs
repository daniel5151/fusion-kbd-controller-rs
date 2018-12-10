use std::fs::File;
use std::io::Read;
use std::str::FromStr;

mod kbd;

use clap::{App, AppSettings, Arg, SubCommand};
use strum::IntoEnumIterator;

enum Mode {
    Preset {
        brightness: u8,
        preset: kbd::Preset,
        color: kbd::Color,
        speed: u8,
    },
    Custom {
        brightness: u8,
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
        .setting(AppSettings::SubcommandRequired)
        .arg(Arg::with_name("brightness")
            .global(true)
            .takes_value(true)
            .short("b")
            .long("brightness")
            .validator(|bstr| {
                let bval = bstr.parse::<u8>();
                if bval.is_err() || bval.unwrap() > 0x50 {
                    return Err("brightness must be a number from 0 - 80!".to_string())
                }
                Ok(())
            })
            .help("keyboard brightness (0 - 80)"))
        .subcommand(SubCommand::with_name("preset")
            .about("Set lighting from Preset profiles")
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
            .about("Set a custom lighting profile")
            .arg(Arg::with_name("config")
                .required(true)
                .index(1)
                .help("RGB Configuration File (binary)")))
        .get_matches();

    // handle args

    let brightness = match app_m.value_of("brightness") {
        Some(bstr) => bstr.parse::<u8>().unwrap(),
        None => 0x50 / 3,
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

            Mode::Preset {
                brightness,
                preset,
                color,
                speed,
            }
        }
        ("custom", Some(custom_m)) => {
            let config = custom_m.value_of("config").unwrap().to_string();

            Mode::Custom { brightness, config }
        }
        _ => unimplemented!(), // this will never happen happen
    };

    // actually do the interesting stuff

    // set-up libusb devices, aquire handle to keyboard
    let context = libusb::Context::new()?;
    let kbd = kbd::FusionKBD::new(&context)?;

    match mode {
        Mode::Preset {
            brightness,
            preset,
            color,
            speed,
        } => {
            kbd.set_preset(preset, speed, brightness, color)?;
        }
        Mode::Custom { brightness, config } => {
            let mut f = match File::open(&config) {
                Ok(file) => file,
                Err(_) => {
                    println!("couldn't open '{}'", config);
                    return Err(libusb::Error::Other);
                }
            };

            let mut cfg = [0; 512];
            f.read_exact(&mut cfg).unwrap();

            kbd.set_custom(brightness, &cfg)?;
        }
    }

    Ok(())
}
