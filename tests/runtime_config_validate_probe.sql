\set ON_ERROR_STOP 1

CREATE FUNCTION pggomtm_config_validate_probe(text, text, text, boolean)
RETURNS boolean
AS '$libdir/pggomtm_abi_runtime_probe', 'pggomtm_config_validate_probe'
LANGUAGE C STRICT;

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-valid.jwt'),
  'gomtm_candidate_ordinary',
  true
);

SELECT pggomtm_config_validate_probe(
  '$libdir/pggomtm_config_gate',
  pg_read_file('/tmp/pggomtm-config-tampered.jwt'),
  'gomtm_candidate_ordinary',
  false
);

DROP FUNCTION pggomtm_config_validate_probe(text, text, text, boolean);
