use crate::hal::register_extras::RegisterExtras;

const ADC_ENABLE_INDEX: u32 = 0;
const ADC_CALIBRATION_INDEX: u32 = 2;
const ADC_INTERNAL_SENSOR_ENABLE_INDEX: u32 = 23;
const ORDINARY_TRIGGER_ENABLE_INDEX: u32 = 20;
const ORDINARY_TRIGGER_SOURCE_INDEX: u32 = 17;
const ORDINARY_SOFTWARE_TRIGGER_INDEX: u32 = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcChannel {
    External(u8),
    Ground,
    Temperature,
    InternalReference,
}

impl AdcChannel {
    fn index(self) -> Result<u8, AdcError> {
        match self {
            Self::External(channel) if channel <= 14 => Ok(channel),
            Self::External(_) => Err(AdcError::InvalidExternalChannel),
            Self::Ground => Ok(15),
            Self::Temperature => Ok(16),
            Self::InternalReference => Ok(17),
        }
    }

    fn uses_internal_sensor(self) -> bool {
        matches!(self, Self::Temperature | Self::InternalReference)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AdcSampleTime {
    Cycles1_5 = 0,
    Cycles7_5 = 1,
    Cycles13_5 = 2,
    Cycles28_5 = 3,
    Cycles41_5 = 4,
    Cycles55_5 = 5,
    Cycles71_5 = 6,
    Cycles239_5 = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcError {
    ZeroSystemClock,
    InvalidExternalChannel,
    NotPowered,
}

pub struct Adc1 {
    register_block: at32f4xx_pac::at32f421::ADC1,
    system_clock_hz: u32,
    powered: bool,
}

impl Adc1 {
    pub(super) fn new() -> Self {
        Self {
            register_block: unsafe { at32f4xx_pac::at32f421::ADC1::steal() },
            system_clock_hz: 0,
            powered: false,
        }
    }

    pub fn power_on(&mut self, system_clock_hz: u32) -> Result<(), AdcError> {
        if system_clock_hz == 0 {
            return Err(AdcError::ZeroSystemClock);
        }

        self.system_clock_hz = system_clock_hz;
        self.register_block.ctrl2().set_bit(true, ADC_ENABLE_INDEX);
        self.wait_us(10);

        self.register_block
            .ctrl2()
            .set_bit(true, ADC_CALIBRATION_INDEX);
        while self.register_block.ctrl2().read_bit(ADC_CALIBRATION_INDEX) {}

        self.powered = true;
        Ok(())
    }

    pub fn enable_internal_sensors(&self) -> Result<(), AdcError> {
        if !self.powered {
            return Err(AdcError::NotPowered);
        }

        self.register_block
            .ctrl2()
            .set_bit(true, ADC_INTERNAL_SENSOR_ENABLE_INDEX);
        self.wait_us(10);
        Ok(())
    }

    pub fn read(
        &mut self,
        channel: AdcChannel,
        sample_time: AdcSampleTime,
    ) -> Result<u16, AdcError> {
        if !self.powered {
            return Err(AdcError::NotPowered);
        }
        let internal_sensor = channel.uses_internal_sensor();
        let channel = channel.index()?;
        if internal_sensor
            && !self
                .register_block
                .ctrl2()
                .read_bit(ADC_INTERNAL_SENSOR_ENABLE_INDEX)
        {
            self.enable_internal_sensors()?;
        }

        if channel < 10 {
            self.register_block
                .spt2()
                .set_bits(sample_time as u32, u32::from(channel) * 3, 3);
        } else {
            self.register_block
                .spt1()
                .set_bits(sample_time as u32, u32::from(channel - 10) * 3, 3);
        }

        self.register_block.osq1().set_bits(0, 20, 4);
        self.register_block
            .osq3()
            .set_bits(u32::from(channel), 0, 5);
        self.register_block
            .ctrl2()
            .set_bits(0b111, ORDINARY_TRIGGER_SOURCE_INDEX, 3);
        self.register_block
            .ctrl2()
            .set_bit(true, ORDINARY_TRIGGER_ENABLE_INDEX);
        self.register_block.sts().write(|w| w.occe().clear());
        self.register_block
            .ctrl2()
            .set_bit(true, ORDINARY_SOFTWARE_TRIGGER_INDEX);

        while !self.register_block.sts().read().occe().is_complete() {}

        let result = self.register_block.odt().read().odt().bits();
        self.register_block.sts().write(|w| w.occe().clear());
        Ok(result)
    }

    fn wait_us(&self, microseconds: u32) {
        let cycles = (u64::from(self.system_clock_hz) * u64::from(microseconds) / 1_000_000)
            .min(u64::from(u32::MAX)) as u32;
        cortex_m::asm::delay(cycles);
    }
}
