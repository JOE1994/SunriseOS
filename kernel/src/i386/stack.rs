//! Kernel stack
//!
//! A kernel stack is structured as follow :
//!
//!     j--------------------j  < 0xaaaa0000 = KernelStack.stack_address
//!     |                    |
//!     |                    |
//!     |     PAGE GUARD     |
//!     |                    |
//!     |                    |
//!     j--------------------j
//!     |                    |
//!     |                    |
//!     |        AAA         |
//!     |        |||         |
//!     |                    |
//!     j--------------------j
//!     |                    |
//!     |       STACK        |
//!     |                    |
//!     | j----------------j |
//!     | |  poison value  | |
//!     j-j----------------j-j < 0xaaaaffff
//!          No Page Guard
//!
//!  Since the stack is several pages long, we must ensure the stack respects some alignment
//!  in order to be able to find its bottom from any page.

use ::core::mem::size_of;
use paging::*;
use i386::mem::VirtualAddress;
use spin::RwLock;

/// The size of a kernel stack, not accounting for the page guard
pub const STACK_SIZE: usize            = 4;
pub const STACK_SIZE_WITH_GUARD: usize = STACK_SIZE + 1;

/// The alignment of the stack. ceil(log2(STACK_SIZE_WITH_GUARD * PAGE_SIZE))
const STACK_ALIGNEMENT: usize = 15;

/// A structure representing a kernel stack
#[derive(Debug)]
pub struct KernelStack {
    stack_address: VirtualAddress // This falls in the page guard
}

impl KernelStack {
    /// Allocates the kernel stack of a process.
    pub fn allocate_stack() -> Option<KernelStack> {
        let mut tables = ACTIVE_PAGE_TABLES.lock();
        tables.find_available_virtual_space_aligned::<KernelLand>(STACK_SIZE_WITH_GUARD, STACK_ALIGNEMENT)
            .map(|va| {
                tables.map_range_allocate(VirtualAddress(va.addr() + PAGE_SIZE), STACK_SIZE,
                                          EntryFlags::WRITABLE);
                tables.map_page_guard(va);

                let mut me = KernelStack { stack_address: va };

                // This is safe because va points to valid memory
                unsafe { me.create_poison_pointers(); };

                me
            })
    }

    /// Gets the bottom of the stack by and'ing $esp with STACK_ALIGNMENT
    ///
    /// extern "C" to make sure it is called with a sane ABI
    extern "C" fn get_current_stack_bottom() -> usize {
        let esp_ptr: usize;
        unsafe { asm!("mov $0, esp" : "=r"(esp_ptr) ::: "intel" ) };
        esp_ptr & (0xFFFFFFFF << STACK_ALIGNEMENT) // 0x....0000
    }

    /// Retrieves the current stack from $esp
    ///
    /// Should be used only to retrieve the KernelStack that was given to us by the bootstrap.
    ///
    /// # Safety
    ///
    /// Unsafe because it creates duplicates of the stack structure,
    /// whose only owner should be the ProcessStruct it belongs to.
    /// This enables having several mut references pointing to the same underlying memory.
    /// Caller has to make sure no references to the stack exists when calling this function.
    ///
    /// The safe method of getting the stack is by getting current ProcessStruct, *lock it*,
    /// and use its pstack.
    pub unsafe fn get_current_stack() -> KernelStack {
        let stack_bottom = Self::get_current_stack_bottom();
        KernelStack { stack_address: VirtualAddress(stack_bottom) }
    }

    /// We keep 2 poison pointers for fake saved ebp and saved esp at the base of the stack
    const STACK_POISON_SIZE: usize = 2 * size_of::<usize>();

    /// Puts two poisons pointers at the base of the stack for the saved ebp and saved eip
    unsafe fn create_poison_pointers(&mut self) {
        let saved_eip: *mut usize = (self.stack_address.addr() + STACK_SIZE_WITH_GUARD * PAGE_SIZE
                                                               - size_of::<usize>()
                                    ) as *mut usize;
        let saved_ebp: *mut usize = saved_eip.offset(-1);
        *saved_eip = 0x00000000;
        *saved_ebp = 0x00000000;
    }

    /// Get the address of the beginning of usable stack.
    /// Used for initializing $esp and $ebp of a newborn process
    /// Points to the last poison pointer, for saved $ebp
    pub fn get_stack_start(&self) -> usize {
         self.stack_address.addr() + STACK_SIZE_WITH_GUARD * PAGE_SIZE
                                   - Self::STACK_POISON_SIZE
    }

    /// Dumps the stack on all the Loggers, displaying it in a frame-by-frame format
    pub fn dump_current_stack() {
        let mut ebp;
        let mut esp;
        let mut eip;
        unsafe {
            asm!("
                mov $0, ebp
                mov $1, esp

                // eip can only be read through the stack after a call instruction
                call read_eip
            read_eip:
                pop $2"
            : "=r"(ebp), "=r"(esp), "=r"(eip) ::: "volatile", "intel" );
        }

        let stack_bottom = (Self::get_current_stack_bottom() + PAGE_SIZE) as *const u8;
        let stack_slice = unsafe { ::core::slice::from_raw_parts(stack_bottom,
                                                        STACK_SIZE * PAGE_SIZE) };

        dump_stack(stack_slice, stack_bottom as usize, esp, ebp, eip);
    }
}

impl Drop for KernelStack {
    /// We deallocate the stack when it is dropped
    fn drop(&mut self) {
        debug!("Dropping KernelStack {:?}", self);
        let mut tables = ACTIVE_PAGE_TABLES.lock();
        for i in 0..STACK_SIZE_WITH_GUARD {
            tables.unmap(self.stack_address + i * PAGE_SIZE);
        }
    }
}

/* ********************************************************************************************** */

/// Dumps a stack on all the Loggers, displaying it in a frame-by-frame format
/// The stack is passed as a slice. The function starts at given esp, and goes down, frame by frame.
/// The original address of the stack must be given, this way it can even work on a stack that is not identity mapped,
/// therefore it should even be possible to use it on a user stack
///
/// The function will stop if it encounters:
/// * a null pointer as saved ebp/eip (expected at the bottom of the stack)
/// * any ebp/esp falling outside of the stack
///
/// The data of every stack frame will be hexdumped
pub fn dump_stack(stack: &[u8], orig_address: usize, mut esp: usize, mut ebp: usize, mut eip: usize) {
    use logger::*;
    use core::fmt::Write;
    use utils::print_hexdump_as_if_at_addr;

    writeln!(Loggers, "---------- Dumping stack ---------");
    writeln!(Loggers, "# Stack start: {:#010x}, Stack end: {:#010x}", orig_address, orig_address + stack.len());
    let mut frame_nb = 0;
    loop {
        if eip == 0x00000000 || ebp == 0x00000000 { break; } // reached end of stack

        writeln!(Loggers, "> Frame #{} - eip: {:#010x} - esp: {:#010x} - ebp: {:#010x}", frame_nb, eip, esp, ebp);
        let esp_off = esp - orig_address;
        let ebp_off = ebp - orig_address;
        if esp_off >= stack.len() { writeln!(Loggers, "Invalid esp"); break; }
        if ebp_off >  stack.len() { writeln!(Loggers, "Invalid ebp"); break; }
        let frame_slice = &stack[esp_off..ebp_off];
        print_hexdump_as_if_at_addr(frame_slice, orig_address + esp_off);

        // fetch saved ebp/eip at [ebp]
        if ebp_off + 8 > stack.len() { writeln!(Loggers, "Cannot access saved ebp/eip"); break; }
        let saved_ebp_addr = &stack[ebp_off + 0] as *const u8 as *const usize;
        let saved_eip_addr = &stack[ebp_off + 4] as *const u8 as *const usize;

        writeln!(Loggers, "Saved ebp: {:#010x} @ {:#010x} (ebp) - Saved eip: {:#010x} @ {:#010x} (ebp + 4)",
                 unsafe {*saved_ebp_addr}, saved_ebp_addr as usize,
                 unsafe {*saved_eip_addr}, saved_eip_addr as usize);

        // move esp down one stack frame
        esp = ebp;

        // move ebp and eip to the saved value
        ebp = unsafe { *saved_ebp_addr };
        eip = unsafe { *saved_eip_addr };

        frame_nb += 1;
    }
    writeln!(Loggers, "-------- End of stack dump --------");
}
