/* StegoEggo C ABI v1 artifact smoke.
 *
 * Dependency-free C11 consumer shipped inside every release-c artifact
 * bundle (examples/smoke.c). It synthesizes its own tiny PNG, protects
 * it metadata-only with a deterministic seed + timestamp, verifies the
 * result, exercises the verification JSON buffer, triggers one
 * invalid-argument error, and frees every handle.
 *
 * Build (example, Linux):
 *   cc -std=c11 -Wall -Wextra -Werror smoke.c -I../include \
 *     -L../lib -lstegoeggo_c -o smoke
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "stegoeggo.h"

static int failures = 0;

#define CHECK(cond)                                                 \
    do {                                                            \
        if (!(cond)) {                                              \
            printf("FAIL line %d: %s\n", __LINE__, #cond);          \
            failures++;                                             \
        }                                                           \
    } while (0)

static uint32_t crc_table[256];
static int crc_ready = 0;

static uint32_t crc_update(uint32_t c, const uint8_t *data, size_t len) {
    size_t i;
    int n, k;
    uint32_t cc;
    if (!crc_ready) {
        for (n = 0; n < 256; n++) {
            cc = (uint32_t)n;
            for (k = 0; k < 8; k++) {
                cc = (cc & 1) ? (0xedb88320u ^ (cc >> 1)) : (cc >> 1);
            }
            crc_table[n] = cc;
        }
        crc_ready = 1;
    }
    for (i = 0; i < len; i++) {
        c = crc_table[(c ^ data[i]) & 0xff] ^ (c >> 8);
    }
    return c;
}

static void put32(uint8_t *out, uint32_t v) {
    out[0] = (uint8_t)(v >> 24);
    out[1] = (uint8_t)(v >> 16);
    out[2] = (uint8_t)(v >> 8);
    out[3] = (uint8_t)v;
}

static void chunk(uint8_t *out, size_t *pos, const char *type,
                  const uint8_t *data, size_t len) {
    put32(out + *pos, (uint32_t)len);
    *pos += 4;
    memcpy(out + *pos, type, 4);
    *pos += 4;
    if (len > 0) {
        memcpy(out + *pos, data, len);
        *pos += len;
    }
    put32(out + *pos, crc_update(0xffffffffu, out + *pos - len - 4, len + 4) ^
                          0xffffffffu);
    *pos += 4;
}

static uint8_t *make_png(size_t *out_len) {
    static const uint8_t sig[8] = {137, 80, 78, 71, 13, 10, 26, 10};
    const uint32_t width = 64, height = 64;
    size_t row = (size_t)1 + (size_t)width * 3;
    size_t raw = row * height;
    size_t idat = 2 + 5 + raw + 4;
    size_t total = 8 + 25 + 12 + idat + 12;
    uint8_t *out = (uint8_t *)malloc(total);
    uint8_t ihdr[13];
    uint8_t *z;
    size_t pos, i = 0, left, n;
    uint32_t a = 1, b = 0;
    if (out == NULL) {
        return NULL;
    }
    memcpy(out, sig, 8);
    pos = 8;
    put32(ihdr, width);
    put32(ihdr + 4, height);
    ihdr[8] = 8;
    ihdr[9] = 2;
    ihdr[10] = 0;
    ihdr[11] = 0;
    ihdr[12] = 0;
    chunk(out, &pos, "IHDR", ihdr, 13);
    z = (uint8_t *)malloc(idat);
    if (z == NULL) {
        free(out);
        return NULL;
    }
    z[0] = 0x78;
    z[1] = 0x01;
    pos = 2;
    left = raw;
    while (left > 0) {
        size_t col;
        n = left > 65535 ? 65535 : left;
        left -= n;
        z[pos++] = (uint8_t)(left == 0 ? 1 : 0);
        z[pos++] = (uint8_t)(n & 0xff);
        z[pos++] = (uint8_t)((n >> 8) & 0xff);
        z[pos++] = (uint8_t)(~n & 0xff);
        z[pos++] = (uint8_t)((~n >> 8) & 0xff);
        for (col = 0; col < n; col++) {
            size_t cell = i % row;
            uint8_t v = 0;
            if (cell != 0) {
                v = (uint8_t)(31 + i * 37);
            }
            z[pos++] = v;
            i++;
        }
    }
    for (i = 2; i < pos; i++) {
        a = (a + z[i]) % 65521;
        b = (b + a) % 65521;
    }
    z[pos++] = (uint8_t)(a >> 8);
    z[pos++] = (uint8_t)(a & 0xff);
    z[pos++] = (uint8_t)(b >> 8);
    z[pos++] = (uint8_t)(b & 0xff);
    {
        size_t cpos = 8 + 25;
        chunk(out, &cpos, "IDAT", z, pos);
        chunk(out, &cpos, "IEND", NULL, 0);
        *out_len = cpos;
    }
    free(z);
    return out;
}

int main(void) {
    uint8_t *png = NULL;
    size_t png_len = 0;
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *request = NULL;
    stegoeggo_v1_buffer_t *protected = NULL;
    stegoeggo_v1_verification_report_t *report = NULL;
    stegoeggo_v1_error_t *error = NULL;
    static const char ts[] = "2026-01-01T00:00:00Z";

    CHECK(stegoeggo_abi_version_major() == 1);
    CHECK(stegoeggo_abi_version_minor() == 0);
    CHECK(stegoeggo_source_version() != NULL);
    CHECK(stegoeggo_source_version()[0] != '\0');

    png = make_png(&png_len);
    CHECK(png != NULL && png_len > 0);

    CHECK(stegoeggo_v1_notice_create(&notice, &error) == STEGOEGGO_V1_OK);
    CHECK(notice != NULL && error == NULL);
    CHECK(stegoeggo_v1_notice_set_copyright_holder(notice, "smoke", 5,
                                                   &error) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_metadata_only(
              notice, STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING, &request,
              &error) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_seed(request, 1, 7, &error) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_timestamp_override(request, 1, ts,
                                                      sizeof(ts) - 1,
                                                      &error) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_protect(png, png_len, request, &protected, &error) ==
          STEGOEGGO_V1_OK);
    CHECK(protected != NULL);
    CHECK(stegoeggo_v1_buffer_len(protected) > 0);
    CHECK(stegoeggo_v1_buffer_data(protected) != NULL);

    CHECK(stegoeggo_v1_verify(stegoeggo_v1_buffer_data(protected),
                              stegoeggo_v1_buffer_len(protected), NULL, 0,
                              NULL, &report, &error) == STEGOEGGO_V1_OK);
    CHECK(report != NULL);
    CHECK(stegoeggo_v1_verification_report_rights_found(report) == 1);
    {
        stegoeggo_v1_buffer_t *json = stegoeggo_v1_verification_report_json(report);
        CHECK(json != NULL);
        if (json != NULL) {
            CHECK(stegoeggo_v1_buffer_len(json) > 2);
            stegoeggo_v1_buffer_free(json);
        }
    }

    CHECK(stegoeggo_v1_request_set_jpeg_quality(request, 0, &error) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(error != NULL);
    if (error != NULL) {
        stegoeggo_v1_error_free(error);
        error = NULL;
    }

    stegoeggo_v1_verification_report_free(report);
    stegoeggo_v1_buffer_free(protected);
    stegoeggo_v1_request_free(request);
    stegoeggo_v1_notice_free(notice);
    free(png);

    if (failures == 0) {
        printf("artifact smoke OK\n");
        return 0;
    }
    printf("artifact smoke FAILURES=%d\n", failures);
    return 1;
}
