pub use embedded_hal::digital::OutputPin;
use crate::hal::{input::{Input, InputMode}, output::{Output, OutputConfig}, register_extras::RegisterExtras};


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
    pub(super) fn new(bank: Bank, number: u32) -> Self {
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

    /// Set the pin as input mode, and consume it to form an input
    pub fn into_input(self, mode: InputMode) -> Input {
        self.register_block.cfgr().set_bits(ModeConfiguration::Input as u32, self.number * 2, 2);

        Input::new(self, mode)
    }

    /// Set the pin as output mode, and consume it to form an output
    pub fn into_output(self, config: OutputConfig) -> Output {
        self.register_block.cfgr().set_bits(ModeConfiguration::Output as u32, self.number * 2, 2);

        Output::new(self, config)
    }
    
    
}



pub struct GPIOMultiplexed {

}

pub struct GPIOAnalog {

}