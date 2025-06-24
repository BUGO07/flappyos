/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#![no_std]
#![no_main]
#![allow(
    static_mut_refs,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::new_ret_no_self
)]

extern crate alloc;

pub mod asm;
pub mod bootloader;
pub mod fb;
pub mod ints;
pub mod keyboard;
pub mod mem;
pub mod time;

use core::cell::OnceCell;

use glam::Vec2;
use pc_keyboard::KeyCode;

use crate::{
    bootloader::get_framebuffers,
    fb::Framebuffer,
    keyboard::{KEYBOARD_STATE, keyboard},
    time::current_pit_ticks,
};

pub static mut FB: OnceCell<Framebuffer> = OnceCell::new();
pub static mut MARIO_POS: Vec2 = Vec2::new(50.0, 50.0);
pub static mut MARIO_VELOCITY: Vec2 = Vec2::new(0.0, 0.0);
pub static mut VELOCITY_DAMPENING: Vec2 = Vec2::splat(0.9);

pub mod mario;

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    mem::init();
    ints::init();
    ints::pic::init();
    asm::toggle_ints(true);
    time::init();

    let framebuffer = get_framebuffers().next().unwrap();
    unsafe { FB.set(Framebuffer::new_from_limine(&framebuffer)).ok() };
    let fb = unsafe { FB.get_mut().unwrap() };

    crate::ints::pic::unmask(1);
    let keyboard_state = unsafe { KEYBOARD_STATE.get_mut() };

    let mut last_time = current_pit_ticks();
    let mut i: i32 = 0;
    let mut x: i32 = 1;

    loop {
        let now = current_pit_ticks();
        let delta = now - last_time;
        let delta_secs = delta as f32 / 1000.0;

        keyboard(keyboard_state);

        unsafe {
            let mut input = Vec2::ZERO;

            for key in &keyboard_state.keys_down {
                match key {
                    KeyCode::W => input.y -= 1.0,
                    KeyCode::A => input.x -= 1.0,
                    KeyCode::S => input.y += 1.0,
                    KeyCode::D => input.x += 1.0,
                    _ => {}
                }
            }

            if input != Vec2::ZERO {
                MARIO_VELOCITY = input.normalize_or_zero() * 150.0;
            } else {
                MARIO_VELOCITY = Vec2::ZERO;
            }

            MARIO_POS += MARIO_VELOCITY * delta_secs;

            MARIO_POS.x = MARIO_POS
                .x
                .clamp(0.0, fb.width as f32 - mario::SPRITE_WIDTH as f32);
            MARIO_POS.y = MARIO_POS
                .y
                .clamp(0.0, fb.height as f32 - mario::SPRITE_HEIGHT as f32);

            let is_moving = keyboard_state
                .keys_down
                .iter()
                .any(|k| matches!(k, KeyCode::W | KeyCode::A | KeyCode::S | KeyCode::D));

            if !is_moving {
                MARIO_VELOCITY = Vec2::ZERO;
            } else {
                let friction = 0.0005;
                let drag = 1.0 / (1.0 + friction * delta as f32);
                MARIO_VELOCITY *= drag;
            }
        }

        if delta >= 16 {
            last_time = now;

            if i > 200 {
                x = -1;
            }
            if i <= 1 {
                x = 1;
            }
            i += x * (delta as i32 / 10);

            fb.clear(0x000000);
            unsafe {
                fb.draw_str(
                    50,
                    250,
                    alloc::format!("shice\n{}", fb.pitch).as_str(),
                    0xFFFFFF,
                    0x000000,
                );
                fb.draw_sprite(
                    MARIO_POS.x as usize,
                    MARIO_POS.y as usize,
                    mario::SPRITE_WIDTH,
                    mario::SPRITE_HEIGHT,
                    &mario::SPRITE_DATA,
                    Some(0),
                );
            }
            fb.draw_rect((100 + i) as usize, 100, 50, 50, 0xFFFF00);
            fb.present();
        }
        asm::halt();
    }
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    asm::halt_loop();
}
