use crate::hal::{
    input::{Input, InputMode},
    output::{Output, OutputConfig},
    register_extras::RegisterExtras,
};
pub use embedded_hal::digital::OutputPin;

use at32f4xx_pac::at32f421::{self, gpioa::RegisterBlock};

#[repr(u8)]
enum ModeConfiguration {
    Input = 0b00,
    Output = 0b01,
    Multiplexed = 0b10,
    Analog = 0b11,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

        Pin {
            bank,
            number,
            register_block,
        }
    }

    /// Set the pin as input mode, and consume it to form an input
    pub fn into_input(self, mode: InputMode) -> Input {
        self.register_block
            .cfgr()
            .set_bits(ModeConfiguration::Input as u32, self.number * 2, 2);

        Input::new(self, mode)
    }

    /// Set the pin as output mode, and consume it to form an output
    pub fn into_output(self, config: OutputConfig) -> Output {
        self.register_block
            .cfgr()
            .set_bits(ModeConfiguration::Output as u32, self.number * 2, 2);

        Output::new(self, config)
    }

    pub fn into_analog(self) -> Self {
        self.register_block
            .cfgr()
            .set_bits(ModeConfiguration::Analog as u32, self.number * 2, 2);
        self
    }

    /// Select a GPIO mux function using the function number from the reference manual.
    pub fn into_multiplexed(self, function: u8) -> Self {
        let mut pin = self;
        pin.set_multiplexed(function);
        pin
    }

    pub fn set_multiplexed(&mut self, function: u8) {
        assert!(
            function < 8,
            "GPIO multiplexing function must be between 0 and 7"
        );

        if self.number < 8 {
            self.register_block
                .muxl()
                .set_bits(u32::from(function), self.number * 4, 3);
        } else {
            self.register_block
                .muxh()
                .set_bits(u32::from(function), (self.number - 8) * 4, 3);
        }
        self.register_block.cfgr().set_bits(
            ModeConfiguration::Multiplexed as u32,
            self.number * 2,
            2,
        );
    }

    pub fn set_gpio_output_low(&mut self) {
        self.register_block
            .scr()
            .write(|w| unsafe { w.bits(1 << (self.number + 16)) });
        self.register_block
            .cfgr()
            .set_bits(ModeConfiguration::Output as u32, self.number * 2, 2);
    }
}

pub struct GPIOMultiplexed {}

pub struct GPIOAnalog {}
