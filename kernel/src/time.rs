/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::sync::atomic::{AtomicU64, Ordering};

use crate::asm::{outb, outl};

pub const PIT_FREQUENCY: u32 = 1193182;
pub static ELAPSED_MS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    outb(0x43, 0b00110100);
    outl(0x40, (PIT_FREQUENCY / 1000) & 0xFF);
    outl(0x40, (PIT_FREQUENCY / 1000) >> 8);
    crate::ints::pic::unmask(0);
}

pub fn timer_interrupt_handler(_stack_frame: *mut crate::ints::StackFrame) {
    pit_tick();
    crate::ints::pic::send_eoi(0);
}

pub fn pit_tick() {
    ELAPSED_MS.fetch_add(1, Ordering::Relaxed);
}

pub fn current_pit_ticks() -> u64 {
    ELAPSED_MS.load(Ordering::Relaxed)
}

pub fn busywait(ms: u64) {
    let start = current_pit_ticks();
    while current_pit_ticks() - start < ms {
        core::hint::spin_loop();
    }
}
