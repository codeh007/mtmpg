#include "postgres.h"

#include <string.h>

#include "fmgr.h"
#include "libpq/oauth.h"
#include "utils/builtins.h"

PG_MODULE_MAGIC;

PG_FUNCTION_INFO_V1(pggomtm_abi_runtime_probe);

Datum
pggomtm_abi_runtime_probe(PG_FUNCTION_ARGS)
{
	char *library_name = text_to_cstring(PG_GETARG_TEXT_PP(0));
	OAuthValidatorModuleInit validator_init;
	const OAuthValidatorCallbacks *callbacks;
	ValidatorModuleState state = {
		.sversion = PG_VERSION_NUM,
		.private_data = NULL,
	};
	ValidatorModuleResult result = {0};
	ValidatorModuleState wrong_minor_state = {
		.sversion = PG_VERSION_NUM - 1,
		.private_data = NULL,
	};
	volatile bool wrong_minor_rejected = false;

	validator_init = (OAuthValidatorModuleInit)
		load_external_function(library_name,
						   "_PG_oauth_validator_module_init",
						   false,
						   NULL);
	if (validator_init == NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate could not resolve validator init")));

	callbacks = validator_init();
	if (callbacks == NULL || callbacks->magic != PG_OAUTH_VALIDATOR_MAGIC ||
		callbacks->startup_cb == NULL || callbacks->shutdown_cb == NULL ||
		callbacks->validate_cb == NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate received an invalid callback table")));

	callbacks->startup_cb(&state);
	if (state.private_data == NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate startup did not initialize state")));

	if (!callbacks->validate_cb(&state,
							 "header.payload.signature",
							 "gomtm_candidate_ordinary",
							 &result) ||
		result.authorized || result.authn_id != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate did not fail closed by default")));

	memset(&result, 0, sizeof(result));
	if (!callbacks->validate_cb(&state,
							 "pggomtm-abi-allocator",
							 "gomtm_candidate_ordinary",
							 &result) ||
		result.authorized || result.authn_id == NULL ||
		strcmp(result.authn_id, "pggomtm-abi-allocator") != 0)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate did not return its allocator probe")));
	pfree(result.authn_id);

	result.authorized = true;
	result.authn_id = (char *) 1;
	if (callbacks->validate_cb(&state,
							"pggomtm-abi-panic",
							"gomtm_candidate_ordinary",
							&result) ||
		result.authorized || result.authn_id != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate allowed a panic across the callback boundary")));

	PG_TRY();
	{
		callbacks->startup_cb(&wrong_minor_state);
	}
	PG_CATCH();
	{
		FlushErrorState();
		wrong_minor_rejected = true;
	}
	PG_END_TRY();

	if (!wrong_minor_rejected || wrong_minor_state.private_data != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted an unsupported PostgreSQL minor")));

	callbacks->shutdown_cb(&state);
	if (state.private_data != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate shutdown left initialized state")));

	pfree(library_name);
	PG_RETURN_BOOL(true);
}
