#![no_std]

#![no_main]

use freely_rc_v2 as _;
use at32f4xx_pac::at32f421 as pac;



#[cortex_m_rt::entry]
fn main() -> ! {
    let peripherals = pac::Peripherals::take().expect("Already took peripherals");


    // peripherals.GPIOA.idt()

    loop {
    }
}