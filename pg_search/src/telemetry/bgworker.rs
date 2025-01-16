// Copyright (c) 2023-2024 Retake, Inc.
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

use super::{TelemetryConfigStore, TermPoll};
use crate::telemetry::controller::{TelemetryController, TelemetrySender};
use crate::telemetry::postgres::PostgresDirectoryStore;
use crate::telemetry::posthog::PosthogStore;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use pgrx::bgworkers::{self, BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::{pg_guard, pg_sys, FromDatum, IntoDatum};
use std::ffi::CStr;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

/// Enumerating our extensions. It's important that these can be enumerated as integers
/// so that this enum can be passed as an i32 datum to a background worker.
/// Only primitive integers and booleans can be passed to workers, so we there's no
/// way otherwise for a background worker to tell which extension it is working for.
pub enum ParadeExtension {
    PgSearch = 1,
}

impl ParadeExtension {
    fn name(&self) -> String {
        match self {
            Self::PgSearch => "pg_search",
        }
        .into()
    }

    fn from_i32(n: i32) -> Option<Self> {
        match n {
            1 => Some(Self::PgSearch),
            _ => None,
        }
    }
}

pub struct SigtermHandler {}

impl SigtermHandler {
    fn new() -> Self {
        // You must listen to both SIGTERM and to SIGHUP.
        // Without SIGTERM, the background worker has no way to know when to terminate.
        // Without SIGHUP, the background worker will not be able to see configuration changes.
        // You need to poll for SIGTERM as a check for whether to quit.
        // You don't need to poll SIGHUP, you just need to have it here.
        BackgroundWorker::attach_signal_handlers(
            SignalWakeFlags::SIGTERM | SignalWakeFlags::SIGHUP,
        );
        Self {}
    }
}

impl TermPoll for SigtermHandler {
    fn term_poll(&self) -> bool {
        BackgroundWorker::sigterm_received()
    }
}

#[pg_guard]
#[no_mangle]
pub fn setup_telemetry_background_worker(extension: ParadeExtension) {
    // A background worker to read and send telemetry data to PostHog.
    BackgroundWorkerBuilder::new(&format!("{}_telemetry_worker", extension.name()))
        // Must be the name of a function in this file.
        .set_function("telemetry_worker")
        // Must be the name of the extension it will be loaded from.
        .set_library(&extension.name())
        // We pass the extension name to retrieve the associated data directory to read telemetry data from.
        .set_argument((extension as i32).into_datum())
        // It doesn't seem like bgworkers will start without this.
        .enable_spi_access()
        // RecoveryFinished is the last available stage for bgworker startup.
        // Allows time for all bootstrapped tables to be created.
        .set_start_time(bgworkers::BgWorkerStartTime::RecoveryFinished)
        .load();
}

#[pg_guard]
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn telemetry_worker(extension_name_datum: pg_sys::Datum) {
    let extension_i32 = unsafe { i32::from_datum(extension_name_datum, false) }
        .expect("extension enum i32 not passed to bgworker");
    let extension = ParadeExtension::from_i32(extension_i32)
        .unwrap_or_else(|| panic!("unexpected extension i32 passed to bgworker {extension_i32}"));
    let extension_name = extension.name();

    // If telemetry is not enabled at compile time, return early.
    if option_env!("PARADEDB_TELEMETRY") != Some("true") {
        pgrx::log!("PARADEDB_TELEMETRY var not set at compile time for {extension_name}");
        return;
    }

    pgrx::log!(
        "starting {extension_name} telemetry worker at PID {}",
        process::id()
    );

    let config_store = BgWorkerTelemetryConfig::new(&extension_name)
        .map(Box::new)
        .expect("could not initialize telemetry config");

    let telemetry_store = Box::new(PosthogStore {
        config_store: config_store.clone(),
    });

    let directory_store = Box::new(PostgresDirectoryStore {
        config_store: config_store.clone(),
    });

    let sender = TelemetrySender {
        telemetry_store,
        directory_store,
        config_store,
    };

    // These are the signals we want to receive. If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification.
    let sigterm_handler = SigtermHandler::new();

    let controller = TelemetryController {
        sender,
        directory_check_interval: Duration::from_secs(12 * 3600), // 12 hours
        sleep_interval: Duration::from_secs(1),
        term_poll: Box::new(sigterm_handler),
    };

    pgrx::log!("starting {extension_name} telemetry event loop");
    controller.run().expect("error in telemetry server");
    pgrx::log!("exiting {extension_name} telemetry event loop");
}

// The bgworker must only connect once to SPI, or it will segfault. We'll
// use this global variable to track whether that connection has happened.
static CONNECTED_TO_SPI: Mutex<bool> = Mutex::new(false);

#[derive(Clone)]
pub struct BgWorkerTelemetryConfig {
    pub posthog_api_key: String,
    pub posthog_host_url: String,
    pub extension_name: String,
    pub root_data_directory: PathBuf,
}

impl BgWorkerTelemetryConfig {
    pub fn new(extension_name: &str) -> Result<Self> {
        Ok(Self {
            posthog_api_key: option_env!("POSTHOG_API_KEY")
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("posthog api key missing"))?,
            posthog_host_url: option_env!("POSTHOG_HOST")
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("posthog host missing"))?,
            extension_name: extension_name.to_string(),
            root_data_directory: unsafe {
                PathBuf::from(
                    CStr::from_ptr(pgrx::pg_sys::DataDir)
                        .to_string_lossy()
                        .into_owned(),
                )
            },
        })
    }

    pub fn check_telemetry_setting(&self) -> Result<bool> {
        // PGRX seems to have a problem with GUC variables in background workers.
        // This means that we can't check if telemetry has been disabled by ALTER SYSTEM.
        // Instead, we need to connect to an existing database to check with an SPI query.
        // Users aren't supposed to delete the default 'postgres' database, so we'll connect to that.
        // If for some reason it doesn't exist, the telemetry worker will crash,
        // but other extension operations will be unaffected.
        let mut has_connected_to_spi = CONNECTED_TO_SPI.lock();

        if !(*has_connected_to_spi) {
            // This must be the only time in the background worker that you call
            // `connect_worker_to_spi`. If it is called again, the worker will segfault.
            // It's possible to pass "None" here for the database argument, but you will
            // only be able to access system catalogs, and not any GUC settings or use
            // any SPI queries.
            BackgroundWorker::connect_worker_to_spi(Some("postgres"), None);
            *has_connected_to_spi = true
        }

        let guc_setting_query = format!("SHOW paradedb.{}_telemetry", self.extension_name);

        // Check the GUC setting for telemetry.
        BackgroundWorker::transaction(|| match pgrx::Spi::get_one::<&str>(&guc_setting_query) {
            Ok(Some("true")) => Ok(true),
            Ok(Some("on")) => Ok(true),
            Err(err) => Err(anyhow!("error checking telemetry guc setting: {err}")),
            other => {
                pgrx::log!("{guc_setting_query} = {other:?}");
                Ok(false)
            }
        })
    }
}

impl TelemetryConfigStore for BgWorkerTelemetryConfig {
    fn telemetry_enabled(&self) -> Result<bool> {
        self.check_telemetry_setting()
    }
    fn extension_name(&self) -> Result<String> {
        Ok(self.extension_name.to_string())
    }
    fn telemetry_api_key(&self) -> Result<String> {
        Ok(self.posthog_api_key.to_string())
    }
    fn telemetry_host_url(&self) -> Result<String> {
        Ok(self.posthog_host_url.to_string())
    }
    fn root_data_directory(&self) -> Result<PathBuf> {
        Ok(self.root_data_directory.clone())
    }
}
