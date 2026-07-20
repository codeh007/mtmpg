\set ON_ERROR_STOP on

CREATE ROLE ordinary LOGIN;
CREATE ROLE business_admin LOGIN;
CREATE ROLE database_developer LOGIN;
CREATE DATABASE gomtm;

\connect gomtm

CREATE SCHEMA app;
CREATE TABLE app.executor_probe (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    value text NOT NULL UNIQUE
);

CREATE PROCEDURE app.record_probe(input text)
LANGUAGE sql
AS $$
    INSERT INTO app.executor_probe(value) VALUES (input);
$$;

CREATE PROCEDURE app.fail_after_insert()
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO app.executor_probe(value) VALUES ('failed');
    RAISE EXCEPTION 'synthetic integration failure';
END;
$$;

GRANT CONNECT ON DATABASE gomtm TO ordinary, business_admin, database_developer;
GRANT USAGE ON SCHEMA app TO ordinary, business_admin, database_developer;
GRANT SELECT ON app.executor_probe TO ordinary;
GRANT SELECT, INSERT ON app.executor_probe TO business_admin;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA app TO database_developer;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA app TO business_admin, database_developer;
GRANT EXECUTE ON PROCEDURE app.record_probe(text) TO business_admin;
GRANT EXECUTE ON PROCEDURE app.fail_after_insert() TO business_admin;
