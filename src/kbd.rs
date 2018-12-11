use std::time;

use strum_macros::*;

#[derive(Display, EnumIter, EnumString, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum Preset {
    Static = 0x01,
    Breathing = 0x02,
    Wave = 0x03,
    FadeOnKeypress = 0x04,
    Marquee = 0x05,
    Ripple = 0x06,
    FlashOnKeypress = 0x07,
    Neon = 0x08,
    RainbowMarquee = 0x09,
    Raindrop = 0x0a,
    CircleMarquee = 0x0b,
    Hedge = 0x0c,
    Rotate = 0x0d,
}

#[derive(Display, EnumIter, EnumString, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum Color {
    #[strum(serialize = "rand", serialize = "rainbow", serialize = "cycle")]
    Rand = 0x00,
    Red = 0x01,
    Green = 0x02,
    Yellow = 0x03,
    Blue = 0x04,
    Orange = 0x05,
    Purple = 0x06,
    White = 0x07,
}

#[repr(C, packed)]
struct Header {
    kind: u8,         // Kind of the control transfer
    reserved: u8,     // ??
    mode: u8,         // mode or config slot
    speed_length: u8, // Speed or length of usb packets to follow
    brightness: u8,   // Brightness. 0 to 50
    color: u8,        // Predefined color
    reserved2: u8,    // ??
    checksum: u8,
}

impl Header {
    /// creates valid header (computes checksum)
    fn new(kind: u8, mode: u8, speed_length: u8, brightness: u8, color: u8) -> Header {
        let mut header = Header {
            kind,
            mode,
            speed_length,
            brightness,
            color,
            reserved: 0,
            reserved2: 0,
            checksum: 0,
        };

        // calculate checksum byte
        header.checksum = !(header
            .as_bytes()
            .iter()
            .take(7)
            .fold(0, |sum, x| sum.wrapping_add(*x)));

        header
    }

    /// used when sending over-the-wire with libusb
    fn as_bytes(&self) -> &[u8; std::mem::size_of::<Self>()] {
        unsafe { &*(self as *const Header as *const [u8; 8]) }
    }
}

static KIND_PRESET: u8 = 0x08;
static KIND_CUSTOM_CONFIG: u8 = 0x12;
// static KIND_READ_CONFIG: u8 = 0x92;

pub struct FusionKBD<'a> {
    handle: libusb::DeviceHandle<'a>,
}

impl<'a> FusionKBD<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(context: &'a libusb::Context) -> Result<Self, libusb::Error> {
        let mut handle = match context.open_device_with_vid_pid(0x1044, 0x7a39) {
            Some(handle) => handle,
            None => {
                eprintln!("Failed to open device! Are you running as root?");
                return Err(libusb::Error::Access);
            }
        };

        if handle.kernel_driver_active(0).unwrap() {
            handle.detach_kernel_driver(0)?;
        }
        if handle.kernel_driver_active(3).unwrap() {
            handle.detach_kernel_driver(3)?;
        }

        handle.claim_interface(0)?;
        handle.claim_interface(3)?;

        Ok(FusionKBD { handle })
    }

    fn write_control_kbd(&self, header: &Header) -> Result<usize, libusb::Error> {
        self.handle.write_control(
            libusb::request_type(
                libusb::Direction::Out,
                libusb::RequestType::Class,
                libusb::Recipient::Interface,
            ),
            0x09,   // bRequest
            0x0300, // wValue
            0x0003, // wIndex
            header.as_bytes(),
            time::Duration::new(0, 0),
        )
    }

    /// switch lighting to built-in preset
    pub fn set_preset(
        &self,
        preset: Preset,
        speed: u8,
        brightness: u8,
        color: Color,
    ) -> Result<(), libusb::Error> {
        let header = Header::new(
            KIND_PRESET,
            preset as u8,
            speed,
            brightness,
            color as u8, // COLOR_RED
        );
        self.write_control_kbd(&header)?;

        Ok(())
    }

    /// upload custom lighting scheme to selected custom mode slot
    pub fn upload_custom(&self, slot: u8, data: &[u8]) -> Result<(), libusb::Error> {
        assert!(slot < 5);
        let header = Header::new(KIND_CUSTOM_CONFIG, slot, 0x08, 0x00, 0x00);
        self.write_control_kbd(&header)?;

        print!("Interrupt transfers...");
        for i in 0..8 {
            let start = i * 64;
            let end = start + 64;
            let tf =
                self.handle
                    .write_interrupt(6, &data[start..end], time::Duration::new(0, 0))?;
            if tf != 64 {
                eprintln!("Interrupt transfer {} failed: {}", i, tf);
            }
        }
        println!("Ok!");

        // will NOT automatically switch to the new mode!
        // requires call to set_custom

        Ok(())
    }

    /// switch to custom lighting scheme in selected custom mode slot
    pub fn set_custom(&self, slot: u8, brightness: u8) -> Result<(), libusb::Error> {
        assert!(slot < 5);
        // 33..37 are the custom-mode slots
        let header = Header::new(KIND_PRESET, 0x33 + slot, 0, brightness, 0);
        self.write_control_kbd(&header)?;

        Ok(())
    }
}

impl<'a> Drop for FusionKBD<'a> {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(0);
        let _ = self.handle.release_interface(3);
        let _ = self.handle.attach_kernel_driver(0);
        let _ = self.handle.attach_kernel_driver(3);
    }
}
