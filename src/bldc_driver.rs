use core::cell::RefCell;

use critical_section::Mutex;

use crate::hal::{
    comparator::{Comparator, ComparatorEdge},
    peripherals::Peripherals,
    pin::Pin,
    timer::{
        PwmChannel, TMR1_CH1C_GPIO_MUX_FUNCTION, TMR1_GPIO_MUX_FUNCTION, Timer, TimerError, Tmr1,
    },
};

const CONTROL_PERIOD_US: u32 = 50;
const INITIAL_COMMUTATION_US: u32 = 10_000;
const MIN_COMMUTATION_US: u32 = 2_000;
const COMMUTATION_RAMP_US: u32 = 100;
const MIN_ZERO_CROSS_US: u32 = 200;
const MAX_ZERO_CROSS_US: u32 = 60_000;
const MOTOR_POLE_PAIRS: u32 = 7;
const SPIN_TEST_START_DUTY_PERMILLE: u16 = 80;
const SPIN_TEST_MAX_DUTY_PERMILLE: u16 = 180;
const SPIN_TEST_MAX_RPM: u32 = 3_000;
const SPIN_TEST_RAMP_TICKS: u32 = 100_000;
const SPIN_TEST_HOLD_TICKS: u32 = 20_000;
const SPIN_TEST_LOG_TICKS: u32 = 4_000;
const SPIN_TEST_DUTY_UPDATE_TICKS: u32 = 500;
const SPIN_TEST_LOCK_TIMEOUT_TICKS: u32 = 100_000;
const MOTOR_NOTE_MIN_FREQUENCY_HZ: u32 = 100;
const MOTOR_NOTE_MAX_FREQUENCY_HZ: u32 = 5_000;
const MOTOR_NOTE_DURATION_TICKS: u32 = 20_000;

static MOTOR: Mutex<RefCell<Option<BLDC>>> = Mutex::new(RefCell::new(None));

#[derive(Clone, Copy)]
pub struct SpinTestReport {
    pub stage: u8,
    pub elapsed_ms: u32,
    pub duty_permille: u16,
    pub estimated_rpm: u32,
    pub peak_rpm: u32,
    pub zero_cross_interval_us: u32,
    pub commutations: u32,
    pub closed_loop: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpinTestStage {
    WaitingForLock,
    RampUp,
    Hold,
    RampDown,
    Complete,
    StartupFailed,
    Overspeed,
}

impl SpinTestStage {
    fn code(self) -> u8 {
        match self {
            Self::WaitingForLock => 0,
            Self::RampUp => 1,
            Self::Hold => 2,
            Self::RampDown => 3,
            Self::Complete => 4,
            Self::StartupFailed => 5,
            Self::Overspeed => 6,
        }
    }
}

#[derive(Clone, Copy)]
struct SpinTest {
    stage: SpinTestStage,
    elapsed_ticks: u32,
    stage_ticks: u32,
    log_ticks: u32,
    peak_rpm: u32,
}

#[derive(Clone, Copy)]
struct MotorNote {
    half_period_us: u32,
    elapsed_ticks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommutationState {
    State0,
    State1,
    State2,
    State3,
    State4,
    State5,
}

#[derive(Clone, Copy)]
struct Step {
    pwm: PwmChannel,
    low: PwmChannel,
    floating_phase: usize,
}

impl CommutationState {
    fn next(self) -> Self {
        match self {
            Self::State0 => Self::State1,
            Self::State1 => Self::State2,
            Self::State2 => Self::State3,
            Self::State3 => Self::State4,
            Self::State4 => Self::State5,
            Self::State5 => Self::State0,
        }
    }

    fn step(self) -> Step {
        match self {
            Self::State0 => Step {
                pwm: PwmChannel::Channel3,
                low: PwmChannel::Channel2,
                floating_phase: 2,
            },
            Self::State1 => Step {
                pwm: PwmChannel::Channel1,
                low: PwmChannel::Channel2,
                floating_phase: 0,
            },
            Self::State2 => Step {
                pwm: PwmChannel::Channel1,
                low: PwmChannel::Channel3,
                floating_phase: 1,
            },
            Self::State3 => Step {
                pwm: PwmChannel::Channel2,
                low: PwmChannel::Channel3,
                floating_phase: 2,
            },
            Self::State4 => Step {
                pwm: PwmChannel::Channel2,
                low: PwmChannel::Channel1,
                floating_phase: 0,
            },
            Self::State5 => Step {
                pwm: PwmChannel::Channel3,
                low: PwmChannel::Channel1,
                floating_phase: 1,
            },
        }
    }

    pub fn phase_outputs(self) -> (PwmChannel, PwmChannel, usize) {
        let step = self.step();
        (step.pwm, step.low, step.floating_phase)
    }

    fn zero_cross_edge(self) -> ComparatorEdge {
        // The comparator sees neutral minus phase, so its edge is opposite the phase crossing.
        match self {
            Self::State0 | Self::State2 | Self::State4 => ComparatorEdge::Falling,
            Self::State1 | Self::State3 | Self::State5 => ComparatorEdge::Rising,
        }
    }
}

pub struct BLDC {
    pwm: Tmr1,
    elapsed: Timer,
    control: Timer,
    commutation: Timer,
    comparator: Comparator,
    output_pins: [Pin; 6],
    floating_pins: [Option<Pin>; 3],
    current_floating: Option<usize>,
    state: CommutationState,
    armed: bool,
    closed_loop: bool,
    duty: f32,
    startup_interval_us: u32,
    startup_ticks: u16,
    last_zero_cross_us: Option<u16>,
    watchdog_counter_us: u16,
    watchdog_elapsed_us: u32,
    filtered_interval_us: u32,
    valid_crossings: u8,
    commutations: u32,
    spin_test: Option<SpinTest>,
    spin_test_report: Option<SpinTestReport>,
    motor_note: Option<MotorNote>,
}

// Register access is confined to this single-core MCU and serialized by MOTOR's critical section.
unsafe impl Send for BLDC {}

impl BLDC {
    fn new(peripherals: &mut Peripherals) -> Result<Self, TimerError> {
        // Board mux mapping: PA10/PB1=CH3/CH3N, PA9/PB0=CH2/CH2N,
        // PA8/PA7=CH1/CH1C (PA7 uses mux function 1; the others use 2).
        let mut output_pins = [
            peripherals.pin_a10.take().expect("PA10 already taken"),
            peripherals.pin_b1.take().expect("PB1 already taken"),
            peripherals.pin_a9.take().expect("PA9 already taken"),
            peripherals.pin_b0.take().expect("PB0 already taken"),
            peripherals.pin_a8.take().expect("PA8 already taken"),
            peripherals.pin_a7.take().expect("PA7 already taken"),
        ];
        for pin in &mut output_pins {
            pin.set_gpio_output_low();
        }

        let floating_pins = [
            Some(peripherals.pin_a4.take().expect("PA4 already taken")),
            Some(peripherals.pin_a5.take().expect("PA5 already taken")),
            Some(peripherals.pin_a0.take().expect("PA0 already taken")),
        ];
        let common_emf = peripherals.pin_a1.take().expect("PA1 already taken");

        let mut comparator = peripherals.take_comparator();
        comparator.with_non_inverting_pin(common_emf);

        let mut pwm = peripherals.take_tmr1();
        pwm.configure_three_phase_pwm(24_000, 60)?;
        let mut elapsed = peripherals.take_tmr6();
        elapsed.configure_counter_hz(1_000_000)?;
        elapsed.start();
        let mut control = peripherals.take_tmr14();
        control.configure_period_us(CONTROL_PERIOD_US)?;
        let commutation = peripherals.take_tmr16();

        Ok(Self {
            pwm,
            elapsed,
            control,
            commutation,
            comparator,
            output_pins,
            floating_pins,
            current_floating: None,
            state: CommutationState::State0,
            armed: false,
            closed_loop: false,
            duty: 0.0,
            startup_interval_us: INITIAL_COMMUTATION_US,
            startup_ticks: 0,
            last_zero_cross_us: None,
            watchdog_counter_us: 0,
            watchdog_elapsed_us: 0,
            filtered_interval_us: 0,
            valid_crossings: 0,
            commutations: 0,
            spin_test: None,
            spin_test_report: None,
            motor_note: None,
        })
    }

    fn start(&mut self, duty: f32) -> Result<(), TimerError> {
        if !duty.is_finite() {
            return Err(TimerError::InvalidDuty);
        }
        if self.armed {
            return Ok(());
        }

        self.duty = duty.clamp(0.05, 0.9);
        self.state = CommutationState::State0;
        self.startup_interval_us = INITIAL_COMMUTATION_US;
        self.startup_ticks = 0;
        self.last_zero_cross_us = None;
        self.watchdog_counter_us = self.elapsed.counter();
        self.watchdog_elapsed_us = 0;
        self.filtered_interval_us = 0;
        self.valid_crossings = 0;
        self.commutations = 0;
        self.spin_test = None;
        self.spin_test_report = None;
        self.motor_note = None;
        self.closed_loop = false;
        self.armed = true;

        self.set_output_mux(true);
        if let Err(error) = self.apply_step().and_then(|_| self.pwm.start()) {
            self.stop();
            return Err(error);
        }
        self.comparator.disable_interrupt();
        self.control.clear_update_flag();
        self.control.enable_update_interrupt();
        self.control.start();
        Ok(())
    }

    fn play_note(&mut self, frequency_hz: u32) -> Result<(), TimerError> {
        let half_period_us = motor_note_half_period_us(frequency_hz)?;
        if self.armed {
            return Err(TimerError::MotorAlreadyRunning);
        }

        self.comparator.disable_interrupt();
        self.control.stop();
        self.control.disable_update_interrupt();
        self.commutation.stop();
        self.commutation.disable_update_interrupt();

        self.duty = f32::from(SPIN_TEST_START_DUTY_PERMILLE) / 1_000.0;
        self.state = CommutationState::State1;
        self.armed = true;
        self.closed_loop = false;
        self.spin_test = None;
        self.spin_test_report = None;
        self.set_output_mux(true);

        let result = self
            .apply_step()
            .and_then(|_| self.pwm.start())
            .and_then(|_| {
                self.commutation
                    .configure_one_shot_us(half_period_us)
                    .map(|_| ())
            })
            .and_then(|_| {
                self.motor_note = Some(MotorNote {
                    half_period_us,
                    elapsed_ticks: 0,
                });
                self.commutation.start_one_shot()
            });
        if let Err(error) = result {
            self.stop();
            return Err(error);
        }
        self.control.clear_update_flag();
        self.control.enable_update_interrupt();
        self.control.start();
        Ok(())
    }

    fn stop(&mut self) {
        self.armed = false;
        self.closed_loop = false;
        self.motor_note = None;
        self.comparator.disable_interrupt();
        self.control.stop();
        self.control.disable_update_interrupt();
        self.commutation.stop();
        self.commutation.disable_update_interrupt();
        self.pwm.stop();
        self.set_output_mux(false);
    }

    fn on_control_tick(&mut self) -> Result<(), TimerError> {
        if !self.armed {
            return Ok(());
        }

        if let Some(note) = self.motor_note.as_mut() {
            note.elapsed_ticks = note.elapsed_ticks.saturating_add(1);
            if note.elapsed_ticks >= MOTOR_NOTE_DURATION_TICKS {
                self.stop();
            }
            return Ok(());
        }

        self.advance_spin_test()?;
        if !self.armed {
            return Ok(());
        }

        if self.closed_loop {
            let now = self.elapsed.counter();
            self.watchdog_elapsed_us = self
                .watchdog_elapsed_us
                .saturating_add(u32::from(now.wrapping_sub(self.watchdog_counter_us)));
            self.watchdog_counter_us = now;
            if self.watchdog_elapsed_us > self.filtered_interval_us.saturating_mul(3) {
                if self.spin_test.is_some() {
                    self.finish_spin_test(SpinTestStage::StartupFailed);
                } else {
                    self.stop();
                }
            }
            return Ok(());
        }

        self.startup_ticks = self.startup_ticks.saturating_add(1);
        let ticks_per_step = (self.startup_interval_us / CONTROL_PERIOD_US).max(1) as u16;
        if self.startup_ticks >= ticks_per_step {
            self.startup_ticks = 0;
            self.advance()?;
            self.startup_interval_us = self
                .startup_interval_us
                .saturating_sub(COMMUTATION_RAMP_US)
                .max(MIN_COMMUTATION_US);
            if self.startup_interval_us == MIN_COMMUTATION_US {
                self.comparator
                    .enable_interrupt(self.state.zero_cross_edge());
            }
        }
        Ok(())
    }

    fn on_zero_cross(&mut self) -> Result<(), TimerError> {
        if !self.armed || self.startup_interval_us > MIN_COMMUTATION_US {
            return Ok(());
        }

        let now = self.elapsed.counter();
        if let Some(previous) = self.last_zero_cross_us.replace(now) {
            let interval = u32::from(now.wrapping_sub(previous));
            if !(MIN_ZERO_CROSS_US..=MAX_ZERO_CROSS_US).contains(&interval) {
                self.valid_crossings = 0;
                self.closed_loop = false;
                return Ok(());
            }
            let estimated_rpm = rpm_from_zero_cross_interval(interval, MOTOR_POLE_PAIRS);
            let overspeed = if let Some(test) = self.spin_test.as_mut() {
                test.peak_rpm = test.peak_rpm.max(estimated_rpm);
                estimated_rpm >= SPIN_TEST_MAX_RPM
            } else {
                false
            };
            if overspeed {
                self.finish_spin_test(SpinTestStage::Overspeed);
                return Ok(());
            }
            self.filtered_interval_us =
                filter_crossing_interval(self.filtered_interval_us, interval);
            self.watchdog_counter_us = now;
            self.watchdog_elapsed_us = 0;
            self.valid_crossings = self.valid_crossings.saturating_add(1);
            if self.valid_crossings >= 3 {
                self.closed_loop = true;
                let just_locked = if let Some(test) = self.spin_test.as_mut() {
                    if test.stage == SpinTestStage::WaitingForLock {
                        test.stage = SpinTestStage::RampUp;
                        test.stage_ticks = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if just_locked {
                    self.publish_spin_test_report();
                }
                self.comparator.disable_interrupt();
                let delay_us = zero_cross_delay(self.filtered_interval_us);
                self.commutation.stop();
                self.commutation.configure_one_shot_us(delay_us)?;
                self.commutation.start_one_shot()?;
            }
        }
        Ok(())
    }

    fn on_commutation_timeout(&mut self) -> Result<(), TimerError> {
        self.commutation.stop();
        self.commutation.clear_update_flag();
        if !self.armed {
            return Ok(());
        }
        if let Some(note) = self.motor_note {
            self.state = match self.state {
                CommutationState::State1 => CommutationState::State4,
                CommutationState::State4 => CommutationState::State1,
                _ => return Err(TimerError::NotConfigured),
            };
            self.commutations = self.commutations.saturating_add(1);
            self.apply_step()?;
            self.commutation
                .configure_one_shot_us(note.half_period_us)?;
            self.commutation.start_one_shot()?;
            self.motor_note = Some(note);
            return Ok(());
        }
        self.advance()
    }

    fn advance(&mut self) -> Result<(), TimerError> {
        self.comparator.disable_interrupt();
        self.state = self.state.next();
        self.commutations = self.commutations.saturating_add(1);
        self.apply_step()?;
        if self.closed_loop {
            self.comparator
                .enable_interrupt(self.state.zero_cross_edge());
        }
        Ok(())
    }

    fn apply_step(&mut self) -> Result<(), TimerError> {
        let step = self.state.step();
        if self.current_floating != Some(step.floating_phase) {
            let new_pin = self.floating_pins[step.floating_phase]
                .take()
                .expect("floating phase pin missing");
            let old_pin = self.comparator.with_inverting_pin(new_pin);
            if let (Some(old_phase), Some(old_pin)) = (self.current_floating, old_pin) {
                self.floating_pins[old_phase] = Some(old_pin);
            }
            self.current_floating = Some(step.floating_phase);
        }
        let period = self.pwm.period().ok_or(TimerError::NotConfigured)?;
        let duty_count = (f32::from(period) * self.duty) as u16;
        self.pwm.apply_six_step(step.pwm, step.low, duty_count)
    }

    fn set_output_mux(&mut self, enabled: bool) {
        if enabled {
            for (index, pin) in self.output_pins.iter_mut().enumerate() {
                pin.set_multiplexed(if index == 5 {
                    TMR1_CH1C_GPIO_MUX_FUNCTION
                } else {
                    TMR1_GPIO_MUX_FUNCTION
                });
            }
        } else {
            for pin in &mut self.output_pins {
                pin.set_gpio_output_low();
            }
        }
    }

    fn advance_spin_test(&mut self) -> Result<(), TimerError> {
        let Some(test) = self.spin_test.as_mut() else {
            return Ok(());
        };
        test.elapsed_ticks = test.elapsed_ticks.saturating_add(1);
        test.log_ticks = test.log_ticks.saturating_add(1);
        test.stage_ticks = test.stage_ticks.saturating_add(1);
        let (stage, stage_ticks) = (test.stage, test.stage_ticks);

        if stage == SpinTestStage::WaitingForLock {
            if test.elapsed_ticks >= SPIN_TEST_LOCK_TIMEOUT_TICKS {
                self.finish_spin_test(SpinTestStage::StartupFailed);
            } else if test.log_ticks >= SPIN_TEST_LOG_TICKS {
                self.publish_spin_test_report();
            }
            return Ok(());
        }

        if stage == SpinTestStage::RampUp {
            if stage_ticks % SPIN_TEST_DUTY_UPDATE_TICKS == 0 || stage_ticks >= SPIN_TEST_RAMP_TICKS
            {
                let duty = ramp_duty_permille(
                    SPIN_TEST_START_DUTY_PERMILLE,
                    SPIN_TEST_MAX_DUTY_PERMILLE,
                    stage_ticks,
                    SPIN_TEST_RAMP_TICKS,
                );
                self.set_duty_permille(duty)?;
            }
            if stage_ticks >= SPIN_TEST_RAMP_TICKS {
                if let Some(test) = self.spin_test.as_mut() {
                    test.stage = SpinTestStage::Hold;
                    test.stage_ticks = 0;
                }
                self.publish_spin_test_report();
            }
        } else if stage == SpinTestStage::Hold && stage_ticks >= SPIN_TEST_HOLD_TICKS {
            if let Some(test) = self.spin_test.as_mut() {
                test.stage = SpinTestStage::RampDown;
                test.stage_ticks = 0;
            }
            self.publish_spin_test_report();
        } else if stage == SpinTestStage::RampDown {
            if stage_ticks % SPIN_TEST_DUTY_UPDATE_TICKS == 0 || stage_ticks >= SPIN_TEST_RAMP_TICKS
            {
                let duty = ramp_duty_permille(
                    SPIN_TEST_MAX_DUTY_PERMILLE,
                    SPIN_TEST_START_DUTY_PERMILLE,
                    stage_ticks,
                    SPIN_TEST_RAMP_TICKS,
                );
                self.set_duty_permille(duty)?;
            }
            if stage_ticks >= SPIN_TEST_RAMP_TICKS {
                self.finish_spin_test(SpinTestStage::Complete);
                return Ok(());
            }
        }

        if self
            .spin_test
            .is_some_and(|test| test.log_ticks >= SPIN_TEST_LOG_TICKS)
        {
            self.publish_spin_test_report();
        }
        Ok(())
    }

    fn set_duty_permille(&mut self, duty_permille: u16) -> Result<(), TimerError> {
        self.duty = f32::from(duty_permille) / 1_000.0;
        if self.armed {
            let period = self.pwm.period().ok_or(TimerError::NotConfigured)?;
            let duty_count = (u32::from(period) * u32::from(duty_permille) / 1_000) as u16;
            self.pwm.set_duty_count(self.state.step().pwm, duty_count)?;
        }
        Ok(())
    }

    fn finish_spin_test(&mut self, stage: SpinTestStage) {
        if let Some(test) = self.spin_test.as_mut() {
            test.stage = stage;
        }
        self.publish_spin_test_report();
        self.stop();
    }

    fn fail_controller(&mut self) {
        if self.spin_test.is_some() {
            self.finish_spin_test(SpinTestStage::StartupFailed);
        } else {
            self.stop();
        }
    }

    fn publish_spin_test_report(&mut self) {
        let Some(test) = self.spin_test.as_mut() else {
            return;
        };
        self.spin_test_report = Some(SpinTestReport {
            stage: test.stage.code(),
            elapsed_ms: test.elapsed_ticks / 20,
            duty_permille: (self.duty * 1_000.0) as u16,
            estimated_rpm: rpm_from_zero_cross_interval(
                self.filtered_interval_us,
                MOTOR_POLE_PAIRS,
            ),
            peak_rpm: test.peak_rpm,
            zero_cross_interval_us: self.filtered_interval_us,
            commutations: self.commutations,
            closed_loop: self.closed_loop,
        });
        test.log_ticks = 0;
    }
}

pub fn filter_crossing_interval(previous_us: u32, sample_us: u32) -> u32 {
    if previous_us == 0 {
        sample_us
    } else {
        ((u64::from(previous_us) * 3 + u64::from(sample_us)) / 4) as u32
    }
}

pub fn zero_cross_delay(filtered_interval_us: u32) -> u32 {
    (filtered_interval_us / 2).max(1)
}

pub fn motor_note_half_period_us(frequency_hz: u32) -> Result<u32, TimerError> {
    if !(MOTOR_NOTE_MIN_FREQUENCY_HZ..=MOTOR_NOTE_MAX_FREQUENCY_HZ).contains(&frequency_hz) {
        return Err(TimerError::ToneFrequencyOutOfRange);
    }
    Ok((500_000 + frequency_hz / 2) / frequency_hz)
}

pub fn rpm_from_zero_cross_interval(interval_us: u32, pole_pairs: u32) -> u32 {
    if interval_us == 0 || pole_pairs == 0 {
        0
    } else {
        (10_000_000u64 / (u64::from(interval_us) * u64::from(pole_pairs))) as u32
    }
}

pub fn ramp_duty_permille(start: u16, end: u16, elapsed_ticks: u32, duration_ticks: u32) -> u16 {
    if duration_ticks == 0 || elapsed_ticks >= duration_ticks {
        return end;
    }

    let start = i64::from(start);
    let delta = i64::from(end) - start;
    (start + delta * i64::from(elapsed_ticks) / i64::from(duration_ticks)) as u16
}

pub fn initialize(peripherals: &mut Peripherals) -> Result<(), TimerError> {
    critical_section::with(|cs| {
        let mut motor = MOTOR.borrow(cs).borrow_mut();
        if motor.is_some() {
            return Ok(());
        }
        *motor = Some(BLDC::new(peripherals)?);
        Ok(())
    })?;
    unsafe {
        cortex_m::peripheral::NVIC::unmask(at32f4xx_pac::at32f421::Interrupt::TMR14);
        cortex_m::peripheral::NVIC::unmask(at32f4xx_pac::at32f421::Interrupt::TMR16);
    }
    Ok(())
}

/// Arm the motor explicitly. No startup or power-on motor test is performed.
pub fn start(duty: f32) -> Result<(), TimerError> {
    critical_section::with(|cs| {
        MOTOR
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .ok_or(TimerError::NotConfigured)?
            .start(duty)
    })
}

/// Starts the bounded 8%-to-18%-to-8% performance test.
pub fn start_spin_test() -> Result<(), TimerError> {
    critical_section::with(|cs| {
        let mut motor = MOTOR.borrow(cs).borrow_mut();
        let motor = motor.as_mut().ok_or(TimerError::NotConfigured)?;
        if motor.armed {
            return Err(TimerError::MotorAlreadyRunning);
        }

        motor.start(f32::from(SPIN_TEST_START_DUTY_PERMILLE) / 1_000.0)?;
        motor.spin_test = Some(SpinTest {
            stage: SpinTestStage::WaitingForLock,
            elapsed_ticks: 0,
            stage_ticks: 0,
            log_ticks: 0,
            peak_rpm: 0,
        });
        motor.publish_spin_test_report();
        Ok(())
    })
}

/// Drives the motor as a speaker at 8% duty for at most one second.
/// The requested frequency must be between 100 Hz and 5 kHz.
pub fn play_note(frequency_hz: u32) -> Result<(), TimerError> {
    critical_section::with(|cs| {
        MOTOR
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .ok_or(TimerError::NotConfigured)?
            .play_note(frequency_hz)
    })
}

pub fn take_spin_test_report() -> Option<SpinTestReport> {
    critical_section::with(|cs| {
        MOTOR
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .and_then(|motor| motor.spin_test_report.take())
    })
}

pub fn stop() {
    critical_section::with(|cs| {
        if let Some(motor) = MOTOR.borrow(cs).borrow_mut().as_mut() {
            motor.stop();
        }
    });
}

pub fn on_control_interrupt() {
    critical_section::with(|cs| {
        if let Some(motor) = MOTOR.borrow(cs).borrow_mut().as_mut() {
            if motor.control.update_pending() {
                motor.control.clear_update_flag();
                if motor.on_control_tick().is_err() {
                    defmt::error!("BLDC control failed; motor stopped");
                    motor.fail_controller();
                }
            }
        }
    });
}

pub fn on_comparator_interrupt() {
    critical_section::with(|cs| {
        if let Some(motor) = MOTOR.borrow(cs).borrow_mut().as_mut() {
            if motor.on_zero_cross().is_err() {
                defmt::error!("BLDC zero-cross timing failed; motor stopped");
                motor.fail_controller();
            }
        }
    });
}

pub fn on_commutation_interrupt() {
    critical_section::with(|cs| {
        if let Some(motor) = MOTOR.borrow(cs).borrow_mut().as_mut() {
            if motor.commutation.update_pending() {
                motor.commutation.clear_update_flag();
                if motor.on_commutation_timeout().is_err() {
                    defmt::error!("BLDC commutation failed; motor stopped");
                    motor.fail_controller();
                }
            }
        }
    });
}
