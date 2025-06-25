/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;
use glam::Vec2;

use crate::{
    info,
    player::{Player, Transform, Velocity},
    utils::fb::Framebuffer,
};

#[derive(Component, PartialEq, Eq)]
pub enum RigidBody {
    Static,
    Dynamic,
}

#[derive(Component)]
pub struct Collider {
    size: Vec2,
    offset: Vec2,
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

pub fn aabb_collides(pos1: Vec2, size1: Vec2, pos2: Vec2, size2: Vec2) -> bool {
    let x_overlap = pos1.x < pos2.x + size2.x && pos1.x + size1.x > pos2.x;
    let y_overlap = pos1.y < pos2.y + size2.y && pos1.y + size1.y > pos2.y;
    x_overlap && y_overlap
}

pub fn collision_check(
    collider_query: Query<(Entity, &Collider, &Transform)>,
    rigidbody_query: Query<(Entity, &Collider, &RigidBody, &Transform)>,
) {
    for (collider_entity, collider, transform) in collider_query.iter() {
        for (rigidbody_entity, rigidbody_collider, rigidbody, rigidbody_transform) in
            rigidbody_query.iter()
        {
            if rigidbody == &RigidBody::Dynamic
                && collider_entity != rigidbody_entity
                && aabb_collides(
                    transform.position,
                    collider.size,
                    rigidbody_transform.position,
                    rigidbody_collider.size,
                )
            {
                info!("collision");
            }
        }
    }
}

pub fn physics_update(
    fb: Res<Framebuffer>,
    player: Single<(&mut Transform, &Velocity, &Player)>,
    // box_q: Single<&Transform, (With<Box>, Without<Player>)>,
) {
    let (mut transform, velocity, player) = player.into_inner();

    // * don't going thru the box
    // let test_x = Vec2::new(next_pos.x, transform.position.y);
    // if aabb_collides(test_x, SPRITE_SIZE, box_q.position, box_q.scale) {
    //     next_pos.x = transform.position.x;
    // }

    // let test_y = Vec2::new(next_pos.x, next_pos.y);
    // if aabb_collides(test_y, SPRITE_SIZE, box_q.position, box_q.scale) {
    //     next_pos.y = transform.position.y;
    // }

    transform.position += velocity.linear * player.speed * crate::FRAMETIME_60FPS;

    transform.position.x = transform.position.x.clamp(
        0.0,
        fb.size.x as f32 - crate::flappy_bird::SPRITE_WIDTH as f32,
    );
    transform.position.y = transform.position.y.clamp(
        0.0,
        fb.size.y as f32 - crate::flappy_bird::SPRITE_HEIGHT as f32,
    );
}
