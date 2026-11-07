/* GnuTLS on NT 4.0: a server (with a certificate made here) and a client, each on one end
   of a connected socket, in two threads (see src/main.rs). */
#include <winsock2.h>
#include <stdio.h>
#include <string.h>
#include <gnutls/gnutls.h>
#include <gnutls/x509.h>
#include <gnutls/crypto.h>

static ssize_t push(gnutls_transport_ptr_t p, const void *d, size_t n)
{
    int r = send((SOCKET)(UINT_PTR)p, d, (int)n, 0);
    return r == SOCKET_ERROR ? -1 : r;
}

static ssize_t pull(gnutls_transport_ptr_t p, void *d, size_t n)
{
    int r = recv((SOCKET)(UINT_PTR)p, d, (int)n, 0);
    return r == SOCKET_ERROR ? -1 : r;
}

static gnutls_certificate_credentials_t server_creds;

static void logger(int level, const char *msg)
{
    fprintf(stderr, "gnutls<%d>: %s", level, msg);
    fflush(stderr);
}

static __declspec(thread) int tls_probe = 7;

/* NT4_TLS_DEBUG=1: GnuTLS's own log on standard error, and a thread-local's value. */
int tls_debug(int level)
{
    gnutls_global_set_log_function(logger);
    gnutls_global_set_log_level(level);
    fprintf(stderr, "C thread-local: %d\n", tls_probe);
    fflush(stderr);
    return tls_probe;
}

int tls_version(char *out, int size)
{
    _snprintf(out, size, "%s", gnutls_check_version(NULL));
    out[size - 1] = 0;
    return 0;
}

int tls_random(unsigned char *out, int n)
{
    return gnutls_rnd(GNUTLS_RND_KEY, out, (size_t)n);
}

/* A self-signed ECDSA P-256 certificate for the server. */
int tls_setup_server(void)
{
    gnutls_x509_privkey_t key;
    gnutls_x509_crt_t crt;
    int r;
    unsigned char serial[8] = { 1, 2, 3, 4, 5, 6, 7, 8 };
    time_t now = time(NULL);

    if ((r = gnutls_x509_privkey_init(&key)) < 0) return r;
    if ((r = gnutls_x509_privkey_generate(key, GNUTLS_PK_ECDSA, GNUTLS_CURVE_TO_BITS(GNUTLS_ECC_CURVE_SECP256R1), 0)) < 0) return r;
    if ((r = gnutls_x509_crt_init(&crt)) < 0) return r;
    gnutls_x509_crt_set_version(crt, 3);
    gnutls_x509_crt_set_serial(crt, serial, sizeof serial);
    gnutls_x509_crt_set_activation_time(crt, now - 3600);
    gnutls_x509_crt_set_expiration_time(crt, now + 86400);
    gnutls_x509_crt_set_dn_by_oid(crt, GNUTLS_OID_X520_COMMON_NAME, 0, "pint", 4);
    gnutls_x509_crt_set_key(crt, key);
    if ((r = gnutls_x509_crt_sign2(crt, crt, key, GNUTLS_DIG_SHA256, 0)) < 0) return r;
    if ((r = gnutls_certificate_allocate_credentials(&server_creds)) < 0) return r;
    return gnutls_certificate_set_x509_key(server_creds, &crt, 1, key);
}

/* Runs one end; returns 0 when the message from the other end arrived as expected. */
static int run(SOCKET s, int server, char *info, int size)
{
    gnutls_session_t ses;
    gnutls_certificate_credentials_t client_creds = NULL;
    char buf[64];
    int r;
    const char *mine = server ? "from server" : "from client";
    const char *theirs = server ? "from client" : "from server";

    if ((r = gnutls_init(&ses, server ? GNUTLS_SERVER : GNUTLS_CLIENT)) < 0) return r;
    gnutls_set_default_priority(ses);
    if (server) {
        gnutls_credentials_set(ses, GNUTLS_CRD_CERTIFICATE, server_creds);
    } else {
        gnutls_certificate_allocate_credentials(&client_creds);
        gnutls_credentials_set(ses, GNUTLS_CRD_CERTIFICATE, client_creds);
        gnutls_server_name_set(ses, GNUTLS_NAME_DNS, "pint", 4);
    }
    gnutls_transport_set_ptr(ses, (gnutls_transport_ptr_t)(UINT_PTR)s);
    gnutls_transport_set_push_function(ses, push);
    gnutls_transport_set_pull_function(ses, pull);
    do r = gnutls_handshake(ses); while (r < 0 && !gnutls_error_is_fatal(r));
    if (r < 0) {
        _snprintf(info, size, "handshake: %s", gnutls_strerror(r));
        info[size - 1] = 0;
        goto out;
    }
    _snprintf(info, size, "%s, %s", gnutls_protocol_get_name(gnutls_protocol_get_version(ses)),
              gnutls_cipher_get_name(gnutls_cipher_get(ses)));
    info[size - 1] = 0;
    if ((r = (int)gnutls_record_send(ses, mine, strlen(mine))) < 0) goto out;
    r = (int)gnutls_record_recv(ses, buf, sizeof buf - 1);
    if (r < 0) goto out;
    buf[r] = 0;
    r = strcmp(buf, theirs) == 0 ? 0 : -1;
    gnutls_bye(ses, GNUTLS_SHUT_WR);
out:
    gnutls_deinit(ses);
    if (client_creds) gnutls_certificate_free_credentials(client_creds);
    return r;
}

int tls_server(UINT_PTR s, char *info, int size) { return run((SOCKET)s, 1, info, size); }
int tls_client(UINT_PTR s, char *info, int size) { return run((SOCKET)s, 0, info, size); }

const char *tls_error(int r) { return gnutls_strerror(r); }
