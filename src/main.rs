#![no_std]
#![no_main]

use at32f4xx_pac::at32f421::interrupt;
#[cfg(not(feature = "spin-test"))]
use defmt::println;
use freely_rc_v2::hal;
use freely_rc_v2::{self as _};

#[cfg(all(feature = "spin-test", feature = "gate-driver-test"))]
compile_error!("Enable only one motor test feature at a time");

#[cortex_m_rt::entry]
fn main() -> ! {
    #[cfg(not(feature = "spin-test"))]
    println!("Firmware ready; motor bridge disarmed");
    defmt::info!("boot: entered main");
    let mut peripherals = hal::peripherals::Peripherals::init().expect("Already took peripherals");
    defmt::info!("boot: peripherals and system clock ready");
    freely_rc_v2::bldc_driver::initialize(&mut peripherals)
        .expect("Failed to initialize motor controller");
    defmt::info!("boot: motor controller initialized, bridge disarmed");

    #[cfg(feature = "spin-test")]
    {
        defmt::info!("spin_test: feature enabled; starting bounded test");
        defmt::info!(
            "spin_test,metric,stage,elapsed_ms,duty_permille,rpm,peak_rpm,zero_cross_us,commutations,closed_loop"
        );
        freely_rc_v2::bldc_driver::start_spin_test().expect("Failed to start opt-in spin test");
    }

    #[cfg(feature = "gate-driver-test")]
    {
        defmt::info!("gate_driver_test: playing 1000 Hz note for at most 1 second");
        freely_rc_v2::bldc_driver::play_note(1_000).expect("Failed to start gate-driver note test");
    }

    loop {
        cortex_m::asm::wfi();
        #[cfg(feature = "spin-test")]
        {
            if let Some(report) = freely_rc_v2::bldc_driver::take_spin_test_report() {
                defmt::info!(
                    "spin_test,data,{},{},{},{},{},{},{},{}",
                    report.stage,
                    report.elapsed_ms,
                    report.duty_permille,
                    report.estimated_rpm,
                    report.peak_rpm,
                    report.zero_cross_interval_us,
                    report.commutations,
                    report.closed_loop
                );
            }
        }
    }
}

#[interrupt]
fn ADC_CMP() {
    let exint = unsafe { at32f4xx_pac::at32f421::EXINT::steal() };
    if !exint.intsts().read().line21().bit_is_set() {
        return;
    }

    exint.intsts().write(|w| w.line21().clear_bit_by_one());

    freely_rc_v2::bldc_driver::on_comparator_interrupt();
}

#[interrupt]
fn TMR14() {
    freely_rc_v2::bldc_driver::on_control_interrupt();
}

#[interrupt]
fn TMR16() {
    freely_rc_v2::bldc_driver::on_commutation_interrupt();
}
