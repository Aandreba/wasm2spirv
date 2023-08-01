#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "wasm2spirv.h"

void report_and_abort(const char* str) {
    if (str == NULL) {
        const w2s_string err = w2s_take_last_error_message();
        puts(err.ptr);
        w2s_string_destroy(err);
        exit(1);
    } else {
        puts(str);
        exit(1);
    }
}

int main() {
    const int saxpy_config = open("../../examples/saxpy/saxpy.json", O_RDONLY);
    if (saxpy_config < 0) {
        report_and_abort("Error opening config file");
    }

    w2s_config cfg = w2s_config_from_json_fd(saxpy_config);
    close(saxpy_config);
    if (cfg == NULL) report_and_abort(NULL);

    puts("Read config successfully");
}
