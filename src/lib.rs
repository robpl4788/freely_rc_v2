#![no_main]
#![no_std]

use defmt_rtt as _; // global logger

use at32f4xx_pac as _; // memory layout

use panic_probe as _;

pub mod bldc_driver;
pub mod board;
pub mod hal;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes a semihosting-capable debug tool exit
/// with status code 0.
pub fn exit() -> ! {
    semihosting::process::exit(0);
}

/// Hardfault handler.
///
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with an error. This seems better than the default, which is to spin in a
/// loop.
#[cortex_m_rt::exception]
unsafe fn HardFault(frame: &cortex_m_rt::ExceptionFrame) -> ! {
    defmt::error!(
        "HardFault: PC={:#010x}, LR={:#010x}",
        frame.pc(),
        frame.lr()
    );
    loop {
        cortex_m::asm::wfi();
    }
}

// defmt-test 0.3.0 has the limitation that this `#[tests]` attribute can only be used
// once within a crate. the module can be in any file but there can only be at most
// one `#[tests]` module in this library crate
#[cfg(test)]
#[defmt_test::tests]
mod unit_tests {
    use crate::bldc_driver::{
        CommutationState, filter_crossing_interval, motor_note_half_period_us, ramp_duty_permille,
        rpm_from_zero_cross_interval, zero_cross_delay,
    };
    use crate::hal::timer::{PwmChannel, TimerError};
    use defmt::assert;

    #[test]
    fn it_works() {
        assert!(true)
    }

    #[test]
    fn six_step_outputs_match_phase_sequence() {
        let expected = [
            (PwmChannel::Channel3, PwmChannel::Channel2, 2),
            (PwmChannel::Channel1, PwmChannel::Channel2, 0),
            (PwmChannel::Channel1, PwmChannel::Channel3, 1),
            (PwmChannel::Channel2, PwmChannel::Channel3, 2),
            (PwmChannel::Channel2, PwmChannel::Channel1, 0),
            (PwmChannel::Channel3, PwmChannel::Channel1, 1),
        ];
        let states = [
            CommutationState::State0,
            CommutationState::State1,
            CommutationState::State2,
            CommutationState::State3,
            CommutationState::State4,
            CommutationState::State5,
        ];

        for (state, expected_step) in states.iter().zip(expected.iter()) {
            let (pwm, low, floating) = state.phase_outputs();
            assert!(pwm == expected_step.0);
            assert!(low == expected_step.1);
            assert!(floating == expected_step.2);
        }
    }

    #[test]
    fn zero_cross_interval_is_filtered_and_halved() {
        assert!(filter_crossing_interval(0, 1_000) == 1_000);
        assert!(filter_crossing_interval(1_000, 1_400) == 1_100);
        assert!(zero_cross_delay(1_100) == 550);
        assert!(zero_cross_delay(0) == 1);
    }

    #[test]
    fn spin_test_ramps_duty_and_estimates_rpm() {
        assert!(ramp_duty_permille(80, 180, 0, 100_000) == 80);
        assert!(ramp_duty_permille(80, 180, 50_000, 100_000) == 130);
        assert!(ramp_duty_permille(180, 80, 50_000, 100_000) == 130);
        assert!(ramp_duty_permille(80, 180, 100_000, 100_000) == 180);
        assert!(rpm_from_zero_cross_interval(476, 7) == 2_998);
        assert!(rpm_from_zero_cross_interval(475, 7) > 3_000);
        assert!(rpm_from_zero_cross_interval(199, 7) > 3_000);
        assert!(rpm_from_zero_cross_interval(0, 7) == 0);
    }

    #[test]
    fn motor_note_frequency_is_bounded_and_rounded() {
        assert!(motor_note_half_period_us(1_000) == Ok(500));
        assert!(motor_note_half_period_us(100) == Ok(5_000));
        assert!(motor_note_half_period_us(5_000) == Ok(100));
        assert!(motor_note_half_period_us(99) == Err(TimerError::ToneFrequencyOutOfRange));
        assert!(motor_note_half_period_us(5_001) == Err(TimerError::ToneFrequencyOutOfRange));
    }
}
