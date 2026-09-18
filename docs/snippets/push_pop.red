;name push-pop-demo
;strategy PUSH stores the B-field value found through its first operand (here
;strategy 1234, not an address) at the stack pointer and advances it. POP
;strategy retreats the pointer and writes the popped value into dst's B-field.
        PUSH   val, sp
        POP    dst, sp
        JMP    $0
val     DAT.F  #0, #1234
dst     DAT.F  #0, #0
sp      DAT.F  #0, #stack
stack   DAT.F  #0, #0
