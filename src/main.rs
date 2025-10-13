// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image::{Rgb, RgbImage};
use slint::{Color, Image, SharedPixelBuffer};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

#[derive(Clone)]
struct CircleConfig {
    radius: f32,
    color: Color,
}

#[derive(Default)]
struct History {
    undo_stack: Vec<RgbImage>,
    redo_stack: Vec<RgbImage>,
}

fn draw_circle(image: &mut RgbImage, x: u32, y: u32, radius: u32, color: Rgb<u8>) {
    let (width, height) = image.dimensions();
    let r_sq = (radius * radius) as i64;
    for dx in -(radius as i64)..=(radius as i64) {
        for dy in -(radius as i64)..=(radius as i64) {
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= r_sq {
                let px = x as i64 + dx;
                let py = y as i64 + dy;
                if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                    image.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

fn to_slint_image(img: &RgbImage) -> Image {
    let (w, h) = img.dimensions();
    let mut buffer = SharedPixelBuffer::<slint::Rgb8Pixel>::new(w, h);
    buffer.make_mut_bytes().copy_from_slice(img.as_raw());
    Image::from_rgb8(buffer)
}

fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    let app = MainWindow::new().unwrap();

    let image = Rc::new(RefCell::new(RgbImage::from_pixel(
        WIDTH,
        HEIGHT,
        Rgb([255, 255, 255]),
    )));

    let history = Rc::new(RefCell::new(History::default()));

    let config = Rc::new(RefCell::new(CircleConfig {
        radius: 30.0,
        color: Color::from_rgb_f32(1.0, 0.0, 0.0),
    }));

    app.set_canvas_image(to_slint_image(&image.borrow()));
    app.set_current_radius(config.borrow().radius);
    app.set_current_color(config.borrow().color);

    // --- Add Circle ---
    {
        let app_weak = app.as_weak();
        let img = image.clone();
        let hist = history.clone();
        let cfg = config.clone();

        app.on_add_circle(move |x, y| {
            let mut img_ref = img.borrow_mut();
            let mut h = hist.borrow_mut();

            h.undo_stack.push(img_ref.clone());
            h.redo_stack.clear();

            let cfg_ref = cfg.borrow();
            let radius = cfg_ref.radius as u32;
            let color_rgba = cfg_ref.color.to_argb_u8();
            draw_circle(
                &mut img_ref,
                x as u32,
                y as u32,
                radius,
                Rgb([color_rgba.red, color_rgba.green, color_rgba.blue]),
            );

            if let Some(app) = app_weak.upgrade() {
                app.set_canvas_image(to_slint_image(&img_ref));
            }
        });
    }

    // --- Undo ---
    {
        let app_weak = app.as_weak();
        let img = image.clone();
        let hist = history.clone();
        app.on_undo(move || {
            let mut h = hist.borrow_mut();
            if let Some(prev) = h.undo_stack.pop() {
                let mut img_ref = img.borrow_mut();
                h.redo_stack.push(img_ref.clone());
                *img_ref = prev;
                if let Some(app) = app_weak.upgrade() {
                    app.set_canvas_image(to_slint_image(&img_ref));
                }
            }
        });
    }

    // --- Redo ---
    {
        let app_weak = app.as_weak();
        let img = image.clone();
        let hist = history.clone();
        app.on_redo(move || {
            let mut h = hist.borrow_mut();
            if let Some(next) = h.redo_stack.pop() {
                let mut img_ref = img.borrow_mut();
                h.undo_stack.push(img_ref.clone());
                *img_ref = next;
                if let Some(app) = app_weak.upgrade() {
                    app.set_canvas_image(to_slint_image(&img_ref));
                }
            }
        });
    }

    // --- Config ---
    {
        let cfg = config.clone();
        app.on_apply_config(move |radius, color| {
            let mut cfg_ref = cfg.borrow_mut();
            cfg_ref.radius = radius;
            cfg_ref.color = color;
        });
    }

    app.run().unwrap();
}
