// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let main_window = AppWindow::new()?;

    main_window.on_celsius_changed({
        let ui_weak = main_window.as_weak();
        move |c| {
            let f = match extract_number_and_parse(&c) {
                Some(cv) => (cv as f64 * 9.0 / 5.0 + 32.0) as i32,
                None => 0,
            };
            let ui = ui_weak.unwrap();
            ui.set_fahrenheit(f);
        }
    });

    main_window.on_fahrenheit_changed({
        let ui_weak = main_window.as_weak();
        move |f: slint::SharedString| {
            let c = match extract_number_and_parse(&f) {
                Some(fv) => ((fv as f64 - 32.0) * 5.0 / 9.0) as i32,
                None => 0,
            };
            let ui = ui_weak.unwrap();
            ui.set_celsius(c);
        }
    });

    main_window.run()?;

    Ok(())
}

fn extract_number_and_parse(s: &str) -> Option<i32> {
    let mut chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    let mut found_digit = false;

    // Handle negative sign at the beginning
    if chars.first() == Some(&'-') {
        result.push('-');
        chars.remove(0);
    }

    // Keep only digits
    for ch in chars {
        if ch.is_ascii_digit() {
            result.push(ch);
            found_digit = true;
        }
    }

    if found_digit && !result.is_empty() {
        result.parse().ok()
    } else {
        None
    }
}
