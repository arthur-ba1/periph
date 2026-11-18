# VT2 Max: interfaces HID (saída do `vt2-probe`, 2026-11-17)

USB `24AE:1423`, produto "Rapoo Gaming Device", fabricante "Rapoo", serial `2607-54L`.

| Interface | Usage page/usage | Report ID | Tipo | Bytes (incl. ID) | Papel provável |
|---|---|---|---|---|---|
| MI_00 | 0x0001/0x0002 | 0x01 | In | 8 | Mouse: 5 botões, X/Y de 16 bits, roda vertical e horizontal |
| MI_01 Col01 | 0x0001/0x0006 | 0x02 | In/Out | 9/2 | Teclado (macros e teclas remapeadas) |
| MI_01 Col02 | 0x0001/0x0006 | 0x03 | In | 19 | Teclado NKRO (bitmap) |
| MI_01 Col03 | 0x000C/0x0001 | 0x05 | In | 3 | Consumer (mídia) |
| MI_01 Col04 | 0x0001/0x0080 | 0x04 | In | 2 | System control (sleep/power) |
| MI_01 Col05 | 0xFF00/0x000E | 0x06 | In/Out | 33 | **Vendor: candidato a canal de comandos** |
| MI_01 Col06 | 0xFF00/0x000F | 0x08 | Feature | 33 | Vendor feature |
| MI_01 Col07 | 0xFF00/0x0010 | 0x09 | Feature | 33 | Vendor feature |
| MI_01 Col08 | 0xFF00/0x0011 | 0x0B | In/Out | 32 | Vendor |
| MI_01 Col09 | 0xFF00/0x0002 | 0x07 | In/Out | 19 | **Vendor: status (envia sozinho)** |
| MI_01 Col0A | 0xFF00/0x0012 | 0x0C | In/Out | 24 | Vendor |
| MI_02 | 0xFF00/0x0001 | — | In/Out | 512 | Vendor de bloco grande (provavelmente firmware/macros) |

## Report 0x07 espontâneo
Durante 8 s parado, o report 0x07 foi recebido 3 vezes, sempre igual:
```
07 20 02 B0 04 B0 04 01 53 00 84 84 00 00 00 00 01 05 00
```
Hipóteses, **ainda não confirmadas**:
- `B0 04` `B0 04`: 0x04B0 = 1200, que seria o DPI X/Y do estágio atual (little-endian).
- `53`: 83, que seria a bateria em %.
- `20 02`: possivelmente tipo da mensagem e subtipo (status).

Para confirmar: trocar o DPI pelo botão do mouse e ver se `B0 04` muda; comparar `53` com o valor do app oficial.
