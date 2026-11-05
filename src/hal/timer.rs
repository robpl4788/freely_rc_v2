use crate::hal::register_extras::RegisterExtras;

const COUNTER_ENABLE_INDEX: u32 = 0;
const PERIOD_BUFFER_ENABLE_INDEX: u32 = 7;
const OVERFLOW_INTERRUPT_ENABLE_INDEX: u32 = 0;
const TMR1_OUTPUT_ENABLE_INDEX: u32 = 15;
const TMR1_CENTER_ALIGNED_MODE_INDEX: u32 = 5;
const TMR1_CLOCK_DIVIDER_INDEX: u32 = 8;

pub const TMR1_GPIO_MUX_FUNCTION: u8 = 2;
pub const TMR1_CH1C_GPIO_MUX_FUNCTION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerError {
    ZeroClock,
    ZeroFrequency,
    ZeroPeriod,
    PeriodTooShort,
    PeriodTooLong,
    UnsupportedDeadTime,
    UnsupportedResolution,
    OneShotUnsupported,
    NotConfigured,
    InvalidDuty,
    MotorAlreadyRunning,
    ToneFrequencyOutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerPeriod {
    pub prescaler: u16,
    pub period: u16,
    pub actual_period_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PwmConfig {
    pub prescaler: u16,
    pub period: u16,
    pub actual_frequency_hz: u32,
    pub dead_time_ticks: u8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmChannel {
    Channel1,
    Channel2,
    Channel3,
}

#[derive(Clone, Copy)]
enum TimerRegisters {
    Tmr6(&'static at32f4xx_pac::at32f421::tmr6::RegisterBlock),
    Tmr14(&'static at32f4xx_pac::at32f421::tmr14::RegisterBlock),
    Tmr16(&'static at32f4xx_pac::at32f421::tmr16::RegisterBlock),
    Tmr17(&'static at32f4xx_pac::at32f421::tmr16::RegisterBlock),
}

pub struct Timer {
    registers: TimerRegisters,
    timer_clock_hz: u32,
    one_shot_configured: bool,
}

impl Timer {
    pub(super) fn tmr6(timer_clock_hz: u32) -> Self {
        Self {
            registers: TimerRegisters::Tmr6(unsafe { &*at32f4xx_pac::at32f421::TMR6::ptr() }),
            timer_clock_hz,
            one_shot_configured: false,
        }
    }

    pub(super) fn tmr14(timer_clock_hz: u32) -> Self {
        Self {
            registers: TimerRegisters::Tmr14(unsafe { &*at32f4xx_pac::at32f421::TMR14::ptr() }),
            timer_clock_hz,
            one_shot_configured: false,
        }
    }

    pub(super) fn tmr16(timer_clock_hz: u32) -> Self {
        Self {
            registers: TimerRegisters::Tmr16(unsafe { &*at32f4xx_pac::at32f421::TMR16::ptr() }),
            timer_clock_hz,
            one_shot_configured: false,
        }
    }

    pub(super) fn tmr17(timer_clock_hz: u32) -> Self {
        Self {
            registers: TimerRegisters::Tmr17(unsafe { &*at32f4xx_pac::at32f421::TMR17::ptr() }),
            timer_clock_hz,
            one_shot_configured: false,
        }
    }

    /// Configure an up-counting period without starting the timer.
    pub fn configure_period_us(&mut self, period_us: u32) -> Result<TimerPeriod, TimerError> {
        if self.timer_clock_hz == 0 {
            return Err(TimerError::ZeroClock);
        }
        if period_us == 0 {
            return Err(TimerError::ZeroPeriod);
        }

        let requested_ticks =
            (u64::from(self.timer_clock_hz) * u64::from(period_us)).div_ceil(1_000_000);
        let prescaler_divisor = requested_ticks.div_ceil(1 << 16).max(1);
        let period_divisor = requested_ticks.div_ceil(prescaler_divisor);
        if prescaler_divisor > 1 << 16 || period_divisor > 1 << 16 {
            return Err(TimerError::PeriodTooLong);
        }

        let actual_ticks = prescaler_divisor * period_divisor;
        let actual_period_us = (actual_ticks * 1_000_000).div_ceil(u64::from(self.timer_clock_hz));

        self.set_counter_enabled(false);
        self.set_period_buffer_enabled(true);
        self.write_divider((prescaler_divisor - 1) as u16);
        self.write_period((period_divisor - 1) as u16);
        self.write_counter(0);
        self.generate_update_event();
        self.clear_update_flag();

        Ok(TimerPeriod {
            prescaler: (prescaler_divisor - 1) as u16,
            period: (period_divisor - 1) as u16,
            actual_period_us,
        })
    }

    /// Configure a free-running counter at the requested tick rate.
    pub fn configure_counter_hz(&mut self, counter_hz: u32) -> Result<TimerPeriod, TimerError> {
        if self.timer_clock_hz == 0 {
            return Err(TimerError::ZeroClock);
        }
        if counter_hz == 0 {
            return Err(TimerError::ZeroFrequency);
        }

        let prescaler_divisor = u64::from(self.timer_clock_hz)
            .div_ceil(u64::from(counter_hz))
            .max(1);
        if prescaler_divisor > 1 << 16 {
            return Err(TimerError::PeriodTooLong);
        }

        let actual_tick_hz = u64::from(self.timer_clock_hz) / prescaler_divisor;
        if actual_tick_hz != u64::from(counter_hz) {
            return Err(TimerError::UnsupportedResolution);
        }
        self.set_counter_enabled(false);
        self.write_divider((prescaler_divisor - 1) as u16);
        self.write_period(u16::MAX);
        self.write_counter(0);
        self.generate_update_event();
        self.clear_update_flag();

        Ok(TimerPeriod {
            prescaler: (prescaler_divisor - 1) as u16,
            period: u16::MAX,
            actual_period_us: (65_536_000_000u64).div_ceil(actual_tick_hz),
        })
    }

    pub fn configure_one_shot_us(&mut self, period_us: u32) -> Result<TimerPeriod, TimerError> {
        let registers = match self.registers {
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => registers,
            TimerRegisters::Tmr6(_) | TimerRegisters::Tmr14(_) => {
                return Err(TimerError::OneShotUnsupported);
            }
        };
        let period = self.configure_period_us(period_us)?;
        registers.ctrl1().set_bit(true, 3);
        self.one_shot_configured = true;
        Ok(period)
    }

    pub fn start(&self) {
        self.set_counter_enabled(true);
    }

    pub fn start_one_shot(&self) -> Result<(), TimerError> {
        if !self.one_shot_configured {
            return Err(TimerError::NotConfigured);
        }

        self.set_counter_enabled(false);
        self.clear_update_flag();
        self.write_counter(0);
        self.set_update_interrupt_enabled(true);
        self.set_counter_enabled(true);
        Ok(())
    }

    pub fn stop(&self) {
        self.set_counter_enabled(false);
    }

    /// Enable the timer's overflow interrupt source; the application must unmask its NVIC vector.
    pub fn enable_update_interrupt(&self) {
        self.set_update_interrupt_enabled(true);
    }

    pub fn disable_update_interrupt(&self) {
        self.set_update_interrupt_enabled(false);
    }

    pub fn update_pending(&self) -> bool {
        match self.registers {
            TimerRegisters::Tmr6(registers) => registers.ists().read().ovfif().is_overflow(),
            TimerRegisters::Tmr14(registers) => registers.ists().read().ovfif().is_overflow(),
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.ists().read().ovfif().is_overflow()
            }
        }
    }

    pub fn clear_update_flag(&self) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers.ists().write(|w| w.ovfif().clear());
            }
            TimerRegisters::Tmr14(registers) => {
                registers.ists().write(|w| w.ovfif().clear());
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.ists().write(|w| w.ovfif().clear());
            }
        }
    }

    pub fn counter(&self) -> u16 {
        match self.registers {
            TimerRegisters::Tmr6(registers) => registers.cval().read().bits(),
            TimerRegisters::Tmr14(registers) => registers.cval().read().bits(),
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.cval().read().bits()
            }
        }
    }

    fn set_counter_enabled(&self, enabled: bool) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers.ctrl1().set_bit(enabled, COUNTER_ENABLE_INDEX);
            }
            TimerRegisters::Tmr14(registers) => {
                registers.ctrl1().set_bit(enabled, COUNTER_ENABLE_INDEX);
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.ctrl1().set_bit(enabled, COUNTER_ENABLE_INDEX);
            }
        }
    }

    fn set_period_buffer_enabled(&self, enabled: bool) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers
                    .ctrl1()
                    .set_bit(enabled, PERIOD_BUFFER_ENABLE_INDEX);
            }
            TimerRegisters::Tmr14(registers) => {
                registers
                    .ctrl1()
                    .set_bit(enabled, PERIOD_BUFFER_ENABLE_INDEX);
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers
                    .ctrl1()
                    .set_bit(enabled, PERIOD_BUFFER_ENABLE_INDEX);
            }
        }
    }

    fn set_update_interrupt_enabled(&self, enabled: bool) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers
                    .iden()
                    .set_bit(enabled, OVERFLOW_INTERRUPT_ENABLE_INDEX);
            }
            TimerRegisters::Tmr14(registers) => {
                registers
                    .iden()
                    .set_bit(enabled, OVERFLOW_INTERRUPT_ENABLE_INDEX);
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers
                    .iden()
                    .set_bit(enabled, OVERFLOW_INTERRUPT_ENABLE_INDEX);
            }
        }
    }

    fn write_divider(&self, divider: u16) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers.div().write(|w| unsafe { w.bits(divider) });
            }
            TimerRegisters::Tmr14(registers) => {
                registers.div().write(|w| unsafe { w.bits(divider) });
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.div().write(|w| unsafe { w.bits(divider) });
            }
        }
    }

    fn write_period(&self, period: u16) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers.pr().write(|w| unsafe { w.bits(period) });
            }
            TimerRegisters::Tmr14(registers) => {
                registers.pr().write(|w| unsafe { w.bits(period) });
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.pr().write(|w| unsafe { w.bits(period) });
            }
        }
    }

    fn write_counter(&self, counter: u16) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers.cval().write(|w| unsafe { w.bits(counter) });
            }
            TimerRegisters::Tmr14(registers) => {
                registers.cval().write(|w| unsafe { w.bits(counter) });
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.cval().write(|w| unsafe { w.bits(counter) });
            }
        }
    }

    fn generate_update_event(&self) {
        match self.registers {
            TimerRegisters::Tmr6(registers) => {
                registers.swevt().write(|w| unsafe { w.bits(1) });
            }
            TimerRegisters::Tmr14(registers) => {
                registers.swevt().write(|w| unsafe { w.bits(1) });
            }
            TimerRegisters::Tmr16(registers) | TimerRegisters::Tmr17(registers) => {
                registers.swevt().write(|w| unsafe { w.bits(1) });
            }
        }
    }
}

pub struct Tmr1 {
    registers: &'static at32f4xx_pac::at32f421::tmr1::RegisterBlock,
    timer_clock_hz: u32,
    configuration: Option<PwmConfig>,
    running: bool,
}

impl Tmr1 {
    pub(super) fn new(timer_clock_hz: u32) -> Self {
        Self {
            registers: unsafe { &*at32f4xx_pac::at32f421::TMR1::ptr() },
            timer_clock_hz,
            configuration: None,
            running: false,
        }
    }

    /// Configure center-aligned three-phase PWM while leaving the timer and outputs disabled.
    /// GPIO pins must be routed to the TMR1 alternate function before starting.
    pub fn configure_three_phase_pwm(
        &mut self,
        nominal_frequency_hz: u32,
        dead_time_ticks: u8,
    ) -> Result<PwmConfig, TimerError> {
        if self.timer_clock_hz == 0 {
            return Err(TimerError::ZeroClock);
        }
        if nominal_frequency_hz == 0 {
            return Err(TimerError::ZeroFrequency);
        }
        if dead_time_ticks > 127 {
            return Err(TimerError::UnsupportedDeadTime);
        }

        let prescaler_divisor = (u64::from(self.timer_clock_hz))
            .div_ceil(2 * u64::from(nominal_frequency_hz) * u64::from(u16::MAX))
            .max(1);
        let period_denominator = 2 * u64::from(nominal_frequency_hz) * prescaler_divisor;
        let period_value =
            (u64::from(self.timer_clock_hz) + period_denominator / 2) / period_denominator;
        if period_value == 0 {
            return Err(TimerError::PeriodTooShort);
        }
        if prescaler_divisor > 1 << 16 || period_value > u64::from(u16::MAX) {
            return Err(TimerError::PeriodTooLong);
        }

        let actual_frequency_hz =
            (u64::from(self.timer_clock_hz) / (2 * prescaler_divisor * period_value)) as u32;
        let configuration = PwmConfig {
            prescaler: (prescaler_divisor - 1) as u16,
            period: period_value as u16,
            actual_frequency_hz,
            dead_time_ticks,
        };

        self.stop();
        self.registers
            .ctrl1()
            .set_bits(0, TMR1_CLOCK_DIVIDER_INDEX, 2);
        self.registers
            .ctrl1()
            .set_bits(0b01, TMR1_CENTER_ALIGNED_MODE_INDEX, 2);
        self.registers
            .ctrl1()
            .set_bit(true, PERIOD_BUFFER_ENABLE_INDEX);
        self.registers
            .div()
            .write(|w| unsafe { w.bits(configuration.prescaler) });
        self.registers
            .pr()
            .write(|w| unsafe { w.bits(configuration.period) });
        self.registers.cval().write(|w| unsafe { w.bits(0) });
        self.registers.c1dt().write(|w| unsafe { w.bits(0) });
        self.registers.c2dt().write(|w| unsafe { w.bits(0) });
        self.registers.c3dt().write(|w| unsafe { w.bits(0) });
        self.registers
            .brk()
            .set_bits(u32::from(dead_time_ticks), 0, 8);

        self.registers.cm1_output().set_bits(0b110, 4, 3);
        self.registers.cm1_output().set_bit(true, 3);
        self.registers.cm1_output().set_bits(0b110, 12, 3);
        self.registers.cm1_output().set_bit(true, 11);
        self.registers.cm2_output().set_bits(0b110, 4, 3);
        self.registers.cm2_output().set_bit(true, 3);

        self.generate_update_event();
        self.clear_update_flag();
        self.configuration = Some(configuration);
        Ok(configuration)
    }

    /// Set a channel's compare value in timer counts, from zero through the period value.
    pub fn set_duty_count(&self, channel: PwmChannel, count: u16) -> Result<(), TimerError> {
        let configuration = self.configuration.ok_or(TimerError::NotConfigured)?;
        if count > configuration.period {
            return Err(TimerError::InvalidDuty);
        }

        let register = match channel {
            PwmChannel::Channel1 => self.registers.c1dt(),
            PwmChannel::Channel2 => self.registers.c2dt(),
            PwmChannel::Channel3 => self.registers.c3dt(),
        };
        register.write(|w| unsafe { w.bits(count) });
        Ok(())
    }

    pub fn period(&self) -> Option<u16> {
        self.configuration.map(|configuration| configuration.period)
    }

    pub fn apply_six_step(
        &self,
        pwm_channel: PwmChannel,
        low_channel: PwmChannel,
        duty_count: u16,
    ) -> Result<(), TimerError> {
        let configuration = self.configuration.ok_or(TimerError::NotConfigured)?;
        if pwm_channel == low_channel || duty_count > configuration.period {
            return Err(TimerError::InvalidDuty);
        }

        self.registers.cctrl().set_bits(0, 0, 12);
        for channel in [
            PwmChannel::Channel1,
            PwmChannel::Channel2,
            PwmChannel::Channel3,
        ] {
            let mode = if channel == pwm_channel {
                0b110
            } else if channel == low_channel {
                0b100
            } else {
                0
            };
            self.set_channel_mode(channel, mode);
        }

        self.set_duty_count(pwm_channel, duty_count)?;
        let pwm_index = pwm_channel as u32;
        let low_index = low_channel as u32;
        let enable_mask =
            (1 << (pwm_index * 4)) | (1 << (pwm_index * 4 + 2)) | (1 << (low_index * 4 + 2));
        self.registers.cctrl().set_bits(enable_mask, 0, 12);
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), TimerError> {
        if self.configuration.is_none() {
            return Err(TimerError::NotConfigured);
        }

        self.generate_update_event();
        self.clear_update_flag();
        self.registers.brk().set_bit(true, TMR1_OUTPUT_ENABLE_INDEX);
        self.registers.ctrl1().set_bit(true, COUNTER_ENABLE_INDEX);
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.registers
            .brk()
            .set_bit(false, TMR1_OUTPUT_ENABLE_INDEX);
        self.registers.ctrl1().set_bit(false, COUNTER_ENABLE_INDEX);
        self.registers.cctrl().set_bits(0, 0, 12);
        self.running = false;
    }

    fn generate_update_event(&self) {
        self.registers.swevt().write(|w| unsafe { w.bits(1) });
    }

    fn clear_update_flag(&self) {
        self.registers.ists().write(|w| w.ovfif().clear());
    }

    fn set_channel_mode(&self, channel: PwmChannel, mode: u32) {
        match channel {
            PwmChannel::Channel1 => self.registers.cm1_output().set_bits(mode, 4, 3),
            PwmChannel::Channel2 => self.registers.cm1_output().set_bits(mode, 12, 3),
            PwmChannel::Channel3 => self.registers.cm2_output().set_bits(mode, 4, 3),
        }
    }
}

impl Drop for Tmr1 {
    fn drop(&mut self) {
        if self.running {
            self.stop();
        }
    }
}
