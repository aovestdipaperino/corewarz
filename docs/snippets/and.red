;name and-demo
;strategy AND.AB masks the immediate value 12 (1100b) against target's
;strategy B-field, 10 (1010b), leaving 8 (1000b) there.
        AND.AB #12, target
        JMP     $0
target  DAT.F   #0, #10
