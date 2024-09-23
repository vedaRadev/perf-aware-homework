    global nop_3x1_all_bytes
    global nop_1x3_all_bytes
    global nop_3x3_all_bytes
    global nop_1x9_all_bytes
    global nop_5x3_all_bytes
    global nop_1x15_all_bytes

    section .text

    ; same as nop_all_bytes_asm from write_all_bytes.asm file
nop_3x1_all_bytes:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    inc rax
    jmp .loop_start
.loop_end:
    ret

nop_1x3_all_bytes:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    nop
    nop
    nop
    inc rax
    jmp .loop_start
.loop_end:
    ret

nop_3x3_all_bytes:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    inc rax
    jmp .loop_start
.loop_end:
    ret

nop_1x9_all_bytes:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    inc rax
    jmp .loop_start
.loop_end:
    ret

nop_5x3_all_bytes:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    db 0x0f, 0x1f, 0x00 ; 3-byte NOP
    inc rax
    jmp .loop_start
.loop_end:
    ret

nop_1x15_all_bytes:
    xor rax, rax
.loop_start:
    cmp rcx, rax
    jz .loop_end
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    inc rax
    jmp .loop_start
.loop_end:
    ret
