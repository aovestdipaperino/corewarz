;name shr-demo
;strategy SHR.AB shifts target's B-field (20) right by the immediate count (2),
;strategy leaving 5 there.
        SHR.AB #2, target
        JMP     $0
target  DAT.F   #0, #20
