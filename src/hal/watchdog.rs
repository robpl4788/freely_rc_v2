use crate::hal::register_extras::RegisterExtras;

use at32f4xx_pac::at32f421::wdt::cmd::CMD_A;

const MAX_RELOAD_COUNT: u64 = 1 << 12;

const PRESCALER_DIVISORS: [u16; 7] = [4, 8, 16, 32, 64, 128, 256];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogConfig {
    pub prescaler: u16,
    pub reload: u16,
    pub actual_timeout_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogConfigError {
    ZeroClock,
    ZeroTimeout,
    TimeoutTooLong,
}

pub struct Watchdog {
    register_block: at32f4xx_pac::at32f421::WDT,
}

impl Watchdog {
    pub(super) fn new() -> Self {
        Self {
            register_block: unsafe { at32f4xx_pac::at32f421::WDT::steal() },
        }
    }

    pub fn start(
        &mut self,
        lick_hz: u32,
        timeout_ms: u32,
    ) -> Result<WatchdogConfig, WatchdogConfigError> {
        let config = Self::calculate_config(lick_hz, timeout_ms)?;

        self.write_command(CMD_A::Unlock);
        let divider_code = PRESCALER_DIVISORS
            .iter()
            .position(|divider| *divider == config.prescaler)
            .expect("watchdog prescaler must be one of the hardware divisors")
            as u32;
        self.register_block.div().set_bits(divider_code, 0, 3);
        while self.register_block.sts().read().divf().is_in_progress() {}

        self.register_block
            .rld()
            .set_bits(config.reload as u32, 0, 12);
        while self.register_block.sts().read().rldf().is_in_progress() {}

        self.write_command(CMD_A::Reload);
        self.write_command(CMD_A::Enable);

        Ok(config)
    }

    pub fn feed(&self) {
        self.write_command(CMD_A::Reload);
    }

    pub fn calculate_config(
        lick_hz: u32,
        timeout_ms: u32,
    ) -> Result<WatchdogConfig, WatchdogConfigError> {
        if lick_hz == 0 {
            return Err(WatchdogConfigError::ZeroClock);
        }
        if timeout_ms == 0 {
            return Err(WatchdogConfigError::ZeroTimeout);
        }

        let requested_ticks = u64::from(lick_hz) * u64::from(timeout_ms);
        let (prescaler, count) = PRESCALER_DIVISORS
            .iter()
            .find_map(|prescaler| {
                let denominator = u64::from(*prescaler) * 1000;
                let ticks =
                    requested_ticks / denominator + u64::from(requested_ticks % denominator != 0);
                (ticks <= MAX_RELOAD_COUNT).then_some((*prescaler, ticks))
            })
            .ok_or(WatchdogConfigError::TimeoutTooLong)?;

        let actual_ticks = count * u64::from(prescaler);
        let actual_timeout_us =
            (actual_ticks * 1_000_000 + u64::from(lick_hz) - 1) / u64::from(lick_hz);

        Ok(WatchdogConfig {
            prescaler,
            reload: (count - 1) as u16,
            actual_timeout_us,
        })
    }

    fn write_command(&self, command: CMD_A) {
        self.register_block.cmd().write(|writer| match command {
            CMD_A::Unlock => writer.cmd().unlock(),
            CMD_A::Reload => writer.cmd().reload(),
            CMD_A::Enable => writer.cmd().enable(),
        });
    }
}
