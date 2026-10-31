#![no_std]

#![no_main]

use defmt::println;
use freely_rc_v2::{self as _, hal::{output::OutputConfig, pin::{Bank, Pin}}};
use freely_rc_v2::hal;


use cortex_m::delay::Delay;

const HCLK_HZ: u32 = 8_000_000; // reset clock, see the clock notes


#[cortex_m_rt::entry]
fn main() -> ! {

    let cp = cortex_m::Peripherals::take().unwrap(); // core peripherals (SysTick)

    let mut peripherals = hal::peripherals::Peripherals::init().expect("Already took peripherals");
    // peripherals.GPIOA.idt()

    let mut delay = Delay::new(cp.SYST, HCLK_HZ);



    let mut a_high = peripherals.pin_a10.take().expect("Already took pin").into_output(OutputConfig::default());
    let mut a_low = peripherals.pin_b1.take().expect("Already took pin").into_output(OutputConfig::default());
    let b_high = peripherals.pin_a9.take().expect("Already took pin").into_output(OutputConfig::default());
    let b_low = peripherals.pin_b0.take().expect("Already took pin").into_output(OutputConfig::default());
    let c_high = peripherals.pin_a8.take().expect("Already took pin").into_output(OutputConfig::default());
    let c_low = peripherals.pin_a7.take().expect("Already took pin").into_output(OutputConfig::default());


    let a_emf = peripherals.pin_a4.take().expect("Already took pin").into_input(hal::input::InputMode::PullDown);
    let b_emf = peripherals.pin_a5.take().expect("Already took pin").into_input(hal::input::InputMode::Float);
    let c_emf = peripherals.pin_a0.take().expect("Already took pin").into_input(hal::input::InputMode::Float);
    
    loop {
        a_high.set_high();
        
        delay.delay_us(500);
  
        a_high.set_low();
        delay.delay_us(100);
       

        
    }
}