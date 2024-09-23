#!/bin/bash

nasm $1 -o orig
cargo run orig > disassembled.asm
nasm disassembled.asm -o new
diff new orig
rm orig disassembled.asm new
