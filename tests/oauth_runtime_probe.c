#include "postgres.h"

#include <stdint.h>
#include <string.h>

#include "fmgr.h"
#include "libpq/oauth.h"
#include "utils/builtins.h"

PG_MODULE_MAGIC;

PG_FUNCTION_INFO_V1(pggomtm_abi_runtime_probe);

#define ABI_RUNTIME_PANIC_SENTINEL ((void *) (uintptr_t) 1)
#define ABI_RUNTIME_ERROR_SENTINEL ((void *) (uintptr_t) 2)

static bool
callback_table_is_loadable(const OAuthValidatorCallbacks *callbacks)
{
	return callbacks != NULL &&
		callbacks->magic == PG_OAUTH_VALIDATOR_MAGIC &&
		callbacks->validate_cb != NULL;
}

static void
expect_startup_error(const OAuthValidatorCallbacks *callbacks,
				 void *sentinel, const char *scenario)
{
	ValidatorModuleState state = {
		.sversion = PG_VERSION_NUM,
		.private_data = sentinel,
	};
	volatile bool rejected = false;

	PG_TRY();
	{
		callbacks->startup_cb(&state);
	}
	PG_CATCH();
	{
		FlushErrorState();
		rejected = true;
	}
	PG_END_TRY();

	if (!rejected || state.private_data != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate did not fail closed for %s",
						scenario)));
}

static void
expect_shutdown_error(const OAuthValidatorCallbacks *callbacks,
				  void *sentinel, const char *scenario)
{
	ValidatorModuleState state = {
		.sversion = PG_VERSION_NUM,
		.private_data = sentinel,
	};
	volatile bool rejected = false;

	PG_TRY();
	{
		callbacks->shutdown_cb(&state);
	}
	PG_CATCH();
	{
		FlushErrorState();
		rejected = true;
	}
	PG_END_TRY();

	if (!rejected || state.private_data != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate did not fail closed for %s",
						scenario)));
}

static void
expect_validate_error(const OAuthValidatorCallbacks *callbacks,
				  ValidatorModuleState *state)
{
	ValidatorModuleResult result = {
		.authorized = true,
		.authn_id = (char *) 1,
	};
	volatile bool rejected = false;

	PG_TRY();
	{
		(void) callbacks->validate_cb(state,
								  "pggomtm-abi-error",
								  "gomtm_candidate_ordinary",
								  &result);
	}
	PG_CATCH();
	{
		FlushErrorState();
		rejected = true;
	}
	PG_END_TRY();

	if (!rejected || result.authorized || result.authn_id != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate allowed PostgreSQL ERROR across validate")));
}

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
	ValidatorModuleState wrong_major_state = {
		.sversion = PG_VERSION_NUM - 10000,
		.private_data = NULL,
	};
	volatile bool wrong_major_rejected = false;
	volatile bool null_startup_rejected = false;
	OAuthValidatorCallbacks invalid_callbacks;

	validator_init = (OAuthValidatorModuleInit)
		load_external_function(library_name,
						   "_PG_oauth_validator_module_init",
						   false,
						   NULL);
	if (validator_init == NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate could not resolve validator init")));

	callbacks = validator_init();
	if (!callback_table_is_loadable(callbacks) ||
		callbacks->startup_cb == NULL || callbacks->shutdown_cb == NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate received an invalid callback table")));

	invalid_callbacks = *callbacks;
	invalid_callbacks.magic ^= 1;
	if (callback_table_is_loadable(&invalid_callbacks))
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted an invalid callback magic")));

	invalid_callbacks = *callbacks;
	invalid_callbacks.validate_cb = NULL;
	if (callback_table_is_loadable(&invalid_callbacks))
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted a missing validate callback")));

	invalid_callbacks = *callbacks;
	invalid_callbacks.startup_cb = NULL;
	invalid_callbacks.shutdown_cb = NULL;
	if (!callback_table_is_loadable(&invalid_callbacks))
		ereport(ERROR,
				(errmsg("pggomtm ABI gate rejected optional callbacks")));

	PG_TRY();
	{
		callbacks->startup_cb(NULL);
	}
	PG_CATCH();
	{
		FlushErrorState();
		null_startup_rejected = true;
	}
	PG_END_TRY();
	if (!null_startup_rejected)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted a null startup state")));

	expect_startup_error(callbacks, ABI_RUNTIME_PANIC_SENTINEL,
					 "startup panic");
	expect_startup_error(callbacks, ABI_RUNTIME_ERROR_SENTINEL,
					 "startup PostgreSQL ERROR");

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

	result.authorized = true;
	result.authn_id = (char *) 1;
	if (callbacks->validate_cb(NULL,
							"header.payload.signature",
							"gomtm_candidate_ordinary",
							&result) ||
		result.authorized || result.authn_id != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted a null validate state")));

	result.authorized = true;
	result.authn_id = (char *) 1;
	if (callbacks->validate_cb(&state, NULL,
							"gomtm_candidate_ordinary", &result) ||
		result.authorized || result.authn_id != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted a null token")));

	result.authorized = true;
	result.authn_id = (char *) 1;
	if (callbacks->validate_cb(&state, "header.payload.signature", NULL,
							&result) ||
		result.authorized || result.authn_id != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted a null role")));

	if (callbacks->validate_cb(&state,
							"header.payload.signature",
							"gomtm_candidate_ordinary",
							NULL))
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted a null validate result")));

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

	expect_validate_error(callbacks, &state);

	PG_TRY();
	{
		callbacks->startup_cb(&wrong_major_state);
	}
	PG_CATCH();
	{
		FlushErrorState();
		wrong_major_rejected = true;
	}
	PG_END_TRY();

	if (!wrong_major_rejected || wrong_major_state.private_data != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate accepted an unsupported PostgreSQL major")));

	callbacks->shutdown_cb(&state);
	if (state.private_data != NULL)
		ereport(ERROR,
				(errmsg("pggomtm ABI gate shutdown left initialized state")));

	callbacks->shutdown_cb(NULL);
	expect_shutdown_error(callbacks, ABI_RUNTIME_PANIC_SENTINEL,
					  "shutdown panic");
	expect_shutdown_error(callbacks, ABI_RUNTIME_ERROR_SENTINEL,
					  "shutdown PostgreSQL ERROR");

	pfree(library_name);
	PG_RETURN_BOOL(true);
}
