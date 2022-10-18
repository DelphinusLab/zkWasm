LIBS  = -lkernel32 -luser32 -lgdi32 -lopengl32
CFLAGS = -Wall

# Should be equivalent to your list of C files, if you don't build selectively
CFILES = $(wildcard *.c)
CLANG = clang-15
FLAGS = -flto -O3 -nostdlib -fno-builtin -ffreestanding -mexec-model=reactor --target=wasm32 -Wl,--strip-all -Wl,--initial-memory=131072 -Wl,--max-memory=131072 -Wl,--no-entry -Wl,--allow-undefined -Wl,--compress-relocations -Wl,--export-dynamic

WASMS = $(patsubst %.c, %.wasm, $(CFILES))

%.wasm: %.c
	$(CLANG) -o $@ $^ $(FLAGS)

all: $(WASMS)
