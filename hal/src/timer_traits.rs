use core::convert::Infallible;

use fugit::NanosDurationU32;

/// Specifies a timer that can enable & disable an interrupt that fires
/// when the timer expires
pub trait InterruptDrivenTimer {
    /// Enable the timer interrupt
    fn enable_interrupt(&mut self);

    /// Start the timer with a given timeout in nanoseconds
    fn start<T: Into<NanosDurationU32>>(&mut self, timeout: T);

    /// Wait for the timer to finish counting down **without blocking**.
    fn wait(&mut self) -> nb::Result<(), Infallible>;

    /// Return the current count of the timer
    /// TODO! This assumes a 32-bit timer, because that's the maximum size of counters,
    /// but this will return a 16-bit value for 16-bit timers.
    fn count(&self) -> u32;

    /// Reset the counter
    fn retrigger(&mut self);

    /// Disable the timer interrupt
    fn disable_interrupt(&mut self);
}
