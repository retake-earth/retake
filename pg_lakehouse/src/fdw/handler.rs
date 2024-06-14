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

use anyhow::Result;
use pgrx::*;
use std::collections::HashMap;
use url::Url;

use crate::datafusion::format::TableFormat;

// use super::azblob::AzblobFdw;
// use super::azdls::AzdlsFdw;
use super::base::BaseFdw;
// use super::gcs::GcsFdw;
// use super::local::LocalFileFdw;
// use super::s3::S3Fdw;

#[derive(PartialEq)]
pub enum FdwHandler {
    S3,
    LocalFile,
    Gcs,
    Azblob,
    Azdls,
    Other,
}

/// These names are auto-generated by supabase-wrappers
/// If the FDW is called MyContainerFdw, the handler name will be my_container_fdw_handler
impl From<&str> for FdwHandler {
    fn from(handler_name: &str) -> Self {
        match handler_name {
            "s3_fdw_handler" => FdwHandler::S3,
            "local_file_fdw_handler" => FdwHandler::LocalFile,
            "gcs_fdw_handler" => FdwHandler::Gcs,
            "azblob_fdw_handler" => FdwHandler::Azblob,
            "azdls_fdw_handler" => FdwHandler::Azdls,
            _ => FdwHandler::Other,
        }
    }
}

impl From<*mut pg_sys::ForeignServer> for FdwHandler {
    fn from(server: *mut pg_sys::ForeignServer) -> Self {
        let oid = unsafe { (*server).fdwid };
        let fdw = unsafe { pg_sys::GetForeignDataWrapper(oid) };
        let handler_oid = unsafe { (*fdw).fdwhandler };
        let proc_tuple = unsafe {
            pg_sys::SearchSysCache1(
                pg_sys::SysCacheIdentifier_PROCOID as i32,
                handler_oid.into_datum().unwrap(),
            )
        };
        let pg_proc = unsafe { pg_sys::GETSTRUCT(proc_tuple) as pg_sys::Form_pg_proc };
        let handler_name = unsafe { name_data_to_str(&(*pg_proc).proname) };
        unsafe { pg_sys::ReleaseSysCache(proc_tuple) };

        FdwHandler::from(handler_name)
    }
}

impl From<*mut pg_sys::ForeignTable> for FdwHandler {
    fn from(table: *mut pg_sys::ForeignTable) -> Self {
        let server = unsafe { pg_sys::GetForeignServer((*table).serverid) };
        FdwHandler::from(server)
    }
}

pub fn register_object_store(
    handler: FdwHandler,
    url: &Url,
    format: TableFormat,
    server_options: HashMap<String, String>,
    user_mapping_options: HashMap<String, String>,
) -> Result<()> {
    match handler {
        // FdwHandler::S3 => {
        //     S3Fdw::register_object_store(url, format, server_options, user_mapping_options)?;
        // }
        // FdwHandler::LocalFile => {
        //     LocalFileFdw::register_object_store(url, format, server_options, user_mapping_options)?;
        // }
        // FdwHandler::Gcs => {
        //     GcsFdw::register_object_store(url, format, server_options, user_mapping_options)?;
        // }
        // FdwHandler::Azdls => {
        //     AzdlsFdw::register_object_store(url, format, server_options, user_mapping_options)?;
        // }
        // FdwHandler::Azblob => {
        //     AzblobFdw::register_object_store(url, format, server_options, user_mapping_options)?;
        // }
        _ => {}
    }

    Ok(())
}
