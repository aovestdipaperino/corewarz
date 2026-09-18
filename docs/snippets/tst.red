;name tst-demo
;strategy TST.AB compares the immediate 3 against target's B-field, 9. 3 < 9,
;strategy so target's B-field becomes -1 (core-size minus one).
        TST.AB #3, target
        JMP     $0
target  DAT.F   #0, #9
