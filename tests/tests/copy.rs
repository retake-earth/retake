mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn test_copy_to_table(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test_copy_to_table;
        CREATE TABLE test_copy_to_table (id SERIAL PRIMARY KEY, name TEXT);
        CREATE INDEX idx_test_copy_to_table ON test_copy_to_table USING bm25(id, name) WITH (key_field = 'id');
    "#.execute(&mut conn);

    let mut copyin = conn
        .copy_in_raw("COPY test_copy_to_table(name) FROM STDIN")
        .await
        .unwrap();
    copyin.send("one\ntwo\nthree".as_bytes()).await.unwrap();
    copyin.finish().await.unwrap();

    let (count,) = "SELECT COUNT(*) FROM test_copy_to_table WHERE name @@@ 'one'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
