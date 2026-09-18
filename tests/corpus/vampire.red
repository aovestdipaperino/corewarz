        ORG start
pit     SPL.B  $0,      $0
fang    DAT.F  #0,      #-100
start   ADD.AB #7,      $fang
        MOV.I  $pit,    @fang
        DJN.B  $start,  #50
        JMP.B  $pit,    $0
