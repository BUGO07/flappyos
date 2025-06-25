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

pub mod flappy_bird;
pub mod gdt;
pub mod heapless;
pub mod ints;
pub mod keyboard;
pub mod mem;
pub mod player;
pub mod time;
pub mod utils;

use core::{cell::OnceCell, sync::atomic::AtomicU64};

use alloc::{collections::vec_deque::VecDeque, string::ToString, vec::Vec};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use glam::{UVec2, Vec2};
use pc_keyboard::{HandleControl, Keyboard, ScancodeSet1, layouts::Us104Key};

use crate::{
    keyboard::KeyboardState,
    player::{BoxEntity, Player, SPRITE_SIZE, Transform},
    time::preferred_timer_ns,
    utils::{bootloader::get_framebuffers, fb::Framebuffer},
};

pub static CPU_FREQ: AtomicU64 = AtomicU64::new(0);
pub const FRAMETIME_60FPS: f32 = 1.0 / 60.0;
pub static mut WORLD: OnceCell<World> = OnceCell::new();

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    mem::init();
    gdt::init();
    ints::init();
    ints::pic::init();
    utils::asm::toggle_ints(true);
    ints::pic::unmask(1);
    time::init();
    unsafe { WORLD.set(World::new()).unwrap() };
    game_loop();
}

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct Startup;

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct Update;

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct FixedUpdate;

#[derive(Resource, Default, Debug)]
pub struct Time {
    pub last_time: u64,
    pub elapsed_ns: u64,
    pub delta_secs: f32,
    pub fixed_delta_secs: f32, // idfk very sheice
}

pub fn game_loop() -> ! {
    let world = unsafe { WORLD.get_mut().unwrap() };

    world.insert_resource(Framebuffer::new_from_limine(
        &get_framebuffers().next().unwrap(),
    ));

    let time = preferred_timer_ns();

    world.insert_resource(Time {
        last_time: time,
        elapsed_ns: time,
        delta_secs: 0.0,
        fixed_delta_secs: 0.0,
    });

    world.insert_resource(KeyboardState {
        keyboard: Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore),
        keys_down: Vec::new(),
        scancodes: VecDeque::new(),
    });

    let mut startup_schedule = Schedule::new(Startup);
    startup_schedule.add_systems(player::setup);
    startup_schedule.run(world);

    let mut update_schedule = Schedule::new(Update);
    update_schedule.add_systems((player::player_update, keyboard::keyboard_system));

    let mut fixed_update_schedule = Schedule::new(FixedUpdate);
    fixed_update_schedule.add_systems((
        player::physics::physics_update,
        player::physics::collision_check,
        self::render,
    ));

    loop {
        let mut time = world.get_resource_mut::<Time>().unwrap();
        time.last_time = time.elapsed_ns;
        time.elapsed_ns = preferred_timer_ns();

        let delta = time.elapsed_ns - time.last_time;
        time.delta_secs = delta as f32 / 1_000_000_000.0;
        time.fixed_delta_secs += time.delta_secs;
        if time.fixed_delta_secs >= FRAMETIME_60FPS {
            time.fixed_delta_secs -= FRAMETIME_60FPS;
            fixed_update_schedule.run(world);
        }
        update_schedule.run(world);
    }
}

pub fn render(
    mut fb: ResMut<Framebuffer>,
    player: Single<&Transform, (With<Player>, Without<BoxEntity>)>,
    box_q: Single<&Transform, (With<BoxEntity>, Without<Player>)>,
) {
    fb.clear(0x000000);

    fb.draw_sprite(
        player.position.as_uvec2(),
        SPRITE_SIZE.as_uvec2(),
        &flappy_bird::SPRITE_DATA,
        Some(0),
    );

    let s = "WELCOME TO FLAPPYOS";

    let pos = UVec2::new(fb.centered_str_x(s, 2.0), fb.font_height * 2);

    fb.draw_str(pos, s, 0xFFFFFF, None, Vec2::splat(2.0));

    fb.draw_rect(box_q.position.as_uvec2(), box_q.scale.as_uvec2(), 0xFFFF00);

    fb.present();
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
    fb.draw_str(pos, s, 0xFFFFFF, None, Vec2::splat(2.0));
    let s = &alloc::format!(
        "{}:{}:{}",
        location.file(),
        location.line(),
        location.column(),
    );
    let pos: UVec2 = UVec2::new(fb.centered_str_x(s, 1.0), fb.centered_str_y(1.0));
    fb.draw_str(pos, s, 0xFFFFFF, None, Vec2::splat(1.0));
    let pos = UVec2::new(
        fb.centered_str_x(&msg, 1.5),
        fb.centered_str_y(1.5) + fb.font_height * 5,
    );
    fb.draw_str(pos, &msg, 0xFFFFFF, None, Vec2::splat(1.5));
    fb.present();
    utils::asm::halt_loop();
}
