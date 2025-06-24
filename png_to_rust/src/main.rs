/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <image.png>", args[0]);
        return;
    }

    let img_path = &args[1];
    let img = image::ImageReader::open(img_path)
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image")
        .to_rgba8();

    let (width, height) = img.dimensions();
    println!("pub const SPRITE_WIDTH: usize = {width};");
    println!("pub const SPRITE_HEIGHT: usize = {height};");
    println!("pub const SPRITE_DATA: [u32; {}] = [", width * height);

    for y in 0..height {
        print!("    ");
        for x in 0..width {
            let px = img.get_pixel(x, y).0;
            let argb = ((px[3] as u32) << 24)
                | ((px[0] as u32) << 16)
                | ((px[1] as u32) << 8)
                | (px[2] as u32);
            print!("0x{argb:08X}, ");
        }
        println!();
    }

    println!("];");
}
