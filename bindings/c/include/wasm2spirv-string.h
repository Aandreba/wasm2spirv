#ifndef WASM2SPIRV_STRING
#define WASM2SPIRV_STRING

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// A view into a UTF-8 string owned by wasm2spirv
typedef struct w2s_string_view_s {
    const uint8_t* ptr;
    const size_t len;
} w2s_string_view;

// A view into an array of bytes owned by wasm2spirv
typedef struct w2s_byte_view_s {
    const uint8_t* ptr;
    const size_t len;
} w2s_byte_view;

// A view into an array of words owned by wasm2spirv
typedef struct w2s_word_view_s {
    const uint32_t* ptr;
    const size_t len;
} w2s_word_view;

// A null-terminated, UTF-8 string allocated by wasm2spirv
typedef struct w2s_string_s {
    const char* ptr;
    // Doesn't include the null character
    const size_t len;
} w2s_string;

w2s_string w2s_string_clone(w2s_string str);
void w2s_string_destroy(w2s_string str);

#ifdef __cplusplus
}
#endif

#endif
