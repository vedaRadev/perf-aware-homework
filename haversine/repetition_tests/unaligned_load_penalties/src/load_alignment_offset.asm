    global load_with_alignment_offset

    section .text

    ; Written for x64 windows
    ; rcx - size of memory region to read, must be multiple of 256
    ; rdx - number of times to read memory region
    ; r8  - alignment offset
    ; r9  - start of buffer
load_with_alignment_offset:
    align 64
.read_setup:
    mov rax, r9 ; reset to start of buffer
    mov r10, rcx ; reset region read count

.read_region:
    ; The load/store unit on the ryzen 9 5900x can perform at most 2 256-bit
    ; loads per cycle. We have some dependency chains here that can't all be
    ; resolved on a single cycle so issuing 8 256-bit loads here should ensure
    ; we're still loading data instead of just waiting for things to compute.
    ; Also according to the amd family 19h optimization guide, addressing modes
    ; with base + index + displacement are considered complex addressing modes
    ; and require an additional cycle of latency to compute the address.
    vmovdqu ymm0, [rax + r8]
    vmovdqu ymm0, [rax + r8 + 32]
    vmovdqu ymm0, [rax + r8 + 64]
    vmovdqu ymm0, [rax + r8 + 96]
    vmovdqu ymm0, [rax + r8 + 128]
    vmovdqu ymm0, [rax + r8 + 160]
    vmovdqu ymm0, [rax + r8 + 192]
    vmovdqu ymm0, [rax + r8 + 224]

    add rax, 256
    sub r10, 256
    jnz .read_region

    dec rdx
    jnz .read_setup
    ret
