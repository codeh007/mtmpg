#include <libpq-fe.h>

#include <stddef.h>
#include <stdint.h>

#define TYPE_MATCHES(expression, type) \
	_Generic((expression), type: 1, default: 0)

typedef PostgresPollingStatusType (*ExpectedOAuthAsync)(
	PGconn *, PGoauthBearerRequest *, int *);
typedef void (*ExpectedOAuthCleanup)(PGconn *, PGoauthBearerRequest *);

_Static_assert(sizeof(PGauthData) == sizeof(int),
			   "unexpected PGauthData representation");
_Static_assert(PQAUTHDATA_OAUTH_BEARER_TOKEN != PQAUTHDATA_PROMPT_OAUTH_DEVICE,
			   "OAuth Bearer auth data must have a distinct type");

_Static_assert(sizeof(PGoauthBearerRequest) == 48,
			   "unexpected PGoauthBearerRequest size");
_Static_assert(offsetof(PGoauthBearerRequest, openid_configuration) == 0,
			   "unexpected openid_configuration offset");
_Static_assert(offsetof(PGoauthBearerRequest, scope) == 8,
			   "unexpected scope offset");
_Static_assert(offsetof(PGoauthBearerRequest, async) == 16,
			   "unexpected async offset");
_Static_assert(offsetof(PGoauthBearerRequest, cleanup) == 24,
			   "unexpected cleanup offset");
_Static_assert(offsetof(PGoauthBearerRequest, token) == 32,
			   "unexpected token offset");
_Static_assert(offsetof(PGoauthBearerRequest, user) == 40,
			   "unexpected user offset");
_Static_assert(TYPE_MATCHES(((PGoauthBearerRequest *) 0)->async,
							ExpectedOAuthAsync),
			   "unexpected OAuth async callback signature");
_Static_assert(TYPE_MATCHES(((PGoauthBearerRequest *) 0)->cleanup,
							ExpectedOAuthCleanup),
			   "unexpected OAuth cleanup callback signature");

_Static_assert(TYPE_MATCHES(&PQsetAuthDataHook,
							void (*)(PQauthDataHook_type)),
			   "unexpected PQsetAuthDataHook signature");
_Static_assert(TYPE_MATCHES(&PQgetAuthDataHook,
							PQauthDataHook_type (*)(void)),
			   "unexpected PQgetAuthDataHook signature");
_Static_assert(TYPE_MATCHES(&PQconnectStartParams,
							PGconn *(*)(const char *const *,
									   const char *const *, int)),
			   "unexpected PQconnectStartParams signature");
_Static_assert(TYPE_MATCHES(&PQconnectPoll,
							PostgresPollingStatusType (*)(PGconn *)),
			   "unexpected PQconnectPoll signature");
_Static_assert(TYPE_MATCHES(&PQsendQueryParams,
							int (*)(PGconn *, const char *, int, const Oid *,
									const char *const *, const int *, const int *,
									int)),
			   "unexpected PQsendQueryParams signature");
_Static_assert(TYPE_MATCHES(&PQcancelCreate,
							PGcancelConn *(*)(PGconn *)),
			   "unexpected PQcancelCreate signature");
_Static_assert(TYPE_MATCHES(&PQcancelBlocking, int (*)(PGcancelConn *)),
			   "unexpected PQcancelBlocking signature");
_Static_assert(TYPE_MATCHES(&PQcancelFinish, void (*)(PGcancelConn *)),
			   "unexpected PQcancelFinish signature");
_Static_assert(TYPE_MATCHES(&PQsocketPoll,
							int (*)(int, int, int, pg_usec_time_t)),
			   "unexpected PQsocketPoll signature");

int
main(void)
{
	return 0;
}
