;name not-demo
;strategy NOT.B complements target's B-field in place; the source operand's
;strategy value is ignored.
        NOT.B  $0, target
        JMP     $0
target  DAT.F   #0, #7
