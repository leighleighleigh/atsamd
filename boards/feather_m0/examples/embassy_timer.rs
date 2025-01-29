#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

#[no_mangle]
pub static RTIC_ASYNC_MAX_LOGICAL_PRIO: u8 = 240;
// This is required because we are not using RTIC as a runtime.
// DISCUSSION: https://github.com/rtic-rs/rtic/issues/956#issuecomment-2426930973

#[cfg(not(feature = "use_semihosting"))]
use panic_halt as _;
#[cfg(feature = "use_semihosting")]
use panic_semihosting as _;

use bsp::{hal, pac, pin_alias};
use feather_m0 as bsp;

use hal::{
    clock::{ClockGenId, ClockSource, GenericClockController},
    ehal::digital::StatefulOutputPin,
    rtc::time_driver::time_driver_init,
};

use embassy_time::Timer;


#[embassy_executor::main]
async fn main(_s: embassy_executor::Spawner) {
    let mut peripherals = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();

    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.gclk,
        &mut peripherals.pm,
        &mut peripherals.sysctrl,
        &mut peripherals.nvmctrl,
    );
    let pins = bsp::Pins::new(peripherals.port);
    let mut red_led: bsp::RedLed = pin_alias!(pins.red_led).into();

    let timer_clock = clocks.configure_gclk_divider_and_source(ClockGenId::Gclk2, 1, ClockSource::Xosc32k, false).unwrap();
    clocks.configure_standby(ClockGenId::Gclk2, true);
    let _rtc_clock = clocks.rtc(&timer_clock).unwrap();
    time_driver_init(peripherals.rtc, &_s); // This calls Mono::start(), makes embassy-time stuff work.

    loop {
        red_led.toggle().unwrap();
        Timer::after_secs(1).await;
    }
}
