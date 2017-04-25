// Copyright 2016 The rustc-perf Project Developers. See the COPYRIGHT
// file at the top-level directory.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::{Add, Sub};
use std::fmt;

use chrono::{UTC, DateTime, TimeZone, Duration, Datelike};
use serde::{self, Serialize, Deserialize};
use util;

use errors::*;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DeltaTime(#[serde(with = "util::round_float")] pub f64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date(pub DateTime<UTC>);

impl Date {
    pub fn from_str(s: &str) -> Result<Date> {
        match DateTime::parse_from_rfc3339(s) {
            Ok(value) => Ok(Date(value.with_timezone(&UTC))),
            Err(err) => {
                Err(err).chain_err(|| format!("parse failure of date {} with RFC 3339 format", s))
            }
        }
    }

    pub fn from_format(date: &str, format: &str) -> Result<Date> {
        match DateTime::parse_from_str(date, format) {
            Ok(value) => Ok(Date(value.with_timezone(&UTC))),
            Err(_) => {
                match UTC.datetime_from_str(date, format) {
                    Ok(dt) => Ok(Date(dt)),
                    Err(err) => {
                        Err(err).chain_err(|| {
                            format!("parse failure of date {} with format string: {}",
                                    date,
                                    format)
                        })
                    }
                }
            }
        }
    }

    pub fn ymd_hms(year: i32, month: u32, day: u32, h: u32, m: u32, s: u32) -> Date {
        Date(UTC.ymd(year, month, day).and_hms(h, m, s))
    }

    pub fn start_of_week(&self) -> Date {
        let weekday = self.0.weekday();
        // num_days_from_sunday is 0 for Sunday
        Date(self.0 - Duration::days(weekday.num_days_from_sunday() as i64))
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_rfc3339())
    }
}

impl From<DateTime<UTC>> for Date {
    fn from(datetime: DateTime<UTC>) -> Date {
        Date(datetime)
    }
}

impl PartialEq<DateTime<UTC>> for Date {
    fn eq(&self, other: &DateTime<UTC>) -> bool {
        self.0 == *other
    }
}

impl Sub<Duration> for Date {
    type Output = Date;
    fn sub(self, rhs: Duration) -> Date {
        Date(self.0 - rhs)
    }
}

impl Add<Duration> for Date {
    type Output = Date;
    fn add(self, rhs: Duration) -> Date {
        Date(self.0 + rhs)
    }
}

impl Serialize for Date {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where S: serde::ser::Serializer
    {
        serializer.serialize_str(&self.0.to_rfc3339())
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Date, D::Error>
        where D: serde::de::Deserializer<'de>
    {
        struct DateVisitor;

        impl<'de> serde::de::Visitor<'de> for DateVisitor {
            type Value = Date;

            fn visit_str<E>(self, value: &str) -> ::std::result::Result<Date, E>
                where E: serde::de::Error
            {
                Date::from_str(value)
                    .map_err(|_| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Str(value), &self)
                    })
            }

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an RFC 3339 date")
            }
        }

        deserializer.deserialize_str(DateVisitor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionalDate {
    Date(Date),
    CouldNotParse(String),
}

impl OptionalDate {
    pub fn is_date(&self) -> bool {
        match *self {
            OptionalDate::Date(_) => true,
            _ => false,
        }
    }

    pub fn as_start(&self, last_date: Date) -> Date {
        // Handle missing start by returning 30 days before end.
        if let OptionalDate::Date(date) = *self {
            date
        } else {
            let end = self.as_end(last_date);
            end - Duration::days(30)
        }
    }

    pub fn as_end(&self, last_date: Date) -> Date {
        // Handle missing end by using the last available date.
        if let OptionalDate::Date(date) = *self {
            date
        } else {
            last_date
        }
    }
}

impl<'de> serde::Deserialize<'de> for OptionalDate {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<OptionalDate, D::Error>
        where D: serde::de::Deserializer<'de>
    {
        struct DateVisitor;

        impl<'de> serde::de::Visitor<'de> for DateVisitor {
            type Value = OptionalDate;

            fn visit_str<E>(self, value: &str) -> ::std::result::Result<OptionalDate, E>
                where E: serde::de::Error
            {
                match Date::from_str(value) {
                    Ok(date) => Ok(OptionalDate::Date(date)),
                    Err(err) => {
                        if !value.is_empty() {
                            error!("bad date {:?}: {:?}", value, err);
                        }
                        Ok(OptionalDate::CouldNotParse(value.to_string()))
                    }
                }
            }

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("OptionalDate")
            }
        }

        deserializer.deserialize_str(DateVisitor)
    }
}

impl serde::Serialize for OptionalDate {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        match *self {
            OptionalDate::Date(date) => date.serialize(serializer),
            OptionalDate::CouldNotParse(_) => serializer.serialize_str(""), // TODO: Warning?
        }
    }
}
