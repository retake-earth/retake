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
use serde_json::json;

use super::{event::TelemetryEvent, TelemetryConfigStore, TelemetryConnection, TelemetryStore};

struct PosthogConnection {
    api_key: String,
    host: String,
    client: reqwest::blocking::Client,
}

impl PosthogConnection {
    fn new(api_key: &str, host: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            host: host.to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/capture", self.host)
    }
}

pub struct PosthogStore {
    pub config_store: Box<dyn TelemetryConfigStore>,
}

impl TelemetryStore for PosthogStore {
    fn get_connection(&self) -> Result<Box<dyn TelemetryConnection>> {
        Ok(Box::new(PosthogConnection::new(
            &self.config_store.telemetry_api_key()?,
            &self.config_store.telemetry_host_url()?,
        )))
    }
}

impl TelemetryConnection for PosthogConnection {
    fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<()> {
        let mut properties = serde_json::to_value(event)?;
        properties["commit_sha"] = serde_json::to_value(event.commit_sha())?;
        properties["paradedb_version"] = serde_json::to_value(event.paradedb_version())?;

        let data = json!({
            "api_key": self.api_key,
            "event": event.name(),
            "distinct_id": uuid,
            "properties": properties,
        });

        self.client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .body(data.to_string())
            .send()?;

        Ok(())
    }
}
