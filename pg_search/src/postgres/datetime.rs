// Copyright (c) 2023-2025 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use chrono::{NaiveDate, NaiveDateTime};
use pgrx::datum::datetime_support::DateTimeConversionError;

pub static MICROSECONDS_IN_SECOND: u32 = 1_000_000;

pub fn datetime_components_to_tantivy_date(
    ymd: Option<(i32, u8, u8)>,
    hms_micro: (u8, u8, u8, u32),
) -> Result<tantivy::schema::OwnedValue, DateTimeConversionError> {
    let naive_dt = match ymd {
        Some(ymd) => NaiveDate::from_ymd_opt(ymd.0, ymd.1.into(), ymd.2.into())
            .expect("ymd should be valid for NaiveDate::from_ymd_opt"),
        None => NaiveDateTime::UNIX_EPOCH.date(),
    }
    .and_hms_micro_opt(
        hms_micro.0.into(),
        hms_micro.1.into(),
        hms_micro.2.into(),
        hms_micro.3 % MICROSECONDS_IN_SECOND,
    )
    .ok_or(DateTimeConversionError::OutOfRange)?
    .and_utc();

    Ok(tantivy::schema::OwnedValue::Date(
        tantivy::DateTime::from_timestamp_micros(naive_dt.timestamp_micros()),
    ))
}
