use chrono::{DateTime, Datelike, Days, Weekday, prelude::Local};
use std::ops::Sub;
use std::{fs, path::PathBuf};
use sysinfo::{Pid, Signal, System};

const BOX: &str = "[ ]";
const _COMPLETED_BOX: &str = "[x]";
const WEEKDAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];

fn main() {
    let this_month_progress = show_month_progress();
    println!("{this_month_progress:?}");

    // let system_name = System::name().unwrap();
    // let open_apps: Result<Vec<String>, String> = match system_name.as_str() {
    //     "Windows" => Ok(running_system_info_apps()),
    //     "Ubuntu" => Ok(running_proc_apps()),
    //     _ => Err("? Operation System not found".to_string()),
    // };

    // let test = app_pid("chrome.exe").unwrap();

    let mut apps: Vec<String> = Vec::new();

    // if it's windows still need to do a bool
    // also need to create folder and filesname
    // next I need to create TUI or CLI for adding .exe
    let mut path = PathBuf::from(std::env::var("APPDATA").unwrap());
    path.push("abs_todo");
    path.push("apps.txt");

    if let Ok(lines) = fs::read_to_string(path) {
        println!("nothign?");
        for line in lines.lines() {
            apps.push(line.to_string());
            println!("why not pushing? {}", line.to_string());
        }
    }
    println!("{apps:?}");
    let running_apps = RunningApplications { apps };

    running_apps.close_apps();
}

struct RunningApplications {
    apps: Vec<String>,
}

impl RunningApplications {
    fn new() -> RunningApplications {
        RunningApplications {
            apps: vec![String::from("")],
        }
    }

    fn close_apps(&self) {
        let closing_apps = &self.apps;

        println!("{:?}", closing_apps);
        if let Ok(sys) = System::new_all() {
            for app in closing_apps {
                println!("should run twice");
                for process in sys.processes_by_name(app.as_ref()) {
                    println!("{} {:?}", process.pid(), process.name());
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

// fn running_proc_apps() -> Vec<String> {
//     let proc = "/proc";
//     let testfile = fs::read_dir(proc).unwrap();
//
//     let mut file_names: Vec<String> = Vec::new();
//
//     for i in testfile {
//         let test = i.unwrap();
//         let name = test.file_name();
//         let as_str = name.to_str().unwrap();
//
//         if as_str.chars().next().unwrap().is_ascii_digit() {
//             let new_file = fs::read_to_string(format!("/proc/{}/comm", as_str));
//             file_names.push(new_file.unwrap());
//         }
//     }
//
//     file_names
// }

// fn running_system_info_apps() -> Vec<String> {
//     let mut apps = Vec::new();
//
//     if let Ok(mut sys) = System::new_all() {
//         sys.refresh_all();
//
//         for (pid, process) in sys.processes() {
//             let process_name_str = process.name().to_str().unwrap();
//             apps.push(process_name_str.to_string());
//         }
//     };
//
//     apps
// }

fn app_pid(app_name: &str) -> Result<u32, String> {
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
