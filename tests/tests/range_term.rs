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

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::postgres::types::PgRange;
use sqlx::types::time::{Date, OffsetDateTime, PrimitiveDateTime};
use sqlx::PgConnection;
use std::fmt::{Debug, Display};
use std::ops::Bound;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use time::macros::{date, datetime};

const TARGET_INT4_LOWER_BOUNDS: [i32; 2] = [2, 10];
const TARGET_INT4_UPPER_BOUNDS: [i32; 1] = [10];
const QUERY_INT4_LOWER_BOUNDS: [i32; 7] = [-10, 1, 2, 3, 9, 10, 11];
const QUERY_INT4_UPPER_BOUNDS: [i32; 8] = [-10, 1, 2, 3, 9, 10, 11, 12];

const TARGET_INT8_LOWER_BOUNDS: [i64; 2] = [2, 10];
const TARGET_INT8_UPPER_BOUNDS: [i64; 1] = [10];
const QUERY_INT8_LOWER_BOUNDS: [i64; 7] = [-10, 1, 2, 3, 9, 10, 11];
const QUERY_INT8_UPPER_BOUNDS: [i64; 8] = [-10, 1, 2, 3, 9, 10, 11, 12];

const TARGET_NUMERIC_LOWER_BOUNDS: [f64; 2] = [2.5, 10.5];
const TARGET_NUMERIC_UPPER_BOUNDS: [f64; 1] = [10.5];
const QUERY_NUMERIC_LOWER_BOUNDS: [f64; 7] = [-10.5, 1.5, 2.5, 3.5, 9.5, 10.5, 11.5];
const QUERY_NUMERIC_UPPER_BOUNDS: [f64; 8] = [-10.5, 1.5, 2.5, 3.5, 9.5, 10.5, 11.5, 12.5];

const TARGET_DATE_LOWER_BOUNDS: [Date; 2] = [date!(2021 - 01 - 01), date!(2021 - 01 - 10)];
const TARGET_DATE_UPPER_BOUNDS: [Date; 1] = [date!(2021 - 01 - 10)];
const QUERY_DATE_LOWER_BOUNDS: [Date; 7] = [
    date!(2020 - 12 - 01),
    date!(2020 - 12 - 31),
    date!(2021 - 01 - 01),
    date!(2021 - 01 - 02),
    date!(2021 - 01 - 09),
    date!(2021 - 01 - 10),
    date!(2021 - 01 - 11),
];
const QUERY_DATE_UPPER_BOUNDS: [Date; 8] = [
    date!(2020 - 12 - 01),
    date!(2020 - 12 - 31),
    date!(2021 - 01 - 01),
    date!(2021 - 01 - 02),
    date!(2021 - 01 - 09),
    date!(2021 - 01 - 10),
    date!(2021 - 01 - 11),
    date!(2021 - 01 - 12),
];

const TARGET_TIMESTAMP_LOWER_BOUNDS: [PrimitiveDateTime; 2] =
    [datetime!(2019-01-01 0:00), datetime!(2019-01-10 0:00)];
const TARGET_TIMESTAMP_UPPER_BOUNDS: [PrimitiveDateTime; 1] = [datetime!(2019-01-10 0:00)];
const QUERY_TIMESTAMP_LOWER_BOUNDS: [PrimitiveDateTime; 7] = [
    datetime!(2018-12-31 23:59:59),
    datetime!(2018-12-31 23:59:59),
    datetime!(2019-01-01 0:00:00),
    datetime!(2019-01-01 0:00:01),
    datetime!(2019-01-09 23:59:59),
    datetime!(2019-01-10 0:00:00),
    datetime!(2019-01-10 0:00:01),
];
const QUERY_TIMESTAMP_UPPER_BOUNDS: [PrimitiveDateTime; 8] = [
    datetime!(2018-12-31 23:59:59),
    datetime!(2018-12-31 23:59:59),
    datetime!(2019-01-01 0:00:00),
    datetime!(2019-01-01 0:00:01),
    datetime!(2019-01-09 23:59:59),
    datetime!(2019-01-10 0:00:00),
    datetime!(2019-01-10 0:00:01),
    datetime!(2019-01-11 0:00:00),
];

const TARGET_TIMESTAMPTZ_LOWER_BOUNDS: [OffsetDateTime; 2] = [
    datetime!(2021-01-01 00:00:00 +02:00),
    datetime!(2021-01-10 00:00:00 +02:00),
];
const TARGET_TIMESTAMPTZ_UPPER_BOUNDS: [OffsetDateTime; 1] =
    [datetime!(2021-01-10 00:00:00 +02:00)];
const QUERY_TIMESTAMPTZ_LOWER_BOUNDS: [OffsetDateTime; 7] = [
    datetime!(2020-12-30 23:59:59 UTC),
    datetime!(2021-01-01 00:00:00 +02:00),
    datetime!(2021-01-01 00:00:00 UTC),
    datetime!(2021-01-01 00:00:00 -02:00),
    datetime!(2021-01-10 00:00:00 +02:00),
    datetime!(2021-01-10 00:00:00 UTC),
    datetime!(2021-01-10 00:00:00 -02:00),
];
const QUERY_TIMESTAMPTZ_UPPER_BOUNDS: [OffsetDateTime; 8] = [
    datetime!(2020-12-30 23:59:59 UTC),
    datetime!(2021-01-01 00:00:00 +02:00),
    datetime!(2021-01-01 00:00:00 UTC),
    datetime!(2021-01-01 00:00:00 -02:00),
    datetime!(2021-01-10 00:00:00 +02:00),
    datetime!(2021-01-10 00:00:00 UTC),
    datetime!(2021-01-10 00:00:00 -02:00),
    datetime!(2021-01-11 00:00:00 +02:00),
];

#[derive(Clone, Copy, Debug, EnumIter, PartialEq)]
enum BoundType {
    Included,
    Excluded,
    Unbounded,
}

impl BoundType {
    fn to_bound<T>(self, val: T) -> Bound<T> {
        match self {
            BoundType::Included => Bound::Included(val),
            BoundType::Excluded => Bound::Excluded(val),
            BoundType::Unbounded => Bound::Unbounded,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RangeRelation {
    Intersects,
    Contains,
    Within,
}

#[rstest]
async fn range_term_contains_int4range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "weights",
        "int4range",
        &TARGET_INT4_LOWER_BOUNDS,
        &TARGET_INT4_UPPER_BOUNDS,
        &QUERY_INT4_LOWER_BOUNDS,
        &QUERY_INT4_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_int8range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "quantities",
        "int8range",
        &TARGET_INT8_LOWER_BOUNDS,
        &TARGET_INT8_UPPER_BOUNDS,
        &QUERY_INT8_LOWER_BOUNDS,
        &QUERY_INT8_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_numrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "prices",
        "numrange",
        &TARGET_NUMERIC_LOWER_BOUNDS,
        &TARGET_NUMERIC_UPPER_BOUNDS,
        &QUERY_NUMERIC_LOWER_BOUNDS,
        &QUERY_NUMERIC_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_daterange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "ship_dates",
        "daterange",
        &TARGET_DATE_LOWER_BOUNDS,
        &TARGET_DATE_UPPER_BOUNDS,
        &QUERY_DATE_LOWER_BOUNDS,
        &QUERY_DATE_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_tsrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "facility_arrival_times",
        "tsrange",
        &TARGET_TIMESTAMP_LOWER_BOUNDS,
        &TARGET_TIMESTAMP_UPPER_BOUNDS,
        &QUERY_TIMESTAMP_LOWER_BOUNDS,
        &QUERY_TIMESTAMP_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_tstzrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "delivery_times",
        "tstzrange",
        &TARGET_TIMESTAMPTZ_LOWER_BOUNDS,
        &TARGET_TIMESTAMPTZ_UPPER_BOUNDS,
        &QUERY_TIMESTAMPTZ_LOWER_BOUNDS,
        &QUERY_TIMESTAMPTZ_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_int4range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "weights",
        "int4range",
        &TARGET_INT4_LOWER_BOUNDS,
        &TARGET_INT4_UPPER_BOUNDS,
        &QUERY_INT4_LOWER_BOUNDS,
        &QUERY_INT4_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_int8range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "quantities",
        "int8range",
        &TARGET_INT8_LOWER_BOUNDS,
        &TARGET_INT8_UPPER_BOUNDS,
        &QUERY_INT8_LOWER_BOUNDS,
        &QUERY_INT8_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_numrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "prices",
        "numrange",
        &TARGET_NUMERIC_LOWER_BOUNDS,
        &TARGET_NUMERIC_UPPER_BOUNDS,
        &QUERY_NUMERIC_LOWER_BOUNDS,
        &QUERY_NUMERIC_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_daterange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "ship_dates",
        "daterange",
        &TARGET_DATE_LOWER_BOUNDS,
        &TARGET_DATE_UPPER_BOUNDS,
        &QUERY_DATE_LOWER_BOUNDS,
        &QUERY_DATE_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_tsrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "facility_arrival_times",
        "tsrange",
        &TARGET_TIMESTAMP_LOWER_BOUNDS,
        &TARGET_TIMESTAMP_UPPER_BOUNDS,
        &QUERY_TIMESTAMP_LOWER_BOUNDS,
        &QUERY_TIMESTAMP_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_tstzrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "delivery_times",
        "tstzrange",
        &TARGET_TIMESTAMPTZ_LOWER_BOUNDS,
        &TARGET_TIMESTAMPTZ_UPPER_BOUNDS,
        &QUERY_TIMESTAMPTZ_LOWER_BOUNDS,
        &QUERY_TIMESTAMPTZ_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_int4range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "weights",
        "int4range",
        &TARGET_INT4_LOWER_BOUNDS,
        &TARGET_INT4_UPPER_BOUNDS,
        &QUERY_INT4_LOWER_BOUNDS,
        &QUERY_INT4_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_int8range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "quantities",
        "int8range",
        &TARGET_INT8_LOWER_BOUNDS,
        &TARGET_INT8_UPPER_BOUNDS,
        &QUERY_INT8_LOWER_BOUNDS,
        &QUERY_INT8_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_numrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "prices",
        "numrange",
        &TARGET_NUMERIC_LOWER_BOUNDS,
        &TARGET_NUMERIC_UPPER_BOUNDS,
        &QUERY_NUMERIC_LOWER_BOUNDS,
        &QUERY_NUMERIC_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_daterange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "ship_dates",
        "daterange",
        &TARGET_DATE_LOWER_BOUNDS,
        &TARGET_DATE_UPPER_BOUNDS,
        &QUERY_DATE_LOWER_BOUNDS,
        &QUERY_DATE_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_tsrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "facility_arrival_times",
        "tsrange",
        &TARGET_TIMESTAMP_LOWER_BOUNDS,
        &TARGET_TIMESTAMP_UPPER_BOUNDS,
        &QUERY_TIMESTAMP_LOWER_BOUNDS,
        &QUERY_TIMESTAMP_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_tstzrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "delivery_times",
        "tstzrange",
        &TARGET_TIMESTAMPTZ_LOWER_BOUNDS,
        &TARGET_TIMESTAMPTZ_UPPER_BOUNDS,
        &QUERY_TIMESTAMPTZ_LOWER_BOUNDS,
        &QUERY_TIMESTAMPTZ_UPPER_BOUNDS,
    );
}

#[allow(clippy::too_many_arguments)]
fn execute_range_test<T>(
    conn: &mut PgConnection,
    relation: RangeRelation,
    table: &str,
    field: &str,
    range_type: &str,
    target_lower_bounds: &[T],
    target_upper_bounds: &[T],
    query_lower_bounds: &[T],
    query_upper_bounds: &[T],
) where
    T: Debug + Display + Clone + PartialEq + std::cmp::PartialOrd,
{
    DeliveriesTable::setup().execute(conn);

    // Insert all combinations of ranges
    for lower_bound_type in BoundType::iter() {
        for upper_bound_type in BoundType::iter() {
            for lower_bound in target_lower_bounds {
                for upper_bound in target_upper_bounds {
                    let range = PgRange {
                        start: lower_bound_type.to_bound(lower_bound.clone()),
                        end: upper_bound_type.to_bound(upper_bound.clone()),
                    };
                    format!(
                        "INSERT INTO {} ({}) VALUES ('{}'::{})",
                        table, field, range, range_type
                    )
                    .execute(conn);
                }
            }
        }
    }

    // Insert null range value
    format!("INSERT INTO {} ({}) VALUES (NULL)", table, field).execute(conn);

    // Run all combinations of range queries
    for lower_bound_type in BoundType::iter() {
        for upper_bound_type in BoundType::iter() {
            for lower_bound in query_lower_bounds {
                for upper_bound in query_upper_bounds {
                    let range = PgRange {
                        start: lower_bound_type.to_bound(lower_bound.clone()),
                        end: upper_bound_type.to_bound(upper_bound.clone()),
                    };

                    if lower_bound >= upper_bound {
                        continue;
                    }

                    let expected: Vec<(i32,)> = match relation {
                        RangeRelation::Contains => {
                            postgres_contains_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Within => {
                            postgres_within_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Intersects => {
                            postgres_intersects_query(&range, table, field, range_type).fetch(conn)
                        }
                    };

                    let result_json: Vec<(i32,)> = match relation {
                        RangeRelation::Contains => {
                            pg_search_contains_json_query(&range, table, field, range_type)
                                .fetch(conn)
                        }
                        RangeRelation::Within => {
                            pg_search_within_json_query(&range, table, field, range_type)
                                .fetch(conn)
                        }
                        RangeRelation::Intersects => {
                            pg_search_intersects_json_query(&range, table, field, range_type)
                                .fetch(conn)
                        }
                    };

                    let result: Vec<(i32,)> = match relation {
                        RangeRelation::Contains => {
                            pg_search_contains_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Within => {
                            pg_search_within_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Intersects => {
                            pg_search_intersects_query(&range, table, field, range_type).fetch(conn)
                        }
                    };

                    println!(
                        "expected: {expected:?}, {result:?} {} {}",
                        postgres_contains_query(&range, table, field, range_type),
                        pg_search_contains_query(&range, table, field, range_type),
                    );

                    assert_eq!(expected, result, "query failed for range: {:?}", range);
                    assert_eq!(
                        expected, result_json,
                        "json query failed for range: {:?}",
                        range
                    );
                }
            }
        }
    }
}

fn postgres_contains_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {}
        WHERE '{}'::{} @> {}
        ORDER BY delivery_id",
        table, range, range_type, field
    )
}

fn postgres_within_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {}
        WHERE {} @> '{}'::{}
        ORDER BY delivery_id",
        table, field, range, range_type
    )
}

fn postgres_intersects_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {}
        WHERE '{}'::{} && {}
        ORDER BY delivery_id",
        table, range, range_type, field
    )
}

fn pg_search_contains_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {}
        WHERE delivery_id @@@ paradedb.range_term('{}', '{}'::{}, 'Contains')
        ORDER BY delivery_id",
        table, field, range, range_type
    )
}

fn pg_search_contains_json_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    let is_datetime = ["daterange", "tsrange", "tstzrange"].contains(&range_type);
    let lower_bound = match range.start {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    let upper_bound = match range.end {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    format!(
        r#"
        SELECT delivery_id FROM {}
        WHERE delivery_id @@@ '{{
            "range_contains": {{
                "field": "{}",
                "lower_bound": {},
                "upper_bound": {}
            }}
        }}'::jsonb
        ORDER BY delivery_id"#,
        table, field, lower_bound, upper_bound
    )
}

fn pg_search_within_json_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    let is_datetime = ["daterange", "tsrange", "tstzrange"].contains(&range_type);
    let lower_bound = match range.start {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    let upper_bound = match range.end {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    format!(
        r#"
        SELECT delivery_id FROM {}
        WHERE delivery_id @@@ '{{
            "range_within": {{
                "field": "{}",
                "lower_bound": {},
                "upper_bound": {}
            }}
        }}'::jsonb
        ORDER BY delivery_id"#,
        table, field, lower_bound, upper_bound
    )
}

fn pg_search_intersects_json_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    let is_datetime = ["daterange", "tsrange", "tstzrange"].contains(&range_type);
    let lower_bound = match range.start {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    let upper_bound = match range.end {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{}""#, val)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    format!(
        r#"
        SELECT delivery_id FROM {}
        WHERE delivery_id @@@ '{{
            "range_intersects": {{
                "field": "{}",
                "lower_bound": {},
                "upper_bound": {}
            }}
        }}'::jsonb
        ORDER BY delivery_id"#,
        table, field, lower_bound, upper_bound
    )
}

fn pg_search_within_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {}
        WHERE delivery_id @@@ paradedb.range_term('{}', '{}'::{}, 'Within')
        ORDER BY delivery_id",
        table, field, range, range_type
    )
}

fn pg_search_intersects_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {}
        WHERE delivery_id @@@ paradedb.range_term('{}', '{}'::{}, 'Intersects')
        ORDER BY delivery_id",
        table, field, range, range_type
    )
}
