# Periph

Configure seu mouse, teclado ou outro periférico sem o software do fabricante.
[English](README.md) · **Português**

## Usar

Abra o `Periph.exe`. Feche antes o software do fabricante: os dois não conseguem usar o
dispositivo ao mesmo tempo. Fechar a janela deixa o Periph na bandeja.

## Adicionar o seu dispositivo

O Periph suporta um dispositivo por meio de um **pacote de dispositivo**: uma pasta de arquivos
JSON que descreve como o dispositivo é configurado. Os pacotes ficam em
`%APPDATA%\Periph\devices\` e são carregados quando o Periph abre; nada é compilado.

O jeito mais fácil de escrever um é com um agente de IA de programação e a skill `add-device`
deste kit:

1. Instale o Python e, para capturar o app oficial, `pip install frida-tools`.
2. Claude Code: copie a pasta `add-device` para `%USERPROFILE%\.claude\skills\`. Outros
   agentes: indique a eles o arquivo `add-device/SKILL.md`.
3. Abra o agente nesta pasta e peça para adicionar o seu dispositivo ("adicione suporte ao meu
   <marca modelo> no Periph"). Ele identifica o dispositivo, pede para você usar o app oficial
   enquanto captura o tráfego, escreve o pacote, verifica e ajuda a testar.

O `periph-pack.exe` é a caixa de ferramentas que a skill usa (tudo só leitura no dispositivo):

```
periph-pack probe                       lista os dispositivos HID
periph-pack probe --vid 046D            interfaces de um dispositivo
periph-pack new <pasta> --vid ...       começa um pacote
periph-pack check <pasta>               valida um pacote e reproduz os exemplos capturados
periph-pack dir                         onde o Periph procura os pacotes
```

O formato está descrito em `add-device/docs/device-packages.md` (em inglês), com dois exemplos
completos em `add-device/examples/`.

## Segurança

Um pacote só consegue enviar comandos capturados do software oficial, com valores que o
software oficial permite; o motor garante isso. Pacotes nunca atualizam firmware, nunca
resetam nem calibram o dispositivo e não alcançam nada além do dispositivo que descrevem.

Código-fonte e issues: https://github.com/arthur-ba1/periph (licença MIT).
