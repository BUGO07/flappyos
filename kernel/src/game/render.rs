/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;

use crate::utils::fb::Framebuffer;

use super::ecs::*;

pub fn render_fixed_update(
    mut fb: ResMut<Framebuffer>,
    sprites: Query<(&Sprite, &Transform)>,
    rects: Query<(&Rect, &Transform)>,
    texts: Query<(&Text, &Transform)>,
) {
    fb.clear(0x000000);

    for (rect, transform) in &rects {
        fb.draw_rect(
            transform.position.as_uvec2(),
            (rect.size * transform.scale).as_uvec2(),
            rect.color,
        );
    }

    for (sprite, transform) in &sprites {
        fb.draw_sprite(
            transform.position.as_uvec2(),
            (sprite.size * transform.scale).as_uvec2(),
            sprite.data,
            Some(0),
        );
    }

    for (text, transform) in &texts {
        fb.draw_str_with_shadow(
            transform.position.as_uvec2(),
            &text.text,
            text.fg,
            text.bg,
            transform.scale,
            text.shadow,
        );
    }

    fb.present();
}
