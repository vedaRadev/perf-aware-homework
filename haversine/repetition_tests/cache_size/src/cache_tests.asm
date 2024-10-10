    global read_buffer_power_of_two_mask
    global read_buffer_non_power_of_two

    section .text

    ; Written for win64 abi
    ; rcx - buffer size
    ; rdx - address of buffer
    ; r8  - 64-bit address mask (2^n - 1)
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
read_buffer_power_of_two_mask:
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

    ; Written for win64 abi
    ; rcx   - address of start of full buffer
    ; rdx   - size of the sub-buffer region in bytes (must be a multiple of 128)
    ; r8    - how many times to read the sub-buffer
read_buffer_non_power_of_two:
    align 64
.sub_buffer_read_setup:
    mov r10, rcx
    mov r11, rdx

.sub_buffer_read:
    vmovdqu ymm0, [r10 + 0]
    vmovdqu ymm1, [r10 + 32]
    vmovdqu ymm2, [r10 + 64]
    vmovdqu ymm3, [r10 + 96]

    ; advance read offset
    add r10, 32 * 4
    sub r11, 32 * 4
    jnz .sub_buffer_read

    ; if here then we've finished a repetition of reading through the sub-buffer.
    dec r8
    jnz .sub_buffer_read_setup
    ret
