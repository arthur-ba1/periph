# Roteiro de capturas USB (VT2 Max, 24AE:1423)

Objetivo: descobrir o protocolo de configuração gravando o tráfego USB enquanto o
software oficial da Rapoo muda **uma configuração por vez**.

## Preparação (uma vez)
1. Instale o Wireshark e marque **USBPcap** durante a instalação. Depois reinicie o PC.
2. Instale o software oficial da Rapoo para o VT2 Max (temporariamente).
3. Descubra em qual barramento USBPcap o mouse está:
   - Abra o Wireshark. Cada interface `USBPcap1`, `USBPcap2`… é um controlador USB.
   - Comece a capturar em uma delas e mexa o mouse. A certa é aquela que mostra pacotes.

Filtro de exibição útil (mostra só tráfego de controle/HID, sem o movimento do mouse):
```
usb.idVendor == 0x24ae || (usb.transfer_type == 0x02) || (usbhid)
```
Depois de identificar o endereço do aparelho (ex.: `1.5`), use `usb.device_address == 5`.

## Regras
- **Uma mudança por arquivo.** Comece a captura, espere 2 s, faça a mudança, espere 2 s, pare.
- Não mexa o mouse durante a captura, se puder (gera ruído).
- Salve como `.pcapng` em `captures/` com o nome indicado abaixo.
- Anote em `captures/NOTES.md` o valor exato antes e depois.

## Lista
| # | Arquivo | Ação |
|---|---------|------|
| 01 | `01-idle.pcapng` | App oficial aberto, sem mexer em nada, 15 s (leitura inicial e polling de bateria) |
| 02 | `02-open-app.pcapng` | Capturar **durante a abertura** do app oficial (lê todas as configurações) |
| 03 | `03-reset-default.pcapng` | "Restaurar padrão" — rota de recuperação |
| 04 | `04-dpi1-800.pcapng` | DPI estágio 1 → 800 |
| 05 | `05-dpi1-1600.pcapng` | DPI estágio 1 → 1600 |
| 06 | `06-dpi1-1650.pcapng` | DPI estágio 1 → 1650 (mostra o passo/unidade) |
| 07 | `07-dpi-stages-count.pcapng` | Mudar quantidade de estágios ativos |
| 08 | `08-dpi-active-stage.pcapng` | Trocar estágio ativo |
| 09–1x | `09-poll-125.pcapng`, `10-poll-500.pcapng`, `11-poll-1000.pcapng`, … | Um arquivo por polling rate disponível |
| 20 | `20-lod-1mm.pcapng` / `21-lod-2mm.pcapng` | Lift-off distance |
| 22 | `22-debounce-min.pcapng` / `23-debounce-max.pcapng` | Debounce |
| 24 | `24-motion-sync-on.pcapng` / `25-…-off` | Motion sync |
| 26 | `26-angle-snap-on.pcapng` / `27-…-off` | Angle snapping |
| 28 | `28-ripple-on.pcapng` / `29-…-off` | Ripple control (se existir) |
| 30 | `30-btn4-to-middle.pcapng` | Botão lateral 4 → clique do meio |
| 31 | `31-btn4-to-key-a.pcapng` | Botão 4 → tecla `A` |
| 32 | `32-btn4-default.pcapng` | Botão 4 → padrão |
| 40 | `40-macro-ab.pcapng` | Criar macro "A down, 50 ms, A up, B down, 50 ms, B up" e atribuir ao botão 5 |
| 50 | `50-sleep-time.pcapng` | Tempo de suspensão (se existir) |

Os arquivos 01–05 já bastam para começar a decodificação. Os demais podem vir depois.
