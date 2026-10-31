use embedded_hal::digital::{ErrorType, InputPin};

use crate::hal::pin::Pin;
use crate::hal::register_extras::RegisterExtras;


#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum InputMode {
    Float = 0b00,
    PullUp = 0b01,
    PullDown = 0b10,
}

pub struct  Input {
    pin: Pin,
    mode: InputMode,
}

impl ErrorType for Input {
    type Error = core::convert::Infallible;
}




impl Input {

    pub(super) fn new (pin: Pin, mode: InputMode) -> Self {
        let mut i = Self {
            pin,
            mode,
        };
        
        
        i.set_mode(mode);

        i
    }

    pub fn out_of(self) -> Pin {
        self.pin
    }

    pub fn read(&self) -> bool {
        self.pin.register_block.idt().read().bits() & (1 << self.pin.number) != 0
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;

        self.pin.register_block.pull().set_bits(mode as u32, self.pin.number * 2, 2);
    }

}

impl InputPin for Input {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        
        Ok::<bool, core::convert::Infallible>(self.read() == true)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        
        Ok(self.read() == false)
    }
}