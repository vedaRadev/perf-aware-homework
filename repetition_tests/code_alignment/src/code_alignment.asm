    global loop_aligned_64
    global loop_aligned_1
    global loop_aligned_15
    global loop_aligned_31
    global loop_aligned_63

    section .text

    ; ASM routines here are written for 64-bit Windows ABI.
    ; Param 1 - RAX
    ; Param 2 - RDX

    ; Align to start of 64-byte boundary
loop_aligned_64:
    xor rax, rax
align 64
.loop:
    inc rax
    cmp rax, rcx
    jb .loop
    ret

    ; Align 1 into 64 byte boundary
loop_aligned_1:
    xor rax, rax
align 64
    nop
.loop:
    inc rax
    cmp rax, rcx
    jb .loop
    ret

    ; Align 15 into 64 byte boundary
loop_aligned_15:
    xor rax, rax
align 64
%rep 15
    nop
%endrep
.loop:
    inc rax
    cmp rax, rcx
    jb .loop
    ret

    ; Align 31 into 64 byte boundary
loop_aligned_31:
    xor rax, rax
align 64
%rep 31
    nop
%endrep
.loop:
    inc rax
    cmp rax, rcx
    jb .loop
    ret

    ; Align 63 into 64 byte boundary
loop_aligned_63:
    xor rax, rax
align 64
%rep 63
    nop
%endrep
.loop:
    inc rax
    cmp rax, rcx
    jb .loop
    ret
