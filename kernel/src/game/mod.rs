/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{cell::OnceCell, ops::Range};

use bevy_ecs::prelude::*;

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use bevy_math::{UVec2, Vec2};

pub mod ecs;
pub mod physics;
pub mod player;
pub mod render;

use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};
use rand::{Rng, SeedableRng, distr::uniform::SampleUniform};

use crate::{
    arch::{
        keyboard::{KeyboardState, keyboard_system},
        time::preferred_timer_ns,
    },
    game::{
        ecs::*,
        physics::{collision_check, physics_update},
        player::{Score, game_over, player_setup, player_update, update_score},
        render::render_fixed_update,
    },
    utils::{bootloader::get_framebuffers, fb::Framebuffer},
};

pub const FRAMETIME_60FPS: f32 = 1.0 / 60.0;

pub static mut WORLD: OnceCell<World> = OnceCell::new();

#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub enum MenuState {
    #[default]
    Main,
    Playing,
    GameOver,
}

pub fn setup(mut commands: Commands, fb: Res<Framebuffer>) {
    let s = "WELCOME TO FLAPPYOS";

    commands.spawn((
        Text::new(s).with_shadow(UVec2::new(2, 2), 0xABABAB),
        Transform::from_translation(
            UVec2::new(fb.centered_str_x(s, 2.0), fb.font_height * 2).as_vec2(),
        )
        .with_scale(Vec2::splat(2.0)),
        StateScoped(MenuState::Main),
    ));

    let s = "PRESS SPACE TO BEGIN";

    commands.spawn((
        Text::new(s).with_shadow(UVec2::new(2, 2), 0xABABAB),
        Transform::from_translation(
            UVec2::new(
                fb.centered_str_x(s, 2.0),
                fb.centered_str_y(2.0) + fb.font_height * 2,
            )
            .as_vec2(),
        )
        .with_scale(Vec2::splat(2.0)),
        StateScoped(MenuState::Main),
    ));
}

pub fn press_space_to_begin(mut state: ResMut<MenuState>) {
    *state = MenuState::Playing;
}

pub fn game_loop() -> ! {
    unsafe { WORLD.set(World::new()).unwrap() };

    let world = unsafe { WORLD.get_mut().unwrap() };

    world.insert_resource(Framebuffer::new_from_limine(
        &get_framebuffers().next().unwrap(),
    ));

    world.insert_resource(Random {
        rng: rand::SeedableRng::seed_from_u64(preferred_timer_ns()),
    });

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
        last_keys_down: Vec::new(),
        scancodes: VecDeque::new(),
    });

    world.init_resource::<MenuState>();
    world.init_resource::<Score>();

    let mut startup_schedule = Schedule::new(Startup);
    startup_schedule.add_systems(self::setup);
    startup_schedule.run(world);

    let mut update_schedule = Schedule::new(Update);

    // actual update schedule
    update_schedule.add_systems((
        player_update,
        keyboard_system,
        update_score,
        press_space_to_begin.run_if(
            not(resource_exists_and_equals(MenuState::Playing))
                .and(input_just_pressed(KeyCode::Spacebar)),
        ),
    ));

    // onenter
    update_schedule.add_systems(
        (
            state_scoped,
            setup.run_if(in_state(MenuState::Main)),
            player_setup.run_if(in_state(MenuState::Playing)),
            game_over.run_if(in_state(MenuState::GameOver)),
        )
            .run_if(resource_changed::<MenuState>),
    );

    let mut fixed_update_schedule = Schedule::new(FixedUpdate);
    fixed_update_schedule.add_systems((physics_update, collision_check, render_fixed_update));

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

pub fn get_random<T: SampleUniform + PartialOrd>(range: Range<T>) -> T {
    let mut rng = rand::prelude::SmallRng::seed_from_u64(preferred_timer_ns());
    rng.random_range(range)
}
