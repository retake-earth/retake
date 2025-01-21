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

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TelemetryEvent {
    Deployment {
        timestamp: String,
        arch: String,
        extension_name: String,
        extension_version: String,
        extension_path: PathBuf,
        os_type: String,
        os_version: String,
        replication_mode: Option<String>,
        postgres_version: String,
        postgres_version_details: String,
    },
    DirectoryStatus {
        extension_name: String,
        replication_mode: Option<String>,
        path: PathBuf,
        size: u64,
        humansize: String,
    },
}

impl TelemetryEvent {
    pub fn name(&self) -> String {
        match self {
            Self::Deployment { extension_name, .. } => format!("{extension_name} Deployment"),
            Self::DirectoryStatus { extension_name, .. } => {
                format!("{extension_name} Directory Status")
            }
        }
    }

    pub fn commit_sha(&self) -> Option<String> {
        option_env!("COMMIT_SHA").map(String::from)
    }

    pub fn paradedb_version(&self) -> Option<String> {
        option_env!("PARADEDB_VERSION").map(String::from)
    }
}
