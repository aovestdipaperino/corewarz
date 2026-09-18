;name dwarf
;author A. K. Dewdney
;strategy Bombs the core at four-cell intervals.
bomb    DAT.F  $0,    $0
start   ADD.AB #4,    $bomb
        MOV.I  $bomb, @bomb
        JMP.B  $start, $0
        END start
