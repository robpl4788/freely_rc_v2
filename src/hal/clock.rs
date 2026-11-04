use crate::hal::peripherals;
use crate::hal::register_extras::{register_enum, RegisterExtras, RegisterField};

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
    pub fn init_clock_120mhz(&mut self) {
        let target_frequency= 120_000_000;
        let crm = &self.pac.CRM;
        // Set flash wait cycles
        self.pac.FLASH.psr().set_register_enum(FlashWaitCycles::ThreeWait);

        // Clear pllcfgen
        crm.pll().set_bit(false, PLL_CONFIG_ENABLE_INDEX);

        //  Ensure pll is using internal high speed clock
        crm.cfg().set_register_enum(PLLReferenceClockSource::HICKDivided);
        
        // set pll input clock frequency
        crm.pll().set_bits(
            pll_frequency_reference(PLL_HICK_INPUT_FREQUENCY), 
            PLL_FREQUENCY_REFERENCE_INDEX, 
            PLL_FREQUENCY_REFERENCE_COUNT);
        
        // set pll frequency multiplcation factor
        let pll_frequency_multiplication = target_frequency / PLL_HICK_INPUT_FREQUENCY;
        Self::set_pll_frequency_multiplication(crm, pll_frequency_multiplication);
        
        // enable pll
        crm.ctrl().set_bit(true, PLL_ENABLE_INDEX);

        // wait for pll to be stabilised
        while crm.ctrl().read_bit(PLL_STABLE_INDEX) == false {
            
        }

        // Set auto step by step system clock switch enable bit
        crm.misc2().set_register_enum(AutoStepEnable::Enable);


        // Switch system clock source to pll

        crm.cfg().set_register_enum(SystemClock::PLL);

        // 20 nops
        for _ in 0..20 { cortex_m::asm::nop(); }

        // check system clock source is pll
        while crm.cfg().read_bits(
            SYSTEM_CLOCK_STATUS_INDEX,
            SystemClock::BIT_COUNT) != SystemClock::PLL.bits() {
            defmt::warn!("System Clock is not PLL")
        }
        defmt::info!("System Clock is PLL");


        // disable step by step switch
        crm.misc2().set_register_enum(AutoStepEnable::Disable);


        self.clk_hz = 120_000_000;
    }

    fn set_pll_frequency_multiplication(crm: &at32f4xx_pac::Periph<at32f4xx_pac::at32f421::crm::RegisterBlock, 1073876992>, desired_factor: u32) {
        // IDK why the datasheet is like this
        let desired_binary = 
            if desired_factor < 16 
                {desired_factor - 2} 
            else {desired_factor - 1};
        
        assert!((desired_factor >= 2) & (desired_factor <= 64), "Cannot achieve desired pll frequency multiplcation");
        
        let desired_top = desired_binary >> PLL_FREQUENCY_MULTIPLICATION_BOTTOM_COUNT;
        let desired_bottom = desired_binary & ((0b1 << PLL_FREQUENCY_MULTIPLICATION_BOTTOM_COUNT) - 1);
    
        crm.cfg().set_bits(desired_top, PLL_FREQUENCY_MULTIPLICATION_TOP_INDEX, PLL_FREQUENCY_MULTIPLICATION_TOP_COUNT);
        crm.cfg().set_bits(desired_bottom, PLL_FREQUENCY_MULTIPLICATION_BOTTOM_INDEX, PLL_FREQUENCY_MULTIPLICATION_BOTTOM_COUNT);
    }
}