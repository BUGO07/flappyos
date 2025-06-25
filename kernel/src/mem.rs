/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{alloc::Layout, cell::OnceCell, ptr::null_mut};

use alloc::sync::Arc;
use limine::memory_map::EntryType;
use spin::mutex::Mutex;
use talc::*;

use crate::utils::bootloader::{
    get_executable_address, get_executable_file, get_hhdm_offset, get_memory_map,
};

pub const KERNEL_STACK_SIZE: usize = 64 * 1024;
pub static KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

#[global_allocator]
pub static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> =
    Talc::new(unsafe { ClaimOnOom::new(Span::from_array((&raw const KERNEL_STACK).cast_mut())) })
        .lock();

pub fn init() {
    unsafe {
        let hhdm_offset = get_hhdm_offset();
        let mem_map = get_memory_map();

        let mut allocator = ALLOCATOR.lock();

        for entry in mem_map {
            if entry.entry_type == EntryType::USABLE {
                allocator
                    .claim(talc::Span::from_base_size(
                        (entry.base + hhdm_offset) as *mut u8,
                        entry.length as usize,
                    ))
                    .ok();
            } else if entry.entry_type == EntryType::RESERVED {
            }
        }
    }

    {
        let mem_map = get_memory_map();
        let hhdm_offset = get_hhdm_offset();

        let mut pmap = Pagemap::new(); // Use x86_64 specific Pagemap

        for entry in mem_map {
            let etype = entry.entry_type;
            if etype != EntryType::USABLE
                && etype != EntryType::BOOTLOADER_RECLAIMABLE
                && etype != EntryType::EXECUTABLE_AND_MODULES
                && etype != EntryType::FRAMEBUFFER
            {
                continue;
            }

            // ! hard setting this to LARGE stops my laptop from crashing
            let psize = if entry.length >= page_size::LARGE {
                page_size::LARGE
            } else if entry.length >= page_size::MEDIUM {
                page_size::MEDIUM
            } else {
                page_size::SMALL
            };

            let base = align_down(entry.base, psize);
            let end = align_up(entry.base + entry.length, psize);

            for i in (base..end).step_by(psize as usize) {
                if !(i <= !0 - hhdm_offset || i >= hhdm_offset) {
                    continue;
                }
                if !pmap.map(i + hhdm_offset, i, flag::RW, psize) {
                    panic!("couldn't map 0x{:X} -> 0x{:X}", i, i + hhdm_offset);
                }
            }
        }

        let executable_address_response = get_executable_address();
        let phys_base = executable_address_response.physical_base();
        let virt_base = executable_address_response.virtual_base();

        let size = get_executable_file().size();

        for i in (0..size).step_by(page_size::SMALL as usize) {
            if !pmap.map(virt_base + i, phys_base + i, flag::RW, page_size::SMALL) {
                panic!(
                    "couldn't map kernel executable 0x{:X} -> 0x{:X}",
                    virt_base + i,
                    phys_base + i
                );
            }
        }

        unsafe {
            // Set CR3 register with the physical address of the PML4 table
            core::arch::asm!("mov cr3, {}", in(reg) pmap.top_level as u64, options(nostack));
            PAGEMAP.set(Arc::new(Mutex::new(pmap))).ok();
        }
    }
}

pub mod page_size {
    pub const SMALL: u64 = 0x1000; // 4KiB
    pub const MEDIUM: u64 = 0x200000; // 2MiB
    pub const LARGE: u64 = 0x40000000; // 1GiB
}

pub mod flag {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const LPAGES: u64 = 1 << 7;
    pub const NO_EXEC: u64 = 1 << 63;

    pub const RW: u64 = PRESENT | WRITE;
}

pub static mut PAGEMAP: OnceCell<Arc<Mutex<Pagemap>>> = OnceCell::new();

unsafe impl Send for Pagemap {}
unsafe impl Sync for Pagemap {}

#[repr(C, packed)]
pub struct Table {
    pub entries: [u64; 512],
}

#[derive(Copy, Clone)]
pub struct Pagemap {
    pub top_level: *mut Table,
    pub used_pages: u64,
}

impl Default for Pagemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Pagemap {
    pub fn new() -> Pagemap {
        Pagemap {
            top_level: alloc_table(),
            used_pages: 0,
        }
    }

    pub fn map(&mut self, virt: u64, phys: u64, mut flags: u64, psize: u64) -> bool {
        let hhdm = get_hhdm_offset();
        if !(phys <= !0 - hhdm || phys >= hhdm) {
            // error!("illegal physical address: 0x{:X}", phys);
            return false;
        }
        let pml4_entry = (virt & (0x1ff << 39)) >> 39;
        let pml3_entry = (virt & (0x1ff << 30)) >> 30;
        let pml2_entry = (virt & (0x1ff << 21)) >> 21;
        let pml1_entry = (virt & (0x1ff << 12)) >> 12;

        let pml4 = (self.top_level as u64 + hhdm) as *mut Table;

        let pml3 = get_next_level(pml4, pml4_entry, true);
        if pml3.is_null() {
            return false;
        }
        if psize == page_size::LARGE {
            flags |= flag::LPAGES;
            unsafe {
                (*pml3).entries[pml3_entry as usize] = phys | flags;
            }
            return true;
        }

        let pml2 = get_next_level(pml3, pml3_entry, true);
        if pml2.is_null() {
            return false;
        }
        if psize == page_size::MEDIUM {
            flags |= flag::LPAGES;
            unsafe {
                (*pml2).entries[pml2_entry as usize] = phys | flags;
            }
            return true;
        }

        let pml1 = get_next_level(pml2, pml2_entry, true);
        if pml1.is_null() {
            return false;
        }

        unsafe {
            (*pml1).entries[pml1_entry as usize] = phys | flags;
            core::arch::asm!("invlpg [{}]", in(reg) virt as *const u8, options(nostack, preserves_flags));
            self.used_pages += psize / 0x1000;
        }

        true
    }

    pub fn unmap(&mut self, virt: u64, psize: u64) -> bool {
        let pml4_entry = (virt >> 39) & 0x1ff;
        let pml3_entry = (virt >> 30) & 0x1ff;
        let pml2_entry = (virt >> 21) & 0x1ff;
        let pml1_entry = (virt >> 12) & 0x1ff;

        let hhdm = get_hhdm_offset();
        let pml4 = (self.top_level as u64 + hhdm) as *mut Table;

        let pml3 = get_next_level(pml4, pml4_entry, false);
        if pml3.is_null() {
            return false;
        }

        if psize == page_size::LARGE {
            unsafe {
                (*pml3).entries[pml3_entry as usize] = 0;
            }
            return true;
        }

        let pml2 = get_next_level(pml3, pml3_entry, false);
        if pml2.is_null() {
            return false;
        }

        if psize == page_size::MEDIUM {
            unsafe {
                (*pml2).entries[pml2_entry as usize] = 0;
            }
            return true;
        }

        let pml1 = get_next_level(pml2, pml2_entry, false);
        if pml1.is_null() {
            return false;
        }

        unsafe {
            (*pml1).entries[pml1_entry as usize] = 0;
            core::arch::asm!("invlpg [{}]", in(reg) virt as *const u8, options(nostack, preserves_flags));
            self.used_pages = self.used_pages.saturating_sub(psize / 0x1000);
        }

        true
    }

    pub fn copy_kernel_map(self) -> Self {
        let src = unsafe { PAGEMAP.get().unwrap().clone() };
        let hhdm = get_hhdm_offset();
        let src_pml4 = (src.lock().top_level as u64 + hhdm) as *const Table;
        let dst_pml4 = (self.top_level as u64 + hhdm) as *mut Table;

        unsafe {
            for i in 256..512 {
                (*dst_pml4).entries[i] = (*src_pml4).entries[i];
            }
        }

        self
    }

    pub fn translate(&self, virt: u64) -> Option<u64> {
        let pml4_index = (virt >> 39) & 0x1ff;
        let pml3_index = (virt >> 30) & 0x1ff;
        let pml2_index = (virt >> 21) & 0x1ff;
        let pml1_index = (virt >> 12) & 0x1ff;
        let offset = virt & 0xfff;
        let hhdm = get_hhdm_offset();

        unsafe {
            let pml4 = (self.top_level as u64 + hhdm) as *const Table;
            let pml4e = (*pml4).entries[pml4_index as usize];
            if pml4e & flag::PRESENT == 0 {
                return None;
            }

            let pml3 = ((pml4e & 0x000FFFFFFFFFF000) + hhdm) as *const Table;
            let pml3e = (*pml3).entries[pml3_index as usize];
            if pml3e & flag::PRESENT == 0 {
                return None;
            }
            if pml3e & flag::LPAGES != 0 {
                let phys = (pml3e & 0x000FFFFFE00000) + (virt & 0x3FFFFFFF); // 1GiB offset
                return Some(phys);
            }

            let pml2 = ((pml3e & 0x000FFFFFFFFFF000) + hhdm) as *const Table;
            let pml2e = (*pml2).entries[pml2_index as usize];
            if pml2e & flag::PRESENT == 0 {
                return None;
            }
            if pml2e & flag::LPAGES != 0 {
                let phys = (pml2e & 0x000FFFFFFFE00000) + (virt & 0x1FFFFF); // 2MiB offset
                return Some(phys);
            }

            let pml1 = ((pml2e & 0x000FFFFFFFFFF000) + hhdm) as *const Table;
            let pml1e = (*pml1).entries[pml1_index as usize];
            if pml1e & flag::PRESENT == 0 {
                return None;
            }

            let phys = (pml1e & 0x000FFFFFFFFFF000) + offset;
            Some(phys)
        }
    }
}

pub fn alloc_table() -> *mut Table {
    unsafe {
        (alloc::alloc::alloc_zeroed(Layout::from_size_align(0x1000, 0x1000).unwrap()) as u64
            - get_hhdm_offset()) as *mut Table
    }
}

fn get_next_level(top_level: *mut Table, idx: u64, allocate: bool) -> *mut Table {
    unsafe {
        let hhdm = get_hhdm_offset();
        let entry = top_level.cast::<u64>().add(idx as usize);
        if !(*entry <= !0 - hhdm || *entry >= hhdm) {
            panic!("illegal entry: 0x{:X}", *entry);
        }
        if *entry & flag::PRESENT != 0 {
            return ((*entry & 0x000FFFFFFFFFF000) + hhdm) as *mut Table;
        }

        if !allocate {
            return null_mut();
        }

        let next_level = alloc_table() as u64;
        *entry = next_level | flag::RW | flag::USER;
        (next_level + hhdm) as *mut Table
    }
}

#[inline(always)]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}

#[inline(always)]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two());
    addr & !(align - 1)
}
