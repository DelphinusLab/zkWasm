#include <stdint.h>
unsigned long long wasm_input(int);
__attribute__((visibility("default")))
int zkmain() {
    uint32_t a = (uint32_t)wasm_input(0);
    uint32_t b = (uint32_t)wasm_input(0);
    return a+b;
}
