# Periph

[English](README.md) · **Português (Brasil)**

App aberto (Rust + Tauri 2 + Svelte 5) para configurar periféricos sem o software do fabricante.
Cada dispositivo é uma implementação plugável: um driver em Rust e um módulo de interface em
Svelte. Qualquer pessoa pode adicionar o próprio mouse, teclado ou outro periférico.

## Dispositivos suportados
| Dispositivo | Id | O que dá para configurar |
| --- | --- | --- |
| **Rapoo VT2 Max** (USB `24AE:1423` dongle, `24AE:4623` cabo) | `vt2` | DPI e estágios, polling rate, LOD, motion sync, linear/ripple, modo vidro, debounce, botões, bateria |
| **Teclados VIA** (ex.: AKKO 5075) | `via` | keymap por camada (com knob), iluminação RGB |

Para um teclado VIA novo basta colocar a definição JSON em `app/src/devices/via/keyboards/`
(o repositório traz só uma definição de exemplo; confira a licença antes de commitar a de outro teclado).

## Adicionar um dispositivo
- O seu próprio dispositivo, sem recompilar o Periph: um pacote de dispositivo (arquivos JSON, sem código),
  veja [docs/device-packages.md](docs/device-packages.md) (em inglês). O kit de release (página Releases) traz o
  `Periph.exe`, a caixa de ferramentas `periph-pack.exe` e a skill `add-device`, para um agente de IA escrever o pacote com você.
- Guia: [docs/adding-a-device.md](docs/adding-a-device.md) (contrato do driver e do módulo)
- Método de engenharia reversa: [docs/reverse-engineering.md](docs/reverse-engineering.md)
- Para agentes de IA (Claude Code, Codex, etc.): skill em
  [.claude/skills/add-device/SKILL.md](.claude/skills/add-device/SKILL.md), com templates prontos.
  Veja também [AGENTS.md](AGENTS.md).

## Estrutura
- `crates/periph-core`: contrato `Driver`/`Session` e o loop genérico de cada dispositivo
- `crates/periph-app`: o app Tauri (`periph.exe`); drivers em `src/drivers/`, bandeja genérica
- `crates/vt2-proto`, `crates/vt2-device`: protocolo e transporte do VT2 Max ([docs/protocol.md](docs/protocol.md))
- `crates/via`: protocolo VIA
- `crates/vt2-probe`: descoberta HID somente leitura; `crates/vt2-cli`: CLI do VT2 Max
- `app/src/devices/<id>/`: módulos de interface, descobertos automaticamente
- `tools/`: hook Frida, resumo de logs, extração de recursos

## Compilar
```
npm --prefix app install
npm --prefix app run build            # gera app/dist (embutido no exe)
cargo build --release -p periph-app   # target\release\periph.exe
target\release\periph.exe --tray      # inicia só na bandeja
npm --prefix app run dev              # só a interface, sem hardware: http://localhost:5173/?demo
cargo test --workspace
```
- Fechar a janela só a esconde na bandeja. Cada dispositivo conectado tem seu próprio ícone na bandeja, com a bateria: clique esquerdo abre as configurações rápidas, duplo clique abre o dispositivo no Periph e clique direito mostra o menu.
- Feche o software oficial antes de usar o Periph: os dois não podem usar o canal HID ao mesmo tempo.
- Ícone: `crates/periph-app/app-icon.svg`. Para regenerar: `npx --prefix app tauri icon crates/periph-app/app-icon.svg -o crates/periph-app/icons`

## Idiomas
A interface, a bandeja e as mensagens estão em português (Brasil) e inglês. O idioma segue o
sistema e pode ser trocado na barra de título. Para adicionar um idioma: [docs/translating.md](docs/translating.md).

## Contribuir
- Ative o hook do repositório uma vez por clone: `git config core.hooksPath .githooks`.
  Ele recusa commits com `Co-Authored-By:` ou "Generated with Claude Code"; o CI faz a mesma checagem.
- Antes de abrir um PR: `cargo test --workspace`, `npm --prefix app run build` e `npm --prefix app run check:i18n`.

## Imagens dos fabricantes
Os renders dos produtos pertencem aos fabricantes e não estão no repositório. Cada módulo
funciona sem eles (mostra um contorno neutro) e explica em `app/src/devices/<id>/assets/NOTICE.md`
como extraí-los do seu próprio software oficial, para uso local.

## CLI do VT2 Max
```
cargo build --release -p vt2-cli
target\release\vt2 info
target\release\vt2 status
target\release\vt2 polling 1000
target\release\vt2 dpi 2 800
target\release\vt2 button back key:f13
```

## Regras de segurança
- Só enviar comandos de escrita no formato que o app oficial usa (verificado por testes contra bytes capturados).
- Sem fuzzing de comandos de escrita e sem atualização de firmware, bootloader ou reset de fábrica.
- Ferramentas de descoberta só leem.

## Pendências
- [ ] VT2 Max: macros, factory reset, sleep time (ainda não capturados)

## Licença
MIT. Veja [LICENSE](LICENSE).
