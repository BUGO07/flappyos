/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::string::{String, ToString};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use glam::{UVec2, Vec2};

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct Startup;

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct Update;

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct FixedUpdate;

#[derive(Resource, Default, Debug)]
pub struct Time {
    pub last_time: u64,
    pub elapsed_ns: u64,
    pub delta_secs: f32,
    pub fixed_delta_secs: f32, // idfk very sheice
}

#[derive(Component)]
pub struct Sprite {
    pub data: &'static [u32],
    pub size: Vec2,
}

impl Sprite {
    pub fn new(data: &'static [u32], size: Vec2) -> Self {
        Self { data, size }
    }
}

#[derive(Component)]
pub struct Rect {
    pub size: Vec2,
    pub color: u32,
}

impl Rect {
    pub fn new(size: Vec2, color: u32) -> Self {
        Self { size, color }
    }
}

#[derive(Component)]
pub struct Text {
    pub text: String,
    pub fg: u32,
    pub bg: Option<u32>,
    pub shadow: Option<(UVec2, u32)>,
}

impl Text {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            fg: 0xFFFFFFFF,
            bg: None,
            shadow: None,
        }
    }

    pub fn with_color(mut self, fg: u32) -> Self {
        self.fg = fg;
        self
    }

    pub fn with_background(mut self, bg: u32) -> Self {
        self.bg = Some(bg);
        self
    }

    pub fn with_shadow(mut self, offset: UVec2, color: u32) -> Self {
        self.shadow = Some((offset, color));
        self
    }
}

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

#[derive(Component, PartialEq, Eq)]
pub enum RigidBody {
    Static,
    Dynamic,
}

#[derive(Component)]
pub struct Collider {
    pub size: Vec2,
    pub offset: Vec2,
}

impl Collider {
    pub fn new(size: Vec2) -> Self {
        Self {
            size,
            offset: Vec2::ZERO,
        }
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }
}
