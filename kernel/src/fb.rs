/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::ffi::c_void;

use alloc::vec::Vec;

use crate::asm::memcpy;

#[derive(Debug)]
pub struct Framebuffer {
    pub backbuffer: Vec<u32>,
    pub addr: *mut u8,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bpp: usize,
    pub font: &'static [u8],
    pub font_width: usize,
    pub font_height: usize,
    pub font_spacing: usize,
}

impl Framebuffer {
    pub fn new_from_limine(fb: &limine::framebuffer::Framebuffer) -> Self {
        Framebuffer {
            backbuffer: alloc::vec![0; fb.width() as usize * fb.height() as usize],
            addr: fb.addr(),
            width: fb.width() as usize,
            height: fb.height() as usize,
            pitch: fb.pitch() as usize,
            bpp: fb.bpp() as usize,
            font: include_bytes!("../res/font.bin"),
            font_width: 8,
            font_height: 16,
            font_spacing: 1,
        }
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }

        self.backbuffer[y * self.width + x] = color;
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for y in y..y + h {
            for x in x..x + w {
                self.draw_pixel(x, y, color);
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
                self.draw_pixel(x0 as usize, y0 as usize, color);
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
        x: usize,
        y: usize,
        ch: u8,
        fg: u32,
        bg: Option<u32>,
        scale_x: f32,
        scale_y: f32,
    ) {
        let bytes_per_row = self.font_width.div_ceil(8);
        let char_offset = ch as usize * self.font_height * bytes_per_row;

        let scaled_width = libm::ceilf(self.font_width as f32 * scale_x) as usize;
        let scaled_height = libm::ceilf(self.font_height as f32 * scale_y) as usize;

        for sy in 0..scaled_height {
            for sx in 0..scaled_width {
                let font_x = libm::floorf(sx as f32 / scale_x) as usize;
                let font_y = libm::floorf(sy as f32 / scale_y) as usize;

                if font_x >= self.font_width || font_y >= self.font_height {
                    continue;
                }

                let byte_index = char_offset + font_y * bytes_per_row + (font_x / 8);
                let bit_index = 7 - (font_x % 8);
                let byte = self.font.get(byte_index).copied().unwrap_or(0);
                let is_on = (byte >> bit_index) & 1 != 0;
                let color = if is_on { Some(fg) } else { bg };

                if let Some(color) = color {
                    self.draw_pixel(x + sx, y + sy, color);
                }
            }
        }
    }

    pub fn draw_str(
        &mut self,
        mut x: usize,
        mut y: usize,
        s: &str,
        fg: u32,
        bg: Option<u32>,
        scale_x: f32,
        scale_y: f32,
    ) {
        let scaled_width = libm::ceilf(self.font_width as f32 * scale_x) as usize;
        let scaled_height = libm::ceilf(self.font_height as f32 * scale_y) as usize;

        for ch in s.bytes() {
            if ch == b'\n' {
                x = 0;
                y += scaled_height + self.font_spacing;
                continue;
            }
            self.draw_char(x, y, ch, fg, bg, scale_x, scale_y);
            x += scaled_width + self.font_spacing;
        }
    }

    pub fn centered_str_x(&self, s: &str, scale_x: f32) -> usize {
        self.width / 2 - (s.len() as f32 * self.font_width as f32 * scale_x / 2.0) as usize
    }

    pub fn centered_str_y(&self, scale_y: f32) -> usize {
        self.height / 2 - (self.font_height as f32 * scale_y / 2.0) as usize
    }

    pub fn draw_sprite(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        data: &[u32],
        transparent: Option<u32>,
    ) {
        for sy in 0..height {
            for sx in 0..width {
                let screen_x = x + sx;
                let screen_y = y + sy;

                if screen_x >= self.width || screen_y >= self.height {
                    continue;
                }

                let color = data[sy * width + sx];

                if Some(color) == transparent {
                    continue;
                }

                self.draw_pixel(screen_x, screen_y, color);
            }
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.backbuffer.fill(color);
    }

    pub fn present(&mut self) {
        for y in 0..self.height {
            let src = &self.backbuffer[y * self.width..(y + 1) * self.width];
            let dst = unsafe { self.addr.add(y * self.pitch) as *mut u32 };
            memcpy(
                dst as *mut c_void,
                src.as_ptr() as *const c_void,
                self.width * 4,
            );
        }
    }
}
