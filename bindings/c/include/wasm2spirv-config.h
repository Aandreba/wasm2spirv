#ifndef WASM2SPIRV_CONFIG
#define WASM2SPIRV_CONFIG

#include <stddef.h>
#include <stdint.h>

#include "spirv.h"
#include "wasm2spirv.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct w2s_version_s {
    uint8_t major;
    uint8_t minor;
} w2s_version;

typedef struct w2s_target_s {
    w2s_target_platform platform;
    w2s_version version;
} w2s_target;

typedef struct w2s_capabilities_s {
    w2s_capability_model model;
    const SpvCapability* capabilities;
    const size_t capabilities_len;
} w2s_capabilities;

typedef enum w2s_target_platform_e {
    UNIVERSAL = 0,
    VULKAN = 1
} w2s_target_platform;

typedef enum w2s_capability_model_e {
    STATIC = 0,
    DYNAMIC = 1,
} w2s_capability_model;

typedef struct w2s_config_s* w2s_config;
typedef struct w2s_config_builder_s* w2s_config_builder;

w2s_config_builder w2s_config_builder_new(w2s_target target, w2s_capabilities capabilities, const w2s_string_view* extensions, const size_t extensions_len);

#ifdef __cplusplus
}
#endif

#endif
