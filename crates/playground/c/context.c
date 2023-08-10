#include <stdint.h>>
#include "foreign.h"

__attribute__((visibility("default")))
uint64_t
zkmain()
{
    uint64_t v1 = wasm_read_context();
    uint64_t v2 = wasm_read_context();

    uint64_t r = v1 + v2;
    wasm_write_context(r);
    wasm_write_context(r);

    return r;
}