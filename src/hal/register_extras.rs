use at32f4xx_pac::{Readable, Reg, RegisterSpec, Writable};



pub trait RegisterExtras {

    /// Index is least significant bit
    /// Setting 3 bits at index 2 to 0b000 would give ..x000xx
    fn set_bits(&self, set_to: u32, index: u32, bit_count: u32);

    fn set_register_enum<F: RegisterField>(&self, value: F) {
        self.set_bits(value.bits(), F::INDEX, F::BIT_COUNT);
    }

    fn set_bit(&self, set_to: bool, index: u32) {
        self.set_bits(set_to.into(), index, 1);
    }

    fn read_bits(&self, index: u32, bit_count: u32) -> u32;

    fn read_bit(&self, index: u32) -> bool{
        let val = self.read_bits(index, 1);
        assert!((val == 0) | (val == 1), "Read a bit that was more than one bit");

        val == 1
    }
}

impl<REG> RegisterExtras for Reg<REG>
where
    REG: RegisterSpec<Ux = u32> + Readable + Writable,
{

    fn set_bits(&self, set_to: u32, index: u32, bit_count: u32) {
        assert!(set_to < (0b1 << bit_count), "Setting more bits than specified");
        assert!(index + bit_count <= 32, "Setting bits outside the register");

        // Get 1's in every position that will be set and 0's elsewhere
        // Gets a list of 1s the right length then shifts to the right position
        let modification_mask: u32 = ((0b1 << bit_count) - 1) << index;
        

        let keep_mask: u32 = !modification_mask;

        let modification_set: u32 = (set_to << index) & modification_mask;

        
        critical_section::with(|_| {
            self.modify(|r, w|  
                unsafe { w.bits(
                    (r.bits() & keep_mask) | (modification_set)
                )});
        }

        )

    }

    fn read_bits(&self, index: u32, bit_count: u32) -> u32 {
        assert!(index + bit_count <= 32, "Reading bits outside the register");

        
        // Get 1's in every position that will be read and 0's elsewhere
        // Gets a list of 1s the right length then shifts to the right position
        let read_mask: u32 = ((0b1 << bit_count) - 1) << index;

        // Mask to keep only the important bits, then shift accross as needed
        (self.read().bits() & read_mask) >> index
    }
}


pub trait RegisterField {
    const BIT_COUNT: u32;
    const INDEX: u32;

    fn bits(self) -> u32;

    fn mask() -> u32 {
        ((1u32 << Self::BIT_COUNT) - 1) << Self::INDEX
    }
}

// Default value is the first member of the enum
#[macro_export]
macro_rules! register_enum {
    (
        $(#[$attribute:meta])*
        $vis:vis enum $name:ident {
            $first:ident = $first_val:expr,
            $($variant:ident = $value:expr),* $(,)?
        }
        bit_count = $bits:expr,
        index = $index:expr
    ) => {
        #[repr(u32)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        $(#[$attribute])*
        $vis enum $name {
            #[default]
            $first = $first_val,
            $(
                $variant = $value,
            )*
        }

        impl $crate::hal::register_extras::RegisterField for $name {
            const BIT_COUNT: u32 = $bits;
            const INDEX: u32 = $index;

            fn bits(self) -> u32 {
                self as u32
            }
        }
    };
}

pub(crate) use register_enum;