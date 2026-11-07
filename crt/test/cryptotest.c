/* GMP and Nettle built for NT 4.0 (the primitives rdesktop uses), against known values. */
#include <stdio.h>
#include <string.h>
#include <gmp.h>
#include <nettle/md5.h>
#include <nettle/sha1.h>
#include <nettle/hmac.h>
#include <nettle/arcfour.h>
#include <nettle/rsa.h>

static int failures;
static void hex(char *out, const unsigned char *d, size_t n) { while (n--) out += sprintf(out, "%02x", *d++); }
static void check(const char *name, const char *got, const char *want)
{
    int ok = strcmp(got, want) == 0;
    printf("  %-22s %s%s\n", name, ok ? "ok" : "FAILED: ", ok ? "" : got);
    failures += !ok;
}

int main(void)
{
    unsigned char d[64];
    char h[200];
    struct md5_ctx m; struct sha1_ctx s; struct hmac_md5_ctx hm; struct arcfour_ctx rc4;

    md5_init(&m); md5_update(&m, 3, (const uint8_t *)"abc"); md5_digest(&m, 16, d);
    hex(h, d, 16); check("MD5", h, "900150983cd24fb0d6963f7d28e17f72");
    sha1_init(&s); sha1_update(&s, 3, (const uint8_t *)"abc"); sha1_digest(&s, 20, d);
    hex(h, d, 20); check("SHA-1", h, "a9993e364706816aba3e25717850c26c9cd0d89d");
    hmac_md5_set_key(&hm, 4, (const uint8_t *)"Jefe");
    hmac_md5_update(&hm, 28, (const uint8_t *)"what do ya want for nothing?");
    hmac_md5_digest(&hm, 16, d);
    hex(h, d, 16); check("HMAC-MD5 (RFC 2104)", h, "750c783e6ab0b503eaa86e310a5db738");
    arcfour_set_key(&rc4, 3, (const uint8_t *)"Key");
    arcfour_crypt(&rc4, 9, d, (const uint8_t *)"Plaintext");
    hex(h, d, 9); check("RC4", h, "bbf316e8d940af0ad3");

    {   /* RSA public operation, as rdesktop does it: 42^65537 mod n */
        struct rsa_public_key k; mpz_t x;
        rsa_public_key_init(&k);
        mpz_set_str(k.n, "c4f8e9e15dcadf2b96c763d981006a644ffb4415030a16ed1283883340f2aa0e2be2be8fa60150b9046965837c3e7d151b7de237ebb957c20663898850e3a5d8a5" "0b0e6e18b4a4c6a9ac43ef1c6fc5d8f36dc45d3e60a6131e2aeb6c51e93a5d6125c7f0d0d1fdfb57ad2e9d5c1cb3e3dd7ec0f3d8ac2a3c4ad5e0b3f1e52b7", 16);
        mpz_set_ui(k.e, 65537);
        rsa_public_key_prepare(&k);
        mpz_init_set_ui(x, 42);
        mpz_powm(x, x, k.e, k.n);
        mpz_get_str(h, 16, x);
        printf("  %-22s %s\n", "RSA public (mpz_powm)", strlen(h) > 100 ? "ok" : "FAILED");
        failures += strlen(h) <= 100;
        mpz_set_str(x, "123456789012345678901234567890", 10);
        mpz_mul(x, x, x);
        mpz_get_str(h, 10, x);
        check("GMP multiply", h, "15241578753238836750495351562536198787501905199875019052100");
        mpz_clear(x); rsa_public_key_clear(&k);
    }
    printf("cryptotest: %s\n", failures ? "FAILED" : "ok");
    return failures;
}
