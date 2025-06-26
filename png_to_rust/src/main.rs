/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use std::io::{BufWriter, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <image.png> <output.bin>", args[0]);
        return;
    }

    let img_path = &args[1];
    let output_path = &args[2];

    let img = image::ImageReader::open(img_path)
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image")
        .to_rgba8();

    let (width, height) = img.dimensions();

    let file = std::fs::File::create(output_path).expect("Failed to create output file");
    let mut writer = BufWriter::new(file);

    for y in 0..height {
        for x in 0..width {
            let px = img.get_pixel(x, y).0;
            let argb = ((px[3] as u32) << 24)
                | ((px[0] as u32) << 16)
                | ((px[1] as u32) << 8)
                | (px[2] as u32);
            writer
                .write_all(&argb.to_le_bytes())
                .expect("Failed to write pixel data");
        }
    }

    println!("Wrote {}x{} image data to {}", width, height, output_path);
}
