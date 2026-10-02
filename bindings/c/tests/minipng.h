#ifndef STEGOEGGO_MINIPNG_H
#define STEGOEGGO_MINIPNG_H

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

static uint32_t minipng_crc_table[256];
static int minipng_crc_ready = 0;

static void minipng_crc_init(void) {
    uint32_t c;
    int n, k;
    if (minipng_crc_ready) {
        return;
    }
    for (n = 0; n < 256; n++) {
        c = (uint32_t)n;
        for (k = 0; k < 8; k++) {
            c = (c & 1) ? (0xedb88320u ^ (c >> 1)) : (c >> 1);
        }
        minipng_crc_table[n] = c;
    }
    minipng_crc_ready = 1;
}

static uint32_t minipng_crc(const uint8_t *data, size_t len) {
    uint32_t c = 0xffffffffu;
    size_t i;
    minipng_crc_init();
    for (i = 0; i < len; i++) {
        c = minipng_crc_table[(c ^ data[i]) & 0xff] ^ (c >> 8);
    }
    return c ^ 0xffffffffu;
}

static void minipng_put32(uint8_t *out, uint32_t v) {
    out[0] = (uint8_t)(v >> 24);
    out[1] = (uint8_t)(v >> 16);
    out[2] = (uint8_t)(v >> 8);
    out[3] = (uint8_t)v;
}

static void minipng_chunk(uint8_t *out, size_t *pos, const char *type,
                          const uint8_t *data, size_t len) {
    minipng_put32(out + *pos, (uint32_t)len);
    *pos += 4;
    memcpy(out + *pos, type, 4);
    *pos += 4;
    if (len > 0) {
        memcpy(out + *pos, data, len);
        *pos += len;
    }
    minipng_put32(out + *pos, minipng_crc(out + *pos - len - 4, len + 4));
    *pos += 4;
}

static uint8_t *minipng_rgb(uint32_t width, uint32_t height, uint8_t r, uint8_t g,
                            uint8_t b, size_t *out_len) {
    static const uint8_t sig[8] = {137, 80, 78, 71, 13, 10, 26, 10};
    size_t row = (size_t)1 + (size_t)width * 3;
    size_t raw = row * height;
    size_t blocks = (raw + 65534) / 65535;
    size_t idat = 2 + blocks * 5 + raw + 4;
    size_t total = 8 + 25 + 12 + idat + 12;
    uint8_t *out = (uint8_t *)malloc(total);
    uint8_t ihdr[13];
    uint8_t *z;
    size_t pos = 0, i, y, left, n;
    uint32_t a = 1, bb = 0;
    if (out == NULL) {
        return NULL;
    }
    memcpy(out, sig, 8);
    pos = 8;
    minipng_put32(ihdr, width);
    minipng_put32(ihdr + 4, height);
    ihdr[8] = 8;
    ihdr[9] = 2;
    ihdr[10] = 0;
    ihdr[11] = 0;
    ihdr[12] = 0;
    minipng_chunk(out, &pos, "IHDR", ihdr, 13);
    z = (uint8_t *)malloc(idat);
    if (z == NULL) {
        free(out);
        return NULL;
    }
    z[0] = 0x78;
    z[1] = 0x01;
    pos = 2;
    left = raw;
    i = 0;
    while (left > 0) {
        n = left > 65535 ? 65535 : left;
        left -= n;
        z[pos++] = (uint8_t)(left == 0 ? 1 : 0);
        z[pos++] = (uint8_t)(n & 0xff);
        z[pos++] = (uint8_t)((n >> 8) & 0xff);
        z[pos++] = (uint8_t)(~n & 0xff);
        z[pos++] = (uint8_t)((~n >> 8) & 0xff);
        for (y = 0; y < n; y++) {
            size_t row_idx = i / row;
            size_t col = i % row;
            uint8_t v;
            if (col == 0) {
                v = 0;
            } else if ((col - 1) % 3 == 0) {
                v = (uint8_t)(r + row_idx + col);
            } else if ((col - 1) % 3 == 1) {
                v = (uint8_t)(g + row_idx * 2 + col);
            } else {
                v = b;
            }
            z[pos++] = v;
            i++;
        }
    }
    for (i = 2; i < pos; i++) {
        a = (a + z[i]) % 65521;
        bb = (bb + a) % 65521;
    }
    z[pos++] = (uint8_t)(a >> 8);
    z[pos++] = (uint8_t)(a & 0xff);
    z[pos++] = (uint8_t)(bb >> 8);
    z[pos++] = (uint8_t)(bb & 0xff);
    {
        size_t chunk_pos = 8 + 25;
        minipng_chunk(out, &chunk_pos, "IDAT", z, pos);
        minipng_chunk(out, &chunk_pos, "IEND", NULL, 0);
        *out_len = chunk_pos;
    }
    free(z);
    return out;
}

#endif
