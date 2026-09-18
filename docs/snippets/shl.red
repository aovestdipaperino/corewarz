;name shl-demo
;strategy SHL.AB shifts target's B-field (5) left by the immediate count (2),
;strategy leaving 20 there.
        SHL.AB #2, target
        JMP     $0
target  DAT.F   #0, #5
