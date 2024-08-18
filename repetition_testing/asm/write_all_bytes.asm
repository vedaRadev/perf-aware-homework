    global mov_all_bytes_asm
    global nop_all_bytes_asm
    global nop_all_bytes_alt_asm
    global cmp_all_bytes_asm
    global dec_all_bytes_asm

    section .text

    ; Going to link to this in rust and use the c calling convention
    ; rcx - contains buffer length
    ; rdx - pointer to buffer
    ;
    ; This is nearly a one-to-one copy of the assembly extracted for the write_to_all_bytes function
    ; in the repetition tester when compiled using the following:
    ; rustc version 1.80.1
    ; -C opt-level=z (optimize for binary size, no loop vectorization)
    ; release profile (to force elision of overflow/bounds checks wherever possible)
    ;
    ; Slight modifications have been made to allow Rust to call the function using the C calling
    ; convention on x64 windows.
mov_all_bytes_asm:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    mov byte [rdx + rax], al
    inc rax
    jmp .loop_start
.loop_end:
    ret

    ; Same as mov_all_bytes_asm but mov instruction replaced with a nop of similar instruction
    ; length (3 in this case)
nop_all_bytes_asm:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    inc rax
    jmp .loop_start
.loop_end:
    ret

    ; removed the mov instruction entirely instead of replacing it with an instruction of similar
    ; length
cmp_all_bytes_asm:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    inc rax
    jmp .loop_start
.loop_end:
    ret

    ; entirely different function which just decrements to 0 then returns
    ; rcx must not be 0 or it'll underflow
dec_all_bytes_asm:
.loop_start:
    dec rcx
    jnz .loop_start
.loop_end:
    ret
