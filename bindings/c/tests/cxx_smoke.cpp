#include <cassert>
#include <cstddef>
#include <cstdint>

#include "stegoeggo.h"

int main() {
    assert(stegoeggo_abi_version_major() == 1);
    assert(stegoeggo_abi_version_minor() == 0);
    assert(stegoeggo_source_version() != nullptr);

    stegoeggo_v1_notice_t *notice = nullptr;
    stegoeggo_v1_error_t *error = nullptr;
    assert(stegoeggo_v1_notice_create(&notice, &error) == STEGOEGGO_V1_OK);
    assert(notice != nullptr);
    assert(error == nullptr);

    stegoeggo_v1_request_t *request = nullptr;
    assert(stegoeggo_v1_request_metadata_only(
               notice, STEGOEGGO_V1_POLICY_ALLOWED, &request, &error) ==
           STEGOEGGO_V1_OK);

    stegoeggo_v1_image_format_t format = STEGOEGGO_V1_FORMAT_UNKNOWN;
    const uint8_t png_sig[8] = {137, 80, 78, 71, 13, 10, 26, 10};
    assert(stegoeggo_v1_detect_format(png_sig, sizeof(png_sig), &format,
                                     &error) == STEGOEGGO_V1_OK);
    assert(format == STEGOEGGO_V1_FORMAT_PNG);

    uint32_t code = stegoeggo_v1_error_code(nullptr);
    assert(code == STEGOEGGO_V1_ERR_INVALID_ARGUMENT);

    stegoeggo_v1_request_free(request);
    stegoeggo_v1_notice_free(notice);
    return 0;
}
