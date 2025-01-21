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

pub mod benchlogs;

use serde::de::DeserializeOwned;
use std::path::PathBuf;

// A trait that abstracts the source of paths
pub trait PathSource {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>>;
}

// Implement PathSource for a glob pattern
impl PathSource for &String {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>> {
        // Use a globbing library to interpret the string as a glob pattern
        // and return an iterator over the matching paths
        let glob_pattern = glob::glob(self).expect("glob pattern should be valid");
        Box::new(glob_pattern.filter_map(Result::ok))
    }
}

impl PathSource for &str {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>> {
        // Use a globbing library to interpret the string as a glob pattern
        // and return an iterator over the matching paths
        let glob_pattern = glob::glob(self).expect("glob pattern should be valid");
        Box::new(glob_pattern.filter_map(Result::ok))
    }
}

// Implement PathSource for a vector of paths
impl PathSource for Vec<PathBuf> {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>> {
        // Simply iterate over the paths in the vector
        Box::new(self.clone().into_iter())
    }
}

#[allow(clippy::type_complexity)]
pub trait PathReader: DeserializeOwned + 'static {
    type Error;

    #[allow(dead_code)]
    fn read_all<S: PathSource>(
        source: S,
    ) -> Result<Box<dyn Iterator<Item = Result<Self, Self::Error>>>, Self::Error>;

    #[allow(dead_code)]
    fn read_ok<S: PathSource>(
        source: S,
        offset: usize,
        limit: usize,
    ) -> Result<Box<dyn Iterator<Item = Self>>, Self::Error> {
        Ok(Box::new(
            Self::read_all(source)?
                .filter_map(|r| r.ok())
                .skip(offset)
                .take(limit),
        ))
    }

    #[allow(dead_code)]
    fn read_all_ok<S: PathSource>(
        source: S,
    ) -> Result<Box<dyn Iterator<Item = Self>>, Self::Error> {
        Ok(Box::new(Self::read_all(source)?.filter_map(|r| r.ok())))
    }
}
