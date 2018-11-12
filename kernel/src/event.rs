//! The core event handling primitives of KFS.
//!
//! The KFS kernel sports a couple sources of events, such as IRQs, timers,
//! userspace-triggered events, and other. It must be possible to await for
//! one or multiple events at the same time.
//!
//! In order to do this, we have the Event trait. It works by (in theory)
//! registering an interest in an event, and putting the current process to
//! sleep (deregistering it from the scheduler). When the event is triggered,
//! the scheduler will wake the process up, allowing it work.

use core::sync::atomic::{AtomicUsize, Ordering};
use core::fmt::Debug;
use alloc::sync::Arc;
use sync::SpinLockIRQ;
use alloc::vec::Vec;
use error::{KernelError, UserspaceError};
use process::{ProcessStructArc, ProcessState};
use scheduler;

// TODO: maybe we should use the libcore's task:: stuff...

/// A waitable item.
///
/// There are essentially two kinds of Waitables: user-signaled and IRQ-backed.
/// Right now, only IRQ-backed waitables are implemented. See IRQEvent for more
/// information on them.
///
/// It is possible that the raw IRQEvent is not flexible enough though. For
/// instance, if we want to wait for 1 second, it might be necessary to wait on
/// the timer event multiple times. To do this, it is possible to implement our
/// own Waitable, that defers register to the underlying IRQEvent, but adds
/// additional logic to is_signaled. For example:
///
/// ```
/// use kfs_kernel::event::{IRQEvent, Waitable};
/// use core::sync::atomic::{AtomicUsize, Ordering};
/// struct WaitFor5Ticks(IRQEvent, AtomicUsize);
/// impl Waitable for WaitFor5Ticks {
///     fn is_signaled(&self) -> bool {
///         self.1.compare_and_swap(0, 5, Ordering::SeqCst);
///         if self.0.is_signaled() {
///             if self.1.fetch_sub(1) == 0 {
///                 return true;
///             } else {
///                 return false;
///             }
///         } else {
///             return false;
///         }
///     }
///     fn register(&self) {
///         self.0.register()
///     }
/// }
/// ```
pub trait Waitable: Debug + Send + Sync {
    /// Checks whether the Waitable was signalled.
    ///
    /// If it returns false, the register function will be called again, in order
    /// to get notified of the next wakeup.
    ///
    /// This will likely require to change state - and yet it takes self by value.
    /// the reason for this is that it's possible for multiple threads, and
    /// potentially multiple CPUs, to wait on the same Waitable. Think of servers:
    /// you might want to wait for multiple threads for the arrival of a new socket.
    /// When this happens, **only a single thread should return true**. Make extra
    /// sure your Atomic operations are written properly!
    ///
    /// You'll probably want to check out AtomicUsize::fetch_update to make sure your
    /// atomic update loops are correct.
    fn is_signaled(&self) -> bool;

    /// Register the waitable with the scheduler.
    ///
    /// This should ensure that when the event is (or is likely to be) triggered,
    /// the scheduler puts the Process back in the running Vec. Most implementors
    /// will want to defer this to an IRQEvent. For instance:
    ///
    /// ```
    /// #use kfs_kernel::event::{IRQEvent, Waitable};
    /// #struct Wait(IRQEvent);
    /// #impl Waitable for WaitFor5Ticks {
    /// #fn is_signaled(&self) -> bool {
    /// #self.0.is_signaled()
    /// #}
    /// fn register(&self) {
    ///     self.0.register()
    /// }
    /// #}
    /// ```
    fn register(&self);
}

/// Waits for an event to occur on one of the given Waitable objects.
pub fn wait<'wait, INTOITER>(waitable_intoiter: INTOITER) -> Result<&'wait Waitable, UserspaceError>
where
    INTOITER: IntoIterator<Item=&'wait Waitable>,
    <INTOITER as IntoIterator>::IntoIter: Clone
{
    let waitable = waitable_intoiter.into_iter();
    let interrupt_manager = SpinLockIRQ::new(());

    loop {
        // Early-check for events that have already been signaled.
        for item in waitable.clone() {
            if item.is_signaled() {
                return Ok(item);
            }
        }

        // Disable interrupts between registration and unschedule.
        let lock = interrupt_manager.lock();

        // Register the process for wakeup on all the possible events
        for item in waitable.clone() {
            item.register();
        }

        // TODO: check that the current process is registered for an event,
        // bug otherwise.

        // Schedule
        scheduler::unschedule(&interrupt_manager, lock)?;
    }
}

/// An event waiting for an IRQ.
///
/// When created, is_signaled is called and the IRQ was triggered, it will
/// increment the ACK count by 1. This means that if multiple IRQs happened
/// between wait calls, it will immediately return true.
// TODO: Allow configuring edge vs level triggering.
#[derive(Debug)]
pub struct IRQEvent {
    state: &'static IRQState,
    ack: AtomicUsize,
}

impl Waitable for IRQEvent {
    fn is_signaled(&self) -> bool {
        if self.ack.fetch_update(|x| {
            if x < self.state.counter.load(Ordering::SeqCst) {
                // TODO: If level-triggered, set this to the counter.
                Some(x + 1)
            } else {
                None
            }
        }, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            true
        } else {
            false
        }
    }

    fn register(&self) {
        let curproc = scheduler::get_current_process();
        let mut veclock = self.state.waiting_processes.lock();
        info!("Registering {:010x} for irq {}", &*curproc as *const _ as usize, self.state.irqnum);
        if veclock.iter().find(|v| Arc::ptr_eq(&curproc, v)).is_none() {
            veclock.push(scheduler::get_current_process());
        }
    }
}

/// Signal the scheduler and waiters that an IRQ has been triggered.
///
/// Usually, the IRQ handling code calls this. But it may be used to generate
/// synthetic IRQs.
pub fn dispatch_event(irq: usize) {
    IRQ_STATES[irq].counter.fetch_add(1, Ordering::SeqCst);
    let mut processes = IRQ_STATES[irq].waiting_processes.lock();
    while let Some(process) = processes.pop() {
        info!("Unregistering {:010x} for irq {}", &*process as *const _ as usize, IRQ_STATES[irq].irqnum);
        scheduler::add_to_schedule_queue(process);
    }
}

/// Creates an IRQEvent waiting for the given IRQ number.
pub fn wait_event(irq: usize) -> IRQEvent {
    IRQEvent {
        state: &IRQ_STATES[irq], ack: AtomicUsize::new(IRQ_STATES[irq].counter.load(Ordering::SeqCst))
    }
}

#[derive(Debug)]
struct IRQState {
    irqnum: usize,
    counter: AtomicUsize,
    waiting_processes: SpinLockIRQ<Vec<ProcessStructArc>>
}

impl IRQState {
    pub const fn new(irqnum: usize) -> IRQState {
        IRQState {
            irqnum,
            counter: AtomicUsize::new(0),
            waiting_processes: SpinLockIRQ::new(Vec::new())
        }
    }
}

static IRQ_STATES: [IRQState; 16] = [
    IRQState::new(20), IRQState::new(21), IRQState::new(22), IRQState::new(23),
    IRQState::new(24), IRQState::new(25), IRQState::new(26), IRQState::new(27),
    IRQState::new(28), IRQState::new(29), IRQState::new(30), IRQState::new(31),
    IRQState::new(32), IRQState::new(33), IRQState::new(34), IRQState::new(35),
];