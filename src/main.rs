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

    let base_image = Rc::new(RefCell::new(RgbImage::from_pixel(
        WIDTH,
        HEIGHT,
        Rgb([255, 255, 255]),
    )));
    let preview_image = Rc::new(RefCell::new(base_image.borrow().clone()));
    let history = Rc::new(RefCell::new(History::default()));

    let config = Rc::new(RefCell::new(CircleConfig {
        radius: 30.0,
        color: Color::from_rgb_f32(1.0, 0.0, 0.0),
    }));

    let is_dragging = Rc::new(RefCell::new(false));
    let start_x = Rc::new(RefCell::new(0.0f32));
    let start_y = Rc::new(RefCell::new(0.0f32));

    app.set_canvas_image(to_slint_image(&base_image.borrow()));
    app.set_current_radius(config.borrow().radius);
    app.set_current_color(config.borrow().color);

    // --- Start Drag ---
    {
        let app_weak = app.as_weak();
        let img = preview_image.clone();
        let base = base_image.clone();
        let dragging = is_dragging.clone();
        let sx = start_x.clone();
        let sy = start_y.clone();

        app.on_start_drag(move |x, y| {
            *sx.borrow_mut() = x;
            *sy.borrow_mut() = y;
            *dragging.borrow_mut() = true;

            *img.borrow_mut() = base.borrow().clone();

            if let Some(app) = app_weak.upgrade() {
                app.set_canvas_image(to_slint_image(&img.borrow()));
            }
        });
    }

    // --- Update Drag ---
    {
        let app_weak = app.as_weak();
        let img = preview_image.clone();
        let base = base_image.clone();
        let cfg = config.clone();
        let dragging = is_dragging.clone();
        let sx = start_x.clone();
        let sy = start_y.clone();

        app.on_update_drag(move |x, y| {
            if !*dragging.borrow() {
                return;
            }
            let mut temp = base.borrow().clone();

            let dx = x - *sx.borrow();
            let dy = y - *sy.borrow();
            let radius = (dx * dx + dy * dy).sqrt() as u32;

            let color_rgba = cfg.borrow().color.to_argb_u8();
            draw_circle(
                &mut temp,
                *sx.borrow() as u32,
                *sy.borrow() as u32,
                radius,
                Rgb([color_rgba.red, color_rgba.green, color_rgba.blue]),
            );

            *img.borrow_mut() = temp;

            if let Some(app) = app_weak.upgrade() {
                app.set_canvas_image(to_slint_image(&img.borrow()));
            }
        });
    }

    // --- End Drag ---
    {
        let app_weak = app.as_weak();
        let img = preview_image.clone();
        let base = base_image.clone();
        let cfg = config.clone();
        let hist = history.clone();
        let dragging = is_dragging.clone();
        let sx = start_x.clone();
        let sy = start_y.clone();

        app.on_end_drag(move |x, y| {
            if !*dragging.borrow() {
                return;
            }
            *dragging.borrow_mut() = false;

            let mut base_ref = base.borrow_mut();
            let mut h = hist.borrow_mut();
            h.undo_stack.push(base_ref.clone());
            h.redo_stack.clear();

            let dx = x - *sx.borrow();
            let dy = y - *sy.borrow();
            let radius = (dx * dx + dy * dy).sqrt() as u32;

            let color_rgba = cfg.borrow().color.to_argb_u8();
            draw_circle(
                &mut base_ref,
                *sx.borrow() as u32,
                *sy.borrow() as u32,
                radius,
                Rgb([color_rgba.red, color_rgba.green, color_rgba.blue]),
            );

            *img.borrow_mut() = base_ref.clone();

            if let Some(app) = app_weak.upgrade() {
                app.set_canvas_image(to_slint_image(&img.borrow()));
            }
        });
    }

    // --- Undo ---
    {
        let app_weak = app.as_weak();
        let base = base_image.clone();
        let preview = preview_image.clone();
        let hist = history.clone();

        app.on_undo(move || {
            let mut h = hist.borrow_mut();
            if let Some(prev) = h.undo_stack.pop() {
                let mut base_ref = base.borrow_mut();
                h.redo_stack.push(base_ref.clone());
                *base_ref = prev.clone();
                *preview.borrow_mut() = prev;

                if let Some(app) = app_weak.upgrade() {
                    app.set_canvas_image(to_slint_image(&base_ref));
                }
            }
        });
    }

    // --- Redo ---
    {
        let app_weak = app.as_weak();
        let base = base_image.clone();
        let preview = preview_image.clone();
        let hist = history.clone();

        app.on_redo(move || {
            let mut h = hist.borrow_mut();
            if let Some(next) = h.redo_stack.pop() {
                let mut base_ref = base.borrow_mut();
                h.undo_stack.push(base_ref.clone());
                *base_ref = next.clone();
                *preview.borrow_mut() = next;

                if let Some(app) = app_weak.upgrade() {
                    app.set_canvas_image(to_slint_image(&base_ref));
                }
            }
        });
    }

    // --- Config ---
    {
        let cfg = config.clone();
        app.on_apply_config(move |color| {
            let mut cfg_ref = cfg.borrow_mut();
            cfg_ref.color = color;
        });
    }

    app.run().unwrap();
}
