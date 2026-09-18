; A JMZ-controlled bomber: counts down a loop counter with SUB, uses JMZ
; to detect when the counter hits zero, and otherwise bombs core through a
; postincrementing pointer (`>` addressing mode). Covers JMZ, `>`, and the
; DAT/SUB/MOV/JMP/SPL opcodes already used elsewhere, but with a genuinely
; different control-flow shape than dwarf.red.
        ORG start
count   DAT.F  #0,      #30
bombv   DAT.F  #0,      #0
ptr     DAT.F  #0,      #2000
start   SUB.AB #1,      $count
        JMZ.B  stop,    $count
        MOV.I  $bombv,  >ptr
        JMP.B  start,   $0
stop    SPL.B  $0,      $0
        JMP.B  start,   $0
