#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "wasm2spirv.h"

long file_size(FILE* file) {
    const long prev = ftell(file);
    fseek(file, 0L, SEEK_END);
    const long size = ftell(file);
    fseek(file, prev, SEEK_SET);
    return size;
}

w2s_string_view create_string_view(const char* str) {
    const w2s_string_view result = {
        .ptr = (const uint8_t*)str,
        .len = strlen(str)};
    return result;
}

int report_and_abort(const char* str) {
    if (str == NULL) {
        const w2s_string err = w2s_take_last_error_message();
        puts(err.ptr);
        w2s_string_destroy(err);
    } else {
        puts(str);
    }
    return 1;
}

w2s_config manual_saxpy_config() {
    static const uint32_t LOCAL_SIZE[3] = {1, 1, 1};
    static const SpvCapability INITIAL_CAPABILITIES[1] = {SpvCapabilityVariablePointers};

    w2s_version target_version = {
        .major = 1,
        .minor = 1};

    w2s_target target = {
        .platform = W2STargetPlatformVulkan,
        .version = target_version};

    w2s_capabilities capabilities = {
        .model = W2SCapabilityModelDynamic,
        .capabilities = &INITIAL_CAPABILITIES,
        .capabilities_len = 1};

    w2s_string_view variable_pointers_ext = create_string_view("VH_KHR_variable_pointers");

    w2s_config_builder builder = w2s_config_builder_new(target, capabilities, &variable_pointers_ext, 1, W2SAddressingModelLogical, SpvMemoryModelGLSL450);
    if (builder == NULL) exit(report_and_abort(NULL));

    w2s_function_config_builder saxpy_builder = w2s_function_config_builder_new();
    if (w2s_function_config_builder_add_execution_mode(saxpy_builder, SpvExecutionModeLocalSize, &LOCAL_SIZE, sizeof(LOCAL_SIZE)) == false)
        exit(report_and_abort(NULL));

    return w2s_config_builder_build(builder);
}

int main() {
    const int saxpy_config = open("../../examples/saxpy/saxpy.json", O_RDONLY);
    if (saxpy_config < 0) return report_and_abort("Error opening config file");

    w2s_config config = w2s_config_from_json_fd(saxpy_config);
    close(saxpy_config);
    if (config == NULL) return report_and_abort(NULL);
    puts("Read config successfully");

    FILE* saxpy_file = fopen("../../examples/saxpy/saxpy.wasm", "rb");
    const size_t saxpy_byte_count = file_size(saxpy_file);

    uint8_t* saxpy_bytes = (uint8_t*)malloc(saxpy_byte_count);
    const size_t res = fread(saxpy_bytes, sizeof(uint8_t), saxpy_byte_count, saxpy_file);
    if (res != saxpy_byte_count) return report_and_abort("Error reading wasm file");
    if (fclose(saxpy_file) < 0) return report_and_abort("Error closing wasm file");

    w2s_compilation compilation = w2s_compilation_new(config, saxpy_bytes, saxpy_byte_count);
    if (compilation == NULL) return report_and_abort(NULL);
    free(saxpy_bytes);

    const w2s_string assembly = w2s_compilation_assembly(compilation);
    if (assembly.ptr == NULL) return report_and_abort(NULL);
    printf("%s\n", assembly.ptr);
    w2s_string_destroy(assembly);

    const w2s_string msl = w2s_compilation_msl(compilation);
    if (msl.ptr == NULL) return report_and_abort(NULL);
    printf("%s\n", msl.ptr);
    w2s_string_destroy(msl);

    w2s_compilation_destroy(compilation);
}
