/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;
use glam::Vec2;
use pc_keyboard::KeyCode;

use crate::{
    arch::keyboard::KeyboardState,
    assets::{FLAPPY_BIRD_DATA, FLAPPY_BIRD_SIZE},
    game::{MenuState, StateScoped},
};

use super::ecs::*;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
}

#[derive(Component)]
pub struct BoxEntity;

pub fn player_setup(mut commands: Commands) {
    commands.spawn((
        Transform::from_translation(Vec2::new(50.0, 50.0)),
        Velocity::linear(Vec2::ZERO),
        Collider::new(FLAPPY_BIRD_SIZE),
        Sprite::new(*FLAPPY_BIRD_DATA, FLAPPY_BIRD_SIZE),
        RigidBody::Dynamic,
        Player { speed: 200.0 },
        StateScoped(MenuState::Game),
    ));

    commands.spawn((
        Transform::from_translation(Vec2::new(250.0, 250.0)),
        Velocity::linear(Vec2::ZERO),
        Collider::new(Vec2::splat(50.0)),
        Rect::new(Vec2::splat(50.0), 0xFF00FF),
        RigidBody::Static,
        BoxEntity,
        StateScoped(MenuState::Game),
    ));
}

pub fn player_update(
    mut velocity: Single<&mut Velocity, With<Player>>,
    keyboard_state: Res<KeyboardState>,
) {
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
        velocity.linear = input.normalize_or_zero();
    } else {
        velocity.linear = Vec2::ZERO;
    }
}
