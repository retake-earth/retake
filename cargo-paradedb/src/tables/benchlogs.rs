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

use anyhow::Result;
use serde::Deserialize;
use sqlx::query_builder::Separated;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::Postgres;
use std::{fs::File, io::BufReader};

use super::{PathReader, PathSource};

#[derive(Debug, Deserialize)]
pub struct EsLog {
    #[serde(rename = "@timestamp")]
    timestamp: DateTime<Utc>,
    #[serde(rename = "aws.cloudwatch")]
    aws_cloudwatch: serde_json::Value,
    cloud: serde_json::Value,
    #[serde(rename = "log.file.path")]
    log_file_path: String,
    input: serde_json::Value,
    data_stream: serde_json::Value,
    process: serde_json::Value,
    message: String,
    event: serde_json::Value,
    host: serde_json::Value,
    #[serde(rename = "metrics", deserialize_with = "deserialize_metrics_size")]
    metrics_size: i32,
    agent: serde_json::Value,
    tags: Vec<String>,
}

impl EsLog {
    pub fn insert_header(table: &str) -> String {
        format!(
            r#"INSERT INTO {table} (
            timestamp, aws_cloudwatch, cloud, log_file_path, input, data_stream,
            process, message, event, host, metrics_size, agent, tags
        )"#
        )
    }

    pub fn insert_push_values(
        mut b: Separated<'_, '_, Postgres, &'static str>,
        row: Result<Self, anyhow::Error>,
    ) {
        // The `push_bind` calls below must be in the same order as the columns
        // in the `insert_header` statement.
        if let Ok(row) = row {
            b.push_bind(row.timestamp);
            b.push_bind(row.aws_cloudwatch);
            b.push_bind(row.cloud);
            b.push_bind(row.log_file_path);
            b.push_bind(row.input);
            b.push_bind(row.data_stream);
            b.push_bind(row.process);
            b.push_bind(row.message);
            b.push_bind(row.event);
            b.push_bind(row.host);
            b.push_bind(row.metrics_size);
            b.push_bind(row.agent);
            b.push_bind(row.tags);
        }
    }

    pub fn create_table_statement(table: &str) -> String {
        format!(
            r#"CREATE TABLE IF NOT EXISTS {table} (
                id SERIAL PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                aws_cloudwatch JSONB,
                cloud JSONB,
                log_file_path TEXT,
                input JSONB,
                data_stream JSONB,
                process JSONB,
                message TEXT,
                event JSONB,
                host JSONB,
                metrics_size INT,
                agent JSONB,
                tags text[]
            );
            "#
        )
    }
}

// We flatten the `metrics` field on the logs object into a single integer
// field so that we have some numerical data to run tests on.

fn deserialize_metrics_size<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let metrics = serde_json::Value::deserialize(deserializer)?;
    metrics
        .get("size")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or(serde::de::Error::custom(
            "size field is missing or not an integer",
        ))
}

impl PathReader for EsLog {
    type Error = anyhow::Error;

    fn read_all<S: PathSource>(
        path_source: S,
    ) -> Result<Box<dyn Iterator<Item = Result<Self, Self::Error>>>, Self::Error> {
        let iterators: Result<Vec<_>, Self::Error> = path_source
            .paths()
            .map(|path| {
                let file = File::open(path)?;
                let buffered = BufReader::new(file);
                let deserializer = serde_json::Deserializer::from_reader(buffered)
                    .into_iter::<Self>()
                    .map(|result| result.map_err(Self::Error::from));
                Ok(deserializer)
            })
            .collect();

        let chained = Box::new(iterators?.into_iter().flatten());

        Ok(chained)
    }
}
