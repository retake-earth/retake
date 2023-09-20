DO $$
BEGIN
   IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='${POSTGRES_USER}') THEN
      CREATE ROLE ${POSTGRES_USER} WITH SUPERUSER LOGIN;
   END IF;
END
$$;

DO $$
BEGIN
   IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='${POSTGRES_USER}') THEN
      CREATE ROLE ${POSTGRES_USER} WITH PASSWORD '${POSTGRES_PASSWORD}' SUPERUSER LOGIN;
   END IF;
END
$$;

DO $$
BEGIN
   IF NOT EXISTS (SELECT 1 FROM pg_database WHERE datname='${POSTGRES_DB}') THEN
      CREATE DATABASE ${POSTGRES_DB} WITH OWNER = ${POSTGRES_USER};
   END IF;
END
$$;
