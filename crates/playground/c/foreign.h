#ifndef FOREIGN_H
#define FOREIGN_H

#include <stdint.h>

/* Wasm Input Plugin */
extern uint64_t wasm_input(uint32_t);

static inline uint64_t read_public_input()
{
	return wasm_input(1);
}

static inline uint64_t read_private_input()
{
	return wasm_input(0);
}

/* Require Plugin */
extern void require(uint32_t);

/* Context Cont Plugin */
extern uint64_t wasm_read_context();
extern void wasm_write_context(uint64_t);

#endif
