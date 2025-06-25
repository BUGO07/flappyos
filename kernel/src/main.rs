/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#![no_std]
#![no_main]
#![allow(
    static_mut_refs,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::new_ret_no_self,
    clippy::too_many_arguments
)]

extern crate alloc;

pub mod flappy_bird;
pub mod gdt;
pub mod heapless;
pub mod ints;
pub mod keyboard;
pub mod mem;
pub mod player;
pub mod sched;
pub mod time;
pub mod utils;

use core::{cell::OnceCell, sync::atomic::AtomicU64};

use crate::{
    keyboard::keyboard_thread,
    player::player_thread,
    utils::{bootloader::get_framebuffers, fb::Framebuffer},
};

pub static CPU_FREQ: AtomicU64 = AtomicU64::new(0);
pub static mut FB: OnceCell<Framebuffer> = OnceCell::new();
pub const FRAMETIME_60FPS: f32 = 1.0 / 60.0;

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    mem::init();
    gdt::init();
    ints::init();
    ints::pic::init();
    utils::asm::toggle_ints(true);
    time::init();

    let framebuffer = get_framebuffers().next().unwrap();
    unsafe { FB.set(Framebuffer::new_from_limine(&framebuffer)).ok() };

    sched::init();
    sched::spawn_thread(
        sched::get_proc_by_pid(0).unwrap(),
        player_thread as usize,
        "main",
        false,
    );

    sched::spawn_thread(
        sched::get_proc_by_pid(0).unwrap(),
        keyboard_thread as usize,
        "main",
        false,
    );
    sched::start();
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    utils::asm::halt_loop();
}
