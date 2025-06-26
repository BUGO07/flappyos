/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::cell::OnceCell;

use bevy_ecs::prelude::*;

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use glam::{UVec2, Vec2};

pub mod ecs;
pub mod physics;
pub mod player;
pub mod render;

use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};

use crate::{
    arch::{
        keyboard::{KeyboardState, keyboard_system},
        time::preferred_timer_ns,
    },
    game::{
        ecs::*,
        physics::{collision_check, physics_update},
        player::{player_setup, player_update},
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
    Game,
}

#[derive(Component)]
pub struct StateScoped(pub MenuState);

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
    *state = MenuState::Game;
}

pub fn in_state<R: Resource + PartialEq>(state: R) -> impl FnMut(Option<Res<R>>) -> bool {
    move |current_state: Option<Res<R>>| match current_state {
        Some(current_state) => *current_state == state,
        None => false,
    }
}

pub fn input_just_pressed(key: KeyCode) -> impl FnMut(Option<Res<KeyboardState>>) -> bool {
    move |current_state: Option<Res<KeyboardState>>| match current_state {
        Some(current_state) => current_state.just_pressed(key),
        None => false,
    }
}

pub fn input_pressed(key: KeyCode) -> impl FnMut(Option<Res<KeyboardState>>) -> bool {
    move |current_state: Option<Res<KeyboardState>>| match current_state {
        Some(current_state) => current_state.pressed(key),
        None => false,
    }
}

pub fn state_scoped(
    mut commands: Commands,
    state: Res<MenuState>,
    query: Query<(Entity, &StateScoped)>,
) {
    for (entity, scope) in &query {
        if scope.0 != *state {
            commands.entity(entity).despawn();
        }
    }
}

pub fn game_loop() -> ! {
    unsafe { WORLD.set(World::new()).unwrap() };

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
        last_keys_down: Vec::new(),
        scancodes: VecDeque::new(),
    });

    world.init_resource::<MenuState>();

    let mut startup_schedule = Schedule::new(Startup);
    startup_schedule.add_systems(self::setup);
    startup_schedule.run(world);

    let mut update_schedule = Schedule::new(Update);
    // true updates
    update_schedule.add_systems((
        player_update,
        keyboard_system,
        state_scoped.run_if(resource_changed::<MenuState>),
        press_space_to_begin.run_if(
            resource_exists_and_equals(MenuState::Main).and(input_just_pressed(KeyCode::Spacebar)),
        ),
    ));

    // onenter
    update_schedule.add_systems(
        (
            player_setup.run_if(in_state(MenuState::Game)),
            setup.run_if(in_state(MenuState::Main)),
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
