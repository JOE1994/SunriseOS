//! IRQ Handling
//!
//! IRQs are asynchronous interruptions coming from an external source,
//! generally a device. Each platform has its own IRQ handlers. The API exposed
//! by this module consists solely of an IRQ_HANDLERS array containing function
//! pointers for all the IRQs, redirecting them to the generic IRQ management
//! defined in the event module. It is expected that these pointer will then be
//! inserted in an architecture-specific interrupt table (such as i386's IDT).

use crate::i386::structures::idt::ExceptionStackFrame;
use crate::devices::pic;

#[allow(clippy::missing_docs_in_private_items)]
extern "x86-interrupt" fn timer_handler(_stack_frame: &mut ExceptionStackFrame) {
    // TODO: Feed the timer handler into a kernel preemption handler.
    pic::get().acknowledge(0);
}

macro_rules! irq_handler {
    ($irq:expr, $name:ident) => {{
        #[allow(clippy::missing_docs_in_private_items)]
        extern "x86-interrupt" fn $name(_stack_frame: &mut ExceptionStackFrame) {
            pic::get().acknowledge($irq);
            crate::event::dispatch_event($irq);
        }
        $name
    }}
}

/// Array of interrupt handlers. The position in the array defines the IRQ this
/// handler is targeting. See the module documentation for more information.
pub static IRQ_HANDLERS : [extern "x86-interrupt" fn(stack_frame: &mut ExceptionStackFrame); 16] = [
    irq_handler!(0, timer_handler),
    irq_handler!(1, keyboard_handler),
    irq_handler!(2, cascade_handler),
    irq_handler!(3, serial2_handler),
    irq_handler!(4, serial1_handler),
    irq_handler!(5, sound_handler),
    irq_handler!(6, floppy_handler),
    irq_handler!(7, parallel1_handler),
    irq_handler!(8, rtc_handler),
    irq_handler!(9, acpi_handler),
    irq_handler!(10, irq10_handler),
    irq_handler!(11, irq11_handler),
    irq_handler!(12, mouse_handler),
    irq_handler!(13, irq13_handler),
    irq_handler!(14, primary_ata_handler),
    irq_handler!(15, secondary_ata_handler),
];