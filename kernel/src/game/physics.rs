/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;
use glam::Vec2;

use crate::{game::player::Player, info, utils::fb::Framebuffer};

use super::ecs::*;

pub fn aabb_collides(pos1: Vec2, size1: Vec2, pos2: Vec2, size2: Vec2) -> bool {
    let x_overlap = pos1.x < pos2.x + size2.x && pos1.x + size1.x > pos2.x;
    let y_overlap = pos1.y < pos2.y + size2.y && pos1.y + size1.y > pos2.y;
    x_overlap && y_overlap
}

pub fn collision_check(
    collider_query: Query<(Entity, &Collider, &Transform)>,
    rigidbody_query: Query<(Entity, &Collider, &RigidBody, &Transform)>,
) {
    for (collider_entity, collider, collider_transform) in collider_query.iter() {
        for (rigidbody_entity, rigidbody_collider, rigidbody, rigidbody_transform) in
            rigidbody_query.iter()
        {
            if rigidbody == &RigidBody::Dynamic
                && collider_entity != rigidbody_entity
                && aabb_collides(
                    collider_transform.position,
                    collider.size * collider_transform.scale,
                    rigidbody_transform.position,
                    rigidbody_collider.size * rigidbody_transform.scale,
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

    transform.position += velocity.linear * player.speed * super::FRAMETIME_60FPS;

    transform.position = transform.position.clamp(
        Vec2::ZERO,
        fb.size.as_vec2() - crate::assets::FLAPPY_BIRD_SIZE,
    );
}
