EXE ?= engine
CC = cargo
BIN ?= rusty_engine

ifeq ($(OS),Windows_NT)
    OUTPUT := $(EXE).exe
else
    OUTPUT := $(EXE)
endif

.PHONY: all clean

all:
	$(CC) rustc --release --bin $(BIN) -- --emit link="$(OUTPUT)"

clean:
	$(CC) clean