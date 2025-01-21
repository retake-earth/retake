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

use pgrx::pg_sys;
use pgrx::pg_sys::AsPgCStr;
use std::ptr::NonNull;

pub struct Explainer {
    state: NonNull<pg_sys::ExplainState>,
}

#[allow(dead_code)]
impl Explainer {
    pub fn new(state: *mut pg_sys::ExplainState) -> Option<Self> {
        NonNull::new(state).map(|state| Self { state })
    }

    pub fn is_verbose(&self) -> bool {
        unsafe { (*self.state.as_ptr()).verbose }
    }

    pub fn is_analyze(&self) -> bool {
        unsafe { (*self.state.as_ptr()).analyze }
    }

    pub fn add_text<S: AsRef<str>>(&mut self, key: &str, value: S) {
        unsafe {
            pg_sys::ExplainPropertyText(
                key.as_pg_cstr(),
                value.as_ref().as_pg_cstr(),
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_integer(&mut self, key: &str, value: i64, unit: Option<&str>) {
        unsafe {
            pg_sys::ExplainPropertyInteger(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_unsigned_integer(&mut self, key: &str, value: u64, unit: Option<&str>) {
        unsafe {
            pg_sys::ExplainPropertyUInteger(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_float(&mut self, key: &str, value: f64, unit: Option<&str>, ndigits: i32) {
        unsafe {
            pg_sys::ExplainPropertyFloat(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                ndigits,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_bool(&mut self, key: &str, value: bool) {
        unsafe {
            pg_sys::ExplainPropertyBool(key.as_pg_cstr(), value, self.state.as_ptr());
        }
    }

    pub fn add_list(&mut self, key: &str, values: &mut pgrx::list::List<*mut std::ffi::c_char>) {
        unsafe {
            pg_sys::ExplainPropertyList(key.as_pg_cstr(), values.as_mut_ptr(), self.state.as_ptr())
        }
    }
}
