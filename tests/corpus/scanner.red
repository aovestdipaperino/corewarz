        ORG start
step    EQU 10
scan    DAT.F  #0,      #0
start   SUB.AB #step,   $scan
        SEQ.I  *scan,   @scan
        JMP.B  $attack, $0
        JMP.B  $start,  $0
attack  MOV.AB #0,      @scan
        JMP.B  $start,  $0
