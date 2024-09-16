    global read_4x3
    global read_8x3
    global read_16x3
    global read_32x3

    section .text

    ; The following functions are written for use with the win64 ABI.
    ; i.e. rcx is first param, rdx is second param.

    ; The following functions were also written with a Zen 3 AMD (family 19h)
    ; processor in mind. The Zen 3 has a max of 3 read execution ports, so the
    ; following functions do 3 loads per loop iteration.

    ; All functions expect rdx to be a pointer to an array. Regardless of the
    ; size of that array (given in rcx), we are never going to advance our data
    ; pointer so that we remain in the L1 data cache. Also notice that our loops
    ; are aligned to 64-byte cache line boundaries to stay within the L1
    ; instruction cache.
    ; Basically that's the long way of saying that we are trying to filter out
    ; the noise of cache misses when doing the repetition tests.

    ; Note that these functions behave as if they're reading multiple 1-byte values into
    ; differently-sized registers. For example, in read_4x3 `mov r8d, [rdx]` behaves as if it's
    ; loading 4 1-byte values into a 32-bit register. Because of this, the repetition tester may
    ; report extremely high throughput numbers compared to previous tests such as the probe read
    ; exec port tests.

    ; 3 4-byte (32-bit) reads
read_4x3:
    xor rax, rax
    align 64
.loop:
    mov r8d, [rdx]
    mov r8d, [rdx + 4]
    mov r8d, [rdx + 8]
    add rax, 4 * 3
    cmp rax, rcx
    jb .loop
    ret

    ; 3 8-byte (64-bit) reads
read_8x3:
    xor rax, rax
    align 64
.loop:
    mov r8, [rdx]
    mov r8, [rdx + 8]
    mov r8, [rdx + 16]
    add rax, 8 * 3
    cmp rax, rcx
    jb .loop
    ret

    ; 3 16-byte (128-bit) reads
read_16x3:
    xor rax, rax
    align 64
.loop:
    vmovdqu xmm0, [rdx]
    vmovdqu xmm0, [rdx + 16]
    vmovdqu xmm0, [rdx + 32]
    add rax, 16 * 3
    cmp rax, rcx
    jb .loop
    ret

    ; 3 32-byte (256-bit) reads
read_32x3:
    xor rax, rax
    align 64
.loop:
    vmovdqu ymm0, [rdx]
    vmovdqu ymm0, [rdx + 32]
    vmovdqu ymm0, [rdx + 64]
    add rax, 32 * 3
    cmp rax, rcx
    jb .loop
    ret
