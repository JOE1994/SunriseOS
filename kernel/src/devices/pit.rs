//! Programmable Interval Timer
//!
//! ### channels
//!
//! There are 3 channels :
//! * channel 0, wired to irq0.
//!   We use this one in rate generator mode, to keep track of the time
//!
//! * channel 1, "unusable, and may not even exist" ... whoah
//!
//! * channel 2, wired to pc speaker.
//!   We use this one in "one shot" mode to implement a simple wait function.
//!   Output is ANDed with a gate controlled by port 0x61 bit #1 before going to the pc speaker,
//!   we use this to enable/disable the speaker.
//    TODO
//!   However the channel can only track one countdown at a time,
//!   so we have to switch to channel 0 timers when we want to wait
//!   as soon as we have interruptions working
//!
//! ### operating modes
//!
//! Each channel can operate in different modes. The ones we use are:
//! * Mode 0 - Countdown (poorly named "Interrupt on Terminal Count").
//!   Used on channel 2 to create countdowns.
//!   Set the reset value, countdowns starts.
//!   When countdown goes to 0, OUT goes HIGH, and stays high.
//!
//! * Mode 2 - Rate generator.
//!   Used on channel 0 to create recurring irqs.
//!   Set the reset value. Countdown starts.
//!   When countdown goes to 0, OUT goes LOW for 1 clock cycle, and HIGH again,
//!   and countdown restarts.
//!
//! ### commands
//!
//! Pit commands are sent on port 0x43.
//! See [OSDEV](https://wiki.osdev.org/Programmable_Interval_Timer#I.2FO_Ports)
//!
//! To write channel reload values we always use the Access mode "lobyte/hibyte"
//!
//! ### port 0x61
//!
//! The PIT makes great use of the IO port 0x61 :
//! * bit #0 is channel 2 GATE control
//! * bit #1 is SPKR control
//! * bit #4 is channel 1 OUT status
//! * bit #5 is channel 2 OUT status
//!
//! ## References
//!
//! * [OSDEV](https://wiki.osdev.org/Programmable_Interval_Timer)
//! * [this very good ppt](https://www.cs.usfca.edu/~cruse/cs630f08/lesson15.ppt)
//!

use sync::SpinLock;
use io::Io;
use ::i386::pio::Pio;
use event::{self, IRQEvent, Waitable};
use utils::div_round_up;

use core::sync::atomic::{AtomicUsize, Ordering};

/// The oscillator frequency when not divided, in hertz.
const OSCILLATOR_FREQ: usize = 1193182;

/// The frequency of channel 0 irqs, in hertz.
/// One every 10 millisecond
pub const CHAN_0_FREQUENCY: usize = 100;

/// The channel 0 reset value
const CHAN_0_DIVISOR: u16 = (OSCILLATOR_FREQ / CHAN_0_FREQUENCY) as u16;

lazy_static! {
    /// The mutex wrapping the ports
    static ref PIT_PORTS: SpinLock<PITPorts> = SpinLock::new(PITPorts {
        port_chan_0: Pio::new(0x40),
        port_chan_2: Pio::new(0x42),
        port_cmd:    Pio::new(0x43),
        port_61:     Pio::new(0x61)
    });
}

/// Used internally to select which channel to apply operations to
enum ChannelSelector {
    Channel0,
    Channel2
}

bitflags! {
    /// The port 0x61 flags we use
    struct Port61Flags: u8 {
        const SPKR_CONTROL = 1 << 1;
        const OUT2_STATUS  = 1 << 5;

        // Other flags so bitflags can work
        const GATE_2       = 1 << 0;
        const OUT1_STATUS  = 1 << 4;
        const OTHER_2 = 1 << 2;
        const OTHER_3 = 1 << 3;
        const OTHER_6 = 1 << 6;
        const OTHER_7 = 1 << 7;
    }
}

/// We put the PIT ports in a structure to have them under a single mutex
struct PITPorts {
    port_chan_0: Pio<u8>,
    port_chan_2: Pio<u8>,
    port_cmd:    Pio<u8>,
    port_61:     Pio<u8>
}

impl PITPorts {
    /// Writes a reload value in lobyte/hibyte access mode
    fn write_reload_value(&mut self, channel_selector: ChannelSelector, value: u16) {
        let mut port = match channel_selector {
            ChannelSelector::Channel0 => &mut self.port_chan_0,
            ChannelSelector::Channel2 => &mut self.port_chan_2
        };
        let lo: u8 = (value & 0xFF) as u8;
        let hi: u8 = (value >> 8) as u8;
        port.write(lo);
        port.write(hi);
    }
}

/// Channel 2
struct PITChannel2<'ports> {
    ports: &'ports mut PITPorts
}

impl<'ports> PITChannel2<'ports> {

    /// Sets mode #0 for channel 2
    fn init(ports: &mut PITPorts) -> PITChannel2 {
        ports.port_cmd.write(
            0b10110000 // channel 2, lobyte/hibyte, interrupt on terminal count
        );
        PITChannel2 { ports }
    }

    /// Sets the countdown reset value by writing to channel 2 data port.
    /// Starts the countdown as a side-effect
    fn start_countdown(&mut self, value: u16) {
        self.ports.write_reload_value(ChannelSelector::Channel2, value);
    }

    /// Checks if the countdown is finished
    fn is_countdown_finished(&self) -> bool {
        Port61Flags::from_bits(self.ports.port_61.read()).unwrap()
            .contains(Port61Flags::OUT2_STATUS)
    }

    /// Waits until countdown is finished
    fn wait_countdown_is_finished(&self) {
        while !(Port61Flags::from_bits(self.ports.port_61.read()).unwrap()
            .contains(Port61Flags::OUT2_STATUS))
            {
                // You spin me right round, baby
                // right round !
                // Like a record, baby
                // right round
                // round round !
            }
    }

    /// Spin waits for at least `ms` amount of milliseconds
    fn spin_wait_ms(&mut self, ms: usize) {
        let ticks_to_wait: usize = (OSCILLATOR_FREQ / 1000) * ms;

        // wait for max amount of time multiples times
        if ticks_to_wait >= u16::max_value() as usize {
            for _ in (0..=ticks_to_wait).step_by(u16::max_value() as usize) {
                self.start_countdown(u16::max_value());
                self.wait_countdown_is_finished();
            }
        }

        // last iter
        let remaining_to_wait = ticks_to_wait as u16;
        if remaining_to_wait > 0 {
            self.start_countdown(remaining_to_wait);
            self.wait_countdown_is_finished();
        }
    }
}

/// Spin waits for at least `ms` amount of milliseconds
pub fn spin_wait_ms(ms: usize) {
    let mut ports = PIT_PORTS.lock();
    let mut chan2 = PITChannel2::init(&mut ports);
    chan2.spin_wait_ms(ms);
}

#[derive(Debug)]
struct WaitFor {
    every_ms: usize,
    parent_event: IRQEvent,
    spins_needed: AtomicUsize
}

impl Waitable for WaitFor {
    fn register(&self) {
        self.parent_event.register()
    }

    fn is_signaled(&self) -> bool {
        // First, reset the spins if necessary
        self.spins_needed.compare_and_swap(0, div_round_up(self.every_ms * CHAN_0_FREQUENCY, 1000), Ordering::SeqCst);

        // Then, check if it's us.
        self.parent_event.is_signaled()
            // Then, check if we need more spins.
            && self.spins_needed.fetch_sub(1, Ordering::SeqCst) == 1
    }
}

/// Returns a stream of event that trigger every `ms` amount of milliseconds
pub fn wait_ms(ms: usize) -> impl Waitable {
    WaitFor {
        every_ms: ms,
        parent_event: event::wait_event(0),
        spins_needed: AtomicUsize::new(0)
    }
}

/// Initialize the channel 0 to send recurring irqs
pub unsafe fn init_channel_0() {
    let mut ports = PIT_PORTS.lock();
    ports.port_cmd.write(
        0b00110100 // channel 0, lobyte/hibyte, rate generator
    );
    ports.write_reload_value(ChannelSelector::Channel0, CHAN_0_DIVISOR);
}