use chrono::{DateTime, Datelike, Days, TimeZone, Weekday, prelude::Local};
use std::ops::Sub;

const BOX: &str = "[ ]";
const COMPLETED_BOX: &str = "[x]";
const WEEKDAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
fn main() {
    let local_time: DateTime<Local> = Local::now();
    let weekday = local_time.weekday();
    let month = local_time.month();
    println!("show todays date {}", local_time);
    println!("show weekday {}", weekday);
    println!("{}{}", BOX, COMPLETED_BOX);
    println!("{month}");
    println!("┌────┐\n│{}{}{}{}│\n└────┘", BOX, BOX, BOX, BOX);

    let current_day = local_time.day() as u8;
    let mut rest_of_the_day = local_time.num_days_in_month() - &current_day;

    let starting_weekday_of_month = local_time.weekday();

    let weekday_index = match starting_weekday_of_month {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    };

    let day = local_time.day().sub(1).into();
    let day = Days::new(day);
    let day = local_time.sub(day);
    let first_weekday = day.weekday();

    let mut weekdays = 0;
    while let Some(_) = rest_of_the_day.checked_sub(7) {
        rest_of_the_day -= 7;
        weekdays += 1;
    }

    for i in 0..rest_of_the_day {
        println!("{i}");
    }
    println!("{first_weekday}");
}
