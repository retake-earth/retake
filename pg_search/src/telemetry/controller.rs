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
use chrono::Utc;
use pgrx::pg_sys::{PG_VERSION, PG_VERSION_STR};
use std::{
    thread,
    time::{Duration, Instant},
};

use super::{
    event::TelemetryEvent, DirectoryStore, TelemetryConfigStore, TelemetryStore, TermPoll,
};

pub struct TelemetrySender {
    pub directory_store: Box<dyn DirectoryStore>,
    pub telemetry_store: Box<dyn TelemetryStore>,
    pub config_store: Box<dyn TelemetryConfigStore>,
}

impl TelemetrySender {
    pub fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<()> {
        let conn = self.telemetry_store.get_connection()?;

        if self.config_store.telemetry_enabled()? {
            conn.send(uuid, event)
        } else {
            pgrx::log!(
                "paradedb telemetry is disabled, not sending event: {}",
                event.name()
            );
            Ok(())
        }
    }
    pub fn send_deployment(&self) -> Result<()> {
        if self.directory_store.extension_uuid_path()?.exists() {
            pgrx::log!("extension has been deployed before, skipping deployment telemetry");
            return Ok(());
        }
        let uuid = self.directory_store.extension_uuid()?;
        let path = self.directory_store.extension_path()?;
        let os_info = os_info::get();
        let event = TelemetryEvent::Deployment {
            timestamp: Utc::now().to_rfc3339(),
            arch: os_info.architecture().unwrap_or_default().to_string(),
            extension_name: self.config_store.extension_name()?,
            extension_version: env!("CARGO_PKG_VERSION").to_string(),
            extension_path: path,
            os_type: os_info.os_type().to_string(),
            os_version: os_info.version().to_string(),
            replication_mode: std::env::var("POSTGRESQL_REPLICATION_MODE").ok(),
            postgres_version: PG_VERSION.to_str()?.trim_end_matches('\0').to_owned(),
            postgres_version_details: PG_VERSION_STR.to_str()?.trim_end_matches('\0').to_owned(),
        };

        self.send(&uuid, &event)
    }

    pub fn send_directory_check(&self) -> Result<()> {
        let uuid = self.directory_store.extension_uuid()?;
        let size = self.directory_store.extension_size()?;
        let path = self.directory_store.extension_path()?;
        let event = TelemetryEvent::DirectoryStatus {
            path,
            size,
            humansize: humansize::format_size(size, humansize::DECIMAL),
            replication_mode: std::env::var("POSTGRESQL_REPLICATION_MODE").ok(),
            extension_name: self.config_store.extension_name()?,
        };

        self.send(&uuid, &event)
    }
}

pub struct TelemetryController {
    pub sender: TelemetrySender,
    pub directory_check_interval: Duration,
    pub sleep_interval: Duration,
    pub term_poll: Box<dyn TermPoll>,
}

impl TelemetryController {
    pub fn run(&self) -> Result<()> {
        let mut last_action_time = Instant::now();
        self.sender.send_deployment()?;
        loop {
            // Sleep for a short period to remain responsive to SIGTERM
            thread::sleep(self.sleep_interval);

            // Check if the wait_duration has passed since the last time we sent telemetry data
            if Instant::now().duration_since(last_action_time) >= self.directory_check_interval {
                self.sender.send_directory_check()?;
                last_action_time = Instant::now();
            }

            // Check for shutdown
            if self.term_poll.term_poll() {
                return Ok(());
            }
        }
    }
}
