    global read_buffer_multiple_times

    section .text

    ; Written for x64 windows
    ; rcx - start of buffer (may or may not be aligned)
    ; rdx - size of memory region to read, must be a multiple of 128
    ; r8  - number of times to read memory region
read_buffer_multiple_times:
    align 64
.read_setup:
    mov rax, rcx ; reset to start of buffer
    mov r10, rdx ; reset bytes to read from sub region

.read_region:
    ; The load/store unit on the ryzen 9 5900x can perform at most 2 256-bit
    ; loads per cycle. We have some dependency chains here that can't all be
    ; resolved on a single cycle so issuing 4 256-bit loads here should ensure
    ; we're still loading data instead of just waiting for things to compute.
    ;
    ; Here's my thinking (and my dependency chain analysis could be wrong):
    ; 1st pair of loads happens on cycle A
    ; 2nd pair of loads happens on cycle B
    ; add and sub instructions are independent and thus can happen on cycle A
    ; jnz depends on the result of the sub and thus happens on cycle B, takes 1 cycle to jump (zen3 x64 jmp has 1-cycle latency)
    ; 
    ; With smaller region sizes we may see slight performance hits because we're going to be hitting
    ; the outer loop more often and doing renames of our registers in the RAT. Maybe that takes an
    ; additional cycle? We already know that our loads depend on rax so sometimes it might take 3
    ; cycles to do a loop including the outer loop setup?
    vmovdqu ymm0, [rax]
    vmovdqu ymm0, [rax + 32]
    vmovdqu ymm0, [rax + 64]
    vmovdqu ymm0, [rax + 96]

    add rax, 128
    sub r10, 128
    jnz .read_region

    dec r8
    jnz .read_setup
    ret
