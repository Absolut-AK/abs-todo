#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::{DateTime, Datelike, Days, Weekday, prelude::Local};
use eframe::egui::{self, FontDefinitions, ThemePreference};
use std::fs::OpenOptions;
use std::io::Write;
use std::ops::Sub;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{fs, path::PathBuf};
use sysinfo::{ProcessRefreshKind, RefreshKind, Signal, System};

const BOX: &str = "[ ]";
const _COMPLETED_BOX: &str = "[x]";
const WEEKDAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
const APP_FOLDER_NAME: &str = "abs_todo";
const BLACKLISTED_APPS_FILE: &str = "apps.txt";
const DAILIES_FILE: &str = "dailies.txt";
static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
    //only windows for now
    let _this_month_progress = show_month_progress();

    let running_apps = RunningApplications;

    let apps = running_apps.get_app_names_as_checkboxes();

    let native_option = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("abs_todo")
            .with_inner_size([440.0, 620.0])
            .with_min_inner_size([360.0, 420.0]),
        ..Default::default()
    };

    let Ok(mut sys) = System::new() else {
        eprintln!("couldn't read system info");
        return;
    };

    let apps = Arc::new(Mutex::new(apps));
    let apps_for_thread = Arc::clone(&apps);

    //closing apps on repeat
    let handle = thread::spawn(move || {
        while RUNNING.load(Ordering::Acquire) {
            thread::sleep(Duration::from_secs(2));
            running_apps.close_apps(&apps_for_thread, &mut sys);
            //
            // while let Ok(receive) = rx.try_recv() {
            //     apps_for_thread.lock().unwrap().push(receive);
            // }
        }
    });

    //eframe
    let _ = eframe::run_native(
        "abs_todo",
        native_option,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc, apps)))),
    );

    //graceful shutdown
    RUNNING.store(false, Ordering::Release);
    handle.join().unwrap();
}
#[derive(Default, Debug)]
struct NameNBox {
    name: String,
    done: bool,
}
#[derive(Default)]
struct MyEguiApp {
    apps: Arc<Mutex<Vec<NameNBox>>>,
    dailies: Vec<NameNBox>,
    app_name: String,
    daily: String,
    first_run: bool,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>, apps: Arc<Mutex<Vec<NameNBox>>>) -> Self {
        let mut fonts = FontDefinitions::default();

        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap();
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.set_theme(ThemePreference::Dark);
        apply_style(&cc.egui_ctx);

        Self {
            apps,
            first_run: true,
            ..Default::default()
        }
    }
}

impl eframe::App for MyEguiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.first_run {
            self.first_run = false;

            let mut path = get_folder_path();
            path.push(DAILIES_FILE);

            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                _ => {
                    fs::write(&path, "").unwrap();
                    String::new()
                }
            };
            for line in content.lines() {
                let namenbox = NameNBox {
                    name: line.to_string(),
                    done: false,
                };
                self.dailies.push(namenbox);
            }
        }

        if !RUNNING.load(Ordering::Acquire) {
            goodbye_screen(ui);
            return;
        }

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                ui.heading("ABS_TODO");
                ui.label(egui::RichText::new("Block distractions, keep your dailies.").weak());
                ui.add_space(12.0);

                card(ui, "Blocked apps", |ui| {
                    ui.label(
                        egui::RichText::new(
                            "1. Open the application first\n\
                     2. Write the .exe name of the application\n\
                     3. It will close if the file name is correct",
                        )
                        .weak(),
                    );
                    ui.add_space(6.0);
                    //Text box so I can add more .exe files
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.app_name)
                                .hint_text("e.g. steam.exe")
                                .desired_width(ui.available_width() - 110.0),
                        );

                        if ui.button("Add app").clicked() {
                            let mut path = get_folder_path();
                            path.push(BLACKLISTED_APPS_FILE);
                            let mut file = OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(path)
                                .unwrap();

                            let namenbox = NameNBox {
                                name: self.app_name.trim().to_string().clone(),
                                done: false,
                            };

                            if !self.app_name.trim().is_empty() {
                                //self.tx.clone().unwrap().send(namenbox).unwrap();

                                let mut apps = self.apps.lock().unwrap();
                                apps.push(namenbox);
                                writeln!(file, "{}", &self.app_name.trim()).unwrap();
                                self.app_name = String::from("");
                            }
                        }
                    });

                    ui.add_space(6.0);
                    let mut apps = self.apps.lock().unwrap();
                    if apps.is_empty() {
                        ui.label(egui::RichText::new("No blocked apps yet.").weak().italics());
                    }
                    let mut removed_app_name = String::from("");
                    for app in apps.iter() {
                        if app_row(ui, &app.name) {
                            removed_app_name = app.name.clone();
                            let mut path = get_folder_path();
                            path.push(BLACKLISTED_APPS_FILE);

                            let content = fs::read_to_string(&path).unwrap();

                            let mut kept = String::new();
                            for line in content.lines() {
                                if line.trim() != app.name {
                                    kept.push_str(line);
                                    kept.push('\n');
                                }
                            }
                            fs::write(path, kept).unwrap();
                        }
                    }
                    apps.retain(|apps| apps.name != removed_app_name);
                });

                ui.add_space(10.0);

                card(ui, "Dailies", |ui| {
                    //Text box for adding daily and would create another line of Text box + checkBox
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.daily)
                                .hint_text("New daily…")
                                .desired_width(ui.available_width() - 110.0),
                        );

                        if ui.button("Add daily").clicked() {
                            let daily = NameNBox {
                                name: std::mem::take(&mut self.daily),
                                done: false,
                            };

                            let mut path = get_folder_path();
                            path.push(DAILIES_FILE);

                            let mut file = OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(path)
                                .unwrap();

                            writeln!(file, "{}", &daily.name).unwrap();

                            self.dailies.push(daily);
                        }
                    });

                    ui.add_space(6.0);
                    if self.dailies.is_empty() {
                        ui.label(egui::RichText::new("No dailies yet.").weak().italics());
                    }

                    let mut remove_daily = String::from("");
                    for daily in &mut self.dailies {
                        let text = if daily.done {
                            egui::RichText::new(&daily.name).strikethrough().weak()
                        } else {
                            egui::RichText::new(&daily.name)
                        };
                        ui.checkbox(&mut daily.done, text);
                        if ui.button("Remove").clicked() {
                            remove_daily = daily.name.clone();
                        };
                    }

                    self.dailies.retain(|d| d.name != remove_daily);
                });

                ui.add_space(10.0);

                card(ui, "Progress", |ui| {
                    let mut is_done = true;
                    for daily in &self.dailies {
                        if !daily.done {
                            is_done = false;
                        }
                    }

                    ui.label(
                        egui::RichText::new("Check off every daily to unblock your apps.").weak(),
                    );
                    ui.add_space(4.0);

                    if ui
                        .add_enabled(
                            is_done,
                            egui::Button::new("Finished all Dailies, Unblock everything")
                                .min_size(egui::vec2(ui.available_width(), 34.0)),
                        )
                        .on_disabled_hover_text("Finish every daily first")
                        .clicked()
                    {
                        RUNNING.store(false, Ordering::Release);
                    };
                });
            });
        });
    }
}

// ---- UI styling (written by Claude) ----

// Dark grayscale palette: no pure white, so it's easy on the eyes in a dark room.
const BG: egui::Color32 = egui::Color32::from_gray(15);
const CARD: egui::Color32 = egui::Color32::from_gray(26);
const CARD_STROKE: egui::Color32 = egui::Color32::from_gray(38);
const ROW: egui::Color32 = egui::Color32::from_gray(34);
const TEXT: egui::Color32 = egui::Color32::from_gray(205);
const TEXT_STRONG: egui::Color32 = egui::Color32::from_gray(232);
const TEXT_WEAK: egui::Color32 = egui::Color32::from_gray(125);

/// One blocked app shown as a small rounded row, with a remove button on the right.
/// Returns true when the remove button was clicked.
fn app_row(ui: &mut egui::Ui, name: &str) -> bool {
    let mut remove_clicked = false;
    egui::Frame::new()
        .fill(ROW)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(name).monospace());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    remove_clicked = ui
                        .small_button(egui::RichText::new("X").color(TEXT_WEAK))
                        .on_hover_text("Remove")
                        .clicked();
                });
            });
        });
    remove_clicked
}

/// Full-window screen shown after all dailies are done and apps are unblocked.
fn goodbye_screen(ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            // push the text down so it sits roughly in the middle of the window
            ui.add_space((ui.available_height() - 110.0).max(0.0) / 2.0);
            ui.label(egui::RichText::new("✔").size(36.0).color(TEXT_WEAK));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Good bye")
                    .size(32.0)
                    .strong()
                    .color(TEXT_STRONG),
            );
            ui.label(egui::RichText::new("See you tomorrow.").size(17.0).weak());
        });
    });
}

/// Draws a dark rounded "card" with a title, and puts `add_contents` inside it.
fn card(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, CARD_STROKE))
        .corner_radius(10.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(title).strong().size(17.0));
            ui.add_space(6.0);
            add_contents(ui);
        });
}

/// Font sizes, spacing, colors and rounded corners for the whole app.
fn apply_style(ctx: &egui::Context) {
    use egui::{Color32, FontId, Stroke, TextStyle};

    ctx.all_styles_mut(|style| {
        style.visuals = egui::Visuals::dark();

        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(26.0));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.0));

        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = 28.0;

        let v = &mut style.visuals;
        v.panel_fill = BG;
        v.window_fill = BG;
        v.extreme_bg_color = Color32::from_gray(12);
        v.text_edit_bg_color = Some(Color32::from_gray(20));
        v.faint_bg_color = Color32::from_gray(22);
        v.weak_text_color = Some(TEXT_WEAK);
        v.hyperlink_color = Color32::from_gray(180);
        v.selection.bg_fill = Color32::from_gray(70);
        v.selection.stroke = Stroke::new(1.0, Color32::from_gray(200));
        v.text_cursor.stroke = Stroke::new(2.0, TEXT);

        let w = &mut v.widgets;
        // labels and other non-clickable things
        w.noninteractive.bg_fill = CARD;
        w.noninteractive.bg_stroke = Stroke::new(1.0, CARD_STROKE);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        // buttons / checkboxes: at rest, hovered, pressed
        w.inactive.weak_bg_fill = Color32::from_gray(40);
        w.inactive.bg_fill = Color32::from_gray(44);
        w.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(50));
        w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        w.hovered.weak_bg_fill = Color32::from_gray(52);
        w.hovered.bg_fill = Color32::from_gray(56);
        w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_gray(90));
        w.hovered.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
        w.active.weak_bg_fill = Color32::from_gray(64);
        w.active.bg_fill = Color32::from_gray(66);
        w.active.bg_stroke = Stroke::new(1.0, Color32::from_gray(110));
        w.active.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
        w.open = w.active;

        for w in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            w.corner_radius = 6.into();
        }
    });
}

struct RunningApplications;

impl RunningApplications {
    fn get_app_names_as_checkboxes(&self) -> Vec<NameNBox> {
        let mut apps: Vec<NameNBox> = Vec::new();
        let mut path = get_folder_path();
        path.push(BLACKLISTED_APPS_FILE);

        if let Ok(lines) = fs::read_to_string(&path) {
            for line in lines.lines() {
                let checkbox = NameNBox {
                    name: line.to_string(),
                    done: false,
                };
                apps.push(checkbox);
            }
        } else {
            fs::write(&path, "").unwrap();
        }

        apps
    }

    fn close_apps(&self, app_names: &Arc<Mutex<Vec<NameNBox>>>, sys: &mut System) {
        sys.refresh_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()));

        for app in app_names.lock().unwrap().iter() {
            for process in sys.processes_by_name(app.name.as_ref()) {
                println!(
                    "come on man... why you are trying to procastinate?: going to close it now {:?}",
                    process.name()
                );
                if process.kill_with(Signal::Kill).is_none() {
                    println!("signal not supported");
                }
            }
        }
    }
}
fn show_month_progress() -> Vec<String> {
    let local_time: DateTime<Local> = Local::now();

    let current_day = local_time.day() as u8;
    let mut rest_of_the_day = local_time.num_days_in_month() - &current_day;

    let day = local_time.day().sub(1).into();
    let day = Days::new(day);
    let day = local_time.sub(day);
    let starting_weekday_of_month = day.weekday();

    let mut weekday_index = match starting_weekday_of_month {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    };

    let mut weekdays = 0;
    while let Some(_) = rest_of_the_day.checked_sub(7) {
        rest_of_the_day -= 7;
        weekdays += 1;
    }

    let boxes_weekdays = String::from("");
    let mut v_weekdays = vec![
        boxes_weekdays.clone(),
        boxes_weekdays.clone(),
        boxes_weekdays.clone(),
        boxes_weekdays.clone(),
        boxes_weekdays.clone(),
        boxes_weekdays.clone(),
        boxes_weekdays.clone(),
    ];

    //later i want [?] for if it's hits today [X] for completed day and [ ] for missed days.
    for _ in 0..weekdays {
        for i in 0..WEEKDAYS.len() {
            if v_weekdays[i] == "" {
                v_weekdays[i] += WEEKDAYS[weekday_index];
                if (weekday_index + 1) == WEEKDAYS.len() {
                    weekday_index = 0;
                } else {
                    weekday_index += 1;
                }
            }
            v_weekdays[i] += BOX;
        }
    }

    for i in 0..rest_of_the_day as usize {
        v_weekdays[i] += BOX;
    }

    v_weekdays
}

fn get_folder_path() -> PathBuf {
    let Ok(appdata_path) = std::env::var("APPDATA") else {
        eprintln!("APPDATA is not found");
        std::process::exit(1);
    };

    let mut path = PathBuf::from(appdata_path);
    path.push(APP_FOLDER_NAME);

    if fs::read_dir(&path).is_err() {
        fs::create_dir(&path).unwrap();
    }

    path
}
