;name jgz-demo
;strategy test's B-field, 5, is non-zero and not negative, so JGZ jumps to
;strategy target.
        JGZ.B  target, test
        DAT.F  #0, #0
test    DAT.F  #0, #5
target  JMP    $0
