use chrono::{DateTime, Datelike, Days, Weekday, prelude::Local};
use std::fs;
use std::ops::Sub;
use sysinfo::System;

const BOX: &str = "[ ]";
const _COMPLETED_BOX: &str = "[x]";
const WEEKDAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];

fn main() {
    let this_month_progress = show_month_progress();
    println!("{this_month_progress:?}");
    let _proc_process = running_proc_apps();
    running_sysInfo_apps();
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

fn running_proc_apps() -> Vec<String> {
    println!("test");
    let proc = "/proc";
    let testfile = fs::read_dir(proc).unwrap();

    let mut file_names: Vec<String> = Vec::new();

    for i in testfile {
        let test = i.unwrap();
        let name = test.file_name();
        let as_str = name.to_str().unwrap();

        if as_str.chars().next().unwrap().is_ascii_digit() {
            let new_file = fs::read_to_string(format!("/proc/{}/comm", as_str));
            file_names.push(new_file.unwrap());
        }
    }

    file_names
}

fn running_sysInfo_apps() {
    if let Ok(mut sys) = System::new_all() {
        sys.refresh_all();

        let mut apps = Vec::new();

        for (pid, process) in sys.processes() {
            println!("{pid} {:?} ", process.name());
            apps.push(process.name());
        }

        println!("{:?}", apps);
    }
}
