//! Embassy timer driver.
#![allow(unused_imports)]

use core::cell::{Cell, RefCell};

use critical_section::CriticalSection;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::signal::Signal;

use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use embassy_futures::select::{select,Either};
use embassy_executor::Spawner;

use fugit::{Instant,Duration,ExtU64};
use crate::{rtc_monotonic,rtc::rtic::rtc_clock};
use crate::rtic_time::Monotonic;

rtc_monotonic!(Mono, rtc_clock::Clock32k);

type AlarmState = u64;

struct TimerDriver {
    alarm: Signal<CriticalSectionRawMutex, AlarmState>,
    queue: Mutex<CriticalSectionRawMutex, RefCell<Queue>>,
}

embassy_time_driver::time_driver_impl!(static DRIVER: TimerDriver = TimerDriver{
    alarm:  Signal::new(),
    queue: Mutex::new(RefCell::new(Queue::new()))
});


impl Driver for TimerDriver {
    fn now(&self) -> u64 {
        Mono::now().ticks()
    }
    fn schedule_wake(&self, at: u64, waker: &core::task::Waker) {
        critical_section::with(|cs| {
            let mut queue = self.queue.borrow(cs).borrow_mut();
            if queue.schedule_wake(at, waker) {
                let mut next = queue.next_expiration(self.now());
                while !self.set_alarm(next) {
                    next = queue.next_expiration(self.now());
                }
            }
        })
    }
}

#[embassy_executor::task]
async fn alarm_watcher_task() {
    let mut next_alarm : Option<AlarmState> = None;

    loop {
        if DRIVER.alarm.signaled() {
            next_alarm = DRIVER.alarm.try_take();
        }

        // wait until the alarm time, OR until the alarm signals that it was changed.
        match next_alarm {
            Some(n) => {
                let mut next_instant : fugit::Instant<u64,1,32768> = fugit::Instant::<u64,1,32768>::from_ticks(n);

                if n == u64::MAX { // Instant doesnt like to wait until u64::MAX, it seems to cause an overflow internally.
                    next_instant = Mono::now() + 876000u64.hours(); // 100 years
                } 

                match select(Mono::delay_until(next_instant), DRIVER.alarm.wait()).await {
                    Either::First(_) => {
                        DRIVER.trigger_alarm();
                    },
                    Either::Second(new_alarm) => {
                        next_alarm = Some(new_alarm);
                    }
                };
            },
            None => {
                // Wait for a new alarm
                next_alarm = Some(DRIVER.alarm.wait().await);
            },
        };
    }
}

impl TimerDriver {
    fn set_alarm(&self, timestamp: u64) -> bool {
        self.alarm.signal(timestamp); // triggers a new alarm
        let now = self.now();

        if timestamp <= now {
            // If alarm timestamp has passed the alarm will not fire.
            // Disarm the alarm and return `false` to indicate that.
            self.alarm.signal(u64::MAX);
            false
        } else {
            true
        }
    }

    fn trigger_alarm(&self) {
        critical_section::with(|cs| {
            let mut next = self.queue.borrow(cs).borrow_mut().next_expiration(self.now());
            while !self.set_alarm(next) {
                next = self.queue.borrow(cs).borrow_mut().next_expiration(self.now());
            }
        })
    }
}

/// safety: must be called exactly once at bootup
pub fn time_driver_init(rtc: crate::pac::Rtc, spawner: &embassy_executor::Spawner) {
    DRIVER.alarm.signal(u64::MAX);
    Mono::start(rtc);
    spawner.spawn(alarm_watcher_task()).unwrap();
}
