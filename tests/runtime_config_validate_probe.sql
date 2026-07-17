\set ON_ERROR_STOP 1

CREATE FUNCTION pggomtm_config_validate_probe(text, text, text, boolean)
RETURNS boolean
AS '$libdir/pggomtm_abi_runtime_probe', 'pggomtm_config_validate_probe'
LANGUAGE C STRICT;

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/oauth-ordinary.jwt'),
  'gomtm_candidate_ordinary',
  true
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/tampered.jwt'),
  'gomtm_candidate_ordinary',
  false
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/oauth-ordinary.jwt'),
  'gomtm_candidate_business_admin',
  false
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/oauth-ordinary.jwt'),
  'gomtm_candidate_database_developer',
  false
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/oauth-ordinary.jwt'),
  'gomtm_test_auth_runtime',
  false
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/oauth-ordinary.jwt'),
  'gomtm_test_migration_owner',
  false
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/oauth-ordinary.jwt'),
  'gomtm_platform_admin',
  false
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-fixtures/oauth-ordinary.jwt'),
  'gomtm_candidate_unknown',
  false
);

DROP FUNCTION pggomtm_config_validate_probe(text, text, text, boolean);
