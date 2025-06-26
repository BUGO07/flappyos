/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;
use bevy_math::Vec2;

use crate::{assets::FLAPPY_BIRD_SIZE, game::MenuState, info, utils::fb::Framebuffer};

use super::ecs::*;

pub fn aabb_collides(pos1: Vec2, size1: Vec2, pos2: Vec2, size2: Vec2) -> bool {
    let x_overlap = pos1.x < pos2.x + size2.x && pos1.x + size1.x > pos2.x;
    let y_overlap = pos1.y < pos2.y + size2.y && pos1.y + size1.y > pos2.y;
    x_overlap && y_overlap
}

pub fn collision_check(
    collider_query: Query<(Entity, &Collider, &Transform)>,
    rigidbody_query: Query<(Entity, &Collider, &RigidBody, &Transform)>,
    mut state: ResMut<MenuState>,
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
                *state = MenuState::GameOver;
                info!("Game Over");
            }
        }
    }
}

pub fn physics_update(
    mut state: ResMut<MenuState>,
    query: Query<(&mut Transform, &mut Velocity, &RigidBody)>,
    fb: Res<Framebuffer>,
) {
    for (mut transform, mut velocity, rigidbody) in query {
        if rigidbody == &RigidBody::Dynamic {
            velocity.linear += Vec2::Y * 4.9; // 9.8/2
            velocity.angular = (-velocity.linear.y).min(0.0) * -0.02; // idek
        }

        // * don't going thru the box
        // let test_x = Vec2::new(next_pos.x, transform.position.y);
        // if aabb_collides(test_x, SPRITE_SIZE, box_q.position, box_q.scale) {
        //     next_pos.x = transform.position.x;
        // }

        // let test_y = Vec2::new(next_pos.x, next_pos.y);
        // if aabb_collides(test_y, SPRITE_SIZE, box_q.position, box_q.scale) {
        //     next_pos.y = transform.position.y;
        // }

        transform.position += velocity.linear * super::FRAMETIME_60FPS;

        transform.position.y = transform.position.y.max(0.0);

        transform.rotation = (transform.rotation + velocity.angular * super::FRAMETIME_60FPS)
            .clamp(-100.0_f32.to_radians(), 100.0_f32.to_radians());

        if transform.position.y > fb.size.y as f32 - FLAPPY_BIRD_SIZE.y {
            *state = MenuState::GameOver;
            info!("Game Over");
        }
    }
}
