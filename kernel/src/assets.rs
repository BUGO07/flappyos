/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_math::Vec2;

lazy_static::lazy_static! {
    pub static ref FLAPPY_BIRD_DATA: &'static [u32] =
        U32Aligned(include_bytes!("../res/flappy_bird.bin")).as_u32_slice();
    pub static ref PIPE_DATA: &'static [u32] =
        U32Aligned(include_bytes!("../res/pipe.bin")).as_u32_slice();
    pub static ref PIPE_FLIPPED_DATA: &'static [u32] =
        U32Aligned(include_bytes!("../res/pipe_flipped.bin")).as_u32_slice();
}

pub static FLAPPY_BIRD_SIZE: Vec2 = Vec2::new(57.0, 36.0);
pub static PIPE_SIZE: Vec2 = Vec2::new(22.0, 160.0);

#[repr(C, align(4))]
struct U32Aligned(&'static [u8]);

impl U32Aligned {
    fn as_u32_slice(&self) -> &[u32] {
        bytemuck::cast_slice(self.0)
    }
}
