;name or-demo
;strategy OR.AB sets bits: 12 (1100b) | 10 (1010b) = 14 (1110b).
        OR.AB  #12, target
        JMP     $0
target  DAT.F   #0, #10
