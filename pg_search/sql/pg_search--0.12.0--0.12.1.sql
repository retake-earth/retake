ALTER TYPE TestTable ADD VALUE 'Customers';

-- pg_search/src/api/index.rs:580
-- pg_search::api::index::term
CREATE  FUNCTION "term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"value" anyenum /* pg_search::schema::anyenum::AnyEnum */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_anyenum_wrapper';

DROP PROCEDURE IF EXISTS paradedb.create_bm25(index_name text, table_name text, key_field text, schema_name text, text_fields jsonb, numeric_fields jsonb, boolean_fields jsonb, json_fields jsonb, range_fields jsonb, datetime_fields jsonb, predicates text);
DROP PROCEDURE IF EXISTS paradedb.drop_bm25(index_name text, schema_name text);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/config.rs:84
-- pg_search::api::config::format_create_index
CREATE OR REPLACE FUNCTION paradedb.format_create_index(
    index_name text DEFAULT '',
    table_name text DEFAULT '',
    key_field text DEFAULT '',
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields jsonb DEFAULT '{}',
    numeric_fields jsonb DEFAULT '{}',
    boolean_fields jsonb DEFAULT '{}',
    json_fields jsonb DEFAULT '{}',
    range_fields jsonb DEFAULT '{}',
    datetime_fields jsonb DEFAULT '{}',
    predicates text DEFAULT ''
)
RETURNS text
LANGUAGE c AS 'MODULE_PATHNAME', 'format_create_index_wrapper';