    global write_x1
    global write_x2
    global write_x3
    global write_x4

    section .text
    
    ; functions written for use with win64 ABI

write_x1:
    align 64
.loop:
    mov [rdx], rcx
    sub rcx, 1
    jnle .loop
    ret

write_x2:
    align 64
.loop:
    mov [rdx], rcx
    mov [rdx], rcx
    sub rcx, 2
    jnle .loop
    ret

write_x3:
    align 64
.loop:
    mov [rdx], rcx
    mov [rdx], rcx
    mov [rdx], rcx
    sub rcx, 3
    jnle .loop
    ret

write_x4:
    align 64
.loop:
    mov [rdx], rcx
    mov [rdx], rcx
    mov [rdx], rcx
    mov [rdx], rcx
    sub rcx, 4
    jnle .loop
    ret
