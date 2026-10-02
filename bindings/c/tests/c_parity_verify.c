#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "stegoeggo.h"

int main(int argc, char **argv) {
    FILE *f;
    long n;
    uint8_t *buf;
    stegoeggo_v1_verification_report_t *rep = NULL;
    stegoeggo_v1_error_t *err = NULL;
    stegoeggo_v1_verification_status_t status;
    if (argc != 2) {
        fprintf(stderr, "usage: %s <image>\n", argv[0]);
        return 2;
    }
    f = fopen(argv[1], "rb");
    if (f == NULL) {
        fprintf(stderr, "cannot read %s\n", argv[1]);
        return 2;
    }
    fseek(f, 0, SEEK_END);
    n = ftell(f);
    rewind(f);
    buf = (uint8_t *)malloc(n <= 0 ? 1 : (size_t)n);
    if (buf == NULL) {
        fclose(f);
        return 2;
    }
    if (n > 0 && fread(buf, 1, (size_t)n, f) != (size_t)n) {
        free(buf);
        fclose(f);
        return 2;
    }
    fclose(f);
    if (stegoeggo_v1_verify(buf, (size_t)(n < 0 ? 0 : n), NULL, 0, NULL, &rep,
                            &err) != STEGOEGGO_V1_OK) {
        fprintf(stderr, "verify call failed\n");
        free(buf);
        return 1;
    }
    status = stegoeggo_v1_verification_report_status(rep);
    printf("status=%u rights=%u evidence=%u\n", (unsigned)status,
           (unsigned)stegoeggo_v1_verification_report_rights_found(rep),
           (unsigned)stegoeggo_v1_verification_report_evidence_strength(rep));
    stegoeggo_v1_verification_report_free(rep);
    free(buf);
    return 0;
}
