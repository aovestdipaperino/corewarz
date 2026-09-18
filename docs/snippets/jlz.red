;name jlz-demo
;strategy test's B-field, 7999, reads as negative (greater than core-size/2),
;strategy so JLZ jumps to target.
        JLZ.B  target, test
        DAT.F  #0, #0
test    DAT.F  #0, #7999
target  JMP    $0
