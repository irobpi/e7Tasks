// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

use chrono::NaiveDate;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let main_window = AppWindow::new()?;

    main_window.on_start_changed({
        let ui_weak = main_window.as_weak();
        move |st| {
            let ui = ui_weak.unwrap();
            match dbg!(parse_date(&st)) {
                Ok(_) => {
                    ui.set_start_eligible(true);
                }
                Err(_) => {
                    ui.set_start_eligible(false);
                }
            }
        }
    });

    main_window.on_end_changed({
        let ui_weak = main_window.as_weak();
        move |ed| {
            let ui = ui_weak.unwrap();
            match dbg!(parse_date(&ed)) {
                Ok(_) => {
                    ui.set_end_eligible(true);
                }
                Err(_) => {
                    ui.set_end_eligible(false);
                }
            }
        }
    });

    main_window.on_check_flight_eligible({
        let ui_weak = main_window.as_weak();
        move || {
            let ui = ui_weak.unwrap();
            if ui.get_is_one_way() {
                ui.set_flight_eligible(ui.get_start_eligible());
            } else {
                if ui.get_start_eligible() && ui.get_end_eligible() {
                    if let (Ok(st), Ok(ed)) =
                        (parse_date(&ui.get_start()), parse_date(&ui.get_end()))
                    {
                        ui.set_flight_eligible(st <= ed);
                    } else {
                        ui.set_flight_eligible(false);
                    }
                } else {
                    ui.set_flight_eligible(false);
                }
            }
        }
    });

    main_window.on_book_flight({
        let ui_weak = main_window.as_weak();
        move || {
            let ui = ui_weak.unwrap();

            if ui.get_is_one_way() {
                if ui.get_start_eligible() {
                    ui.set_flight_info(
                        format!("The flight information: {}", ui.get_start()).into(),
                    );
                    ui.set_flight_eligible(true);
                } else {
                    ui.set_flight_eligible(false);
                }
            } else {
                if ui.get_start_eligible() && ui.get_end_eligible() {
                    ui.set_flight_info(
                        format!(
                            "The flight information: {} -> {}",
                            ui.get_start(),
                            ui.get_end()
                        )
                        .into(),
                    );
                    ui.set_flight_eligible(true);
                } else {
                    ui.set_flight_eligible(false);
                }
            }
        }
    });

    main_window.run()?;

    Ok(())
}

fn parse_date(date_str: &str) -> Result<i64, ()> {
    let parts: Vec<&str> = date_str.split('.').collect();
    if parts.len() != 3 {
        return Err(());
    }

    let day = parts[0].parse::<u32>().map_err(|_| ())?;
    let month = parts[1].parse::<u32>().map_err(|_| ())?;
    let year = parts[2].parse::<i32>().map_err(|_| ())?;

    match NaiveDate::from_ymd_opt(year, month, day) {
        Some(date) => {
            let ts = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
            if ts > 0 { Ok(ts) } else { Err(()) }
        }
        None => Err(()),
    }
}
