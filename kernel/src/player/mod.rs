/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;
use glam::Vec2;
use pc_keyboard::KeyCode;

use crate::{flappy_bird, keyboard::KeyboardState};

pub mod physics;

pub const SPRITE_SIZE: Vec2 = Vec2::new(
    flappy_bird::SPRITE_WIDTH as f32,
    flappy_bird::SPRITE_HEIGHT as f32,
);

#[derive(Component)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
}

impl Transform {
    pub fn new(position: Vec2, scale: Vec2, rotation: f32) -> Self {
        Self {
            position,
            scale,
            rotation,
        }
    }

    pub fn from_translation(position: Vec2) -> Self {
        Self {
            position,
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }

    pub fn from_xy(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }

    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }
}

#[derive(Component)]
pub struct Velocity {
    pub linear: Vec2,
    pub angular: f32,
}

impl Velocity {
    pub fn new(linear: Vec2, angular: f32) -> Self {
        Self { linear, angular }
    }

    pub fn linear(linear: Vec2) -> Self {
        Self {
            linear,
            angular: 0.0,
        }
    }

    pub fn angular(angular: f32) -> Self {
        Self {
            linear: Vec2::ZERO,
            angular,
        }
    }
}

#[derive(Component)]
pub struct Player {
    pub speed: f32,
}

#[derive(Component)]
pub struct Box;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        Transform::from_translation(Vec2::new(50.0, 50.0)),
        Velocity::linear(Vec2::ZERO),
        Player { speed: 200.0 },
    ));

    commands.spawn((
        Transform::from_translation(Vec2::new(250.0, 250.0)).with_scale(Vec2::splat(50.0)),
        Box,
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
