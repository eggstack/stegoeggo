#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "stegoeggo.h"

#include "minipng.h"

static int failures = 0;
static int checks = 0;

#define CHECK(cond)                                                        \
    do {                                                                   \
        checks++;                                                          \
        if (!(cond)) {                                                     \
            printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);         \
            failures++;                                                    \
        }                                                                  \
    } while (0)

#define CHECK_STATUS(call)                                                 \
    do {                                                                   \
        stegoeggo_v1_status_t s = (call);                                  \
        checks++;                                                          \
        if (s != STEGOEGGO_V1_OK) {                                        \
            printf("FAIL %s:%d: status %u: %s\n", __FILE__, __LINE__,       \
                   (unsigned)s, #call);                                    \
            failures++;                                                    \
        }                                                                  \
    } while (0)

static int bytes_contain(const uint8_t *hay, size_t hay_len, const char *needle) {
    size_t nlen;
    size_t i;
    if (hay == NULL || needle == NULL) {
        return 0;
    }
    nlen = strlen(needle);
    if (nlen == 0 || nlen > hay_len) {
        return 0;
    }
    for (i = 0; i + nlen <= hay_len; i++) {
        if (memcmp(hay + i, needle, nlen) == 0) {
            return 1;
        }
    }
    return 0;
}

static int error_text_contains(stegoeggo_v1_error_t *err, const char *needle) {
    if (err == NULL || needle == NULL) {
        return 0;
    }
    return bytes_contain((const uint8_t *)stegoeggo_v1_error_message_data(err),
                         stegoeggo_v1_error_message_len(err), needle);
}

static void test_bootstrap(void) {
    const char *v;
    CHECK(stegoeggo_abi_version_major() == 1);
    CHECK(stegoeggo_abi_version_minor() == 0);
    v = stegoeggo_source_version();
    CHECK(v != NULL);
    CHECK(strlen(v) > 0);
}

static void test_null_free_noop(void) {
    stegoeggo_v1_notice_free(NULL);
    stegoeggo_v1_request_free(NULL);
    stegoeggo_v1_resource_limits_free(NULL);
    stegoeggo_v1_buffer_free(NULL);
    stegoeggo_v1_error_free(NULL);
    stegoeggo_v1_execution_report_free(NULL);
    stegoeggo_v1_verification_report_free(NULL);
    checks++;
}

static void test_detect_format(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_image_format_t fmt = 99;
    stegoeggo_v1_status_t s;
    static const uint8_t junk[8] = {0, 1, 2, 3, 4, 5, 6, 7};
    s = stegoeggo_v1_detect_format(png, png_len, &fmt, NULL);
    CHECK(s == STEGOEGGO_V1_OK);
    CHECK(fmt == STEGOEGGO_V1_FORMAT_PNG);
    s = stegoeggo_v1_detect_format(junk, sizeof(junk), &fmt, NULL);
    CHECK(s == STEGOEGGO_V1_OK);
    CHECK(fmt == STEGOEGGO_V1_FORMAT_UNKNOWN);
    s = stegoeggo_v1_detect_format(NULL, 0, &fmt, NULL);
    CHECK(s == STEGOEGGO_V1_OK);
    CHECK(fmt == STEGOEGGO_V1_FORMAT_UNKNOWN);
    s = stegoeggo_v1_detect_format(png, png_len, NULL, NULL);
    CHECK(s == STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    s = stegoeggo_v1_detect_format(NULL, 4, &fmt, NULL);
    CHECK(s == STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
}

static void test_metadata_only_protect_verify(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_buffer_t *out = NULL;
    stegoeggo_v1_execution_report_t *rep = NULL;
    stegoeggo_v1_verification_report_t *vrep = NULL;
    stegoeggo_v1_error_t *err = NULL;
    stegoeggo_v1_status_t s;
    static const char ts[] = "2026-01-01T00:00:00Z";

    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    CHECK(err == NULL);
    CHECK(notice != NULL);
    CHECK_STATUS(stegoeggo_v1_notice_set_copyright_holder(notice, "Acme", 4, &err));
    CHECK_STATUS(
        stegoeggo_v1_notice_set_dmi(notice, STEGOEGGO_V1_DMI_ALLOWED, &err));
    CHECK_STATUS(stegoeggo_v1_notice_set_seed(notice, 1, 0, &err));
    CHECK_STATUS(stegoeggo_v1_request_metadata_only(
        notice, STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING, &req, &err));
    CHECK(req != NULL);
    CHECK_STATUS(stegoeggo_v1_request_set_seed(req, 1, 0, &err));
    CHECK_STATUS(stegoeggo_v1_request_set_timestamp_override(
        req, 1, ts, sizeof(ts) - 1, &err));
    s = stegoeggo_v1_protect_with_report(png, png_len, req, &out, &rep, &err);
    CHECK(s == STEGOEGGO_V1_OK);
    CHECK(err == NULL);
    CHECK(out != NULL);
    CHECK(rep != NULL);
    CHECK(stegoeggo_v1_buffer_len(out) > 0);
    CHECK(stegoeggo_v1_buffer_data(out) != NULL);
    CHECK(stegoeggo_v1_execution_report_effective_policy(rep) ==
          STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING);
    CHECK(stegoeggo_v1_execution_report_has_dmi(rep) == 1);
    CHECK(stegoeggo_v1_execution_report_effective_dmi(rep) ==
          STEGOEGGO_V1_DMI_PROHIBITED_AI_ML_TRAINING);
    CHECK(stegoeggo_v1_execution_report_metadata_injected(rep) == 1);
    CHECK(stegoeggo_v1_execution_report_warning_count(rep) < 100);
    CHECK(stegoeggo_v1_execution_report_warning_at(rep, 1000000) == 0xffffffffu);

    s = stegoeggo_v1_verify(stegoeggo_v1_buffer_data(out),
                            stegoeggo_v1_buffer_len(out), NULL, 0, NULL, &vrep,
                            &err);
    CHECK(s == STEGOEGGO_V1_OK);
    CHECK(vrep != NULL);
    CHECK(stegoeggo_v1_verification_report_status(vrep) ==
          STEGOEGGO_V1_VERIFY_NOT_FOUND);
    CHECK(stegoeggo_v1_verification_report_rights_found(vrep) == 1);
    CHECK(stegoeggo_v1_verification_report_evidence_strength(vrep) ==
          STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_ONLY);
    CHECK(stegoeggo_v1_verification_report_authenticated(vrep) == 0);
    {
        stegoeggo_v1_buffer_t *json =
            stegoeggo_v1_verification_report_json(vrep);
        CHECK(json != NULL);
        if (json != NULL) {
            const uint8_t *jd = stegoeggo_v1_buffer_data(json);
            size_t jl = stegoeggo_v1_buffer_len(json);
            CHECK(jl > 2);
            CHECK(jd != NULL && jd[0] == (uint8_t)'{');
            stegoeggo_v1_buffer_free(json);
        }
    }

    stegoeggo_v1_verification_report_free(vrep);
    stegoeggo_v1_execution_report_free(rep);
    stegoeggo_v1_buffer_free(out);
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
    }
}

static void test_hidden_marker_round_trip(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_buffer_t *out = NULL;
    stegoeggo_v1_verification_report_t *vrep = NULL;
    stegoeggo_v1_error_t *err = NULL;
    static const char ts[] = "2026-01-01T00:00:00Z";

    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_notice_set_contact_email(notice, "a@b.c", 5, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_notice_set_notice_applied_at(notice, NULL, 0, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_with_hidden_marker(
              notice, STEGOEGGO_V1_POLICY_ALLOWED, &req, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_seed(req, 1, 0xffffffffffffffffULL, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_timestamp_override(req, 1, ts,
                                                     sizeof(ts) - 1,
                                                     &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_hidden_marker_mode(
              req, STEGOEGGO_V1_MARKER_BEST_EFFORT, 0, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_protect(png, png_len, req, &out, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(out != NULL);
    CHECK(stegoeggo_v1_verify(stegoeggo_v1_buffer_data(out),
                              stegoeggo_v1_buffer_len(out), NULL, 0, NULL,
                              &vrep, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_verification_report_status(vrep) ==
          STEGOEGGO_V1_VERIFY_VERIFIED);

    stegoeggo_v1_verification_report_free(vrep);
    stegoeggo_v1_buffer_free(out);
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
    }
}

static void test_unprotected_verify(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_verification_report_t *vrep = NULL;
    stegoeggo_v1_error_t *err = NULL;
    CHECK(stegoeggo_v1_verify(png, png_len, NULL, 0, NULL, &vrep, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(vrep != NULL);
    CHECK(stegoeggo_v1_verification_report_status(vrep) ==
          STEGOEGGO_V1_VERIFY_NOT_FOUND);
    CHECK(stegoeggo_v1_verification_report_rights_found(vrep) == 0);
    stegoeggo_v1_verification_report_free(vrep);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
    }
}

static void test_hmac(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_buffer_t *out = NULL;
    stegoeggo_v1_verification_report_t *vrep = NULL;
    stegoeggo_v1_error_t *err = NULL;
    static const uint8_t key[] = "SECRET-c-abi-key-0123456789";
    static const uint8_t wrong[] = "wrong-key-9876543210";
    static const char ts[] = "2026-01-01T00:00:00Z";
    static const uint8_t garbage[8] = {'g', 'a', 'r', 'b', 'a', 'g', 'e', '!'};

    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_with_hidden_marker(
              notice, STEGOEGGO_V1_POLICY_PROHIBITED_ALL_DATA_MINING, &req,
              &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_authentication_mode(req, STEGOEGGO_V1_AUTH_HMAC,
                                                      &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_mac_key(req, key, sizeof(key) - 1, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_seed(req, 1, 99, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_timestamp_override(req, 1, ts,
                                                     sizeof(ts) - 1,
                                                     &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_protect(png, png_len, req, &out, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_verify(stegoeggo_v1_buffer_data(out),
                              stegoeggo_v1_buffer_len(out), key,
                              sizeof(key) - 1, NULL, &vrep,
                              &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_verification_report_status(vrep) ==
          STEGOEGGO_V1_VERIFY_VERIFIED);
    CHECK(stegoeggo_v1_verification_report_authenticated(vrep) == 1);
    stegoeggo_v1_verification_report_free(vrep);
    vrep = NULL;
    CHECK(stegoeggo_v1_verify(stegoeggo_v1_buffer_data(out),
                              stegoeggo_v1_buffer_len(out), wrong,
                              sizeof(wrong) - 1, NULL, &vrep,
                              &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_verification_report_status(vrep) !=
          STEGOEGGO_V1_VERIFY_VERIFIED);
    CHECK(stegoeggo_v1_verification_report_authenticated(vrep) == 0);
    stegoeggo_v1_verification_report_free(vrep);
    vrep = NULL;
    CHECK(stegoeggo_v1_verify(stegoeggo_v1_buffer_data(out),
                              stegoeggo_v1_buffer_len(out), NULL, 0, NULL,
                              &vrep, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_verification_report_authenticated(vrep) == 0);
    CHECK(!error_text_contains(err, "SECRET-c-abi-key"));
    stegoeggo_v1_verification_report_free(vrep);
    vrep = NULL;
    {
        stegoeggo_v1_buffer_t *fail_out = NULL;
        stegoeggo_v1_status_t fs = stegoeggo_v1_protect(
            garbage, sizeof(garbage), req, &fail_out, &err);
        CHECK(fs != STEGOEGGO_V1_OK);
        CHECK(fail_out == NULL);
        CHECK(!error_text_contains(err, "SECRET-c-abi-key"));
        if (err != NULL) {
            stegoeggo_v1_buffer_t *details =
                stegoeggo_v1_error_details_json(err);
            if (details != NULL) {
                CHECK(!bytes_contain(stegoeggo_v1_buffer_data(details),
                                     stegoeggo_v1_buffer_len(details),
                                     "SECRET-c-abi-key"));
                stegoeggo_v1_buffer_free(details);
            }
        }
    }

    stegoeggo_v1_verification_report_free(vrep);
    stegoeggo_v1_buffer_free(out);
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
    }
}

static void test_hmac_missing_key_fails(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_buffer_t *out = NULL;
    stegoeggo_v1_error_t *err = NULL;
    stegoeggo_v1_status_t s;

    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_from_preset(STEGOEGGO_V1_PRESET_MAXIMAL, notice,
                                           STEGOEGGO_V1_POLICY_ALLOWED, &req,
                                           &err) == STEGOEGGO_V1_OK);
    s = stegoeggo_v1_protect(png, png_len, req, &out, &err);
    CHECK(s == STEGOEGGO_V1_ERR_INVALID_CONFIGURATION);
    CHECK(out == NULL);
    CHECK(err != NULL);
    if (err != NULL) {
        CHECK(stegoeggo_v1_error_code(err) ==
              STEGOEGGO_V1_ERR_INVALID_CONFIGURATION);
        CHECK(stegoeggo_v1_error_resource(err) == STEGOEGGO_V1_RESOURCE_NONE);
        CHECK(stegoeggo_v1_error_message_len(err) > 0);
        CHECK(stegoeggo_v1_error_message_data(err) != NULL);
        {
            stegoeggo_v1_buffer_t *details = stegoeggo_v1_error_details_json(err);
            CHECK(details != NULL);
            if (details != NULL) {
                CHECK(stegoeggo_v1_buffer_len(details) > 0);
                stegoeggo_v1_buffer_free(details);
            }
        }
        stegoeggo_v1_error_free(err);
        err = NULL;
    }
    stegoeggo_v1_buffer_free(out);
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
}

static void test_resource_limit_error(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_resource_limits_t *limits = NULL;
    stegoeggo_v1_buffer_t *out = NULL;
    stegoeggo_v1_error_t *err = NULL;
    stegoeggo_v1_status_t s;

    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_metadata_only(
              notice, STEGOEGGO_V1_POLICY_ALLOWED, &req, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_create(&limits, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_input_bytes(limits, 10, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_resource_limits(req, limits, &err) ==
          STEGOEGGO_V1_OK);
    s = stegoeggo_v1_protect(png, png_len, req, &out, &err);
    CHECK(s == STEGOEGGO_V1_ERR_RESOURCE_LIMIT);
    CHECK(out == NULL);
    CHECK(err != NULL);
    if (err != NULL) {
        stegoeggo_v1_buffer_t *details;
        CHECK(stegoeggo_v1_error_resource(err) ==
              STEGOEGGO_V1_RESOURCE_INPUT_BYTES);
        details = stegoeggo_v1_error_details_json(err);
        CHECK(details != NULL);
        if (details != NULL) {
            const uint8_t *d = stegoeggo_v1_buffer_data(details);
            size_t dl = stegoeggo_v1_buffer_len(details);
            CHECK(dl > 0 && d != NULL && d[0] == (uint8_t)'{');
            stegoeggo_v1_buffer_free(details);
        }
        stegoeggo_v1_error_free(err);
        err = NULL;
    }
    stegoeggo_v1_resource_limits_free(limits);
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
}

static void test_malformed_inputs(const uint8_t *png, size_t png_len) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_buffer_t *out = NULL;
    stegoeggo_v1_error_t *err = NULL;
    stegoeggo_v1_status_t s;
    static const uint8_t garbage[16] = "not-an-image!!!";

    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_metadata_only(
              notice, STEGOEGGO_V1_POLICY_ALLOWED, &req, &err) ==
          STEGOEGGO_V1_OK);
    s = stegoeggo_v1_protect(garbage, sizeof(garbage), req, &out, &err);
    CHECK(s == STEGOEGGO_V1_ERR_INVALID_FORMAT);
    CHECK(out == NULL);
    CHECK(err != NULL);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
        err = NULL;
    }
    s = stegoeggo_v1_protect(png, 20, req, &out, &err);
    CHECK(s != STEGOEGGO_V1_OK);
    CHECK(out == NULL);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
        err = NULL;
    }
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
    (void)png_len;
}

static void test_invalid_arguments(void) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_resource_limits_t *limits = NULL;
    stegoeggo_v1_error_t *err = NULL;
    static const uint8_t bad[2] = {0xff, 0xfe};
    static const char nul[4] = {'a', 0, 'b', 'c'};

    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_notice_set_copyright_holder(notice, (const char *)bad,
                                                  2, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_notice_set_copyright_holder(notice, nul, 4, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_notice_set_dmi(notice, 99, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_notice_set_seed(notice, 2, 0, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_notice_set_copyright_holder(NULL, "x", 1, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_metadata_only(NULL,
                                             STEGOEGGO_V1_POLICY_ALLOWED, &req,
                                             &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(req == NULL);
    CHECK(stegoeggo_v1_request_metadata_only(notice, 99, &req, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_from_preset(99, notice,
                                           STEGOEGGO_V1_POLICY_ALLOWED, &req,
                                           &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_metadata_only(
              notice, STEGOEGGO_V1_POLICY_ALLOWED, NULL, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_metadata_only(
              notice, STEGOEGGO_V1_POLICY_ALLOWED, &req, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_intensity(req, 2.0f, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_jpeg_quality(req, 0, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    {
        float nan_value;
        uint32_t bits = 0x7fc00000u;
        memcpy(&nan_value, &bits, sizeof(nan_value));
        CHECK(stegoeggo_v1_request_set_intensity(req, nan_value, &err) ==
              STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    }
    CHECK(stegoeggo_v1_request_set_output_format(
              req, 1, STEGOEGGO_V1_FORMAT_UNKNOWN, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_output_format(req, 7, 1, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_stego_redundancy(req, 1, 11, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_stego_redundancy(req, 1, 0, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_content_hash(req, 1, bad, 2, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_hidden_marker_mode(
              req, STEGOEGGO_V1_MARKER_TILED, 0, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_hidden_marker_mode(
              req, STEGOEGGO_V1_MARKER_DISABLED, 64, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_authentication_mode(req, 9, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_max_dimension(req, 1, 0, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_metadata_update_policy(req, 9, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_hidden_marker_mode(
              req, STEGOEGGO_V1_MARKER_TILED, 64, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_create(&limits, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_width(limits, 1024, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_height(limits, 1024, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_png_chunks(limits, 8, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_png_chunk_bytes(limits, 9, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_jpeg_segments(limits, 8, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_jpeg_segment_bytes(limits, 9,
                                                                 &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_webp_riff_chunks(limits, 8,
                                                               &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_webp_riff_bytes(limits, 9,
                                                              &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_xmp_bytes(limits, 9, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_xml_depth(limits, 9, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_xml_properties(limits, 9,
                                                              &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_metadata_fields(limits, 9,
                                                              &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_metadata_field_bytes(limits, 9,
                                                                   &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_payload_bytes(limits, 9, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_detached_manifest_bytes(
              limits, 9, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_tile_extraction_origins(
              limits, 9, &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_resource_limits_set_max_verification_seeds(limits, 9,
                                                                 &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_resource_limits(req, limits, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_resource_limits(req, NULL, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_notice_create(NULL, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
        err = NULL;
    }
    stegoeggo_v1_resource_limits_free(limits);
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
}

static void test_presets_and_clearing(void) {
    stegoeggo_v1_notice_t *notice = NULL;
    stegoeggo_v1_request_t *req = NULL;
    stegoeggo_v1_error_t *err = NULL;
    static const uint32_t presets[4] = {STEGOEGGO_V1_PRESET_LEGAL_NOTICE,
                                        STEGOEGGO_V1_PRESET_LEGAL_NOTICE_WITH_STEGO,
                                        STEGOEGGO_V1_PRESET_AUTHENTICATED_PROVENANCE,
                                        STEGOEGGO_V1_PRESET_MAXIMAL};
    size_t i;
    CHECK(stegoeggo_v1_notice_create(&notice, &err) == STEGOEGGO_V1_OK);
    for (i = 0; i < 4; i++) {
        CHECK(stegoeggo_v1_request_from_preset(presets[i], notice,
                                               STEGOEGGO_V1_POLICY_ALLOWED,
                                               &req, &err) == STEGOEGGO_V1_OK);
        CHECK(req != NULL);
        stegoeggo_v1_request_free(req);
        req = NULL;
    }
    CHECK(stegoeggo_v1_request_metadata_only(
              notice, STEGOEGGO_V1_POLICY_ALLOWED, &req, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_progressive_jpeg(req, 1, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_progressive_jpeg(req, 0, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_progressive_jpeg(req, 3, &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    CHECK(stegoeggo_v1_request_set_mac_key(req, (const uint8_t *)"k", 1,
                                           &err) == STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_mac_key(req, NULL, 0, &err) ==
          STEGOEGGO_V1_OK);
    CHECK(stegoeggo_v1_request_set_mac_key(req, (const uint8_t *)"k", 0,
                                           &err) ==
          STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    if (err != NULL) {
        stegoeggo_v1_error_free(err);
        err = NULL;
    }
    stegoeggo_v1_request_free(req);
    stegoeggo_v1_notice_free(notice);
}

int main(void) {
    uint8_t *png;
    size_t png_len;
    png = minipng_rgb(64, 64, 31, 57, 83, &png_len);
    if (png == NULL || png_len == 0) {
        printf("FAIL setup: minipng allocation\n");
        return 1;
    }
    test_bootstrap();
    test_null_free_noop();
    test_detect_format(png, png_len);
    test_metadata_only_protect_verify(png, png_len);
    test_hidden_marker_round_trip(png, png_len);
    test_unprotected_verify(png, png_len);
    test_hmac(png, png_len);
    test_hmac_missing_key_fails(png, png_len);
    test_resource_limit_error(png, png_len);
    test_malformed_inputs(png, png_len);
    test_invalid_arguments();
    test_presets_and_clearing();
    free(png);
    printf("c-abi checks=%d failures=%d\n", checks, failures);
    return failures == 0 ? 0 : 1;
}
