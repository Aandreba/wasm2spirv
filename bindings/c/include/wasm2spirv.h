#ifndef WASM2SPIRV
#define WASM2SPIRV

#include <stddef.h>
#include <stdint.h>

#include "wasm2spirv-config.h"
#include "wasm2spirv-string.h"

#ifdef __cplusplus
extern "C" {
#endif

/* COMPILATION */
typedef struct w2s_compilation_s* w2s_compilation;

void* w2s_malloc(size_t size, uint16_t log2_align);
void w2s_free(void* ptr, size_t size, uint16_t log2_align);

// Takes ownership of `config`. If an error ocurred, returns null.
w2s_compilation w2s_compilation_new(const w2s_config config, const uint8_t* bytes, const size_t bytes_len);
w2s_compilation w2s_compilation_optimized(w2s_compilation compilation);
w2s_string w2s_compilation_assembly(w2s_compilation compilation);
w2s_word_view w2s_compilation_words(w2s_compilation compilation);
w2s_byte_view w2s_compilation_bytes(w2s_compilation compilation);
w2s_string w2s_compilation_glsl(w2s_compilation compilation);
w2s_string w2s_compilation_hlsl(w2s_compilation compilation);
w2s_string w2s_compilation_msl(w2s_compilation compilation);
w2s_string w2s_compilation_wgsl(w2s_compilation compilation);
void w2s_compilation_destroy(w2s_compilation str);

/* ERRORS */
typedef struct w2s_panic_location_s {
    w2s_string_view file;
    uint32_t line;
    uint32_t column;
} w2s_panic_location;

typedef struct w2s_panic_info_s {
    w2s_string_view payload;
    w2s_panic_location location;
} w2s_panic_info;

w2s_string w2s_take_last_error_message();
void w2s_set_panic_handler(void (*f)(w2s_panic_info, void*), void* user_data);
void w2s_set_imported_panic_handler(void* user_data);

#ifdef __cplusplus
}
#endif

#endif
