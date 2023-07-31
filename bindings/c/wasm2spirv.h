#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* MISC */

typedef struct w2s_string_view_s {
    const uint8_t* ptr;
    const size_t len;
} w2s_string_view;

typedef struct w2s_byte_view_s {
    const uint8_t* ptr;
    const size_t len;
} w2s_byte_view;

typedef struct w2s_word_view_s {
    const uint32_t* ptr;
    const size_t len;
} w2s_word_view;

// A null-terminated UTF-8 string created by wasm2spirv.
typedef struct w2s_string_s {
    const char* ptr;
    // Doesn't include the null character
    const size_t len;
} w2s_string;

/* ALLOC */
typedef struct w2s_allocator_layout_s {
    size_t size;
    uint16_t log_align;
} w2s_allocator_layout;

typedef struct w2s_allocator_s {
    void* (*alloc)(w2s_allocator_layout);
    void (*free)(void*, w2s_allocator_layout);
} w2s_allocator;

/* COMPILATION */
typedef struct w2s_compilation_s* w2s_compilation;
typedef struct w2s_compilation_config_s* w2s_compilation_config;

// Takes ownership of `config`. If an error ocurred, returns null
w2s_compilation w2s_compilation_new(const w2s_compilation_config config, const uint8_t* bytes, const size_t bytes_len);

/* CONFIG */

/* STRING */
w2s_string w2s_string_clone(w2s_string str, w2s_allocator alloc);
void w2s_string_destroy(w2s_string str, w2s_allocator alloc);

/* ERRORS */
w2s_string w2s_take_last_error_message();

#ifdef __cplusplus
}
#endif
