use crate::hal::pin::{Bank, Pin};
use crate::hal::register_extras::{RegisterExtras, register_enum};
use at32f4xx_pac::at32f421::cmp::RegisterBlock;

register_enum! {
    pub enum SpeedSelection {
        Medium = 0b01,
        High = 0b00,
        Low = 0b10,
        UltraLow = 0b11,
    }
    bit_count = 2,
    index = 4
}

register_enum! {
    pub enum Hysteresis {
        No = 0b00,
        Low = 0b01,
        Medium = 0b10,
        High = 0b11,
    }
    bit_count = 2,
    index = 16
}

register_enum! {
    pub enum Polarity {
        Inverted = 0b0,
        NotInverted = 0b1,

    }
    bit_count = 1,
    index = 15
}



#[derive(Debug, Default, Clone, Copy)]
pub struct ComparatorConfig {
    speed_selection: SpeedSelection,
    hysteresis: Hysteresis,
    polarity: Polarity,
}

register_enum! {
    pub enum NonInvertingSelection {
        VSSA  = 0b11,
        PinA5 = 0b00,
        PinA1 = 0b01,
        PinA0 = 0b10,
    }
    bit_count = 2,
    index = 7
}



register_enum! {
    pub enum InvertingSelection {
        VRefInt1_4 = 0b000,
        VRefInt1_2 = 0b001,
        VRefInt3_4 = 0b010,
        VRefInt  = 0b011,
        PinA4 = 0b100,
        PinA5 = 0b101,
        PinA0 = 0b110,
        PinA2 = 0b111,
    }
    bit_count = 3,
    index = 4
}

const COMPARATOR_VALUE_INDEX: u32 = 30;
const SCALE_ENABLE_INDEX:u32 = 23;
const BRIDGE_ENABLE_INDEX:u32 = 23;
const COMPARATOR_ENABLE_INDEX:u32 = 0;

pub struct Comparator {
    config: ComparatorConfig,
    inverting_pin: Option<Pin>,
    non_inverting_pin: Option<Pin>,
    inverting_selection: InvertingSelection,
    non_inverting_selection: NonInvertingSelection,
    register_block: at32f4xx_pac::at32f421::CMP,
}

impl Comparator {
    pub(super) fn new() -> Comparator {
        let register_block  = unsafe {
            at32f4xx_pac::at32f421::CMP::steal()
        };



        Comparator {
            config: ComparatorConfig::default(),   
            non_inverting_pin: None,
            inverting_pin: None,
            inverting_selection: InvertingSelection::default(),
            non_inverting_selection: NonInvertingSelection::default(),
            register_block,
        }
    }

    pub fn enable(&mut self) {
        self.register_block.ctrlsts().set_bit(true, COMPARATOR_ENABLE_INDEX);
    }

    pub fn disable(&mut self) {
        self.register_block.ctrlsts().set_bit(false, COMPARATOR_ENABLE_INDEX);
    }

    pub fn apply_config(&mut self, new_config: ComparatorConfig) {
        self.config = new_config;

        self.register_block.ctrlsts().set_register_enum(self.config.hysteresis);
        self.register_block.ctrlsts().set_register_enum(self.config.speed_selection);
        self.register_block.ctrlsts().set_register_enum(self.config.polarity);
    }

    pub fn get_value(&self) -> bool {
        self.register_block.ctrlsts().read_bit(COMPARATOR_VALUE_INDEX)
    }

    /// Returns the pin that was the inverting reference if there was one
    pub fn with_inverting_reference(&mut self, inverting_selection: InvertingSelection) -> Option<Pin> {
        match inverting_selection {
            InvertingSelection::VRefInt => {
                // Don't need the full bridge if only using the max value
                self.register_block.ctrlsts().set_register_enum(inverting_selection);
                self.register_block.ctrlsts().set_bit(true, SCALE_ENABLE_INDEX);
                self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
            }
            InvertingSelection::VRefInt1_2 | InvertingSelection::VRefInt1_4 | InvertingSelection::VRefInt3_4 => {
                self.register_block.ctrlsts().set_register_enum(inverting_selection);
                self.register_block.ctrlsts().set_bit(true, SCALE_ENABLE_INDEX);
                self.register_block.ctrlsts().set_bit(true, BRIDGE_ENABLE_INDEX);
            }
            _ => {
                panic!("Can't set comparator to use pins like this");
            }
        }
        self.inverting_selection = inverting_selection;

        self.inverting_pin.take()
    }

    /// Returns the pin that was the non inverting reference if there was one
    pub fn with_non_inverting_reference(&mut self, non_inverting_selection: NonInvertingSelection) -> Option<Pin> {
        match non_inverting_selection {
            NonInvertingSelection::VSSA => {
                // Don't need the full bridge if only using the max value
                self.register_block.ctrlsts().set_register_enum(non_inverting_selection);
                
            }
            _ => {
                panic!("Can't set comparator to use pins like this");
            }
        }
        self.non_inverting_selection = non_inverting_selection;

        self.non_inverting_pin.take()
    }

    /// Returns the pin that was here if there was one
    pub fn with_inverting_pin(&mut self, inverting_pin: Pin) -> Option<Pin>{
        let old_inverting_pin = self.non_inverting_pin.take();
        if ((inverting_pin.bank == Bank::A) & (inverting_pin.number == 0)) {
            self.inverting_selection = InvertingSelection::PinA0;
            self.inverting_pin = Some(inverting_pin);

            self.register_block.ctrlsts().set_register_enum(InvertingSelection::PinA0);
            
            // Don't need the bridge if only using the max value
            self.register_block.ctrlsts().set_bit(false, SCALE_ENABLE_INDEX);
            self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
        } else if ((inverting_pin.bank == Bank::A) & (inverting_pin.number == 2)) {
            self.inverting_selection = InvertingSelection::PinA2;
            self.inverting_pin = Some(inverting_pin);

            self.register_block.ctrlsts().set_register_enum(InvertingSelection::PinA2);
            
            // Don't need the bridge if only using the max value
            self.register_block.ctrlsts().set_bit(false, SCALE_ENABLE_INDEX);
            self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
        } else if ((inverting_pin.bank == Bank::A) & (inverting_pin.number == 4)) {
            self.inverting_selection = InvertingSelection::PinA4;
            self.inverting_pin = Some(inverting_pin);

            self.register_block.ctrlsts().set_register_enum(InvertingSelection::PinA4);
            
            // Don't need the bridge if only using the max value
            self.register_block.ctrlsts().set_bit(false, SCALE_ENABLE_INDEX);
            self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
        } else if ((inverting_pin.bank == Bank::A) & (inverting_pin.number == 5)) {
            self.inverting_selection = InvertingSelection::PinA5;
            self.inverting_pin = Some(inverting_pin);

            self.register_block.ctrlsts().set_register_enum(InvertingSelection::PinA5);
            
            // Don't need the bridge if only using the max value
            self.register_block.ctrlsts().set_bit(false, SCALE_ENABLE_INDEX);
            self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
        } else{
            panic!("Can't use this pin for the comparator inverting pin");
        }
    
        
        
        old_inverting_pin
    }

    /// Returns the pin that was here if there was one
    pub fn with_non_inverting_pin(&mut self, non_inverting_pin: Pin) -> Option<Pin>{
        let old_non_inverting_pin = self.inverting_pin.take();
        if ((non_inverting_pin.bank == Bank::A) & (non_inverting_pin.number == 0)) {
            self.non_inverting_selection = NonInvertingSelection::PinA0;
            self.non_inverting_pin = Some(non_inverting_pin);

            self.register_block.ctrlsts().set_register_enum(self.non_inverting_selection);
            
            // Don't need the bridge if only using the max value
            self.register_block.ctrlsts().set_bit(false, SCALE_ENABLE_INDEX);
            self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
        } else if ((non_inverting_pin.bank == Bank::A) & (non_inverting_pin.number == 1)) {
             self.non_inverting_selection = NonInvertingSelection::PinA1;
            self.non_inverting_pin = Some(non_inverting_pin);

            self.register_block.ctrlsts().set_register_enum(self.non_inverting_selection);
            
            // Don't need the bridge if only using the max value
            self.register_block.ctrlsts().set_bit(false, SCALE_ENABLE_INDEX);
            self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
        } else if ((non_inverting_pin.bank == Bank::A) & (non_inverting_pin.number == 5)) {
             self.non_inverting_selection = NonInvertingSelection::PinA5;
            self.non_inverting_pin = Some(non_inverting_pin);

            self.register_block.ctrlsts().set_register_enum(self.non_inverting_selection);
            
            // Don't need the bridge if only using the max value
            self.register_block.ctrlsts().set_bit(false, SCALE_ENABLE_INDEX);
            self.register_block.ctrlsts().set_bit(false, BRIDGE_ENABLE_INDEX);
        } else{
            panic!("Can't use this pin for the comparator non inverting pin");
        }
    
        old_non_inverting_pin
    }

}