#include <stdint.h>
void require(int cond);
void print(uint64_t value);

unsigned long long wasm_input(int);
__attribute__((visibility("default"))) int zkmain()
{
    uint32_t a = (uint32_t)wasm_input(1);
    uint32_t b = (uint32_t)wasm_input(1);
    require(a > b);
    print(a > b);
    return a + b;
}
