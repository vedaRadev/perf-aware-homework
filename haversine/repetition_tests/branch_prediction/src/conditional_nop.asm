    global conditional_nop

    section .text

    ; Uses windows 64-bit ABI.
    ; I basically just copied Casey's code here instead of using the ASM that Rust
    ; generated then I modified (nop_loops.asm) because we're testing the branch
    ; predictor here. My previous loops have a branch at the start instead of at the
    ; end, so not sure if that would make much of a difference or not.
    ; TODO See if it makes a difference or not.
conditional_nop:
    xor rax, rax
.loop:
    mov r10, [rdx + rax]
    inc rax
    test r10, 1
    jnz .skip
    nop
.skip:
    cmp rax, rcx
    jb .loop
    ret
