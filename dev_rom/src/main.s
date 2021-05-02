;
; Template based on
; Russian Roulette game for NES
; Copyright 2010 Damian Yerrick
;
; Copying and distribution of this file, with or without
; modification, are permitted in any medium without royalty
; provided the copyright notice and this notice are preserved.
; This file is offered as-is, without any warranty.
;
.include "src/nes.h"
.p02

.segment "ZEROPAGE"

.segment "VECTORS"
.addr nmi, reset, irq

.segment "CODE"
end:
  bit PPUSTATUS
  bpl end
  
  lda #$3F
  sta PPUADDR
  lda #$01
  sta PPUADDR
  
  lda #$0f
  sta PPUDATA
  lda #$28
  sta PPUDATA
  lda #$28
  sta PPUDATA
  
  lda #0
  sta $21; state
 
start_timer:
  lda #0
  sta $20; Frame counter

timer:
  bit PPUSTATUS
  bpl timer
  
  inc $20
  lda $20
  
  cmp #$30
  bne timer
 
wait_before_nametable:
  bit PPUSTATUS
  bpl wait_before_nametable
  lda #$21
  sta PPUADDR
  lda #$ce
  sta PPUADDR
  
  lda $21
  cmp $1
  beq beers_away
  
  lda #0
  sta PPUDATA
  lda #1
  sta PPUDATA
  lda #2
  sta PPUDATA
  lda #0
  sta PPUDATA
  jmp beer_end
  
beers_away:
  lda #1
  sta PPUDATA
  lda #0
  sta PPUDATA
  lda #0
  sta PPUDATA
  lda #2
  sta PPUDATA
  
  beer_end:
  lda #1
  eor $21
  sta $21
  jsr changePallette
  
  jmp start_timer

; we don't use irqs yet
.proc irq
  rti
.endproc

.proc nmi
  rti
.endproc

.proc reset
  sei

  ; Acknowledge and disable interrupt sources during bootup
  ldx #0
  stx PPUCTRL    ; disable vblank NMI
  stx PPUMASK    ; disable rendering (and rendering-triggered mapper IRQ)
  lda #$40
  sta $4017      ; disable frame IRQ
  stx $4010      ; disable DPCM IRQ
  bit PPUSTATUS  ; ack vblank NMI
  bit $4015      ; ack DPCM IRQ
  cld            ; disable decimal mode to help generic 6502 debuggers
                 ; http://magweasel.com/2009/08/29/hidden-messagin/
  dex            ; set up the stack
  txs

  ; Wait for the PPU to warm up (part 1 of 2)
vwait1:
  bit PPUSTATUS
  bpl vwait1

  ; While waiting for the PPU to finish warming up, we have about
  ; 29000 cycles to burn without touching the PPU.  So we have time
  ; to initialize some of RAM to known values.
  ; Ordinarily the "new game" initializes everything that the game
  ; itself needs, so we'll just do zero page and shadow OAM.
  ldy #$00
  lda #$F0
  ldx #$00
clear_zp:
  sty $00,x
  inx
  bne clear_zp
  ; the most basic sound engine possible
  lda #$0F
  sta $4015

  ; Wait for the PPU to warm up (part 2 of 2)
vwait2:
  bit PPUSTATUS
  bpl vwait2

  ; Turn screen on
  lda #0
  sta PPUSCROLL
  sta PPUSCROLL
  lda #VBLANK_NMI|BG_0000
  sta PPUCTRL
  lda #BG_ON
  sta PPUMASK
  
  ; Set initial data
  lda #0 
  sta 0  ; value
  lda #$0   
  sta 1  ; Address LSB
  lda #$2
  sta 2  ; Address MSB
  lda #0
  sta 3 ; Background switcher
  
  lda #$11
  sta $10  ; Background color1
  lda #$2A
  sta $11  ; Background color2
  lda #$2C
  sta $12  ; Background color3
  lda #$25
  sta $13  ; Background color4

mainLoop:
  ; Input feed
  lda #$1
  sta $4016
  lda #$0
  sta $4016
  
  ; read A: Increment mem
  lda $4016
  and #1
  bne a_button
  
  ; read B: Next byte
  lda $4016
  and #1
  bne b_button
  
  ; read Start: Hail Mary
  lda $4016
  lda $4016
  and #1
  bne start_button
  jmp mainLoop
  
  a_button:
  inc 0
  a_button_wait_ppu:
  bit PPUSTATUS
  bpl a_button_wait_ppu
  jsr changePallette
  ; Wait for keyup
  keyup_a_loop:
  lda #$1
  sta $4016
  lda #$0
  sta $4016
  lda $4016
  and #1
  bne keyup_a_loop
  jmp mainLoop
  
  b_button:
  lda 0
  ldx 1
  ldy 2
  jsr choosePage
  lda #0
  sta 0
  jsr crossPage
  b_button_wait_ppu:
  bit PPUSTATUS
  bpl b_button_wait_ppu
  jsr changePallette
  keyup_b_loop:
  lda #$1
  sta $4016
  lda #$0
  sta $4016
  lda $4016
  lda $4016
  and #1
  bne keyup_b_loop
  jmp mainLoop
  
  start_button:
  ; Vulnerable part
  lda #$6c; JmpInd
  sta 0
  
  lda #0
  ldx 1
  ldy 2
  jsr choosePage
  
  lda #$c0
  ldy 2
  ldx 1
  inx
  bne dont_increase_page
  iny
  dont_increase_page:
  jsr choosePage
  jmp 0
.endproc


.proc cls
  lda #VBLANK_NMI
  sta PPUCTRL
  lda #$20
  ldx #$00
  stx PPUMASK
  sta PPUADDR
  stx PPUADDR
  ldx #240
:
  sta PPUDATA
  sta PPUDATA
  sta PPUDATA
  sta PPUDATA
  dex
  bne :-
  ldx #64
  lda #0
:
  sta PPUDATA
  dex
  bne :-
  rts
.endproc

.proc crossPage
  inc 1
  bne cross_page_end
  inc 2
  lda 2
  cmp #4
  beq jmp_end
  cross_page_end:
  rts
  jmp_end:
  jmp end
.endproc

.proc choosePage
  cpy #$3
  beq page_300
  sta $0200,X
  jmp page_condition_end
  page_300:
  sta $0300,X
  page_condition_end:
  rts
.endproc

.proc changePallette   
  ; set monochrome palette
  lda #$3F
  sta PPUADDR
  lda #$00
  sta PPUADDR
   
  lda #3; Mask
  and 3
  tax
  lda $10,X
  sta PPUDATA
  
  inc 3
  
  rts
   
.endproc

.segment "RODATA"
flag:
  .byt "FLAG-{db54945cfbee518299963df092f7e98f26ac4754}",0
