use crate::hal::peripherals;
use crate::hal::register_extras::{RegisterExtras, RegisterField, register_enum};

register_enum! {
    #[allow(dead_code)]
    enum FlashWaitCycles {
        ZeroWait  = 0b000,
        OneWait   = 0b001,
        TwoWait   = 0b010,
        ThreeWait = 0b011,
    }
    bit_count = 3,
    index = 0
}

const PLL_CONFIG_ENABLE_INDEX: u32 = 31;

register_enum! {
    #[allow(dead_code)]
    enum PLLReferenceClockSource {
        HICKDivided = 0b0,
        HEXT = 0b1,
    }
    bit_count = 1,
    index = 16
}

fn pll_frequency_reference(f: u32) -> u32 {
    match f {
        3_900_000..=5_000_000 => 0b000,
        5_200_000..=6_250_000 => 0b001,
        7_812_500..=8_330_000 => 0b010,
        8_330_001..=12_500_000 => 0b011,
        15_625_000..=20_830_000 => 0b100,
        20_830_001..=31_250_000 => 0b101,
        _ => panic!("Not allowed PLL input frequency"),
    }
}

const PLL_FREQUENCY_REFERENCE_INDEX: u32 = 24;
const PLL_FREQUENCY_REFERENCE_COUNT: u32 = 3;

const PLL_HICK_INPUT_FREQUENCY: u32 = 4_000_000;

const PLL_FREQUENCY_MULTIPLICATION_BOTTOM_INDEX: u32 = 18;
const PLL_FREQUENCY_MULTIPLICATION_BOTTOM_COUNT: u32 = 4;

const PLL_FREQUENCY_MULTIPLICATION_TOP_INDEX: u32 = 29;
const PLL_FREQUENCY_MULTIPLICATION_TOP_COUNT: u32 = 2;

const PLL_ENABLE_INDEX: u32 = 24;
const PLL_STABLE_INDEX: u32 = 25;
const HICK_ENABLE_INDEX: u32 = 0;
const HICK_STABLE_INDEX: u32 = 1;

const AHB_DIVIDER_INDEX: u32 = 4;
const APB1_DIVIDER_INDEX: u32 = 8;
const APB2_DIVIDER_INDEX: u32 = 11;
const ADC_DIVIDER_INDEX: u32 = 14;
const ADC_DIVIDER_HIGH_BIT_INDEX: u32 = 28;

register_enum! {
    enum AutoStepEnable {
        Disable = 0b00,
        Enable = 0b11,
    }
    bit_count = 2,
    index = 4
}

register_enum! {
    #[allow(dead_code)]
    enum SystemClock {
        HICK = 0b00,
        HEXT = 0b01,
        PLL  = 0b10,
    }
    bit_count = 2,
    index = 0
}

const SYSTEM_CLOCK_STATUS_INDEX: u32 = 2;

impl peripherals::Peripherals {
    pub(super) fn init_clock_120mhz(&mut self) {
        let target_frequency = 120_000_000;
        let crm = &self.pac.CRM;
        self.pac
            .FLASH
            .psr()
            .set_register_enum(FlashWaitCycles::ThreeWait);

        crm.ctrl().set_bit(true, HICK_ENABLE_INDEX);
        while !crm.ctrl().read_bit(HICK_STABLE_INDEX) {}

        // Keep AHB and APB at 120 MHz; the ADC divider yields 20 MHz.
        crm.cfg().set_bits(0, AHB_DIVIDER_INDEX, 4);
        crm.cfg().set_bits(0, APB1_DIVIDER_INDEX, 3);
        crm.cfg().set_bits(0, APB2_DIVIDER_INDEX, 3);
        crm.cfg().set_bits(0b010, ADC_DIVIDER_INDEX, 2);
        crm.cfg().set_bit(false, ADC_DIVIDER_HIGH_BIT_INDEX);

        crm.pll().set_bit(false, PLL_CONFIG_ENABLE_INDEX);
        crm.cfg()
            .set_register_enum(PLLReferenceClockSource::HICKDivided);
        crm.pll().set_bits(
            pll_frequency_reference(PLL_HICK_INPUT_FREQUENCY),
            PLL_FREQUENCY_REFERENCE_INDEX,
            PLL_FREQUENCY_REFERENCE_COUNT,
        );

        let multiplier = target_frequency / PLL_HICK_INPUT_FREQUENCY;
        self.set_pll_frequency_multiplication(multiplier);
        crm.ctrl().set_bit(true, PLL_ENABLE_INDEX);
        while !crm.ctrl().read_bit(PLL_STABLE_INDEX) {}

        crm.misc2().set_register_enum(AutoStepEnable::Enable);
        crm.cfg().set_register_enum(SystemClock::PLL);
        for _ in 0..20 {
            cortex_m::asm::nop();
        }
        while crm
            .cfg()
            .read_bits(SYSTEM_CLOCK_STATUS_INDEX, SystemClock::BIT_COUNT)
            != SystemClock::PLL.bits()
        {}
        defmt::info!("System Clock is PLL");

        crm.misc2().set_register_enum(AutoStepEnable::Disable);
        self.clk_hz = target_frequency;
    }

    fn set_pll_frequency_multiplication(&self, desired_factor: u32) {
        assert!(
            (2..=64).contains(&desired_factor),
            "Cannot achieve desired PLL frequency multiplication"
        );

        // The hardware encoding skips one multiplier code at x16.
        let encoded_multiplier = if desired_factor < 16 {
            desired_factor - 2
        } else {
            desired_factor - 1
        };

        let high_bits = encoded_multiplier >> PLL_FREQUENCY_MULTIPLICATION_BOTTOM_COUNT;
        let low_bits = encoded_multiplier & ((1 << PLL_FREQUENCY_MULTIPLICATION_BOTTOM_COUNT) - 1);

        let crm = &self.pac.CRM;
        crm.cfg().set_bits(
            high_bits,
            PLL_FREQUENCY_MULTIPLICATION_TOP_INDEX,
            PLL_FREQUENCY_MULTIPLICATION_TOP_COUNT,
        );
        crm.cfg().set_bits(
            low_bits,
            PLL_FREQUENCY_MULTIPLICATION_BOTTOM_INDEX,
            PLL_FREQUENCY_MULTIPLICATION_BOTTOM_COUNT,
        );
    }
}
