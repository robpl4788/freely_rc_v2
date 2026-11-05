use crate::hal::{pin::Pin, register_extras::RegisterExtras};

use embedded_hal::digital::{ErrorType, OutputPin};

#[repr(u8)]
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputMode {
    #[default]
    PushPull = 0,
    OpenDrain = 1,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum DriveStrength {
    #[default]
    Normal, // HDRV = 0 ODRV = 00
    Large, // HDRV = 0 ODRV = 01
    Huge,  // HDRV = 1 ODRV = xx
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OutputConfig {
    mode: OutputMode,
    drive_strength: DriveStrength,
    initial_output: bool,
}

impl OutputConfig {
    pub fn with_drive_strength(&self, strength: DriveStrength) -> OutputConfig {
        let mut result = self.clone();
        result.drive_strength = strength;

        result
    }

    pub fn with_mode(&self, mode: OutputMode) -> OutputConfig {
        let mut result = self.clone();
        result.mode = mode;

        result
    }
}

pub struct Output {
    pin: Pin,
    config: OutputConfig,
}

impl Output {
    pub(super) fn new(pin: Pin, config: OutputConfig) -> Output {
        let mut o = Output { pin, config };

        o.apply_config(o.config);

        o
    }

    pub fn apply_config(&mut self, new_config: OutputConfig) {
        self.config = new_config;

        match self.config.drive_strength {
            DriveStrength::Normal => {
                self.pin
                    .register_block
                    .odrvr()
                    .set_bits(0b00, 2 * self.pin.number, 2);
                self.pin
                    .register_block
                    .hdrv()
                    .set_bit(false, self.pin.number);
            }
            DriveStrength::Large => {
                self.pin
                    .register_block
                    .odrvr()
                    .set_bits(0b01, 2 * self.pin.number, 2);
                self.pin
                    .register_block
                    .hdrv()
                    .set_bit(false, self.pin.number);
            }
            DriveStrength::Huge => {
                self.pin
                    .register_block
                    .odrvr()
                    .set_bits(0b00, 2 * self.pin.number, 2);
                self.pin
                    .register_block
                    .hdrv()
                    .set_bit(true, self.pin.number);
            }
        }
        self.pin
            .register_block
            .omode()
            .set_bit(self.config.mode as u32 != 0, self.pin.number);
        self.set(self.config.initial_output);
    }

    pub fn set(&mut self, value: bool) {
        match value {
            true => self.set_high(),
            false => self.set_low(),
        }
    }

    pub fn set_high(&mut self) {
        // Set the bit in the set clear register that represents setting this pin
        self.pin
            .register_block
            .scr()
            .write(|w| unsafe { w.bits(0b1 << self.pin.number) });
    }

    pub fn set_low(&mut self) {
        // Set the bit in the set clear register that represents setting this pin
        self.pin
            .register_block
            .scr()
            .write(|w| unsafe { w.bits(0b1 << (self.pin.number + 16)) });
    }

    pub fn into_pin(self) -> Pin {
        self.pin
    }
}

impl ErrorType for Output {
    type Error = core::convert::Infallible;
}

impl OutputPin for Output {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }
}
