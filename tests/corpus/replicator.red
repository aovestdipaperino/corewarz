        ORG start
start   SPL.B  $2,      $0
        MOV.I  $-1,     $1
        MOV.I  #1000,   $copy
copy    MOV.I  $0,      $1
        JMP.B  $-1,     $0
