///! # Page table / directory

use core::ops::{Index, IndexMut};

// Yeah, I'm ugly. Screw you.
#[path = "entry.rs"]
pub mod entry;

use self::entry::*;
use super::*;
use ::frame_alloc::{Frame, FrameAllocator, MEMORY_FRAME_SIZE, PhysicalAddress, VirtualAddress};
use core::ops::Deref;
use core::ops::DerefMut;

/// A page table
pub struct PageTable {
    entries: [Entry; ENTRY_COUNT]
}

/// The page directory
pub struct PageDirectory(PageTable);

// Assertions
fn __assertions() {
    const_assert!(::core::mem::size_of::<PageDirectory>() >= MEMORY_FRAME_SIZE);
    const_assert!(::core::mem::size_of::<PageTable>() >= MEMORY_FRAME_SIZE);
    const_assert!(::core::mem::size_of::<PageTable>() == ::core::mem::size_of::<PageDirectory>());
}

/// When paging is on, accessing this address loops back to the directory itself thanks to
/// recursive mapping on directory's last entry
pub const DIRECTORY_RECURSIVE_ADDRESS: VirtualAddress = VirtualAddress(0xffff_f000);

/// Implementing Index so we can do `table[42]` to get the 42nd entry easily
impl Index<usize> for PageDirectory {
    type Output = Entry;

    fn index (&self, index: usize) -> &Entry { &self.entries()[index] }
}

impl Index<usize> for PageTable {
    type Output = Entry;

    fn index (&self, index: usize) -> &Entry { &self.entries()[index] }
}

impl IndexMut<usize> for PageDirectory {
    fn index_mut(&mut self, index: usize) -> &mut Entry { &mut self.entries_mut()[index] }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Entry { &mut self.entries_mut()[index] }
}

/// A table of entries, either the directory or one of the page tables
pub trait HierarchicalTable {

    fn entries(&self) -> &[Entry; ENTRY_COUNT];
    fn entries_mut(&mut self) -> &mut [Entry; ENTRY_COUNT];

    /// zero out the whole table
    fn zero(&mut self) {
        for entry in self.entries_mut().iter_mut() {
            entry.set_unused();
        }
    }

    /// Creates a mapping on the nth entry of a table
    /// T is a flusher describing if we should flush the TLB or not
    fn map_nth_entry<T: Flusher>(&mut self, entry: usize, frame: Frame, flags: EntryFlags) {
        self.entries_mut()[entry].set(frame, flags);
        T::flush_cache();
    }

    fn flush_cache() {
        // Don't do anything by default
    }
}

impl HierarchicalTable for PageTable {
    fn entries(&self) -> &[Entry; ENTRY_COUNT] { &self.entries }
    fn entries_mut(&mut self) -> &mut [Entry; ENTRY_COUNT] { &mut self.entries }
}
impl HierarchicalTable for PageDirectory {
    fn entries(&self) -> &[Entry; ENTRY_COUNT] { &self.0.entries }
    fn entries_mut(&mut self) -> &mut [Entry; ENTRY_COUNT] { &mut self.0.entries }
}


/* ********************************************************************************************** */

pub trait PageTableTrait : HierarchicalTable {
    type FlusherType : Flusher;
    /// Used at startup when creating the first page tables.
    fn map_whole_table(&mut self, start_address: PhysicalAddress, flags: EntryFlags) {
        let mut addr = start_address.addr();
        for entry in &mut self.entries_mut()[..] {
            entry.set(Frame::from_physical_addr(PhysicalAddress(addr)), flags);
            addr += PAGE_SIZE;
        }
        Self::FlusherType::flush_cache();
    }
}

pub trait PageDirectoryTrait : HierarchicalTable {
    type PageTableType : PageTableTrait;
    type FlusherType : Flusher;

    /// Gets a reference to a page table through recursive mapping
    fn get_table(&mut self, index: usize) -> Option<SmartHierarchicalTable<Self::PageTableType>>;

    /// Allocates a page table, zero it and add an entry to the directory pointing to it
    fn create_table(&mut self, index: usize) -> SmartHierarchicalTable<Self::PageTableType>;

    /// Gets the page table at given index, or creates it if it does not exist
    fn get_table_or_create(&mut self, index: usize) -> SmartHierarchicalTable<Self::PageTableType> {
        if !self.entries()[index].is_unused() {
            self.get_table(index).unwrap()
        } else {
            self.create_table(index)
        }
    }

    /// Creates a mapping in the page tables with the given flags
    fn map_to(&mut self, page:    Frame,
                         address: VirtualAddress,
                         flags:   EntryFlags) {
        let table_nbr = address.addr() / (ENTRY_COUNT * PAGE_SIZE);
        let table_off = address.addr() % (ENTRY_COUNT * PAGE_SIZE) / PAGE_SIZE;
        let mut table = self.get_table_or_create(table_nbr);
        assert!(table.entries()[table_off].is_unused(), "Tried to map an already mapped entry");
        table.map_nth_entry::<Self::FlusherType>(table_off, page, flags);
    }

    /// Deletes a mapping in the page tables, optionally free the pointed frame
    fn __unmap(&mut self, page: VirtualAddress, free_frame: bool) {
        let table_nbr = page.addr() / (ENTRY_COUNT * PAGE_SIZE);
        let table_off = page.addr() % (ENTRY_COUNT * PAGE_SIZE) / PAGE_SIZE;
        let mut table = self.get_table(table_nbr)
        // TODO: Return an Error if the table was not present
            .unwrap();
        // TODO: Return an Error if the address was not mapped
        let entry= &mut table.entries_mut()[table_off];
        assert_eq!(entry.is_unused(), false);
        if free_frame {
           match entry.pointed_frame() {
               Some(frame_addr) => { FrameAllocator::free_frame(frame_addr as Frame); }
               None => {}
           };
        }
        entry.set_unused();
        Self::FlusherType::flush_cache();
    }

    /// Finds a virtual space hole that can contain page_nb consecutive pages
    /// Alignment is a mask that the first page address must satisfy (ex: 16 for 0x....0000)
    fn find_available_virtual_space_aligned<Land: VirtualSpaceLand>(&mut self,
                                                            page_nb: usize,
                                                            alignement: usize) -> Option<VirtualAddress> {
        fn compute_address(table: usize, page: usize) -> VirtualAddress {
            VirtualAddress(table * ENTRY_COUNT * PAGE_SIZE + page * PAGE_SIZE)
        }
        fn satisfies_alignement(table: usize, page: usize, alignment: usize) -> bool {
            let mask : usize = (1 << alignment) - 1;
            compute_address(table, page).addr() & mask == 0
        }
        let mut considering_hole: bool = false;
        let mut hole_size: usize = 0;
        let mut hole_start_table: usize = 0;
        let mut hole_start_page:  usize = 0;
        let mut counter_curr_table:  usize = Land::start_table();
        let mut counter_curr_page:   usize = 0;
        while counter_curr_table < Land::end_table() && (!considering_hole || hole_size < page_nb) {
            counter_curr_page = 0;
            match self.get_table(counter_curr_table) {
                None => { // The whole page table is free, so add it to our hole_size
                    if !considering_hole
                        && satisfies_alignement(counter_curr_page, 0, alignement) {
                        // This is the start of a hole
                        considering_hole = true;
                        hole_start_table = counter_curr_table;
                        hole_start_page = 0;
                        hole_size = 0;
                    }
                    hole_size += ENTRY_COUNT;
                }
                Some(curr_table) => {
                    while counter_curr_page < ENTRY_COUNT && (!considering_hole || hole_size < page_nb) {
                        if curr_table.entries()[counter_curr_page].is_unused() {
                            if !considering_hole
                                && satisfies_alignement(counter_curr_table, counter_curr_page, alignement) {
                                // This is the start of a hole
                                considering_hole = true;
                                hole_start_table = counter_curr_table;
                                hole_start_page = counter_curr_page;
                                hole_size = 0;
                            }
                            hole_size += 1;
                        } else {
                            // The current hole was not big enough, so reset counter
                            considering_hole = false;
                        }
                        counter_curr_page += 1;
                    }
                }
            }
            counter_curr_table += 1;
        };
        if considering_hole && hole_size >= page_nb { // The last tested hole was big enough
            Some(compute_address(hole_start_table, hole_start_page))
        } else { // No hole was big enough
            None
        }
    }

}

/* ********************************************************************************************** */

/// A trait describing the interface of a PageTable hierarchy
/// Implemented by ActivePageTables and InactivePageTables
pub trait PageTablesSet {
    type PageDirectoryType: PageDirectoryTrait;
    /// Gets a reference to the directory
    fn get_directory(&mut self) -> SmartHierarchicalTable<Self::PageDirectoryType>;

    /// Creates a mapping in the page tables with the given flags
    fn map_to(&mut self, page:    Frame,
                         address: VirtualAddress,
                         flags:   EntryFlags) {
        self.get_directory().map_to(page, address, flags)
    }

    /// Creates a mapping in the page tables with the given flags.
    /// Allocates the pointed page
    fn map_allocate_to(&mut self, address: VirtualAddress,
                                  flags:   EntryFlags) {
        let page = FrameAllocator::alloc_frame();
        self.map_to(page, address, flags);
    }


    /// Maps a given frame in the page tables. Takes care of choosing the virtual address
    fn map_frame<Land: VirtualSpaceLand>(&mut self, frame: Frame, flags: EntryFlags) -> VirtualAddress {
        let va = self.find_available_virtual_space::<Land>(1).unwrap();
        self.map_to(frame,va, flags);
        va
    }

    /// Creates a mapping in the page tables with the given flags.
    /// Allocates the pointed page and chooses the virtual address.
    fn get_page<Land: VirtualSpaceLand>(&mut self, flags: EntryFlags) -> VirtualAddress {
        let va = self.find_available_virtual_space::<Land>(1).unwrap();
        self.map_allocate_to(va, flags);
        va
    }

    /// Reserves a given page as guard page.
    /// This affects only virtual memory and doesn't take any actual physical frame.
    fn map_page_guard(&mut self, address: VirtualAddress) {
        // Just map to frame 0, it will page fault anyway since PRESENT is missing
        self.map_to(Frame::from_physical_addr(PhysicalAddress(0x00000000)), address,EntryFlags::GUARD_PAGE)
    }

    /// Maps a given number of consecutive pages at a given address
    /// Allocates the frames
    fn map_range_allocate(&mut self, address: VirtualAddress, page_nb: usize, flags: EntryFlags) {
        let address_end = VirtualAddress(address.addr() + (page_nb * PAGE_SIZE));
        for current_address in (address.addr()..address_end.addr()).step_by(PAGE_SIZE) {
            self.map_allocate_to(VirtualAddress(current_address), flags);
        }
    }

    /// Maps a memory frame to the same virtual address
    fn identity_map(&mut self, frame: Frame, flags: EntryFlags) {
        self.map_to(frame, VirtualAddress(frame.address().addr()), flags);
    }

    fn identity_map_region(&mut self, start_address: PhysicalAddress, region_size: usize, flags: EntryFlags) {
        assert_eq!(start_address.addr() % PAGE_SIZE, 0, "Tried to map a non paged-aligned region");
        let start = round_to_page(start_address.addr());
        let end = round_to_page_upper(start_address.addr() + region_size);
        for frame_addr in (start..end).step_by(PAGE_SIZE) {
            let frame = Frame::from_physical_addr(PhysicalAddress(frame_addr));
            self.identity_map(frame, flags);
        }
    }

    /// Deletes a mapping in the page tables
    fn unmap(&mut self, page: VirtualAddress) {
       self.get_directory().__unmap(page, false)
    }

    /// Deletes a mapping in the page tables
    /// Frees the pointed frame
    fn unmap_free(&mut self, page: VirtualAddress) {
        self.get_directory().__unmap(page, true)
    }

    /// Finds a virtual space hole that can contain page_nb consecutive pages
    fn find_available_virtual_space_aligned<Land: VirtualSpaceLand>(&mut self, page_nb: usize, alignement: usize) -> Option<VirtualAddress> {
        self.get_directory().find_available_virtual_space_aligned::<Land>(page_nb, alignement)
    }

    /// Finds a virtual space hole that can contain page_nb consecutive pages
    fn find_available_virtual_space<Land: VirtualSpaceLand>(&mut self, page_nb: usize) -> Option<VirtualAddress> {
        // find_available_available_virtual_space_aligned with any alignement
        self.get_directory().find_available_virtual_space_aligned::<Land>(page_nb, 0)
    }
}

/// A macro to easily implement Index and Deref traits on our PageTableSets
macro_rules! inherit_deref_index {
    ($ty:ty, $sub_ty:ty) => {
        impl Deref for $ty {
            type Target = $sub_ty;
            fn deref(&self) -> &<Self as Deref>::Target { &self.0 }
        }
        impl DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut <Self as Deref>::Target { &mut self.0 }
        }
        impl Index<usize> for $ty {
            type Output = <$sub_ty as Index<usize>>::Output;
            fn index (&self, index: usize) -> &Entry { &self.0[index] }
        }
        impl IndexMut<usize> for $ty {
            fn index_mut (&mut self, index: usize) -> &mut Entry { &mut self.0[index] }
        }
    }
}

macro_rules! impl_hierachical_table {
    ($ty: ty) => {
        impl HierarchicalTable for $ty {
            fn entries(&self) -> &[Entry; ENTRY_COUNT] { self.0.entries() }
            fn entries_mut(&mut self) -> &mut [Entry; ENTRY_COUNT] { self.0.entries_mut() }
        }
    };
}

/* ********************************************************************************************** */

/// The page directory currently in use.
/// This struct is used to manage rust ownership.
/// Used when paging is already on (recursive mapping of the directory)
pub struct ActivePageTables ();

impl PageTablesSet for ActivePageTables {
    type PageDirectoryType = ActivePageDirectory;
    fn get_directory(&mut self) -> SmartHierarchicalTable<ActivePageDirectory> {
        SmartHierarchicalTable(DIRECTORY_RECURSIVE_ADDRESS.addr() as *mut ActivePageDirectory)
    }
}

/// The page directory currently in use.
/// Its last entry enables recursive mapping, which we use to access and modify it
pub struct ActivePageDirectory(PageDirectory);
inherit_deref_index!(ActivePageDirectory, PageDirectory);
impl_hierachical_table!(ActivePageDirectory);

impl PageDirectoryTrait for ActivePageDirectory {
    type PageTableType = ActivePageTable;
    type FlusherType = TlbFlush;

    /// Gets a reference to a page table through recursive mapping
    fn get_table(&mut self, index: usize) -> Option<SmartHierarchicalTable<Self::PageTableType>> {
        self.get_table_address(index)
            .map(|addr| SmartHierarchicalTable(unsafe { &mut * (addr as *mut _) }))
    }

    /// Allocates a page table, zero it and add an entry to the directory pointing to it
    fn create_table(&mut self, index: usize) -> SmartHierarchicalTable<Self::PageTableType> {
        assert!(self.entries()[index].is_unused());
        let table_frame = FrameAllocator::alloc_frame();

        self.map_nth_entry::<Self::FlusherType>(index, table_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);

        // Now that table is mapped in page directory we can write to it through recursive mapping
        let mut table= self.get_table(index).unwrap();
        table.zero();
        table
    }
}

impl ActivePageDirectory {
    /// reduce recursive mapping by one time to get further down in table hierarchy
    fn get_table_address(&self, index: usize) -> Option<usize> {
        let entry_flags = self[index].flags();
        if entry_flags.contains(EntryFlags::PRESENT) {
            let table_address = self as *const _ as usize;
            Some((table_address << 10) | (index << 12))
        }
        else {
            None
        }
    }
}

/// A page table currently in use.
pub struct ActivePageTable(PageTable);
inherit_deref_index!(ActivePageTable, PageTable);
impl_hierachical_table!(ActivePageTable);

impl PageTableTrait for ActivePageTable { type FlusherType = TlbFlush; }

/* ********************************************************************************************** */

/// This is just a wrapper for a pointer to a Table or a Directory
/// It enables us to do handle when it is dropped
pub struct SmartHierarchicalTable<T: HierarchicalTable>(*mut T);

impl<T: HierarchicalTable> Deref for SmartHierarchicalTable<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            self.0.as_ref().unwrap()
        }
    }
}

impl<T: HierarchicalTable> DerefMut for SmartHierarchicalTable<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            self.0.as_mut().unwrap()
        }
    }
}

impl<T: HierarchicalTable> Drop for SmartHierarchicalTable<T> {
    fn drop(&mut self) {
        unsafe {
            ::core::ptr::drop_in_place(self.0);
        }
    }
}

/// A set of PageTables that are not the ones currently in use.
/// We can't use recursive mapping to modify them, so instead we have to temporarily
/// map the directory and tables to make changes to them.
pub struct InactivePageTables {
    // The address we must put in cr3 to switch to these pages
    directory_physical_address: PhysicalAddress,
}

impl PageTablesSet for InactivePageTables {
    type PageDirectoryType = InactivePageDirectory;

    /// Temporary map the directory
    fn get_directory(&mut self) -> SmartHierarchicalTable<InactivePageDirectory> {
        let frame = Frame::from_physical_addr(self.directory_physical_address);
        let mut active_pages = ACTIVE_PAGE_TABLES.lock();
        let va = active_pages.map_frame::<KernelLand>(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
        SmartHierarchicalTable(va.addr() as *mut InactivePageDirectory)
    }
}

impl InactivePageTables {
    /// Creates a new set of inactive page tables
    pub fn new() -> InactivePageTables {
        let mut directory_frame = FrameAllocator::alloc_frame();
        let mut pageset = InactivePageTables {
            directory_physical_address: PhysicalAddress(directory_frame.dangerous_as_physical_ptr() as *mut u8 as usize)
        };
        {
            let mut dir = pageset.get_directory();
            dir.zero();
            dir.map_nth_entry::<NoFlush>(ENTRY_COUNT - 1, directory_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
        };
        pageset
    }

    /// Switch to this page tables set.
    /// Returns the old active page tables set after the switch
    pub fn switch_to(&self) -> InactivePageTables {
        let old_pages = super::swap_cr3(self.directory_physical_address);
        InactivePageTables { directory_physical_address: old_pages }
    }

    /// Frees the frames occupied by the page directory and page tables of this set
    /// Does not free the pages mapped by this set
    pub fn delete(mut self) {
        self.get_directory().delete();
    }
}

/// A temporary mapped page directory.
pub struct InactivePageDirectory(PageDirectory);
inherit_deref_index!(InactivePageDirectory, PageDirectory);
impl_hierachical_table!(InactivePageDirectory);

/// A temporary mapped page table.
pub struct InactivePageTable(PageTable);
inherit_deref_index!(InactivePageTable, PageTable);
impl_hierachical_table!(InactivePageTable);


impl PageDirectoryTrait for InactivePageDirectory {
    type PageTableType = InactivePageTable;
    type FlusherType = NoFlush;

    fn get_table(&mut self, index: usize) -> Option<SmartHierarchicalTable<Self::PageTableType>> {
        match self.entries()[index].pointed_frame() {
            None => None,
            Some(frame) => {
                let mut active_pages = ACTIVE_PAGE_TABLES.lock();
                let va = active_pages.map_frame::<KernelLand>(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
                Some(SmartHierarchicalTable(unsafe {va.addr() as *mut InactivePageTable}))
            }
        }
    }

    fn create_table(&mut self, index: usize) -> SmartHierarchicalTable<Self::PageTableType> {
        assert!(self.entries()[index].is_unused());
        let mut table_frame = FrameAllocator::alloc_frame();
        let mut active_pages = ACTIVE_PAGE_TABLES.lock();

        let va = active_pages.map_frame::<KernelLand>(table_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
        let mut mapped_table = SmartHierarchicalTable(unsafe {va.addr() as *mut InactivePageTable});
        mapped_table.zero();

        self.map_nth_entry::<Self::FlusherType>(index, table_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);

        mapped_table
    }
}

impl InactivePageDirectory {
    /// Frees the frames occupied by the page directory and page tables
    /// Does not free the pages mapped by this directory
    fn delete(&mut self) {
        // We're a destructor, we should not take self by reference, but a directory structure is PAGE_SIZE big
        // and the public function in the set takes self by value so it's ok
        for table in self.entries_mut().iter_mut() {
            if let Some(table_frame) = table.pointed_frame() {
                FrameAllocator::free_frame(table_frame);
            }
            // Directory frees itself by freeing the recursive last entry
        }
    }
}

impl PageTableTrait for InactivePageTable { type FlusherType = NoFlush; }

/// When the temporary inactive directory is drop, we unmap it
impl Drop for InactivePageDirectory {
    fn drop(&mut self) {
        let mut active_pages = ACTIVE_PAGE_TABLES.lock();
        active_pages.unmap(VirtualAddress(self as *mut _ as usize));
    }
}

/// When the temporary inactive table is drop, we unmap it
impl Drop for InactivePageTable {
    fn drop(&mut self) {
        let mut active_pages = ACTIVE_PAGE_TABLES.lock();
        active_pages.unmap(VirtualAddress(self as *mut _ as usize));
    }
}

/* ********************************************************************************************** */

/// Used at startup when paging is off to create and initialized the first page tables
///
/// # Safety
///
/// Manipulating this pages set must only be done when paging is off
pub struct PagingOffPageSet {
    // The address we must put in cr3 to switch to these pages
    pub directory_physical_address: PhysicalAddress,
}

impl PageTablesSet for PagingOffPageSet {
    type PageDirectoryType = PagingOffDirectory;
    fn get_directory(&mut self) -> SmartHierarchicalTable<<Self as PageTablesSet>::PageDirectoryType> {
        SmartHierarchicalTable(self.directory_physical_address.addr() as *mut PagingOffDirectory)
    }
}

impl PagingOffPageSet {
    /// Used at startup when the paging is disabled and creating the first page tables.
    ///
    /// # Safety
    ///
    /// Paging **must** be disabled when calling this function.
    pub unsafe fn paging_off_create_page_set() -> Self {
        let dir = FrameAllocator::alloc_frame().dangerous_as_physical_ptr()
            as *mut PagingOffDirectory;
        (*dir).paging_off_init_page_directory();
        Self { directory_physical_address : PhysicalAddress(dir as usize) }
    }
}

/// A directory we can modify by directly accessing physical memory because paging is off
pub struct PagingOffDirectory(PageDirectory);
inherit_deref_index!(PagingOffDirectory, PageDirectory);
impl_hierachical_table!(PagingOffDirectory);

impl PageDirectoryTrait for PagingOffDirectory {
    type PageTableType = PagingOffTable;
    type FlusherType = NoFlush;

    fn get_table(&mut self, index: usize) -> Option<SmartHierarchicalTable<Self::PageTableType>> {
        match self.entries()[index].pointed_frame() {
            None => None,
            Some(frame) => Some(
                SmartHierarchicalTable(unsafe {(frame.dangerous_as_physical_ptr() as *mut PagingOffTable)})
            )
        }
    }
    fn create_table(&mut self, index: usize) -> SmartHierarchicalTable<Self::PageTableType> {
        let mut frame = FrameAllocator::alloc_frame();
        SmartHierarchicalTable(unsafe {(frame.dangerous_as_physical_ptr() as *mut PagingOffTable)})
    }
}

impl PagingOffDirectory {
    /// Used at startup when creating the first page tables.
    /// This function does two things :
    ///     * simply allocates one child page table and fills it with identity mappings entries
    ///       therefore identity mapping the first 4Mb of memory
    ///     * makes the last directory entry a recursive mapping
    ///
    /// # Safety
    ///
    /// Paging **must** be disabled when calling this function.
    unsafe fn paging_off_init_page_directory(&mut self) {
        let first_table_frame = FrameAllocator::alloc_frame();
        let first_table = first_table_frame
            .dangerous_as_physical_ptr() as *mut PagingOffTable;

        (*first_table).zero();
        (*first_table).map_whole_table(PhysicalAddress(0x00000000), EntryFlags::PRESENT | EntryFlags::WRITABLE);

        self.zero();
        self.entries_mut()[0].set(first_table_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);

        let self_frame = Frame::from_physical_addr(PhysicalAddress(self as *mut _ as usize));
        // Make last entry of the directory point to the directory itself
        self.entries_mut()[ENTRY_COUNT - 1].set(self_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
    }
}

pub struct PagingOffTable(PageTable);
inherit_deref_index!(PagingOffTable, PageTable);
impl_hierachical_table!(PagingOffTable);

impl PageTableTrait for PagingOffTable { type FlusherType = NoFlush; }

/* ********************************************************************************************** */

/// A trait used to decide if the TLB cache should be flushed or not
pub trait Flusher {
    fn flush_cache() {}
}

/// When passing this struct the TLB will be flushed. Used by ActivePageTables
pub struct TlbFlush;
impl Flusher for TlbFlush { fn flush_cache() { flush_tlb(); } }

/// When passing this struct the TLB will **not** be flushed. Used by Inactive/PagingOff page tables
pub struct NoFlush;
impl Flusher for NoFlush { fn flush_cache() { } }