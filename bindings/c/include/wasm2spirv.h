#ifndef WASM2SPIRV
#define WASM2SPIRV

#include <stddef.h>
#include <stdint.h>

#include "wasm2spirv-config.h"

#ifdef __cplusplus
extern "C" {
#endif

/* MISC */

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

/* COMPILATION */
typedef struct w2s_compilation_s* w2s_compilation;

// Takes ownership of `config`. If an error ocurred, returns null.
w2s_compilation w2s_compilation_new(const w2s_config config, const uint8_t* bytes, const size_t bytes_len);
w2s_string_view w2s_compilation_assembly(w2s_compilation compilation);
w2s_word_view w2s_compilation_words(w2s_compilation compilation);
w2s_byte_view w2s_compilation_bytes(w2s_compilation compilation);
w2s_string w2s_compilation_glsl(w2s_compilation compilation);
w2s_string w2s_compilation_hlsl(w2s_compilation compilation);
w2s_string w2s_compilation_msl(w2s_compilation compilation);
w2s_string w2s_compilation_wgsl(w2s_compilation compilation);
void w2s_compilation_destroy(w2s_compilation str);

/* CONFIG */
w2s_config w2s_config_new();
void w2s_compilation_confog_destroy(w2s_config str);

/* STRING */
w2s_string w2s_string_clone(w2s_string str);
void w2s_string_destroy(w2s_string str);

/* ERRORS */
w2s_string w2s_take_last_error_message();

#ifdef __cplusplus
}
#endif

#endif
