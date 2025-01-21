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

use crate::schema::SearchFieldId;
use tantivy::schema::OwnedValue;
use tantivy::TantivyDocument;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    pub doc: TantivyDocument,
}

impl SearchDocument {
    #[inline(always)]
    pub fn insert(&mut self, SearchFieldId(key): SearchFieldId, value: OwnedValue) {
        self.doc.add_field_value(key, &value)
    }
}

impl From<SearchDocument> for TantivyDocument {
    fn from(value: SearchDocument) -> Self {
        value.doc
    }
}
