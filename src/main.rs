use chrono::{DateTime, Datelike, Days, Weekday, prelude::Local};
use eframe::egui::{self, FontDefinitions, ThemePreference};
use std::fs::OpenOptions;
use std::io::Write;
use std::ops::Sub;
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::Duration;
use std::{fs, path::PathBuf};
use sysinfo::{ProcessRefreshKind, RefreshKind, Signal, System};

const BOX: &str = "[ ]";
const _COMPLETED_BOX: &str = "[x]";
const WEEKDAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
const APP_FOLDER_NAME: &str = "abs_todo";
const BLACKLISTED_APPS_FILE: &str = "apps.txt";

fn main() {
    //only windows for now
    let _this_month_progress = show_month_progress();

    let running_apps = RunningApplications;

    let mut apps = running_apps.get_app_names();

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

    let (tx, rx) = channel();

    //closing apps on repeat
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(2));
            running_apps.close_apps(&apps, &mut sys);

            while let Ok(receive) = rx.try_recv() {
                apps.push(receive);
            }
        }
    });

    //eframe
    let _ = eframe::run_native(
        "abs_todo",
        native_option,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc, tx)))),
    );
}
#[derive(Default)]
struct Daily {
    name: String,
    done: bool,
}
#[derive(Default)]
struct MyEguiApp {
    tx: Option<Sender<String>>,
    app_name: String,
    daily: String,
    dailies: Vec<Daily>,
    n: i32,
    done: bool,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>, tx: Sender<String>) -> Self {
        let mut fonts = FontDefinitions::default();

        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap();
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.set_theme(ThemePreference::Light);
        apply_style(&cc.egui_ctx);

        Self {
            tx: Some(tx),
            ..Default::default()
        }
    }
}

impl eframe::App for MyEguiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
                                .desired_width(ui.available_width() - 90.0),
                        );
                        if ui.button("Add app").clicked() {
                            let path = get_file_path();

                            let mut file = OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(path)
                                .unwrap();

                            self.tx
                                .clone()
                                .unwrap()
                                .send(self.app_name.clone())
                                .unwrap();

                            writeln!(file, "{}", &self.app_name).unwrap();
                            self.app_name = String::from("");
                        }
                    });
                });

                ui.add_space(10.0);

                card(ui, "Dailies", |ui| {
                    //Text box for adding daily and would create another line of Text box + checkBox
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.daily)
                                .hint_text("New daily…")
                                .desired_width(ui.available_width() - 90.0),
                        );

                        if ui.button("Add daily").clicked() {
                            let daily = Daily {
                                name: std::mem::take(&mut self.daily),
                                done: false,
                            };

                            self.dailies.push(daily);
                        }
                    });

                    ui.add_space(6.0);
                    if self.dailies.is_empty() {
                        ui.label(egui::RichText::new("No dailies yet.").weak().italics());
                    }

                    for daily in &mut self.dailies {
                        let text = if daily.done {
                            egui::RichText::new(&daily.name).strikethrough().weak()
                        } else {
                            egui::RichText::new(&daily.name)
                        };
                        ui.checkbox(&mut daily.done, text);
                    }
                });

                ui.add_space(10.0);

                card(ui, "Progress", |ui| {
                    //Show daily streak progress

                    //Show a new screen for finishing daily (unblock blocking loop)
                    ui.checkbox(&mut self.done, "Done");
                });
            });
        });
    }
}

// ---- UI styling (written by Claude) ----

const ACCENT: egui::Color32 = egui::Color32::from_rgb(79, 70, 229);

/// Draws a white rounded "card" with a title, and puts `add_contents` inside it.
fn card(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(225)))
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
    use egui::{Color32, FontId, TextStyle};

    ctx.all_styles_mut(|style| {
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
        v.panel_fill = Color32::from_rgb(246, 245, 242);
        v.selection.bg_fill = ACCENT.linear_multiply(0.35);
        v.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        v.hyperlink_color = ACCENT;

        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = 6.into();
        }
        v.widgets.inactive.weak_bg_fill = Color32::from_gray(236);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(224, 222, 250);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(205, 201, 247);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    });
}

struct RunningApplications;

impl RunningApplications {
    fn get_app_names(&self) -> Vec<String> {
        let mut apps: Vec<String> = Vec::new();
        let path = get_file_path();

        if let Ok(lines) = fs::read_to_string(&path) {
            for line in lines.lines() {
                apps.push(line.to_string());
            }
        } else {
            fs::write(&path, "").unwrap();
        }

        apps
    }

    fn close_apps(&self, app_names: &Vec<String>, sys: &mut System) {
        sys.refresh_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()));

        for app in app_names {
            for process in sys.processes_by_name(app.as_ref()) {
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

fn get_file_path() -> PathBuf {
    let Ok(appdata_path) = std::env::var("APPDATA") else {
        eprintln!("APPDATA is not found");
        std::process::exit(1);
    };

    let mut path = PathBuf::from(appdata_path);
    path.push(APP_FOLDER_NAME);

    if fs::read_dir(&path).is_err() {
        fs::create_dir(&path).unwrap();
    }

    path.push(BLACKLISTED_APPS_FILE);
    path
}
