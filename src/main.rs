use chrono::{DateTime, Datelike, Days, Weekday, prelude::Local};
use std::ops::Sub;
use std::thread;
use std::time::Duration;
use std::{fs, path::PathBuf};
use sysinfo::{Signal, System};

const BOX: &str = "[ ]";
const _COMPLETED_BOX: &str = "[x]";
const WEEKDAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
const APP_FOLDER_NAME: &str = "abs_todo";
const BLACKLISTED_APPS_FILE: &str = "apps.txt";
fn main() {
    //only windows for now
    let _this_month_progress = show_month_progress();
    // next I need to create TUI or CLI for adding .exe

    let running_apps = RunningApplications;

    let apps = running_apps.get_app_names();

    loop {
        thread::sleep(Duration::from_secs(2));
        running_apps.close_apps(&apps);
    }
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

    fn _add_apps_to_file(&self) {}

    fn close_apps(&self, app_names: &Vec<String>) {
        if let Ok(sys) = System::new_all() {
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

fn _app_pid(app_name: &str) -> Result<u32, String> {
    // i think we can change new_all to processes only
    if let Ok(mut sys) = System::new_all() {
        sys.refresh_all();

        for (pid, process) in sys.processes() {
            let process_name = process.name().to_str().unwrap();
            if process_name == app_name {
                return Ok(pid.as_u32());
            }
        }
    };
    Err("Couldn't find the application".to_string())
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
