bomb    DAT.F  $0,     $0
start   ADD.AB #4,     $bomb
        MOV.I  $bomb,  @bomb
        JMP.B  $start, $0
        END start
