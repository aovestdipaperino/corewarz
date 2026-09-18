;name jnz-demo
;strategy test's B-field is non-zero, so JNZ jumps to target.
        JNZ.B  target, test
        DAT.F  #0, #0
test    DAT.F  #0, #5
target  JMP    $0
