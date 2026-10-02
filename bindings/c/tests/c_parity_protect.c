#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "stegoeggo.h"

static int read_file(const char *path, uint8_t **data, size_t *len) {
    FILE *f;
    long n;
    uint8_t *buf;
    f = fopen(path, "rb");
    if (f == NULL) {
        return 1;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return 1;
    }
    n = ftell(f);
    if (n < 0) {
        fclose(f);
        return 1;
    }
    rewind(f);
    buf = (uint8_t *)malloc((size_t)n == 0 ? 1 : (size_t)n);
    if (buf == NULL) {
        fclose(f);
        return 1;
    }
    if (n > 0 && fread(buf, 1, (size_t)n, f) != (size_t)n) {
        free(buf);
        fclose(f);
        return 1;
    }
    fclose(f);
    *data = buf;
    *len = (size_t)n;
    return 0;
}

int main(int argc, char **argv) {
    uint8_t *input = NULL;
    size_t input_len = 0;
    uint64_t seed;
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_buffer_t *out = NULL;
    stegoeggo_v1_error_t *err = NULL;
    FILE *f;
    if (argc != 5) {
        fprintf(stderr, "usage: %s <in> <out> <seed-u64> <timestamp>\n", argv[0]);
        return 2;
    }
    seed = 0;
    for (const char *p = argv[3]; *p; p++) {
        if (*p < '0' || *p > '9') {
            fprintf(stderr, "seed must be a base-10 u64\n");
            return 2;
        }
        seed = seed * 10 + (uint64_t)(*p - '0');
    }
    if (read_file(argv[1], &input, &input_len) != 0) {
        fprintf(stderr, "cannot read %s\n", argv[1]);
        return 2;
    }
    if (stegoeggo_v1_notice_create(&notice, &err) != STEGOEGGO_V1_OK) {
        fprintf(stderr, "notice_create failed\n");
        return 1;
    }
    if (stegoeggo_v1_notice_set_copyright_holder(notice, "parity", 6, &err) !=
        STEGOEGGO_V1_OK) {
        fprintf(stderr, "notice setter failed\n");
        return 1;
    }
    if (stegoeggo_v1_request_with_hidden_marker(
            notice, STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING, &req, &err) !=
        STEGOEGGO_V1_OK) {
        fprintf(stderr, "request ctor failed\n");
        return 1;
    }
    if (stegoeggo_v1_request_set_seed(req, 1, seed, &err) != STEGOEGGO_V1_OK) {
        fprintf(stderr, "set_seed failed\n");
        return 1;
    }
    if (stegoeggo_v1_request_set_timestamp_override(req, 1, argv[4],
                                                   strlen(argv[4]),
                                                   &err) != STEGOEGGO_V1_OK) {
        fprintf(stderr, "set_timestamp failed\n");
        return 1;
    }
    if (stegoeggo_v1_protect(input, input_len, req, &out, &err) !=
        STEGOEGGO_V1_OK) {
        fprintf(stderr, "protect failed\n");
        if (err != NULL) {
            fprintf(stderr, "code=%u resource=%u len=%lu\n",
                    stegoeggo_v1_error_code(err), stegoeggo_v1_error_resource(err),
                    (unsigned long)stegoeggo_v1_error_message_len(err));
        }
        return 1;
    }
    f = fopen(argv[2], "wb");
    if (f == NULL) {
        fprintf(stderr, "cannot write %s\n", argv[2]);
        return 2;
    }
    if (stegoeggo_v1_buffer_len(out) > 0) {
        fwrite(stegoeggo_v1_buffer_data(out), 1, stegoeggo_v1_buffer_len(out), f);
    }
    fclose(f);
    stegoeggo_v1_buffer_free(out);
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
    free(input);
    return 0;
}
