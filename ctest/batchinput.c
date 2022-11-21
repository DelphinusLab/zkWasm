#include <stdint.h>
#include <stdbool.h>
#if defined(__wasm__)

void assert(int cond)
{
    if (!cond) __builtin_unreachable();
}
#endif

unsigned long long wasm_input(int);

/* Convert list of u64 into bytes */
static __inline__ void read_bytes_from_u64(void *dst, int byte_length, bool is_public) {
    uint64_t *dst64 = (uint64_t*) dst;
    #pragma clang loop unroll(full)
    for (int i = 0; i * 8 < byte_length; i++) {
        if (i*8 + 8 < byte_length) {
            dst64[i] = wasm_input(is_public);
        } else {
            //less then 16 bytes on demand
            uint64_t uint64_cache = wasm_input(is_public);
            uint8_t *u8_p = (uint8_t *)uint64_cache;
            #pragma clang loop unroll(full)
            for (int j = i*8; j<byte_length; j++) {
              ((uint8_t *)dst)[j] = u8_p[j-i*8];
            }
        }
    }
}

__attribute__((visibility("default")))
int zkmain() {
    uint8_t bytes[8];
    read_bytes_from_u64(bytes, 8, 0);
    assert(bytes[7] == 1);
    return 0;
}
