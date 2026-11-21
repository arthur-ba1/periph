# Assemble the release kit from a release build:
#   Periph.exe, periph-pack.exe, README.md, LEIAME.md and the add-device skill with its tools,
#   docs and example packages, so anyone can adapt Periph to their own device.
#
#   npm --prefix app run build
#   cargo build --release -p periph-app -p periph-pack
#   pwsh tools/make-kit.ps1 [-Out dist/kit]          # then zip the folder
param([string]$Out = "dist/kit")

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
$skill = Join-Path $Out "add-device"
New-Item -ItemType Directory -Force -Path $skill, "$skill/tools", "$skill/docs", "$skill/examples" | Out-Null

Copy-Item target/release/periph.exe (Join-Path $Out "Periph.exe")
Copy-Item target/release/periph-pack.exe $Out
Copy-Item docs/kit/README.md, docs/kit/LEIAME.md $Out

Copy-Item .claude/skills/add-device/SKILL.md $skill
Copy-Item -Recurse .claude/skills/add-device/templates $skill
Copy-Item tools/hidhook.py, tools/hidhook.js, tools/summarize.py "$skill/tools"
Copy-Item docs/device-packages.md, docs/reverse-engineering.md, docs/adding-a-device.md "$skill/docs"
Copy-Item -Recurse packages/rapoo-vt2-max "$skill/examples/rapoo-vt2-max"
Copy-Item -Recurse crates/periph-pack/tests/fixtures/sample-reports "$skill/examples/sample-reports"

Get-ChildItem -Recurse $Out | Where-Object { -not $_.PSIsContainer } | ForEach-Object { $_.FullName.Substring((Resolve-Path $Out).Path.Length + 1) }
