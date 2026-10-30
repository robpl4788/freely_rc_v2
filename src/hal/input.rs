use embedded_hal::digital::{ErrorType, InputPin};

use crate::hal::pin::Pin;
use crate::hal::register_extras::RegisterExtras;

const INPUT_MODE: u32 = 0b00;

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

    pub fn new (pin: Pin, mode: InputMode) -> Self {
        let i = Self {
            pin,
            mode,
        };
        

        i.pin.register_block.cfgr().set_bits(INPUT_MODE, i.pin.number * 2, 2);
        
        i.pin.register_block.pull().set_bits(i.mode as u32, i.pin.number * 2, 2);

        i
    }

    pub fn out_of(self) -> Pin {
        self.pin
    }

    pub fn read(&mut self) -> bool {
        self.pin.register_block.odt().read_bit(self.pin.number)
    }

}

impl InputPin for Input {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        
        Ok::<bool, core::convert::Infallible>(self.read() == true)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        
        Ok::<bool, core::convert::Infallible>(self.read() == false)
    }
}