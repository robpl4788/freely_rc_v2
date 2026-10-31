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
    
    let ahben = unsafe { core::ptr::read_volatile(0x4002_1014 as *const u32) };
    let cfgr  = unsafe { core::ptr::read_volatile(0x4800_0000 as *const u32) };
    let odt   = unsafe { core::ptr::read_volatile(0x4800_0014 as *const u32) };
    let idt   = unsafe { core::ptr::read_volatile(0x4800_0010 as *const u32) };
    println!("AHBEN={=u32:#x} CFGR={=u32:#x} ODT={=u32:#x} IDT={=u32:#x}", ahben, cfgr, odt, idt);
    loop {
        a_low.set_high();
        
        delay.delay_ms(100);
        println!("high: {}", a_emf.read());
        let ahben = unsafe { core::ptr::read_volatile(0x4002_1014 as *const u32) };
        let cfgr  = unsafe { core::ptr::read_volatile(0x4800_0000 as *const u32) };
        let odt   = unsafe { core::ptr::read_volatile(0x4800_0014 as *const u32) };
        let idt   = unsafe { core::ptr::read_volatile(0x4800_0010 as *const u32) };
        println!("AHBEN={=u32:#x} CFGR={=u32:#x} ODT={=u32:#x} IDT={=u32:#x}", ahben, cfgr, odt, idt);
        
        delay.delay_ms(500);
        a_low.set_low();
        delay.delay_ms(100);
        println!("low: {}", a_emf.read());
        let ahben = unsafe { core::ptr::read_volatile(0x4002_1014 as *const u32) };
        let cfgr  = unsafe { core::ptr::read_volatile(0x4800_0000 as *const u32) };
        let odt   = unsafe { core::ptr::read_volatile(0x4800_0014 as *const u32) };
        let idt   = unsafe { core::ptr::read_volatile(0x4800_0010 as *const u32) };
        println!("AHBEN={=u32:#x} CFGR={=u32:#x} ODT={=u32:#x} IDT={=u32:#x}", ahben, cfgr, odt, idt);
    
        delay.delay_ms(500);

        
    }
}