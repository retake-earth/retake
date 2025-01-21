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

#![allow(unused_imports)]
use anyhow::{anyhow, bail, Result};
use cmd_lib::*;
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, Executor, PgConnection};
use std::fs::File;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;
use std::{env, io};
use tempfile::TempDir;

fn download_and_verify(url: &str, checksum: &str, filename: &str) -> Result<()> {
    if Path::new(filename).exists() {
        run_cmd!(echo "Dataset '$filename' already exists, verifying checksum...")?;
        if run_cmd!(echo "$checksum $filename" | md5sum -c --status).is_ok() {
            run_cmd!(echo "Dataset '$filename' already exists and is verified, skipping download...")?;
            return Ok(());
        } else {
            run_cmd!(echo "Checksum mismatch. Re-downloading '$filename'...")?;
        }
    }

    run_cmd!(echo "Downloading $filename dataset...")?;
    run_cmd!(wget --no-verbose --no-check-certificate --continue -O $filename $url)?;
    Ok(())
}

pub async fn bench_hits(url: &str, workload: &str, full: bool) -> Result<()> {
    let _os = run_fun!(uname)?;

    println!("\n*********************************************************************************");
    println!("* Benchmarking Analytics against ClickBench");
    println!("*********************************************************************************\n");

    let root_file_path = PathBuf::from_str("/tmp")?;
    let single_file_path = root_file_path.join("hits.parquet").display().to_string();
    let partitioned_dir_path = root_file_path.join("partitioned").display().to_string();

    let single_url = if full {
        "https://paradedb-benchmarks.s3.amazonaws.com/hits.parquet"
    } else {
        "https://paradedb-benchmarks.s3.amazonaws.com/hits_5m_rows.parquet"
    };

    // Including the {} template in the url for xargs to replace.
    let partitioned_url = if full {
        "https://paradedb-benchmarks.s3.amazonaws.com/partitioned/hits_{}.parquet"
    } else {
        "https://paradedb-benchmarks.s3.amazonaws.com/partitioned_5m_rows/hits_{}.parquet"
    };

    if workload == "single" {
        run_cmd!(echo "Using ClickBench's single Parquet file for benchmarking...")?;
        download_and_verify(
            single_url,
            "5182ed3b0ad35137db38faac395d7f55",
            &single_file_path,
        )?;
    } else if workload == "partitioned" {
        run_cmd!(echo "Using ClickBench's one hundred partitioned Parquet files for benchmarking...")?;
        run_cmd!(mkdir -p $partitioned_dir_path)?;
        run_cmd!(
           seq 0 99
          | xargs "-P100" "-I{}" wget --no-verbose --no-check-certificate --directory-prefix $partitioned_dir_path
            --continue $partitioned_url
        )?;
    } else {
        eprintln!("Invalid workload: {}", workload);
        std::process::exit(1);
    }

    run_cmd!(echo "\nLoading dataset...")?;
    env::set_var("PGPASSWORD", "postgres");

    let conn_opts = &PgConnectOptions::from_str(url)?;
    let mut conn = PgConnection::connect_with(conn_opts).await?;

    let file_path = if workload == "single" {
        single_file_path
    } else {
        format!("{partitioned_dir_path}/*.parquet")
    };

    println!("Creating foreign table from '{}'", file_path);

    conn.execute(create_query_sql(&file_path).as_str())
        .await
        .unwrap();

    run_cmd!(echo "\nRunning queries...")?;
    run_queries(&mut conn).await?;

    Ok(())
}

async fn run_queries(conn: &mut PgConnection) -> Result<()> {
    let tries = 3;
    let os = run_fun!(uname)?;
    env::set_var("PGPASSWORD", "postgres");

    for query in run_query_sql().lines() {
        if query.trim().is_empty() {
            continue;
        }

        // Clear the cache on Linux systems
        if os.trim() == "Linux" {
            run_cmd!(sync)?;
            run_cmd!(echo 3 | sudo tee /proc/sys/vm/drop_caches)?;
        }

        println!("{}", query);

        for _ in 1..=tries {
            // Start timing
            let start = Instant::now();

            // Execute the query
            conn.execute(query).await.unwrap();

            // Calculate elapsed time
            let duration = start.elapsed();

            // Print the elapsed time in milliseconds
            println!("Time: {:.3} ms", duration.as_secs_f64() * 1000.0);
        }
    }

    Ok(())
}

// TODO: Convert the ClickBench test from pg_analytics to pg_search once fast facets are implemented.
fn create_query_sql(file_path: &str) -> String {
    format!(
        r#"
CREATE EXTENSION IF NOT EXISTS pg_analytics;
DROP FOREIGN DATA WRAPPER IF EXISTS parquet_wrapper CASCADE;
CREATE FOREIGN DATA WRAPPER parquet_wrapper
    HANDLER parquet_fdw_handler
    VALIDATOR parquet_fdw_validator;
CREATE SERVER parquet_server
    FOREIGN DATA WRAPPER parquet_wrapper;
CREATE FOREIGN TABLE IF NOT EXISTS hits
(
    WatchID BIGINT NOT NULL,
    JavaEnable SMALLINT NOT NULL,
    Title TEXT NOT NULL,
    GoodEvent SMALLINT NOT NULL,
    EventTime BIGINT NOT NULL,
    EventDate INTEGER NOT NULL,
    CounterID INTEGER NOT NULL,
    ClientIP INTEGER NOT NULL,
    RegionID INTEGER NOT NULL,
    UserID BIGINT NOT NULL,
    CounterClass SMALLINT NOT NULL,
    OS SMALLINT NOT NULL,
    UserAgent SMALLINT NOT NULL,
    URL TEXT NOT NULL,
    Referer TEXT NOT NULL,
    IsRefresh SMALLINT NOT NULL,
    RefererCategoryID SMALLINT NOT NULL,
    RefererRegionID INTEGER NOT NULL,
    URLCategoryID SMALLINT NOT NULL,
    URLRegionID INTEGER NOT NULL,
    ResolutionWidth SMALLINT NOT NULL,
    ResolutionHeight SMALLINT NOT NULL,
    ResolutionDepth SMALLINT NOT NULL,
    FlashMajor SMALLINT NOT NULL,
    FlashMinor SMALLINT NOT NULL,
    FlashMinor2 TEXT NOT NULL,
    NetMajor SMALLINT NOT NULL,
    NetMinor SMALLINT NOT NULL,
    UserAgentMajor SMALLINT NOT NULL,
    UserAgentMinor VARCHAR(255) NOT NULL,
    CookieEnable SMALLINT NOT NULL,
    JavascriptEnable SMALLINT NOT NULL,
    IsMobile SMALLINT NOT NULL,
    MobilePhone SMALLINT NOT NULL,
    MobilePhoneModel TEXT NOT NULL,
    Params TEXT NOT NULL,
    IPNetworkID INTEGER NOT NULL,
    TraficSourceID SMALLINT NOT NULL,
    SearchEngineID SMALLINT NOT NULL,
    SearchPhrase TEXT NOT NULL,
    AdvEngineID SMALLINT NOT NULL,
    IsArtifical SMALLINT NOT NULL,
    WindowClientWidth SMALLINT NOT NULL,
    WindowClientHeight SMALLINT NOT NULL,
    ClientTimeZone SMALLINT NOT NULL,
    ClientEventTime BIGINT NOT NULL,
    SilverlightVersion1 SMALLINT NOT NULL,
    SilverlightVersion2 SMALLINT NOT NULL,
    SilverlightVersion3 INTEGER NOT NULL,
    SilverlightVersion4 SMALLINT NOT NULL,
    PageCharset TEXT NOT NULL,
    CodeVersion INTEGER NOT NULL,
    IsLink SMALLINT NOT NULL,
    IsDownload SMALLINT NOT NULL,
    IsNotBounce SMALLINT NOT NULL,
    FUniqID BIGINT NOT NULL,
    OriginalURL TEXT NOT NULL,
    HID INTEGER NOT NULL,
    IsOldCounter SMALLINT NOT NULL,
    IsEvent SMALLINT NOT NULL,
    IsParameter SMALLINT NOT NULL,
    DontCountHits SMALLINT NOT NULL,
    WithHash SMALLINT NOT NULL,
    HitColor CHAR NOT NULL,
    LocalEventTime BIGINT NOT NULL,
    Age SMALLINT NOT NULL,
    Sex SMALLINT NOT NULL,
    Income SMALLINT NOT NULL,
    Interests SMALLINT NOT NULL,
    Robotness SMALLINT NOT NULL,
    RemoteIP INTEGER NOT NULL,
    WindowName INTEGER NOT NULL,
    OpenerName INTEGER NOT NULL,
    HistoryLength SMALLINT NOT NULL,
    BrowserLanguage TEXT NOT NULL,
    BrowserCountry TEXT NOT NULL,
    SocialNetwork TEXT NOT NULL,
    SocialAction TEXT NOT NULL,
    HTTPError SMALLINT NOT NULL,
    SendTiming INTEGER NOT NULL,
    DNSTiming INTEGER NOT NULL,
    ConnectTiming INTEGER NOT NULL,
    ResponseStartTiming INTEGER NOT NULL,
    ResponseEndTiming INTEGER NOT NULL,
    FetchTiming INTEGER NOT NULL,
    SocialSourceNetworkID SMALLINT NOT NULL,
    SocialSourcePage TEXT NOT NULL,
    ParamPrice BIGINT NOT NULL,
    ParamOrderID TEXT NOT NULL,
    ParamCurrency TEXT NOT NULL,
    ParamCurrencyID SMALLINT NOT NULL,
    OpenstatServiceName TEXT NOT NULL,
    OpenstatCampaignID TEXT NOT NULL,
    OpenstatAdID TEXT NOT NULL,
    OpenstatSourceID TEXT NOT NULL,
    UTMSource TEXT NOT NULL,
    UTMMedium TEXT NOT NULL,
    UTMCampaign TEXT NOT NULL,
    UTMContent TEXT NOT NULL,
    UTMTerm TEXT NOT NULL,
    FromTag TEXT NOT NULL,
    HasGCLID SMALLINT NOT NULL,
    RefererHash BIGINT NOT NULL,
    URLHash BIGINT NOT NULL
)
SERVER parquet_server
OPTIONS (files '{file_path}');
"#
    )
}

fn run_query_sql() -> String {
    r#"
SELECT COUNT(*) FROM hits;
SELECT COUNT(*) FROM hits WHERE AdvEngineID <> 0;
SELECT SUM(AdvEngineID), COUNT(*), AVG(ResolutionWidth) FROM hits;
SELECT AVG(UserID) FROM hits;
SELECT COUNT(DISTINCT UserID) FROM hits;
SELECT COUNT(DISTINCT SearchPhrase) FROM hits;
SELECT DATE '1970-01-01' + MIN(EventDate) * INTERVAL '1 day' AS min, DATE '1970-01-01' + MAX(EventDate) * INTERVAL '1 day' AS max FROM hits;
SELECT AdvEngineID, COUNT(*) FROM hits WHERE AdvEngineID <> 0 GROUP BY AdvEngineID ORDER BY COUNT(*) DESC;
SELECT RegionID, COUNT(DISTINCT UserID) AS u FROM hits GROUP BY RegionID ORDER BY u DESC LIMIT 10;
SELECT RegionID, SUM(AdvEngineID), COUNT(*) AS c, AVG(ResolutionWidth), COUNT(DISTINCT UserID) FROM hits GROUP BY RegionID ORDER BY c DESC LIMIT 10;
SELECT MobilePhoneModel, COUNT(DISTINCT UserID) AS u FROM hits WHERE MobilePhoneModel <> '' GROUP BY MobilePhoneModel ORDER BY u DESC LIMIT 10;
SELECT MobilePhone, MobilePhoneModel, COUNT(DISTINCT UserID) AS u FROM hits WHERE MobilePhoneModel <> '' GROUP BY MobilePhone, MobilePhoneModel ORDER BY u DESC LIMIT 10;
SELECT SearchPhrase, COUNT(*) AS c FROM hits WHERE SearchPhrase <> '' GROUP BY SearchPhrase ORDER BY c DESC LIMIT 10;
SELECT SearchPhrase, COUNT(DISTINCT UserID) AS u FROM hits WHERE SearchPhrase <> '' GROUP BY SearchPhrase ORDER BY u DESC LIMIT 10;
SELECT SearchEngineID, SearchPhrase, COUNT(*) AS c FROM hits WHERE SearchPhrase <> '' GROUP BY SearchEngineID, SearchPhrase ORDER BY c DESC LIMIT 10;
SELECT UserID, COUNT(*) FROM hits GROUP BY UserID ORDER BY COUNT(*) DESC LIMIT 10;
SELECT UserID, SearchPhrase, COUNT(*) FROM hits GROUP BY UserID, SearchPhrase ORDER BY COUNT(*) DESC LIMIT 10;
SELECT UserID, SearchPhrase, COUNT(*) FROM hits GROUP BY UserID, SearchPhrase LIMIT 10;
SELECT UserID, extract(minute FROM CAST(to_timestamp(EventTime) AS TIMESTAMP)) AS m, SearchPhrase, COUNT(*) FROM hits GROUP BY UserID, m, SearchPhrase ORDER BY COUNT(*) DESC LIMIT 10;
SELECT UserID FROM hits WHERE UserID = 435090932899640449;
SELECT COUNT(*) FROM hits WHERE URL::VARCHAR LIKE '%google%';
SELECT SearchPhrase, MIN(URL), COUNT(*) AS c FROM hits WHERE URL::VARCHAR LIKE '%google%' AND SearchPhrase <> '' GROUP BY SearchPhrase ORDER BY c DESC LIMIT 10;
SELECT SearchPhrase, MIN(URL), MIN(Title), COUNT(*) AS c, COUNT(DISTINCT UserID) FROM hits WHERE Title::VARCHAR LIKE '%Google%' AND URL::VARCHAR NOT LIKE '%.google.%' AND SearchPhrase <> '' GROUP BY SearchPhrase ORDER BY c DESC LIMIT 10;
SELECT * FROM hits WHERE URL::VARCHAR LIKE '%google%' ORDER BY EventTime LIMIT 10;
SELECT SearchPhrase FROM hits WHERE SearchPhrase <> '' ORDER BY EventTime LIMIT 10;
SELECT SearchPhrase FROM hits WHERE SearchPhrase <> '' ORDER BY SearchPhrase LIMIT 10;
SELECT SearchPhrase FROM hits WHERE SearchPhrase <> '' ORDER BY EventTime, SearchPhrase LIMIT 10;
SELECT CounterID, AVG(length(URL::VARCHAR)) AS l, COUNT(*) AS c FROM hits WHERE URL <> '' GROUP BY CounterID HAVING COUNT(*) > 100000 ORDER BY l DESC LIMIT 25;
SELECT REGEXP_REPLACE(Referer::VARCHAR, '^https?://(?:www\.)?([^/]+)/.*$', '\1') AS k, AVG(length(Referer::VARCHAR)) AS l, COUNT(*) AS c, MIN(Referer) FROM hits WHERE Referer <> '' GROUP BY k HAVING COUNT(*) > 100000 ORDER BY l DESC LIMIT 25;
SELECT SUM(ResolutionWidth), SUM(ResolutionWidth + 1), SUM(ResolutionWidth + 2), SUM(ResolutionWidth + 3), SUM(ResolutionWidth + 4), SUM(ResolutionWidth + 5), SUM(ResolutionWidth + 6), SUM(ResolutionWidth + 7), SUM(ResolutionWidth + 8), SUM(ResolutionWidth + 9), SUM(ResolutionWidth + 10), SUM(ResolutionWidth + 11), SUM(ResolutionWidth + 12), SUM(ResolutionWidth + 13), SUM(ResolutionWidth + 14), SUM(ResolutionWidth + 15), SUM(ResolutionWidth + 16), SUM(ResolutionWidth + 17), SUM(ResolutionWidth + 18), SUM(ResolutionWidth + 19), SUM(ResolutionWidth + 20), SUM(ResolutionWidth + 21), SUM(ResolutionWidth + 22), SUM(ResolutionWidth + 23), SUM(ResolutionWidth + 24), SUM(ResolutionWidth + 25), SUM(ResolutionWidth + 26), SUM(ResolutionWidth + 27), SUM(ResolutionWidth + 28), SUM(ResolutionWidth + 29), SUM(ResolutionWidth + 30), SUM(ResolutionWidth + 31), SUM(ResolutionWidth + 32), SUM(ResolutionWidth + 33), SUM(ResolutionWidth + 34), SUM(ResolutionWidth + 35), SUM(ResolutionWidth + 36), SUM(ResolutionWidth + 37), SUM(ResolutionWidth + 38), SUM(ResolutionWidth + 39), SUM(ResolutionWidth + 40), SUM(ResolutionWidth + 41), SUM(ResolutionWidth + 42), SUM(ResolutionWidth + 43), SUM(ResolutionWidth + 44), SUM(ResolutionWidth + 45), SUM(ResolutionWidth + 46), SUM(ResolutionWidth + 47), SUM(ResolutionWidth + 48), SUM(ResolutionWidth + 49), SUM(ResolutionWidth + 50), SUM(ResolutionWidth + 51), SUM(ResolutionWidth + 52), SUM(ResolutionWidth + 53), SUM(ResolutionWidth + 54), SUM(ResolutionWidth + 55), SUM(ResolutionWidth + 56), SUM(ResolutionWidth + 57), SUM(ResolutionWidth + 58), SUM(ResolutionWidth + 59), SUM(ResolutionWidth + 60), SUM(ResolutionWidth + 61), SUM(ResolutionWidth + 62), SUM(ResolutionWidth + 63), SUM(ResolutionWidth + 64), SUM(ResolutionWidth + 65), SUM(ResolutionWidth + 66), SUM(ResolutionWidth + 67), SUM(ResolutionWidth + 68), SUM(ResolutionWidth + 69), SUM(ResolutionWidth + 70), SUM(ResolutionWidth + 71), SUM(ResolutionWidth + 72), SUM(ResolutionWidth + 73), SUM(ResolutionWidth + 74), SUM(ResolutionWidth + 75), SUM(ResolutionWidth + 76), SUM(ResolutionWidth + 77), SUM(ResolutionWidth + 78), SUM(ResolutionWidth + 79), SUM(ResolutionWidth + 80), SUM(ResolutionWidth + 81), SUM(ResolutionWidth + 82), SUM(ResolutionWidth + 83), SUM(ResolutionWidth + 84), SUM(ResolutionWidth + 85), SUM(ResolutionWidth + 86), SUM(ResolutionWidth + 87), SUM(ResolutionWidth + 88), SUM(ResolutionWidth + 89) FROM hits;
SELECT SearchEngineID, ClientIP, COUNT(*) AS c, SUM(IsRefresh), AVG(ResolutionWidth) FROM hits WHERE SearchPhrase <> '' GROUP BY SearchEngineID, ClientIP ORDER BY c DESC LIMIT 10;
SELECT WatchID, ClientIP, COUNT(*) AS c, SUM(IsRefresh), AVG(ResolutionWidth) FROM hits WHERE SearchPhrase <> '' GROUP BY WatchID, ClientIP ORDER BY c DESC LIMIT 10;
SELECT WatchID, ClientIP, COUNT(*) AS c, SUM(IsRefresh), AVG(ResolutionWidth) FROM hits GROUP BY WatchID, ClientIP ORDER BY c DESC LIMIT 10;
SELECT URL, COUNT(*) AS c FROM hits GROUP BY URL ORDER BY c DESC LIMIT 10;
SELECT 1, URL, COUNT(*) AS c FROM hits GROUP BY 1, URL ORDER BY c DESC LIMIT 10;
SELECT ClientIP, ClientIP - 1, ClientIP - 2, ClientIP - 3, COUNT(*) AS c FROM hits GROUP BY ClientIP, ClientIP - 1, ClientIP - 2, ClientIP - 3 ORDER BY c DESC LIMIT 10;
SELECT URL, COUNT(*) AS PageViews FROM hits WHERE CounterID = 62 AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') >= '2013-07-01' AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') <= '2013-07-31' AND DontCountHits = 0 AND IsRefresh = 0 AND URL <> '' GROUP BY URL ORDER BY PageViews DESC LIMIT 10;
SELECT Title, COUNT(*) AS PageViews FROM hits WHERE CounterID = 62 AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') >= '2013-07-01' AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') <= '2013-07-31' AND DontCountHits = 0 AND IsRefresh = 0 AND Title <> '' GROUP BY Title ORDER BY PageViews DESC LIMIT 10;
SELECT URL, COUNT(*) AS PageViews FROM hits WHERE CounterID = 62 AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') >= '2013-07-01' AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') <= '2013-07-31' AND IsRefresh = 0 AND IsLink <> 0 AND IsDownload = 0 GROUP BY URL ORDER BY PageViews DESC LIMIT 10 OFFSET 1000;
SELECT TraficSourceID, SearchEngineID, AdvEngineID, CASE WHEN (SearchEngineID = 0 AND AdvEngineID = 0) THEN Referer ELSE '' END AS Src, URL AS Dst, COUNT(*) AS PageViews FROM hits WHERE CounterID = 62 AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') >= '2013-07-01' AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') <= '2013-07-31' AND IsRefresh = 0 GROUP BY TraficSourceID, SearchEngineID, AdvEngineID, Src, Dst ORDER BY PageViews DESC LIMIT 10 OFFSET 1000;
SELECT URLHash, EventDate, COUNT(*) AS PageViews FROM hits WHERE CounterID = 62 AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') >= '2013-07-01' AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') <= '2013-07-31' AND IsRefresh = 0 AND TraficSourceID IN (-1, 6) AND RefererHash = 3594120000172545465 GROUP BY URLHash, EventDate ORDER BY PageViews DESC LIMIT 10 OFFSET 100;
SELECT WindowClientWidth, WindowClientHeight, COUNT(*) AS PageViews FROM hits WHERE CounterID = 62 AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') >= '2013-07-01' AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') <= '2013-07-31' AND IsRefresh = 0 AND DontCountHits = 0 AND URLHash = 2868770270353813622 GROUP BY WindowClientWidth, WindowClientHeight ORDER BY PageViews DESC LIMIT 10 OFFSET 10000;
SELECT DATE_TRUNC('minute', CAST(to_timestamp(EventTime) AS TIMESTAMP)) AS M, COUNT(*) AS PageViews FROM hits WHERE CounterID = 62 AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') >= '2013-07-14' AND (DATE '1970-01-01' + EventDate * INTERVAL '1 day') <= '2013-07-15' AND IsRefresh = 0 AND DontCountHits = 0 GROUP BY DATE_TRUNC('minute', CAST(to_timestamp(EventTime) AS TIMESTAMP)) ORDER BY DATE_TRUNC('minute', CAST(to_timestamp(EventTime) AS TIMESTAMP)) LIMIT 10 OFFSET 1000;
"#.into()
}
