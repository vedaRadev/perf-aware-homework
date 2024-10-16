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

    ; The following routines were taken from Casey's implementation because I wasn't able to
    ; construct a situation where bypassing cache hierarchy was faster than going through it. I
    ; kept overcomplicating things... :(

    ; rcx - start of input buffer
    ; rdx - start of output buffer
    ; r8  - buffer sizes (must be equal and the a multiple of 128)
    ; r9  - number of times to read buffers

temporal_stores:
    align 64

.setup:
    mov r10, rcx
    mov r11, rdx
    mov rax, r8

.read_write_buffers:
    vmovdqu ymm0, [r10]
    vmovdqu ymm1, [r10 + 0x20]
    vmovdqu ymm2, [r10 + 0x40]
    vmovdqu ymm3, [r10 + 0x60]
    vmovdqu [r11], ymm0
    vmovdqu [r11 + 0x20], ymm1
    vmovdqu [r11 + 0x40], ymm2
    vmovdqu [r11 + 0x60], ymm3
    add r10, 0x80
    add r11, 0x80
    sub rax, 0x80
    jnz .read_write_buffers

    dec r9
    jnz .setup
    ret

nontemporal_stores:
    align 64

.setup:
    mov r10, rcx
    mov r11, rdx
    mov rax, r8

.read_write_buffers:
    vmovdqu ymm0, [r10]
    vmovdqu ymm1, [r10 + 0x20]
    vmovdqu ymm2, [r10 + 0x40]
    vmovdqu ymm3, [r10 + 0x60]
    vmovntdq [r11], ymm0
    vmovntdq [r11 + 0x20], ymm1
    vmovntdq [r11 + 0x40], ymm2
    vmovntdq [r11 + 0x60], ymm3
    add r10, 0x80
    add r11, 0x80
    sub rax, 0x80
    jnz .read_write_buffers

    dec r9
    jnz .setup
    ret

