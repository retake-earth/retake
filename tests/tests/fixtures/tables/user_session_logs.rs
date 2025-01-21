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

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use soa_derive::StructOfArray;
use sqlx::FromRow;
use time::Date;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Serialize, Deserialize)]
pub struct UserSessionLogsTable {
    pub id: i32,
    pub event_date: Option<Date>,
    pub user_id: Option<i32>,
    pub event_name: Option<String>,
    pub session_duration: Option<i32>,
    pub page_views: Option<i32>,
    pub revenue: Option<BigDecimal>,
}

impl UserSessionLogsTable {
    pub fn setup_parquet() -> String {
        USER_SESSION_LOGS_TABLE_SETUP.replace("{}", "parquet")
    }

    pub fn setup_heap() -> String {
        USER_SESSION_LOGS_TABLE_SETUP.replace("{}", "heap")
    }
}

static USER_SESSION_LOGS_TABLE_SETUP: &str = r#"
CREATE TABLE user_session_logs (
    id SERIAL PRIMARY KEY,
    event_date DATE,
    user_id INT,
    event_name VARCHAR(50),
    session_duration INT,
    page_views INT,
    revenue DECIMAL(10, 2)
);

INSERT INTO user_session_logs
(event_date, user_id, event_name, session_duration, page_views, revenue)
VALUES
('2024-01-01', 1, 'Login', 300, 5, 20.00),
('2024-01-02', 2, 'Purchase', 450, 8, 150.50),
('2024-01-03', 3, 'Logout', 100, 2, 0.00),
('2024-01-04', 4, 'Signup', 200, 3, 0.00),
('2024-01-05', 5, 'ViewProduct', 350, 6, 30.75),
('2024-01-06', 1, 'AddToCart', 500, 10, 75.00),
('2024-01-07', 2, 'RemoveFromCart', 250, 4, 0.00),
('2024-01-08', 3, 'Checkout', 400, 7, 200.25),
('2024-01-09', 4, 'Payment', 550, 11, 300.00),
('2024-01-10', 5, 'Review', 600, 9, 50.00),
('2024-01-11', 6, 'Login', 320, 3, 0.00),
('2024-01-12', 7, 'Purchase', 480, 7, 125.30),
('2024-01-13', 8, 'Logout', 150, 2, 0.00),
('2024-01-14', 9, 'Signup', 240, 4, 0.00),
('2024-01-15', 10, 'ViewProduct', 360, 5, 45.00),
('2024-01-16', 6, 'AddToCart', 510, 9, 80.00),
('2024-01-17', 7, 'RemoveFromCart', 270, 3, 0.00),
('2024-01-18', 8, 'Checkout', 430, 6, 175.50),
('2024-01-19', 9, 'Payment', 560, 12, 250.00),
('2024-01-20', 10, 'Review', 610, 10, 60.00);
"#;
