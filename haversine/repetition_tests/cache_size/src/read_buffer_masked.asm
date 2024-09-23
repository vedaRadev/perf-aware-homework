    global read_buffer_masked

    section .text

    ; read_buffer_masked
    ;
    ; Written for win64 abi
    ; rcx - buffer size
    ; rdx - address of buffer
    ; r8  - 64-bit address mask
    ;
    ; Given a buffer, its size, and an address mask, read an entire buffer's worth of bytes from
    ; a subsection of the buffer. The address mask should be 2^n - 1. The buffer should ideally be
    ; aligned to the start of a page of memory and it should be a power-of-two size.
    ;
    ; Example:
    ; Assume the buffer starts at address 4096 and is 4096 bytes long (one page). The address mask
    ; is 127 (0x7F i.e. 0b111_1111). The function will read 4096 bytes repeatedly from the address
    ; range [4096, 4224), wrapping to the start of the buffer until 4096 bytes have been read.
    ;
    ; If used properly, the function can be invoked multiple times through the repetition tester
    ; with the same buffer and buffer size but with different power-of-two masks. When the reported
    ; bandwidth changes drastically we'll know that we've hit a cache size limit and forced the
    ; system to go to the next level of cache.

read_buffer_masked:
    align 64
    xor rax, rax
    mov r10, rdx
.loop:
    vmovdqu ymm0, [r10 + 0]
    vmovdqu ymm1, [r10 + 32]
    vmovdqu ymm2, [r10 + 64]
    vmovdqu ymm3, [r10 + 96]

    ; advance read offset
    add rax, 32 * 4
    and rax, r8

    ; update base pointer
    mov r10, rdx
    add r10, rax

    ; dec count and repeat
    sub rcx, 32 * 4
    jnbe .loop
    ret
