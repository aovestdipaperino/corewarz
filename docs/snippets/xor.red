;name xor-demo
;strategy XOR.AB toggles bits: 12 (1100b) ^ 10 (1010b) = 6 (0110b).
        XOR.AB #12, target
        JMP     $0
target  DAT.F   #0, #10
