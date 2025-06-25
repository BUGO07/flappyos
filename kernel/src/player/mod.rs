/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use glam::{UVec2, Vec2};
use pc_keyboard::KeyCode;

use crate::{
    FB, FRAMETIME_60FPS, flappy_bird, keyboard::KEYBOARD_STATE, player::physics::aabb_collides,
    time::preferred_timer_ns,
};

pub mod physics;

pub static mut FLAPPY_BIRD_POS: Vec2 = Vec2::new(50.0, 50.0);
pub static mut FLAPPY_BIRD_VELOCITY: Vec2 = Vec2::new(0.0, 0.0);

pub static mut BOX_POS: Vec2 = Vec2::new(250.0, 250.0);
pub static mut BOX_SIZE: Vec2 = Vec2::new(50.0, 50.0);
pub static mut SPRITE_SIZE: Vec2 = Vec2::new(
    flappy_bird::SPRITE_WIDTH as f32,
    flappy_bird::SPRITE_HEIGHT as f32,
);

pub fn player_thread() -> ! {
    let fb = unsafe { FB.get_mut().unwrap() };

    let keyboard_state = unsafe { KEYBOARD_STATE.get_mut() };

    let mut last_time = preferred_timer_ns();
    loop {
        let now = preferred_timer_ns();
        let delta = now - last_time;
        let delta_secs = delta as f32 / 1_000_000_000.0;

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
                FLAPPY_BIRD_VELOCITY = input.normalize_or_zero() * 5.0 * delta_secs;
            } else {
                FLAPPY_BIRD_VELOCITY = Vec2::ZERO;
            }

            let mut next_pos = FLAPPY_BIRD_POS + FLAPPY_BIRD_VELOCITY * delta_secs;

            let test_x = Vec2::new(next_pos.x, FLAPPY_BIRD_POS.y);
            if aabb_collides(test_x, SPRITE_SIZE, BOX_POS, BOX_SIZE) {
                next_pos.x = FLAPPY_BIRD_POS.x;
            }

            let test_y = Vec2::new(next_pos.x, next_pos.y);
            if aabb_collides(test_y, SPRITE_SIZE, BOX_POS, BOX_SIZE) {
                next_pos.y = FLAPPY_BIRD_POS.y;
            }

            FLAPPY_BIRD_POS = next_pos;

            FLAPPY_BIRD_POS.x = FLAPPY_BIRD_POS
                .x
                .clamp(0.0, fb.size.x as f32 - flappy_bird::SPRITE_WIDTH as f32);
            FLAPPY_BIRD_POS.y = FLAPPY_BIRD_POS
                .y
                .clamp(0.0, fb.size.y as f32 - flappy_bird::SPRITE_HEIGHT as f32);

            let is_moving = keyboard_state
                .keys_down
                .iter()
                .any(|k| matches!(k, KeyCode::W | KeyCode::A | KeyCode::S | KeyCode::D));

            if !is_moving {
                FLAPPY_BIRD_VELOCITY = Vec2::ZERO;
            } else {
                let friction = 0.0005;
                let drag = 1.0 / (1.0 + friction * delta_secs as f32);
                FLAPPY_BIRD_VELOCITY *= drag;
            }
        }

        if delta_secs >= FRAMETIME_60FPS {
            last_time = now;

            fb.clear(0x000000);

            unsafe {
                fb.draw_sprite(
                    FLAPPY_BIRD_POS.as_uvec2(),
                    SPRITE_SIZE.as_uvec2(),
                    &flappy_bird::SPRITE_DATA,
                    Some(0),
                );

                let s = "WELCOME TO FLAPPYOS";

                fb.draw_str(
                    UVec2::new(fb.centered_str_x(s, 2.0), fb.font_height * 2),
                    s,
                    0xFFFFFF,
                    None,
                    Vec2::splat(2.0),
                );

                fb.draw_rect(BOX_POS.as_uvec2(), BOX_SIZE.as_uvec2(), 0xFFFF00);
            }

            fb.present();
        }
    }
}
