use crate::bldc_driver::BLDC;

pub struct Board {
    bldc: Option<BLDC>,
}

impl Board {
    pub fn new() -> Self {
        let result = Board { bldc: None };

        // result.build_bldc();

        result
    }

    // fn build_bldc(&mut self) {
    //     let mut bldc_pwm_config = pwm::Config::default();
    //     bldc_pwm_config.top = BLDC_PWM_TOP;
    //     bldc_pwm_config.enable = false;

    //     let mut lowc_lowb_config = bldc_pwm_config.clone();
    //     lowc_lowb_config.invert_a = true;
    //     lowc_lowb_config.invert_b = true;

    //     let mut lowa_highc_config = bldc_pwm_config.clone();
    //     lowa_highc_config.invert_a = true;

    //     let highb_higha_config = bldc_pwm_config.clone();

    //     let lowc_lowb_pwm = Pwm::new_output_ab(
    //         self.pwm_slice_lowc_lowb.take().expect("Already built bldc"),
    //         self.lowc_pin.take().expect("Already built bldc"),
    //         self.lowb_pin.take().expect("Already built bldc"),
    //         lowc_lowb_config,
    //     );
    //     let lowa_highc_pwm = Pwm::new_output_ab(
    //         self.pwm_slice_lowa_highc
    //             .take()
    //             .expect("Already built bldc"),
    //         self.lowa_pin.take().expect("Already built bldc"),
    //         self.highc_pin.take().expect("Already built bldc"),
    //         lowa_highc_config,
    //     );
    //     let highb_higha_pwm = Pwm::new_output_ab(
    //         self.pwm_slice_highb_highc
    //             .take()
    //             .expect("Already built bldc"),
    //         self.highb_pin.take().expect("Already built bldc"),
    //         self.higha_pin.take().expect("Already built bldc"),
    //         highb_higha_config,
    //     );

    //     let (lowc_ch, lowb_ch) = lowc_lowb_pwm.split();
    //     let (lowa_ch, highc_ch) = lowa_highc_pwm.split();
    //     let (highb_ch, higha_ch) = highb_higha_pwm.split();

    //     let phase_a = drivers::bldc::Phase::new(
    //         lowa_ch.expect("Phase A low split failed"),
    //         higha_ch.expect("Phase A high split failed"),
    //     );
    //     let phase_b = drivers::bldc::Phase::new(
    //         lowb_ch.expect("Phase B low split failed"),
    //         highb_ch.expect("Phase B high split failed"),
    //     );
    //     let phase_c = drivers::bldc::Phase::new(
    //         lowc_ch.expect("Phase C low split failed"),
    //         highc_ch.expect("Phase C high split failed"),
    //     );

    //     self.bldc = Some(drivers::bldc::BLDC::new(phase_a, phase_b, phase_c));
    // }

    pub fn take_bldc(&mut self) -> BLDC {
        let result = self.bldc.take();
        result.expect("Already took BLDC")
    }
}
