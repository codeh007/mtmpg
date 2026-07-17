#include <fcntl.h>
#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "libpq-fe.h"

#define MAX_TEST_TOKEN_LENGTH 16384

static const char *const expected_configuration =
	"https://candidate.example.test/oauth/database/.well-known/openid-configuration";
static const char *const expected_scope = "database";
static char *oauth_token;

static int provide_oauth_token(PGauthData type, PGconn *conn, void *data);
static char *read_token(const char *path);
static void clear_token(char *token);
static bool rejected_connection_is_redacted(const char *error,
										 bool expect_startup_reason);
static int verify_authenticated_session(PGconn *conn, const char *expected_role,
									const char *system_user_path);
static int write_system_user_fixture(const char *path, const char *value);

int
main(int argc, char **argv)
{
	bool expect_allowed;
	bool expect_startup_reason;
	const char *expected_role;
	const char *system_user_path = NULL;
	const char *const keywords[] = {
		"host", "port", "dbname", "user", "oauth_issuer",
		"oauth_client_id", "require_auth", "connect_timeout", NULL
	};
	const char *values[] = {
		"/tmp", "5432", "postgres", NULL,
		"https://candidate.example.test/oauth/database",
		"pggomtm-smoke", "oauth", "5", NULL
	};
	PGconn *conn = NULL;
	int status = EXIT_FAILURE;

	if (argc < 4 ||
		(strcmp(argv[1], "--expect-allowed") != 0 &&
		 strcmp(argv[1], "--expect-rejected") != 0 &&
		 strcmp(argv[1], "--expect-startup-rejected") != 0) ||
		(strcmp(argv[1], "--expect-allowed") == 0 && argc != 5) ||
		(strcmp(argv[1], "--expect-allowed") != 0 && argc != 4))
	{
		fprintf(stderr,
				"usage: %s --expect-allowed TOKEN_FILE ROLE SYSTEM_USER_FILE | "
				"--expect-rejected|--expect-startup-rejected TOKEN_FILE ROLE\n",
				argv[0]);
		return EXIT_FAILURE;
	}
	expect_allowed = strcmp(argv[1], "--expect-allowed") == 0;
	expect_startup_reason =
		strcmp(argv[1], "--expect-startup-rejected") == 0;
	expected_role = argv[3];
	values[3] = expected_role;
	if (expect_allowed)
		system_user_path = argv[4];
	oauth_token = read_token(argv[2]);
	if (oauth_token == NULL)
		return EXIT_FAILURE;

	PQsetAuthDataHook(provide_oauth_token);
	conn = PQconnectdbParams(keywords, values, 0);
	if (conn == NULL || PQstatus(conn) != CONNECTION_OK)
	{
		const char *error = conn == NULL ? "libpq returned no connection" :
			PQerrorMessage(conn);

		if (!expect_allowed &&
			rejected_connection_is_redacted(error, expect_startup_reason))
		{
			printf("PG18.4 OAuth rejection smoke passed\n");
			status = EXIT_SUCCESS;
		}
		else
			fprintf(stderr, "OAuth connection failed unexpectedly (redacted)\n");
		goto done;
	}

	if (!expect_allowed)
	{
		fprintf(stderr, "rejected OAuth token unexpectedly authenticated\n");
		goto done;
	}

	status = verify_authenticated_session(conn, expected_role, system_user_path);
	if (status == EXIT_SUCCESS)
		printf("PG18.4 OAuth allow, role and system_user smoke passed\n");

done:
	if (conn != NULL)
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

static bool
rejected_connection_is_redacted(const char *error,
								bool expect_startup_reason)
{
	bool category_matches;

	if (error == NULL)
		return false;
	if (expect_startup_reason)
		category_matches =
			strstr(error, "pggomtm-auth/v1/config-missing") != NULL;
	else
		category_matches =
			strstr(error, "OAuth bearer authentication failed") != NULL &&
			strstr(error, "pggomtm-auth/") == NULL &&
			strstr(error, "reason=") == NULL;

	return category_matches && strstr(error, oauth_token) == NULL &&
		strstr(error, "JWKS") == NULL &&
		strstr(error, "postgresql://") == NULL &&
		strstr(error, "stack backtrace") == NULL &&
		strstr(error, "panicked at") == NULL;
}

static int
verify_authenticated_session(PGconn *conn, const char *expected_role,
							 const char *system_user_path)
{
	PGresult *result = PQexec(conn,
		"SELECT system_user, current_user, current_setting('server_version_num')");
	int status = EXIT_FAILURE;

	if (result == NULL || PQresultStatus(result) != PGRES_TUPLES_OK ||
		PQntuples(result) != 1 || PQnfields(result) != 3 ||
		PQgetisnull(result, 0, 0) ||
		strncmp(PQgetvalue(result, 0, 0), "oauth:pggomtm:v1;",
				strlen("oauth:pggomtm:v1;")) != 0 ||
		strcmp(PQgetvalue(result, 0, 1), expected_role) != 0 ||
		strcmp(PQgetvalue(result, 0, 2), "180004") != 0)
	{
		fprintf(stderr, "OAuth session identity or PG18.4 runtime did not match\n");
		goto done;
	}
	status = write_system_user_fixture(system_user_path,
										 PQgetvalue(result, 0, 0));

done:
	if (result != NULL)
		PQclear(result);
	return status;
}

static int
write_system_user_fixture(const char *path, const char *value)
{
	const char *cursor;
	size_t remaining;
	int fd;
	int status = EXIT_FAILURE;

	if (path == NULL || value == NULL)
		return EXIT_FAILURE;
	cursor = value;
	remaining = strlen(value);
	if (remaining == 0)
		return EXIT_FAILURE;
	fd = open(path, O_WRONLY | O_CREAT | O_EXCL, S_IRUSR | S_IWUSR);
	if (fd < 0)
	{
		perror("could not create system_user fixture");
		return EXIT_FAILURE;
	}

	while (remaining > 0)
	{
		ssize_t written = write(fd, cursor, remaining);

		if (written <= 0)
		{
			perror("could not write system_user fixture");
			goto done;
		}
		cursor += written;
		remaining -= (size_t) written;
	}
	if (fsync(fd) != 0)
	{
		perror("could not sync system_user fixture");
		goto done;
	}
	status = EXIT_SUCCESS;

done:
	if (close(fd) != 0)
	{
		perror("could not close system_user fixture");
		status = EXIT_FAILURE;
	}
	if (status != EXIT_SUCCESS)
		unlink(path);
	return status;
}
