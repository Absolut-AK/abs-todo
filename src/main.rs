#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::{Datelike, prelude::Local};
use eframe::egui::{self, FontDefinitions, ThemePreference};
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Sub;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{fs, path::PathBuf};
use sysinfo::{ProcessRefreshKind, RefreshKind, Signal, System};

const APP_FOLDER_NAME: &str = "abs_todo";
const BLACKLISTED_APPS_FILE: &str = "apps.txt";
const DAILIES_FILE: &str = "dailies.txt";
const DAILY_STREAK_FILE: &str = "daily_streak.txt";
static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
    //only windows for now
    let Ok(mut sys) = System::new() else {
        eprintln!("couldn't read system info");
        return;
    };

    let running_apps = RunningApplications;

    let apps = running_apps.get_app_names_as_checkboxes();
    let native_option = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("abs_todo")
            .with_inner_size([440.0, 620.0])
            .with_min_inner_size([360.0, 420.0]),
        ..Default::default()
    };
    let apps = Arc::new(Mutex::new(apps));
    let apps_for_thread = Arc::clone(&apps);

    //closing apps on repeat
    let handle = thread::spawn(move || {
        while RUNNING.load(Ordering::Acquire) {
            thread::sleep(Duration::from_secs(2));
            running_apps.close_apps(&apps_for_thread, &mut sys);
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

#[derive(Default)]
struct NameNBox {
    name: String,
    done: bool,
}
#[derive(Default)]
struct MyEguiApp {
    apps: Arc<Mutex<Vec<NameNBox>>>,
    dailies: Vec<NameNBox>,
    daily_streaks: Vec<NameNBox>,
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

            //checks dailies
            let content = get_dailies_content();

            for line in content.lines() {
                let namenbox = NameNBox {
                    name: line.to_string(),
                    done: false,
                };
                self.dailies.push(namenbox);
            }

            check_year();

            create_missing_year_streak();

            let content = get_daily_streak_content().unwrap();
            let lines = content.lines();

            for line in lines {
                let (name, flag) = line.split_once('\t').unwrap();
                let flag = match flag {
                    "1" => true,
                    "0" => false,
                    _ => panic!("bad flag: {}", flag),
                };
                let namenbox = NameNBox {
                    name: name.to_string(),
                    done: flag,
                };
                self.daily_streaks.push(namenbox);
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

                            if daily.name.trim() != "" {
                                writeln!(file, "{}", &daily.name).unwrap();
                                self.dailies.push(daily);
                            }
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
                    if !remove_daily.trim().is_empty() {
                        let mut context = String::from("");
                        let mut path = get_folder_path();
                        path.push(DAILIES_FILE);

                        self.dailies.retain(|d| {
                            if d.name != remove_daily {
                                context.push_str(d.name.trim());
                                context.push('\n');
                            }
                            d.name != remove_daily
                        });
                        fs::write(path, context).unwrap();
                    }
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
                        let mut path = get_folder_path();
                        path.push(DAILY_STREAK_FILE);

                        let mut file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(&path)
                            .unwrap();

                        let mut day = get_todays_date();

                        file.seek(SeekFrom::End(-11)).unwrap();
                        let mut buf = [0u8; 10];

                        file.read_exact(&mut buf).unwrap();
                        let buf_day = &buf[0..8];
                        let buf_day = std::str::from_utf8(&buf_day).unwrap().to_string();

                        if day == buf_day {
                            //every line is 10byte 2 2 4 1 1
                            file.seek(SeekFrom::End(-2)).unwrap();
                            file.write_all(b"1").unwrap();
                        } else {
                            file.seek(SeekFrom::End(0)).unwrap();
                            let mut new_day = String::from("");
                            day.push_str("\t1");
                            new_day += &day;
                            file.write_all(new_day.as_bytes()).unwrap();
                            file.write_all(b"\n").unwrap();
                        }
                        RUNNING.store(false, Ordering::Release);
                    };

                    ui.add_space(10.0);
                    streak_grid(ui, &self.daily_streaks);
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
const DONE_SQUARE: egui::Color32 = egui::Color32::from_gray(175);

/// Year tracker: one small square per day, light when that day's dailies were finished.
/// Drawn as a single widget (one allocation, squares painted by hand) so 300+ days stay cheap.
/// Hover a square to see its date.
fn streak_grid(ui: &mut egui::Ui, days: &[NameNBox]) {
    const SIZE: f32 = 11.0;
    const GAP: f32 = 3.0;
    const STEP: f32 = SIZE + GAP;

    let done = days.iter().filter(|d| d.done).count();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(done.to_string())
                .strong()
                .color(TEXT_STRONG),
        );
        ui.label(egui::RichText::new(format!("of {} days completed", days.len())).weak());
    });
    ui.add_space(2.0);

    if days.is_empty() {
        return;
    }

    // as many squares per row as fit in the card
    let columns = (((ui.available_width() + GAP) / STEP).floor() as usize).max(1);
    let rows = days.len().div_ceil(columns);
    let size = egui::vec2(
        columns.min(days.len()) as f32 * STEP - GAP,
        rows as f32 * STEP - GAP,
    );
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    let cell = |i: usize| {
        let min = rect.min + egui::vec2((i % columns) as f32 * STEP, (i / columns) as f32 * STEP);
        egui::Rect::from_min_size(min, egui::vec2(SIZE, SIZE))
    };

    let painter = ui.painter();
    for (i, day) in days.iter().enumerate() {
        let fill = if day.done { DONE_SQUARE } else { ROW };
        painter.rect_filled(cell(i), 2.0, fill);
    }

    // work out which square the mouse is over from its position
    if let Some(pos) = response.hover_pos() {
        let offset = pos - rect.min;
        let i = (offset.y / STEP) as usize * columns + (offset.x / STEP) as usize;
        if let Some(day) = days.get(i) {
            let status = if day.done { "done" } else { "missed" };
            response.on_hover_text(format!("{} · {status}", pretty_date(&day.name)));
        }
    }
}

/// "11112026" -> "2026/11/11" for the hover text; anything else is shown as-is.
fn pretty_date(date: &str) -> String {
    match (date.get(0..2), date.get(2..4), date.get(4..8)) {
        (Some(d), Some(m), Some(y)) if date.len() == 8 => format!("{y}/{m}/{d}"),
        _ => date.to_string(),
    }
}

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

fn get_todays_date() -> String {
    let local_time = Local::now();
    let day = local_time.day();
    let month = local_time.month();
    let year = local_time.year();

    format!("{:02}{:02}{:04}", &day, &month, &year)
}

fn create_missing_year_streak() {
    let mut path = get_folder_path();
    path.push(DAILY_STREAK_FILE);

    let file = OpenOptions::new().write(true).read(true).open(&path);

    let mut starting_day = 1;
    let mut starting_month = 1;

    let mut fresh_start = true;

    if let Ok(mut file) = file {
        fresh_start = false;
        file.seek(SeekFrom::End(-11)).unwrap();
        let mut buf = [0u8; 8];

        file.read_exact(&mut buf).unwrap();
        starting_day = std::str::from_utf8(&buf[0..2]).unwrap().parse().unwrap();
        starting_month = std::str::from_utf8(&buf[2..4]).unwrap().parse().unwrap();
    }

    let local_time = Local::now();
    let mut lines: Vec<String> = Vec::new();
    if starting_day == local_time.day() && starting_month == local_time.month() && !fresh_start {
        return;
    }
    let mut day = local_time.day();

    let mut month = local_time.month();
    let year = local_time.year();

    while starting_month != month || starting_day != day {
        lines.push(format!("{:02}{:02}{:04}\t0", day, month, year));
        day = day.sub(1);

        if day == 0 {
            month = month.sub(1);
            day = local_time
                .with_day(1)
                .unwrap()
                .with_month(month)
                .unwrap()
                .num_days_in_month() as u32;
        }
    }

    if fresh_start {
        lines.push(format!("{:02}{:02}{:04}\t0", 01, 01, year));
    }
    lines.reverse();
    let content = lines.join("\n");

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .unwrap();

    file.write_all(content.as_bytes()).unwrap();
    file.write_all(b"\n").unwrap();
}

fn get_daily_streak_content() -> io::Result<String> {
    let mut path = get_folder_path();
    path.push(DAILY_STREAK_FILE);

    let content = fs::read_to_string(&path)?;
    Ok(content)
}

fn get_dailies_content() -> String {
    let mut path = get_folder_path();
    path.push(DAILIES_FILE);

    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        _ => {
            fs::write(&path, "").unwrap();
            String::new()
        }
    };
    content
}

fn check_year() {
    let content = match get_daily_streak_content() {
        Ok(content) => content,
        Err(_) => return,
    };

    let lines = content.lines();

    let local_time = Local::now();
    let last_line = lines.last();
    let last_line = &last_line.unwrap()[4..8];
    let year: i32 = last_line.parse().unwrap();

    if year < local_time.year() {
        let mut path = get_folder_path();
        path.push(DAILY_STREAK_FILE);
        let mut new_path = get_folder_path();
        let name_new_path = format!("{}_{}", local_time.year().sub(1), DAILY_STREAK_FILE);
        new_path.push(name_new_path);
        fs::rename(&path, new_path).unwrap();
    }
}
