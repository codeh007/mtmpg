\set ON_ERROR_STOP 1

CREATE FUNCTION pggomtm_abi_runtime_probe(text)
RETURNS boolean
AS '$libdir/pggomtm_abi_runtime_probe', 'pggomtm_abi_runtime_probe'
LANGUAGE C STRICT;

SELECT pggomtm_abi_runtime_probe('$libdir/pggomtm_abi_gate');

DROP FUNCTION pggomtm_abi_runtime_probe(text);
