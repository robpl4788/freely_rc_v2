pub use embedded_hal::digital::OutputPin;
use crate::hal::input::{Input, InputMode};


use at32f4xx_pac::at32f421::{self, gpioa::RegisterBlock};

#[repr(u8)]
enum ModeConfiguration {
    Input = 0b00,
    Output = 0b01,
    Multiplexed = 0b10,
    Analog = 0b11,
}




pub enum Bank {
    A,
    B,
    C,
    F,
}

pub struct Pin {
    pub bank: Bank,
    pub number: u32,
    pub register_block: &'static RegisterBlock,
}

impl Pin {
    pub fn new(bank: Bank, number: u8) -> Self {
        assert!(number < 16, "The pins only go up to 15");
        
        let register_block;

        unsafe {
            register_block = &*match bank {
                Bank::A => at32f421::GPIOA::ptr(),
                Bank::B => at32f421::GPIOB::ptr(),
                Bank::C => at32f421::GPIOC::ptr(),
                Bank::F => at32f421::GPIOF::ptr(),
            }
        }

        Pin { bank, number, register_block }
    }

    pub fn into_input(self, mode: InputMode) -> Input {
        Input::new(self, mode)
    }
    
}



pub struct GPIOMultiplexed {

}

pub struct GPIOAnalog {

}