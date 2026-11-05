use crate::hal::peripherals;
use crate::hal::register_extras::RegisterExtras;

const GPIOA_CLK_ENABLE_INDEX: u32 = 17;
const GPIOF_CLK_ENABLE_INDEX: u32 = 22;

impl peripherals::Peripherals {
    pub fn init_gpio(&mut self) {
        // Enable clocks for gpio a b c
        self.pac
            .CRM
            .ahben()
            .set_bits(0b111, GPIOA_CLK_ENABLE_INDEX, 3);

        // Enable clocks for gpio f
        self.pac.CRM.ahben().set_bit(true, GPIOF_CLK_ENABLE_INDEX);
    }
}
