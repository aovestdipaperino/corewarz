; A scanning bomber that walks the core with a growing step, testing the
; scanned cell's value directly with JMN (the mirror image of scanner.red's
; SEQ-based test), and covers the arithmetic and addressing modes the rest
; of the corpus never uses: MUL/DIV/MOD, SLT/SNE, NOP, the
; predecrement/postincrement A-field modes (`{` `}`), and the B-field
; predecrement mode (`<`).
        ORG start
step    EQU  13
wrap    EQU  4001
half    DAT.F  #0,      #0
scan    DAT.F  #0,      #0
bombv   DAT.F  #0,      #0
fptr    DAT.F  #3000,   #0
bptr    DAT.F  #4000,   #0
dptr    DAT.F  #0,      #5000
start   ADD.AB #step,   $scan
        MOD.AB #wrap,   $scan
        MUL.AB #1,      $scan
        DIV.AB #2,      $half
        JMN.B  hit,      $scan
        NOP.B  $0,       $0
        JMP.B  start,    $0
hit     SLT.AB #0,       $scan
        JMP.B  start,    $0
        SNE.AB #0,       $half
        MOV.I  $bombv,   {fptr
        MOV.I  $bombv,   }bptr
        MOV.I  $bombv,   <dptr
        JMP.B  start,    $0
