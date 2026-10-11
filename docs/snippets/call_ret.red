;name call-ret-demo
;strategy sp is the stack-pointer cell: its B-field holds the current stack
;strategy pointer, an address into the core. CALL pushes the return address
;strategy (the instruction after the CALL) at that pointer, advances the
;strategy pointer, and jumps to subr. RET there retreats the pointer and jumps
;strategy back, landing on the JMP just after the CALL.
        CALL   subr, sp
        JMP    $0
subr    RET    sp
sp      DAT.F  #0, #stack
stack   DAT.F  #0, #0
