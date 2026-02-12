// Added by LDemetrios

use chrono::{DateTime, Datelike, FixedOffset, Local, TimeZone, Timelike, Utc};
use serde::{Deserialize};
use std::sync::OnceLock;
use typst_library::foundations::Datetime;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Now {
    Fixed { millis: i64, nanos: i32 },
    System,
}

pub struct TimeHolder(Repr);

enum Repr {
    Fixed { stamp: DateTime<Utc> },
    System { locked: OnceLock<DateTime<Utc>> },
}

impl TimeHolder {
    pub fn new(now: Now) -> Self {
        TimeHolder(match now {
            Now::Fixed { millis, nanos } => {
                let stamp = Utc
                    .timestamp_millis_opt(millis)
                    .single()
                    .unwrap()
                    .with_nanosecond(nanos as u32)
                    .unwrap();
                Repr::Fixed { stamp }
            }
            Now::System => Repr::System { locked: OnceLock::new() },
        })
    }

    pub fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = match &self.0 {
            Repr::Fixed { stamp } => stamp,
            Repr::System { locked } => locked.get_or_init(Utc::now),
        };

        // The time with the specified UTC offset, or within the local time zone.
        let with_offset = match offset {
            None => now.with_timezone(&Local).fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                now.with_timezone(&FixedOffset::east_opt(seconds)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }
}


