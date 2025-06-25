/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;
use glam::Vec2;

use crate::{
    player::{Box, Player, SPRITE_SIZE, Transform, Velocity},
    utils::fb::Framebuffer,
};

pub fn aabb_collides(pos1: Vec2, size1: Vec2, pos2: Vec2, size2: Vec2) -> bool {
    let x_overlap = pos1.x < pos2.x + size2.x && pos1.x + size1.x > pos2.x;
    let y_overlap = pos1.y < pos2.y + size2.y && pos1.y + size1.y > pos2.y;
    x_overlap && y_overlap
}

pub fn physics_update(
    fb: Res<Framebuffer>,
    player: Single<(&mut Transform, &Velocity, &Player)>,
    box_q: Single<&Transform, (With<Box>, Without<Player>)>,
) {
    let (mut transform, velocity, player) = player.into_inner();

    let mut next_pos = transform.position + velocity.linear * player.speed * crate::FRAMETIME_60FPS;

    let test_x = Vec2::new(next_pos.x, transform.position.y);
    if aabb_collides(test_x, SPRITE_SIZE, box_q.position, box_q.scale) {
        next_pos.x = transform.position.x;
    }

    let test_y = Vec2::new(next_pos.x, next_pos.y);
    if aabb_collides(test_y, SPRITE_SIZE, box_q.position, box_q.scale) {
        next_pos.y = transform.position.y;
    }

    transform.position = next_pos;

    transform.position.x = transform.position.x.clamp(
        0.0,
        fb.size.x as f32 - crate::flappy_bird::SPRITE_WIDTH as f32,
    );
    transform.position.y = transform.position.y.clamp(
        0.0,
        fb.size.y as f32 - crate::flappy_bird::SPRITE_HEIGHT as f32,
    );
}
