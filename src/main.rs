// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod entity;

use std::{error::Error, fs::File, io, path::Path};

use sea_orm::{ActiveValue::Set, Database, DatabaseConnection, DbErr};
use slint::{ModelRc, SharedString, StandardListViewItem, VecModel};
use tracing::{debug, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::entity::{
    create_tables, delete_user, insert_default_users, insert_user, update_user,
    user::{ActiveModel, find_user_by_name_surname, get_all_users},
};

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Console Layer
    let console_layer = fmt::layer()
        .with_writer(io::stdout)
        .with_thread_names(true)
        .with_target(true)
        .with_filter(EnvFilter::new("warn"));

    // 2. File Layer
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "./logs", "application.log");
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_appender)
        .with_thread_names(true)
        .with_target(true)
        .with_filter(EnvFilter::new("debug"));

    // Combine layers and initialize the subscriber
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    // let db_url = "sqlite::memory:";
    let db_path_str = "./user.db";
    let db_url = format!("sqlite://{}", db_path_str);

    if !Path::new(db_path_str).exists() {
        File::create(db_path_str)?;
        debug!("database {} not exist and created.", db_path_str);
    }

    let db = Database::connect(db_url).await?;

    if let Ok(()) = create_tables(&db).await {
        insert_default_users(&db).await?;
        debug!("Insert default users successfully.")
    };

    let ui = AppWindow::new()?;
    let filter_prefix = ui.get_filter_prefix();
    debug!("Filter prefix: {}", filter_prefix);

    match filtered_users(&db, Some(&filter_prefix)).await {
        Ok(users) => {
            ui.set_items(ModelRc::new(VecModel::from(users)));
            debug!("Update displayed users successfully.")
        }
        Err(err) => warn!("Filter user failed: `{}`", err),
    }

    ui.on_filter_triggered({
        let ui_weak = ui.as_weak();
        let db_clone = db.clone();

        move |filter_prefix| {
            let ui_weak_clone = ui_weak.clone();
            let db_clone = db_clone.clone();

            tokio::spawn(async move {
                match filtered_users(&db_clone, Some(&filter_prefix)).await {
                    Ok(users) => {
                        ui_weak_clone
                            .upgrade_in_event_loop(|app| {
                                app.set_items(ModelRc::new(VecModel::from(users)));
                            })
                            .unwrap();
                    }
                    Err(err) => warn!("Filter user failed: `{}`", err),
                }
            });
        }
    });

    ui.on_create_clicked({
        let ui_weak = ui.as_weak();
        let db_clone = db.clone();

        move |name, surname, filter_prefix| {
            let ui_weak_clone = ui_weak.clone();
            let db_clone = db_clone.clone();
            let filter_prefix = filter_prefix.clone();

            tokio::spawn(async move {
                if let Err(err) = insert_user(
                    &db_clone,
                    ActiveModel {
                        name: Set(name.to_string()),
                        surname: Set(surname.to_string()),
                        ..Default::default()
                    },
                )
                .await
                {
                    warn!("Insert user failed: `{}`", err)
                }
                match filtered_users(&db_clone, Some(&filter_prefix)).await {
                    Ok(users) => {
                        ui_weak_clone
                            .upgrade_in_event_loop(|app| {
                                app.set_items(ModelRc::new(VecModel::from(users)));
                            })
                            .unwrap();
                    }
                    Err(err) => warn!("Filter user failed: `{}`", err),
                }
            });
        }
    });

    ui.on_delete_clicked({
        let ui_weak = ui.as_weak();
        let db_clone = db.clone();

        move |item, filter_prefix| {
            let db_clone = db_clone.clone();
            let ui_weak_clone = ui_weak.clone();
            let filter_prefix = filter_prefix.clone();

            tokio::spawn(async move {
                if let Some((name, surname)) = item.text.split_once(',')
                    && let Ok(models) = find_user_by_name_surname(&db_clone, name, surname).await
                {
                    for user in models {
                        if let Err(err) = delete_user(&db_clone, user.id).await {
                            warn!("Error: {}", err)
                        }
                    }
                }

                match filtered_users(&db_clone, Some(&filter_prefix)).await {
                    Ok(users) => {
                        ui_weak_clone
                            .upgrade_in_event_loop(|app| {
                                app.set_items(ModelRc::new(VecModel::from(users)));
                            })
                            .unwrap();
                    }
                    Err(err) => warn!("Filter user failed, Error: {}", err),
                }
            });
        }
    });

    ui.on_update_clicked({
        let ui_weak = ui.as_weak();
        let db_clone = db.clone();

        move |item, name, surname, filter_prefix| {
            let db_clone = db_clone.clone();
            let ui_weak_clone = ui_weak.clone();
            let filter_prefix = filter_prefix.clone();

            tokio::spawn(async move {
                if let Some((nm, sur)) = item.text.split_once(',')
                    && let Ok(models) = find_user_by_name_surname(&db_clone, nm, sur).await
                {
                    for user in models {
                        if let Err(err) = update_user(&db_clone, user.id, &name, &surname).await {
                            warn!("Update user failed, Error: {}", err)
                        }
                    }
                }

                match filtered_users(&db_clone, Some(&filter_prefix)).await {
                    Ok(users) => {
                        ui_weak_clone
                            .upgrade_in_event_loop(|app| {
                                app.set_items(ModelRc::new(VecModel::from(users)));
                            })
                            .unwrap();
                    }
                    Err(err) => warn!("Filter user failed, Error: {}", err),
                }
            });
        }
    });

    ui.run()?;

    Ok(())
}

async fn filtered_users(
    db: &DatabaseConnection,
    filter_prefix: Option<&str>,
) -> Result<Vec<StandardListViewItem>, DbErr> {
    debug!("Filter prefix: {:?}", filter_prefix);
    match get_all_users(db).await {
        Ok(all_users) => {
            let users: Vec<StandardListViewItem> = all_users
                .iter()
                .filter(|x| match filter_prefix {
                    Some(prefix) => x.name.starts_with(&prefix.to_string()),
                    None => true,
                })
                .map(|x| {
                    StandardListViewItem::from(SharedString::from(format!(
                        "{},{}",
                        x.name, x.surname
                    )))
                })
                .collect();
            Ok(users)
        }
        Err(err) => Err(err),
    }
}
