;name jez-demo
;strategy test's B-field is zero, so JEZ jumps to target instead of falling
;strategy through to the DAT that would otherwise kill the process.
        JEZ.B  target, test
        DAT.F  #0, #0
test    DAT.F  #0, #0
target  JMP    $0
