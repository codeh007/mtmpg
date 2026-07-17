#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#include "libpq-fe.h"

#define MAX_TEST_TOKEN_LENGTH 16384

static const char *const expected_configuration =
	"https://candidate.example.test/oauth/database/.well-known/openid-configuration";
static const char *const expected_scope = "database";
static const char *const expected_system_user =
	"oauth:pggomtm:v1;u=usr_pgx_gate;actor=client:cli_pgx_gate;d=dlg_pgx_gate;m=oauth;a=1;p=ordinary";
static const char *const expected_role = "gomtm_candidate_ordinary";
static char *oauth_token;

static int provide_oauth_token(PGauthData type, PGconn *conn, void *data);
static char *read_token(const char *path);
static void clear_token(char *token);
static int verify_authenticated_session(PGconn *conn);

int
main(int argc, char **argv)
{
	static const char *const conninfo =
		"host=/tmp port=5432 dbname=postgres user=gomtm_candidate_ordinary "
		"oauth_issuer=https://candidate.example.test/oauth/database "
		"oauth_client_id=pggomtm-smoke require_auth=oauth connect_timeout=5";
	bool expect_allowed;
	PGconn *conn;
	int status = EXIT_FAILURE;

	if (argc != 3 ||
		(strcmp(argv[1], "--expect-allowed") != 0 &&
		 strcmp(argv[1], "--expect-rejected") != 0))
	{
		fprintf(stderr, "usage: %s --expect-allowed|--expect-rejected TOKEN_FILE\n",
				argv[0]);
		return EXIT_FAILURE;
	}
	expect_allowed = strcmp(argv[1], "--expect-allowed") == 0;
	oauth_token = read_token(argv[2]);
	if (oauth_token == NULL)
		return EXIT_FAILURE;

	PQsetAuthDataHook(provide_oauth_token);
	conn = PQconnectdb(conninfo);
	if (PQstatus(conn) != CONNECTION_OK)
	{
		const char *error = PQerrorMessage(conn);

		if (!expect_allowed &&
			strstr(error, "OAuth bearer authentication failed") != NULL &&
			strstr(error, oauth_token) == NULL)
		{
			printf("PG18.4 OAuth tampered-token rejection smoke passed\n");
			status = EXIT_SUCCESS;
		}
		else
			fprintf(stderr, "OAuth connection failed unexpectedly: %s", error);
		goto done;
	}

	if (!expect_allowed)
	{
		fprintf(stderr, "tampered OAuth token unexpectedly authenticated\n");
		goto done;
	}

	status = verify_authenticated_session(conn);
	if (status == EXIT_SUCCESS)
		printf("PG18.4 OAuth allow, role and system_user smoke passed\n");

done:
	PQfinish(conn);
	clear_token(oauth_token);
	return status;
}

static int
provide_oauth_token(PGauthData type, PGconn *conn, void *data)
{
	PGoauthBearerRequest *request = data;

	(void) conn;
	if (type != PQAUTHDATA_OAUTH_BEARER_TOKEN)
		return 0;
	if (request == NULL || request->openid_configuration == NULL ||
		request->scope == NULL ||
		strcmp(request->openid_configuration, expected_configuration) != 0 ||
		strcmp(request->scope, expected_scope) != 0)
		return -1;

	request->token = oauth_token;
	return 1;
}

static char *
read_token(const char *path)
{
	FILE *file;
	long length;
	size_t read_length;
	int close_status;
	char *token;

	file = fopen(path, "rb");
	if (file == NULL)
	{
		perror("could not open OAuth smoke token fixture");
		return NULL;
	}
	if (fseek(file, 0, SEEK_END) != 0 ||
		(length = ftell(file)) <= 0 || length > MAX_TEST_TOKEN_LENGTH ||
		fseek(file, 0, SEEK_SET) != 0)
	{
		fprintf(stderr, "OAuth smoke token fixture has an invalid length\n");
		fclose(file);
		return NULL;
	}

	token = malloc((size_t) length + 1);
	if (token == NULL)
	{
		fprintf(stderr, "could not allocate OAuth smoke token buffer\n");
		fclose(file);
		return NULL;
	}
	read_length = fread(token, 1, (size_t) length, file);
	token[read_length] = '\0';
	close_status = fclose(file);
	if (read_length != (size_t) length || close_status != 0)
	{
		fprintf(stderr, "could not read OAuth smoke token fixture\n");
		clear_token(token);
		return NULL;
	}
	if (strlen(token) != (size_t) length)
	{
		fprintf(stderr, "OAuth smoke token fixture contains a null byte\n");
		clear_token(token);
		return NULL;
	}
	return token;
}

static void
clear_token(char *token)
{
	volatile char *cursor;

	if (token == NULL)
		return;
	for (cursor = token; *cursor != '\0'; cursor++)
		*cursor = '\0';
	free(token);
}

static int
verify_authenticated_session(PGconn *conn)
{
	PGresult *result = PQexec(conn,
		"SELECT system_user, current_user, current_setting('server_version_num')");
	int status = EXIT_FAILURE;

	if (result == NULL || PQresultStatus(result) != PGRES_TUPLES_OK ||
		PQntuples(result) != 1 || PQnfields(result) != 3 ||
		PQgetisnull(result, 0, 0) ||
		strcmp(PQgetvalue(result, 0, 0), expected_system_user) != 0 ||
		strcmp(PQgetvalue(result, 0, 1), expected_role) != 0 ||
		strcmp(PQgetvalue(result, 0, 2), "180004") != 0)
	{
		fprintf(stderr, "OAuth session identity or PG18.4 runtime did not match\n");
		goto done;
	}
	status = EXIT_SUCCESS;

done:
	PQclear(result);
	return status;
}
