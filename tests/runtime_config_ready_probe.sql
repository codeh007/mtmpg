\set ON_ERROR_STOP 1

SELECT pggomtm_config_snapshot_probe('$libdir/pggomtm');

DROP FUNCTION pggomtm_config_missing_probe(text);
DROP FUNCTION pggomtm_config_snapshot_probe(text);
