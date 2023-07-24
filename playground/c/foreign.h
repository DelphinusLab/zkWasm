#ifndef FOREIGN_H
#define FOREIGN_H

#include <stdint.h>

extern uint64_t wasm_input(uint32_t);
extern void require(uint32_t);

static inline uint64_t read_public_input()
{
	return wasm_input(1);
}

static inline uint64_t read_private_input()
{
	return wasm_input(0);
}

#endif
