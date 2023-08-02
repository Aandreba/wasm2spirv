#ifndef WASM2SPIRV_CONFIG
#define WASM2SPIRV_CONFIG

#include <stddef.h>
#include <stdint.h>

#include "spirv.h"
#include "wasm2spirv-string.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum w2s_addressing_model_e {
    Logical = 0,
    Physical = 1,
    PhysicalStorageBuffer = 2,
} w2s_addressing_model;

typedef enum w2s_target_platform_e {
    Universal = 0,
    Vulkan = 1
} w2s_target_platform;

typedef enum w2s_capability_model_e {
    Static = 0,
    Dynamic = 1,
} w2s_capability_model;

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

typedef struct w2s_config_s* w2s_config;
typedef struct w2s_config_builder_s* w2s_config_builder;

/* CONFIG */
w2s_config w2s_config_from_json_string(const uint8_t* json, const size_t json_len);
w2s_config w2s_config_from_json_fd(const int json);
w2s_config w2s_config_clone(w2s_config config);
void w2s_config_destroy(w2s_config builder);

/* BUILDER */

w2s_config_builder w2s_config_builder_new(
    const w2s_target target,
    const w2s_capabilities capabilities,
    const w2s_string_view* extensions,
    const size_t extensions_len,
    const w2s_addressing_model addressing_model,
    const SpvMemoryModel memory_model);

w2s_config w2s_config_builder_build(w2s_config_builder builder);
void w2s_config_builder_destroy(w2s_config_builder builder);

#ifdef __cplusplus
}
#endif

#endif
