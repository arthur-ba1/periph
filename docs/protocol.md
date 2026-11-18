# Protocolo de configuração do Rapoo VT2 Max (24AE:1423)

Descoberto em 2026-11-17 interceptando as chamadas hidapi do A HUB v1.0.19 (`RapooDevice.dll`)
com `tools/hidhook.py`. O log bruto não é publicado (contém dados do aparelho); os trechos usados estão nos testes de `vt2-proto`. Firmware do mouse V0.61, dongle V0.41-V1.6.

## IDs USB
- `24AE:1423`: dongle 2.4 GHz.
- `24AE:4623`: mouse ligado pelo cabo USB. As interfaces e reports são idênticos, e leitura e escrita foram testadas.
- Quando o dongle responde mas o mouse não (dormindo ou fora de alcance), a resposta vem com status `0x00` e ecoa o comando sem dados.

## Transporte
- Interface **MI_01, coleção usage page 0xFF00 / usage 0x000E**, report ID **0x06**.
  - Comando: `hid_write` de report de saída 0x06 (33 bytes; o A HUB manda só os bytes úteis, e o resto é zero).
  - Resposta: `hid_get_input_report` do report 0x06 (33 bytes), repetido até ficar pronto.
- Status espontâneo: report **0x07** (MI_01, usage 0xFF00/0x0002, 19 bytes), lido com `hid_read_timeout`, ~a cada 3 s e quando algo muda.

## Formato do comando (report 0x06)
```
06 A5 <op> <len> <addr_lo> <addr_hi> 00 00 [data × len]
```
- `op = A4`: ler `len` bytes a partir de `addr`.
- `op = A5`: escrever `len` bytes (`data`) em `addr`.
- O A HUB sempre faz **ler → modificar → escrever** no mesmo bloco de 4 bytes.
- `len` máximo visto: 0x18 (24) em leitura e 0x14 (20) em escrita.

Outros comandos vistos: `06 B0 00` e `06 B0 01` (leitura de informações; respostas `01 29 00 00 00 23 14 C8…` e `01 CE 00 00 00 23 14 05…`, ainda não decodificadas).

## Formato da resposta (input report 0x06, como o hook mostrou)
```
<st> 00 00 00 [data...]
```
- `st = 02`: ocupado. Ler de novo (o A HUB repete a cada ~2 ms, por até ~0,5 s).
- `st = 01`: pronto. Os dados lidos começam no byte 4. Em escrita, o resto vem zerado.
- ⚠ O byte 0 no buffer do hidapi deveria ser o report ID (0x06). O hook mostra `01`/`02` nessa posição, então a DLL pode estar removendo o ID. Isso precisa ser confirmado no `vt2-device`.

## Report de status 0x07
```
07 20 SS XL XH YL YH 01 BB 00 PP PP 00 00 00 00 ?? ?? 00
```
| Byte | Significado |
|---|---|
| 1 | `0x20` = sem fio, `0x22` = cabo (bit 1) |
| 2 | estágio de DPI ativo (índice a partir de 0) |
| 7 | `0x01` = na bateria, `0x02` = carregando |
| 3–4 | DPI X (u16 LE) |
| 5–6 | DPI Y (u16 LE) |
| 8 | bateria % (0x53 = 83) |
| 10–11 | código de polling rate (mesmo de 0x0880) |
| 16–17 | alterna `06 04` / `01 05` (desconhecido) |

## Mapa de memória
| Endereço | Tamanho | Conteúdo |
|---|---|---|
| 0x0600 | 4 × 10 | Tabela de botões (ver abaixo) |
| 0x0880 | 1 | Polling rate (código) |
| 0x0881 | 1 | ? (0x06) |
| 0x0884 | 1 | Lift-off distance: `mm = 0.6 + 0.1 × v` (5 = 1,1 mm; a UI vai de 0,7 a 1,7) |
| 0x0885 | 1 | Motion sync (1 = ligado) |
| 0x0888 | 7 × u16 LE | DPI X dos estágios 0–6 |
| 0x0896 | 2 | 0x0006 = último índice de estágio (7 estágios) |
| 0x0898 | 1 | Estágio ativo |
| 0x089A | 1 | ? (01 → 02 ao trocar estágio; talvez o estágio anterior + 1) |
| 0x08C0 | 1 | Debounce ao pressionar (ms) |
| 0x08C1 | 1 | Debounce ao soltar (ms) |
| 0x08C2 | 1 | ? (0x0A) |
| 0x08C3 | 1 | Flags: bit0 = linear correction **desligada**, bit1 = ripple correction **desligada** (padrão 0x02) |
| 0x08C4 | 1 | Ângulo do sensor (passos; 0 = 0°, +1 = um passo à direita; o sinal dos negativos ainda não foi verificado) |
| 0x08C5 | 1 | Glass tracking (1 = ligado) |
| 0x08C8 | 7 × u16 LE | DPI Y dos estágios 0–6 |
| 0x08D8 | 4 | ? (`00 01 00 FF`) |
| 0x08DC | 8 | ? (`00 00 01 05 02 03 03 00`) |
| 0x0060 | 4 | ? (`02 00 00 00`) |
| 0xFD00 | 16 × 12 | Nomes das macros (byte 0 = tamanho, depois ASCII preenchido com espaços) |

### Códigos de polling rate
| Hz | 125 | 250 | 500 | 1000 | 2000 | 4000 | 8000 |
|---|---|---|---|---|---|---|---|
| Código | 0x08 | 0x04 | 0x02 | 0x01 | 0x84 | 0x82 | 0x81 |

Ou seja, o bit 7 seleciona a base de 8 kHz, e os bits baixos são o divisor (base 1 kHz sem o bit 7).

### Tabela de botões (0x0600, 4 bytes cada: `TT 00 CC 00`)
| Índice | Endereço | Botão físico | Padrão |
|---|---|---|---|
| 0 | 0x0600 | Esquerdo | `03 00 01 00` |
| 1 | 0x0604 | Meio | `03 00 04 00` |
| 2 | 0x0608 | Direito | `03 00 02 00` |
| 3 | 0x060C | DPI+ | `08 00 05 00` |
| 4 | 0x0610 | DPI− | `08 00 06 00` |
| 5 | 0x0614 | Forward | `03 00 10 00` |
| 6 | 0x0618 | Back | `03 00 08 00` |
| 7 | 0x061C | Roda p/ frente | `07 00 08 00` |
| 8 | 0x0620 | Roda p/ trás | `07 00 08 00` |
| 9 | 0x0624 | DIY (botão de baixo) | `FF FF FF FF` |

Tipos (`TT`):
- `03`: botão de mouse. Bitmask `CC`: 1 = esquerdo, 2 = direito, 4 = meio, 8 = back, 0x10 = forward.
- `00`: tecla de teclado. `CC` é o HID usage (0x04 = A).
- `08`: função de DPI (5 = DPI+, 6 = DPI−).
- `07`: roda (os dois valores estão iguais, então falta decodificar).
- `FF`: não atribuído.

## Ainda não capturado
Macros (conteúdo), combinações, factory reset, sleep time, RF strategy, key scan rate, aviso de bateria fraca, cor do produto e o significado de `B0 00/01`.
