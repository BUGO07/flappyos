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
    clippy::too_many_arguments,
    clippy::type_complexity
)]

extern crate alloc;

pub mod arch;
pub mod assets;
pub mod game;
pub mod utils;

use core::sync::atomic::AtomicU64;

use alloc::string::ToString;

use glam::{UVec2, Vec2};

use crate::utils::{bootloader::get_framebuffers, fb::Framebuffer};

pub static CPU_FREQ: AtomicU64 = AtomicU64::new(0);

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    arch::mem::init();
    arch::gdt::init();
    arch::ints::init();
    arch::ints::pic::init();
    utils::asm::toggle_ints(true);
    arch::ints::pic::unmask(1);
    arch::time::init();
    game::game_loop();
}

// * horrible design but works
#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    utils::asm::toggle_ints(false);
    let mut fb = Framebuffer::new_from_limine(&get_framebuffers().next().unwrap());
    let location = info.location().unwrap();
    let msg = info.message().to_string();

    fb.clear(0x000000);
    let s = "PANIC";
    let pos = UVec2::new(
        fb.centered_str_x(s, 2.0),
        fb.centered_str_y(2.0) - fb.font_height * 5,
    );
    error!("{s}");
    fb.draw_str(pos, s, 0xFFFFFF, None, Vec2::splat(2.0));
    let s = &alloc::format!(
        "{}:{}:{}",
        location.file(),
        location.line(),
        location.column(),
    );
    error!("{s}");
    let pos: UVec2 = UVec2::new(fb.centered_str_x(s, 1.0), fb.centered_str_y(1.0));
    fb.draw_str(pos, s, 0xFFFFFF, None, Vec2::splat(1.0));
    let pos = UVec2::new(
        fb.centered_str_x(&msg, 1.5),
        fb.centered_str_y(1.5) + fb.font_height * 5,
    );
    error!("{msg}");
    fb.draw_str(pos, &msg, 0xFFFFFF, None, Vec2::splat(1.5));
    fb.present();
    utils::asm::halt_loop();
}
