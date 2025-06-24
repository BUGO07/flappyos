/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use talc::*;

use crate::bootloader::{get_hhdm_offset, get_memory_map};

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
            if entry.entry_type == limine::memory_map::EntryType::USABLE {
                allocator
                    .claim(talc::Span::from_base_size(
                        (entry.base + hhdm_offset) as *mut u8,
                        entry.length as usize,
                    ))
                    .ok();
            } else if entry.entry_type == limine::memory_map::EntryType::RESERVED {
            }
        }
    }
}
