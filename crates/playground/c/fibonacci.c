#include <stdint.h>
#include "foreign.h"

uint64_t fib(uint64_t n)
{
    if (n <= 1)
        return n;
    return fib(n - 1) + fib(n - 2);
}

uint64_t zkmain()
{
    uint64_t input = read_public_input();
    return fib(input);
}
