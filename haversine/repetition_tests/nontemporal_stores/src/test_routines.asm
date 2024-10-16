    global temporal_stores
    global nontemporal_stores

    section .text

    ; The following routines were written for the x64 windows abi
    ;
    ; For my own future reference:
    ; It seems like on windows x64 the stack grows down from high to low. Therefore, pushing rbp to
    ; the stack will subtract 8 bytes from the stack pointer. Then we need to add 32 + 8 (b/c we
    ; just pushed rbp) = 40 to the base pointer to get to our first stack-based parameter.
    ;
    ; The above note was placed here when I was still planning on writing the ASM routines to make
    ; sure they don't go off the end of the output buffer, but I instead decided to just have the
    ; caller make sure that the number of 32-byte writes per input value times the size of the input
    ; buffer doesn't blow past the end of the output buffer. This knocked the number of params from
    ; 5 down to 4.


    ; Write duplicates of each value in the input buffer to the output buffer, going through the
    ; cache hierarchy.
    ;
    ; rcx - start of input buffer
    ; rdx - size of input buffer (must be a multiple of 128)
    ; r8  - start of output buffer
    ; r9  - number of 32-byte writes per input value (must be a multiple of 128)
    ;
    ; INVARIANT: # of 32-byte writes per input value * size of input buffer cannot be more than the
    ; size of the output buffer or bad things may happen.
temporal_stores:
    align 64

.read_from_input:
    vmovdqu ymm0, [rcx]
    vmovdqu ymm1, [rcx + 32]
    vmovdqu ymm2, [rcx + 64]
    vmovdqu ymm3, [rcx + 96]
    mov rax, r9

.write_to_output:
    ; ryzen 9 5900x can do 1 32-byte store per cycle.
    ; Doing 4 32-byte stores here should be more than enough to cover the 2-cycle (I think) cost of
    ; the loop overhead (add, sub, jnz)
    vmovdqu [r8], ymm0
    vmovdqu [r8 + 32], ymm1
    vmovdqu [r8 + 64], ymm2
    vmovdqu [r8 + 96], ymm3
    add r8, 128
    sub rax, 128
    jnz .write_to_output

    sub rdx, 128
    jnz .read_from_input

    ret

    ; Write duplicates of each value in the input buffer to the output buffer, bypassing the cache
    ; hierarchy.
    ;
    ; rcx - start of input buffer
    ; rdx - size of input buffer (must be a multiple of 128)
    ; r8  - start of output buffer
    ; r9  - number of 32-byte writes per input value (must be a multiple of 128)
    ;
    ; INVARIANT: # of 32-byte writes per input value * size of input buffer cannot be more than the
    ; size of the output buffer or bad things may happen.
nontemporal_stores:
    align 64

.read_from_input:
    vmovdqu ymm0, [rcx]
    vmovdqu ymm1, [rcx + 32]
    vmovdqu ymm2, [rcx + 64]
    vmovdqu ymm3, [rcx + 96]
    mov rax, r9

.write_to_output:
    ; ryzen 9 5900x can do 1 32-byte store per cycle.
    ; Doing 4 32-byte stores here should be more than enough to cover the 2-cycle (I think) cost of
    ; the loop overhead (add, sub, jnz)
    vmovntdq [r8], ymm0
    vmovntdq [r8 + 32], ymm1
    vmovntdq [r8 + 64], ymm2
    vmovntdq [r8 + 96], ymm3
    add r8, 128
    sub rax, 128
    jnz .write_to_output

    sub rdx, 128
    jnz .read_from_input

    ret
