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

mod bgworker;
mod controller;
mod event;
mod postgres;
mod posthog;

use self::event::TelemetryEvent;
use anyhow::Result;
pub use bgworker::{setup_telemetry_background_worker, ParadeExtension};
use std::path::PathBuf;

pub trait TelemetryStore {
    fn get_connection(&self) -> Result<Box<dyn TelemetryConnection>>;
}

pub trait TelemetryConnection {
    fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<()>;
}

pub trait DirectoryStore {
    fn root_path(&self) -> Result<PathBuf>;
    fn extension_path(&self) -> Result<PathBuf>;
    fn extension_size(&self) -> Result<u64>;
    fn extension_uuid_path(&self) -> Result<PathBuf>;
    fn extension_uuid(&self) -> Result<String>;
}

pub trait TermPoll {
    fn term_poll(&self) -> bool;
}

pub trait TelemetryConfigStore {
    fn telemetry_enabled(&self) -> Result<bool>;
    fn extension_name(&self) -> Result<String>;
    fn telemetry_api_key(&self) -> Result<String>;
    fn telemetry_host_url(&self) -> Result<String>;
    fn root_data_directory(&self) -> Result<PathBuf>;
}
