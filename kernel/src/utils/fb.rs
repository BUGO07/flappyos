/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::ffi::c_void;

use alloc::vec::Vec;
use bevy_ecs::prelude::*;
use bevy_math::{UVec2, Vec2, ops::*};

use crate::utils::asm::memcpy;

#[derive(Debug, Resource)]
pub struct Framebuffer {
    pub backbuffer: Vec<u32>,
    pub addr: *mut u8,
    pub size: UVec2,
    pub pitch: u32,
    pub bpp: u32,
    pub font: &'static [u8],
    pub font_width: u32,
    pub font_height: u32,
    pub font_spacing: u32,
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

impl Framebuffer {
    pub fn new_from_limine(fb: &limine::framebuffer::Framebuffer) -> Self {
        Framebuffer {
            backbuffer: alloc::vec![0; fb.width() as usize * fb.height() as usize],
            addr: fb.addr(),
            size: UVec2::new(fb.width() as u32, fb.height() as u32),
            pitch: fb.pitch() as u32,
            bpp: fb.bpp() as u32,
            font: include_bytes!("../../res/font.bin"),
            font_width: 8,
            font_height: 16,
            font_spacing: 1,
        }
    }

    pub fn draw_pixel(&mut self, pos: UVec2, color: u32) {
        if pos.x >= self.size.x || pos.y >= self.size.y {
            return;
        }

        self.backbuffer[(pos.y * self.size.x + pos.x) as usize] = color;
    }

    pub fn draw_rect(&mut self, pos: Vec2, size: UVec2, color: u32) {
        let start_x = floor(pos.x) as i32;
        let start_y = floor(pos.y) as i32;
        let end_x = start_x + size.x as i32;
        let end_y = start_y + size.y as i32;

        let screen_w = self.size.x as i32;
        let screen_h = self.size.y as i32;

        for y in start_y.max(0)..end_y.min(screen_h) {
            for x in start_x.max(0)..end_x.min(screen_w) {
                self.draw_pixel(UVec2::new(x as u32, y as u32), color);
            }
        }
    }

    pub fn draw_line(&mut self, mut x0: isize, mut y0: isize, x1: isize, y1: isize, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.draw_pixel(UVec2::new(x0 as u32, y0 as u32), color);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_char(
        &mut self,
        pos: UVec2,
        ch: u8,
        fg: u32,
        bg: Option<u32>,
        scale: Vec2,
        shadow: Option<(UVec2, u32)>,
    ) {
        let bytes_per_row = self.font_width.div_ceil(8);
        let char_offset = ch as u32 * self.font_height * bytes_per_row;

        let scaled_width = ceil(self.font_width as f32 * scale.x) as u32;
        let scaled_height = ceil(self.font_height as f32 * scale.y) as u32;

        for sy in 0..scaled_height {
            for sx in 0..scaled_width {
                let font_x = floor(sx as f32 / scale.x) as u32;
                let font_y = floor(sy as f32 / scale.y) as u32;

                if font_x >= self.font_width || font_y >= self.font_height {
                    continue;
                }

                let byte_index = char_offset + font_y * bytes_per_row + (font_x / 8);
                let bit_index = 7 - (font_x % 8);
                let byte = self.font.get(byte_index as usize).copied().unwrap_or(0);
                let is_on = (byte >> bit_index) & 1 != 0;
                let color = if is_on { Some(fg) } else { bg };

                if let Some(color) = color {
                    self.draw_pixel(UVec2::new(pos.x + sx, pos.y + sy), color);
                    if let Some((shadow, shadow_color)) = shadow {
                        if shadow != UVec2::ZERO {
                            self.draw_pixel(
                                UVec2::new(pos.x + sx + shadow.x, pos.y + sy + shadow.y),
                                shadow_color,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn draw_str_with_shadow(
        &mut self,
        mut pos: UVec2,
        s: &str,
        fg: u32,
        bg: Option<u32>,
        scale: Vec2,
        shadow: Option<(UVec2, u32)>,
    ) {
        let scaled_width = ceil(self.font_width as f32 * scale.x) as u32;
        let scaled_height = ceil(self.font_height as f32 * scale.y) as u32;

        let start_x = pos.x;

        for ch in s.bytes() {
            if ch == b'\n' {
                pos.x = start_x;
                pos.y += scaled_height + self.font_spacing;
                continue;
            }
            self.draw_char(pos, ch, fg, bg, scale, shadow);
            pos.x += scaled_width + self.font_spacing;
        }
    }

    pub fn draw_str(&mut self, pos: UVec2, s: &str, fg: u32, bg: Option<u32>, scale: Vec2) {
        self.draw_str_with_shadow(pos, s, fg, bg, scale, None);
    }

    pub fn centered_str_x(&self, s: &str, scale_x: f32) -> u32 {
        let longest_line = s.lines().map(|line| line.len()).max().unwrap_or(0);
        (self.size.x / 2)
            .saturating_sub((longest_line as f32 * self.font_width as f32 * scale_x / 2.0) as u32)
    }

    pub fn centered_str_y(&self, scale_y: f32) -> u32 {
        self.size.y / 2 - (self.font_height as f32 * scale_y / 2.0) as u32
    }

    pub fn draw_sprite_rotated(
        &mut self,
        pos: Vec2,
        size: UVec2,
        data: &[u32],
        transparent: Option<u32>,
        angle_rad: f32,
    ) {
        let pivot = size.as_vec2() / 2.0;
        let sin = sin(angle_rad);
        let cos = cos(angle_rad);

        let screen_w = size.x;
        let screen_h = size.y;

        for sy in 0..screen_h {
            for sx in 0..screen_w {
                // Translate to origin (pivot), rotate, then translate back
                let dx = sx as f32 - pivot.x;
                let dy = sy as f32 - pivot.y;

                let src_x = cos * dx + sin * dy + pivot.x;
                let src_y = -sin * dx + cos * dy + pivot.y;

                let src_ix = floor(src_x) as i32;
                let src_iy = floor(src_y) as i32;

                if src_ix < 0 || src_iy < 0 || src_ix >= size.x as i32 || src_iy >= size.y as i32 {
                    continue;
                }

                let color = data[(src_iy as u32 * size.x + src_ix as u32) as usize];

                if Some(color) == transparent {
                    continue;
                }

                let screen_x = pos.x as i32 + sx as i32;
                let screen_y = pos.y as i32 + sy as i32;

                if screen_x >= 0
                    && screen_y >= 0
                    && screen_x < self.size.x as i32
                    && screen_y < self.size.y as i32
                {
                    self.draw_pixel(UVec2::new(screen_x as u32, screen_y as u32), color);
                }
            }
        }
    }

    pub fn draw_sprite(&mut self, pos: Vec2, size: UVec2, data: &[u32], transparent: Option<u32>) {
        self.draw_sprite_rotated(pos, size, data, transparent, 0.0);
    }

    pub fn clear(&mut self, color: u32) {
        self.backbuffer.fill(color);
    }

    pub fn present(&mut self) {
        for y in 0..self.size.y {
            let src = &self.backbuffer
                [y as usize * (self.size.x as usize)..(y as usize + 1) * self.size.x as usize];
            let dst = unsafe { self.addr.add((y * self.pitch) as usize) as *mut u32 };
            memcpy(
                dst as *mut c_void,
                src.as_ptr() as *const c_void,
                self.size.x as usize * 4,
            );
        }
    }
}
