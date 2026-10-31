#![no_std]

#![no_main]

use freely_rc_v2::{self as _, hal::{output::OutputConfig, pin::{Bank, Pin}}};
use at32f4xx_pac::at32f421 as pac;



#[cortex_m_rt::entry]
fn main() -> ! {
    let peripherals = pac::Peripherals::take().expect("Already took peripherals");

    let a = Pin::new(Bank::A, 0);
    let b = a.into_output(OutputConfig::default());
    // peripherals.GPIOA.idt()

    loop {
    }
}