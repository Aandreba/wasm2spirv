#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "wasm2spirv.h"

long file_size(FILE* file) {
    const long prev = ftell(file);
    fseek(file, 0L, SEEK_END);
    const long size = ftell(file);
    fseek(file, prev, SEEK_SET);
    return size;
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

int main() {
    const int saxpy_config = open("../../examples/saxpy/saxpy.json", O_RDONLY);
    if (saxpy_config < 0) return report_and_abort("Error opening config file");

    w2s_config config = w2s_config_from_json_fd(saxpy_config);
    close(saxpy_config);
    if (config == NULL) return report_and_abort(NULL);
    puts("Read config successfully");

    FILE* saxpy_file = fopen("tests/saxpy.wasm", "rb");
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
