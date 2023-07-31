#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct w2s_compilation_s* w2s_compilation_t;

w2s_compilation_t w2s_compilation_new(const uint8_t* bytes, const size_t bytes_len);

#ifdef __cplusplus
}
#endif
