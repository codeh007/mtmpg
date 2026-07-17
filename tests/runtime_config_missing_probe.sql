\set ON_ERROR_STOP 1

CREATE FUNCTION pggomtm_config_missing_probe(text)
RETURNS boolean
AS '$libdir/pggomtm_abi_runtime_probe', 'pggomtm_config_missing_probe'
LANGUAGE C STRICT;

CREATE FUNCTION pggomtm_config_snapshot_probe(text)
RETURNS boolean
AS '$libdir/pggomtm_abi_runtime_probe', 'pggomtm_config_snapshot_probe'
LANGUAGE C STRICT;

SELECT pggomtm_config_missing_probe('$libdir/pggomtm_config_gate');
