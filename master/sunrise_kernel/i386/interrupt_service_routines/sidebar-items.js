initSidebarItems({"fn":[["acpi_handler","Auto generated irq handler. See [`irq_handler`]."],["acpi_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["acpi_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["alignment_check_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["alignment_check_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["bound_range_exceeded_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["bound_range_exceeded_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["breakpoint_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["breakpoint_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["cascade_handler","Auto generated irq handler. See [`irq_handler`]."],["cascade_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["cascade_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["check_thread_killed","Checks if our thread was killed, in which case unschedule ourselves."],["debug_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["debug_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["device_not_available_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["device_not_available_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["divide_by_zero_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["divide_by_zero_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["double_fault_handler","Double fault handler. Panics the kernel unconditionally."],["floppy_handler","Auto generated irq handler. See [`irq_handler`]."],["floppy_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["floppy_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["general_protection_fault_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["general_protection_fault_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["hpet_handler","Auto generated irq handler. See [`irq_handler`]."],["hpet_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["hpet_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["init","Initialize the interrupt subsystem. Sets up the PIC and the IDT."],["invalid_opcode_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["invalid_opcode_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["invalid_tss_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["invalid_tss_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["irq10_handler","Auto generated irq handler. See [`irq_handler`]."],["irq10_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["irq10_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["irq11_handler","Auto generated irq handler. See [`irq_handler`]."],["irq11_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["irq11_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["irq13_handler","Auto generated irq handler. See [`irq_handler`]."],["irq13_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["irq13_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["kernel_page_fault_panic","Overriding the default panic strategy so we can display cr2"],["keyboard_handler","Auto generated irq handler. See [`irq_handler`]."],["keyboard_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["keyboard_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["machine_check_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["machinee_check_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["mouse_handler","Auto generated irq handler. See [`irq_handler`]."],["mouse_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["mouse_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["nmi_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["nmi_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["overflow_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["overflow_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["page_fault_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["page_fault_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["parallel1_handler","Auto generated irq handler. See [`irq_handler`]."],["parallel1_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["parallel1_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["pit_handler","Auto generated irq handler. See [`irq_handler`]."],["pit_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["pit_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["primary_ata_handler","Auto generated irq handler. See [`irq_handler`]."],["primary_ata_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["primary_ata_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["rtc_handler","Auto generated irq handler. See [`irq_handler`]."],["rtc_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["rtc_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["secondary_ata_handler","Auto generated irq handler. See [`irq_handler`]."],["secondary_ata_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["secondary_ata_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["security_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["security_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["segment_not_present_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["segment_not_present_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["serial1_handler","Auto generated irq handler. See [`irq_handler`]."],["serial1_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["serial1_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["serial2_handler","Auto generated irq handler. See [`irq_handler`]."],["serial2_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["serial2_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["simd_floating_point_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["simd_floating_point_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["sound_handler","Auto generated irq handler. See [`irq_handler`]."],["sound_handler_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["sound_handler_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["stack_fault_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["stack_fault_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["syscall_interrupt_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["syscall_interrupt_dispatcher","This is the function called on int 0x80."],["syscall_interrupt_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["user_page_fault_handler","Overriding the default kill strategy so we can display cr2"],["user_page_fault_panic","Overriding the default panic strategy so we can display cr2"],["virtualization_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["virtualization_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["x87_floating_point_exception_asm_wrapper","Auto generated function. See [generate_trap_gate_handler]."],["x87_floating_point_exception_rust_wrapper","Auto generated function. See [generate_trap_gate_handler]."]],"static":[["INSIDE_INTERRUPT_COUNT","Contains the number of interrupts we are currently inside."],["IRQ_HANDLERS","Array of interrupt handlers."]],"struct":[["IDT","IDT address. Initialized in `init()`."],["UserspaceHardwareContext","Represents a register backup."]]});