//! Port Io
//!
//! All PIOs implement the [Io] trait, and can be abstracted that way.
//!
//! Stolen from [Redox OS](https://gitlab.redox-os.org/redox-os/syscall/blob/master/src/io/pio.rs).

use core::marker::PhantomData;
use super::Io;

/// Port IO accessor.
#[derive(Copy, Clone, Debug)]
pub struct Pio<T> {
    /// The io port address.
    port: u16,
    /// The width of the port.
    value: PhantomData<T>,
}

impl<T> Pio<T> {
    /// Create a PIO from a given port
    pub const fn new(port: u16) -> Self {
        Pio::<T> {
            port: port,
            value: PhantomData,
        }
    }
}

/// Read/Write for byte PIO
impl Io for Pio<u8> {
    type Value = u8;

    /// Read
    #[inline(always)]
    fn read(&self) -> u8 {
        let value: u8;
        unsafe {
           llvm_asm!("in $0, $1" : "={al}"(value) : "{dx}"(self.port) : "memory" : "intel", "volatile");
        }
        value
    }

    /// Write
    #[inline(always)]
    fn write(&mut self, value: u8) {
        unsafe {
            llvm_asm!("out $1, $0" : : "{al}"(value), "{dx}"(self.port) : "memory" : "intel", "volatile");
        }
    }
}

/// Read/Write for word PIO
impl Io for Pio<u16> {
    type Value = u16;

    /// Read
    #[inline(always)]
    fn read(&self) -> u16 {
        let value: u16;
        unsafe {
            llvm_asm!("in $0, $1" : "={ax}"(value) : "{dx}"(self.port) : "memory" : "intel", "volatile");
        }
        value
    }

    /// Write
    #[inline(always)]
    fn write(&mut self, value: u16) {
        unsafe {
            llvm_asm!("out $1, $0" : : "{ax}"(value), "{dx}"(self.port) : "memory" : "intel", "volatile");
        }
    }
}

/// Read/Write for doubleword PIO
impl Io for Pio<u32> {
    type Value = u32;

    /// Read
    #[inline(always)]
    fn read(&self) -> u32 {
        let value: u32;
        unsafe {
            llvm_asm!("in $0, $1" : "={eax}"(value) : "{dx}"(self.port) : "memory" : "intel", "volatile");
        }
        value
    }

    /// Write
    #[inline(always)]
    fn write(&mut self, value: u32) {
        unsafe {
            llvm_asm!("out $1, $0" : : "{eax}"(value), "{dx}"(self.port) : "memory" : "intel", "volatile");
        }
    }
}
