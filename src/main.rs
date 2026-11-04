#![no_std]

#![no_main]

use defmt::println;
use freely_rc_v2::{self as _, hal::{output::OutputConfig, pin::{Bank, Pin}}};
use freely_rc_v2::hal;


use cortex_m::delay::Delay;


#[cortex_m_rt::entry]
fn main() -> ! {

    let cp = cortex_m::Peripherals::take().unwrap(); // core peripherals (SysTick)

    let mut peripherals = hal::peripherals::Peripherals::init().expect("Already took peripherals");
    // peripherals.GPIOA.idt()

    let mut delay = Delay::new(cp.SYST, peripherals.clk_hz);



    let mut a_high = peripherals.pin_a10.take().expect("Already took pin").into_output(OutputConfig::default());
    let mut a_low = peripherals.pin_b1.take().expect("Already took pin").into_output(OutputConfig::default());
    let mut b_high = peripherals.pin_a9.take().expect("Already took pin").into_output(OutputConfig::default());
    let mut b_low = peripherals.pin_b0.take().expect("Already took pin").into_output(OutputConfig::default());
    let mut c_high = peripherals.pin_a8.take().expect("Already took pin").into_output(OutputConfig::default());
    let mut c_low = peripherals.pin_a7.take().expect("Already took pin").into_output(OutputConfig::default());


    let a_emf = peripherals.pin_a4.take().expect("Already took pin");
    let b_emf = peripherals.pin_a5.take().expect("Already took pin");
    let c_emf = peripherals.pin_a0.take().expect("Already took pin");
    
    let common_emf = peripherals.pin_a1.take().expect("Already took pin");

    let mut comp = peripherals.take_comparator();

    comp.with_non_inverting_pin(common_emf);
    comp.with_inverting_pin(a_emf);

    loop {
       println!("{}", comp.get_value());
        
    }
}