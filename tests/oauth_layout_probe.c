#include "postgres.h"

#include <stddef.h>

#include "libpq/oauth.h"

_Static_assert(PG_VERSION_NUM == 180004,
               "pggomtm layout probe requires PostgreSQL 18.4 headers");
_Static_assert(PG_OAUTH_VALIDATOR_MAGIC == 0x20250220,
               "unexpected PostgreSQL OAuth validator magic");

_Static_assert(sizeof(ValidatorModuleState) == 16,
               "unexpected ValidatorModuleState size");
_Static_assert(offsetof(ValidatorModuleState, sversion) == 0,
               "unexpected ValidatorModuleState.sversion offset");
_Static_assert(offsetof(ValidatorModuleState, private_data) == 8,
               "unexpected ValidatorModuleState.private_data offset");

_Static_assert(sizeof(ValidatorModuleResult) == 16,
               "unexpected ValidatorModuleResult size");
_Static_assert(offsetof(ValidatorModuleResult, authorized) == 0,
               "unexpected ValidatorModuleResult.authorized offset");
_Static_assert(offsetof(ValidatorModuleResult, authn_id) == 8,
               "unexpected ValidatorModuleResult.authn_id offset");

_Static_assert(sizeof(OAuthValidatorCallbacks) == 32,
               "unexpected OAuthValidatorCallbacks size");
_Static_assert(offsetof(OAuthValidatorCallbacks, magic) == 0,
               "unexpected OAuthValidatorCallbacks.magic offset");
_Static_assert(offsetof(OAuthValidatorCallbacks, startup_cb) == 8,
               "unexpected OAuthValidatorCallbacks.startup_cb offset");
_Static_assert(offsetof(OAuthValidatorCallbacks, shutdown_cb) == 16,
               "unexpected OAuthValidatorCallbacks.shutdown_cb offset");
_Static_assert(offsetof(OAuthValidatorCallbacks, validate_cb) == 24,
               "unexpected OAuthValidatorCallbacks.validate_cb offset");

int
main(void)
{
    return 0;
}
