use crate::hal::{pin::Pin, register_extras::RegisterExtras};
use core::sync::atomic::{AtomicBool, Ordering};

static INITIALISED: AtomicBool = AtomicBool::new(false);

pub struct Peripherals {
    pub(super) pac: at32f4xx_pac::at32f421::Peripherals,

    pub clk_hz: u32,

    
    pub pin_a0: Option<Pin>,
    pub pin_a1: Option<Pin>,
    pub pin_a2: Option<Pin>,
    pub pin_a3: Option<Pin>,
    pub pin_a4: Option<Pin>,
    pub pin_a5: Option<Pin>,
    pub pin_a6: Option<Pin>,
    pub pin_a7: Option<Pin>,
    pub pin_a8: Option<Pin>,
    pub pin_a9: Option<Pin>,
    pub pin_a10: Option<Pin>,
    pub pin_a11: Option<Pin>,
    pub pin_a12: Option<Pin>,
    pub pin_a13: Option<Pin>, //SWDIO
    pub pin_a14: Option<Pin>, //SWCLK
    pub pin_a15: Option<Pin>,

    pub pin_b0: Option<Pin>,
    pub pin_b1: Option<Pin>,
    pub pin_b2: Option<Pin>,
    pub pin_b3: Option<Pin>,
    pub pin_b4: Option<Pin>,
    pub pin_b5: Option<Pin>,
    pub pin_b6: Option<Pin>,
    pub pin_b7: Option<Pin>,
    pub pin_b8: Option<Pin>,
    pub pin_b9: Option<Pin>,
    pub pin_b10: Option<Pin>,
    pub pin_b11: Option<Pin>,
    pub pin_b12: Option<Pin>,
    pub pin_b13: Option<Pin>,
    pub pin_b14: Option<Pin>,
    pub pin_b15: Option<Pin>,

    pub pin_c13: Option<Pin>,
    pub pin_c14: Option<Pin>,
    pub pin_c15: Option<Pin>,

    pub pin_f0: Option<Pin>,
    pub pin_f1: Option<Pin>,
    pub pin_f6: Option<Pin>,
    pub pin_f7: Option<Pin>,

}

impl Peripherals {
    pub fn init() -> Option<Self> {
        if INITIALISED.swap(true, Ordering::AcqRel) {
            return None;
        }

        let pac = at32f4xx_pac::at32f421::Peripherals::take().expect("Already took the pac");

        let mut result = Self { 
            pac,
            clk_hz: 8_000_000, // reset clock, see the clock notes

            pin_a0: Some(Pin::new(super::pin::Bank::A, 0)), 
            pin_a1: Some(Pin::new(super::pin::Bank::A, 1)), 
            pin_a2: Some(Pin::new(super::pin::Bank::A, 2)), 
            pin_a3: Some(Pin::new(super::pin::Bank::A, 3)), 
            pin_a4: Some(Pin::new(super::pin::Bank::A, 4)), 
            pin_a5: Some(Pin::new(super::pin::Bank::A, 5)), 
            pin_a6: Some(Pin::new(super::pin::Bank::A, 6)), 
            pin_a7: Some(Pin::new(super::pin::Bank::A, 7)), 
            pin_a8: Some(Pin::new(super::pin::Bank::A, 8)), 
            pin_a9: Some(Pin::new(super::pin::Bank::A, 9)), 
            pin_a10: Some(Pin::new(super::pin::Bank::A, 10)), 
            pin_a11: Some(Pin::new(super::pin::Bank::A, 11)), 
            pin_a12: Some(Pin::new(super::pin::Bank::A, 12)), 
            pin_a13: Some(Pin::new(super::pin::Bank::A, 13)), 
            pin_a14: Some(Pin::new(super::pin::Bank::A, 14)), 
            pin_a15: Some(Pin::new(super::pin::Bank::A, 15)), 

            pin_b0: Some(Pin::new(super::pin::Bank::B, 0)), 
            pin_b1: Some(Pin::new(super::pin::Bank::B, 1)), 
            pin_b2: Some(Pin::new(super::pin::Bank::B, 2)), 
            pin_b3: Some(Pin::new(super::pin::Bank::B, 3)), 
            pin_b4: Some(Pin::new(super::pin::Bank::B, 4)), 
            pin_b5: Some(Pin::new(super::pin::Bank::B, 5)), 
            pin_b6: Some(Pin::new(super::pin::Bank::B, 6)), 
            pin_b7: Some(Pin::new(super::pin::Bank::B, 7)), 
            pin_b8: Some(Pin::new(super::pin::Bank::B, 8)), 
            pin_b9: Some(Pin::new(super::pin::Bank::B, 9)), 
            pin_b10: Some(Pin::new(super::pin::Bank::B, 10)), 
            pin_b11: Some(Pin::new(super::pin::Bank::B, 11)), 
            pin_b12: Some(Pin::new(super::pin::Bank::B, 12)), 
            pin_b13: Some(Pin::new(super::pin::Bank::B, 13)), 
            pin_b14: Some(Pin::new(super::pin::Bank::B, 14)), 
            pin_b15: Some(Pin::new(super::pin::Bank::B, 15)), 
            
            pin_c13: Some(Pin::new(super::pin::Bank::C, 13)), 
            pin_c14: Some(Pin::new(super::pin::Bank::C, 14)), 
            pin_c15: Some(Pin::new(super::pin::Bank::C, 15)), 
            
            pin_f0: Some(Pin::new(super::pin::Bank::F, 0)), 
            pin_f1: Some(Pin::new(super::pin::Bank::F, 1)), 
            pin_f6: Some(Pin::new(super::pin::Bank::F, 6)), 
            pin_f7: Some(Pin::new(super::pin::Bank::F, 7)), 
        };


        result.init_clock_120mhz();
        result.init_gpio();

        Some(result)
    }

}