<#
.SYNOPSIS
Testes do shared e do engine contra o horizon-client da revisão fixada (fdc87ae).

.DESCRIPTION
Copia native\{shared,engine} para <worktree>\native-cc\ e põe o horizon-client por caminho. Junta os 2
membros ao workspace do worktree (uma vez só) e copia o lang do mod para <worktree>\assets, para os testes que o
leem com include_str!. Depois corre cargo test com a toolchain MSVC (a GNU não tem dlltool) e um target próprio
e durável.
No repo não fica nenhum Cargo.lock: as crates da bundle não o podem ter (E_CRATE).

uso: scripts\test-engine.ps1 [argumentos extra para o cargo test]
#>
param([Parameter(ValueFromRemainingArguments)] $Rest)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$wt = Join-Path $env:USERPROFILE 'Desktop\aprender\horizon-fdc'
$target = Join-Path $env:USERPROFILE 'Desktop\aprender\horizon-fdc-target'
$pin = 'fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca'

if (-not (Test-Path (Join-Path $wt 'crates\horizon-client'))) { throw "falta o worktree fdc87ae em $wt" }
$head = (git -C $wt rev-parse HEAD).Trim()
if ($head -ne $pin) { throw "o worktree está em $head e não em $pin" }

# Recurso exclusivo: a cópia do src e o cargo test usam o worktree e o target
# partilhados, por isso dois verify ao mesmo tempo testariam o código um do outro. Lock exclusivo num ficheiro,
# com espera até 30 min; liberta-se no finally (e o sistema liberta-o se o processo morrer).
$lockPath = Join-Path $wt 'native-cc.lock'
$lock = $null; $t0 = Get-Date; $avisou = $false
while (-not $lock) {
    try { $lock = [IO.File]::Open($lockPath, 'OpenOrCreate', 'ReadWrite', 'None') }
    catch {
        if (((Get-Date) - $t0).TotalMinutes -gt 30) { throw "o worktree do Horizon está ocupado há 30 min ($lockPath): $_" }
        if (-not $avisou) { Write-Host 'test-engine: à espera da vez (outro verify está a usar o worktree do Horizon)'; $avisou = $true }
        Start-Sleep -Seconds 5
    }
}
try {
$dst = Join-Path $wt 'native-cc'
foreach ($crate in 'shared', 'engine') {
    $d = Join-Path $dst $crate
    if (Test-Path (Join-Path $d 'src')) { Remove-Item (Join-Path $d 'src') -Recurse -Force }
    New-Item -ItemType Directory -Force $d | Out-Null
    Copy-Item (Join-Path $repo "native\$crate\src") (Join-Path $d 'src') -Recurse
    # O Copy-Item guarda as datas de origem e o cargo só recompila o que é mais novo do que o último build no target
    # partilhado: um checkout mais antigo reutilizava artefactos de outro código.
    $agora = Get-Date
    Get-ChildItem (Join-Path $d 'src') -Recurse -File | ForEach-Object { $_.LastWriteTime = $agora }
    $velhos = @(Get-ChildItem (Join-Path $d 'src') -Recurse -File | Where-Object { $_.LastWriteTime -lt $agora.AddSeconds(-5) })
    if ($velhos.Count -gt 0) { throw "test-engine: $($velhos.Count) ficheiros copiados ficaram com a data antiga" }
    $toml = Get-Content (Join-Path $repo "native\$crate\Cargo.toml") -Raw
    $toml = $toml -replace 'horizon-client = \{[^}]*\}', 'horizon-client = { path = "../../crates/horizon-client" }'
    Set-Content (Join-Path $d 'Cargo.toml') $toml -NoNewline -Encoding utf8
}

$lang = Join-Path $repo 'assets\chestcavity\lang'
if (Test-Path $lang) {
    $langDst = Join-Path $wt 'assets\chestcavity'
    if (Test-Path (Join-Path $langDst 'lang')) { Remove-Item (Join-Path $langDst 'lang') -Recurse -Force }
    New-Item -ItemType Directory -Force $langDst | Out-Null
    Copy-Item $lang (Join-Path $langDst 'lang') -Recurse
}

$ws = Join-Path $wt 'Cargo.toml'
$w = Get-Content $ws -Raw
if ($w -notmatch 'native-cc/engine') {
    $w = $w -replace 'members = \[', "members = [`n    `"native-cc/engine`",`n    `"native-cc/shared`","
    Set-Content $ws $w -NoNewline -Encoding utf8
}

Push-Location (Join-Path $dst 'engine')
try {
    $env:CARGO_TARGET_DIR = $target
    cargo +stable-x86_64-pc-windows-msvc test -p chestcavity-shared -p mod-chestcavity @Rest
    if ($LASTEXITCODE -ne 0) { throw "cargo test falhou (código $LASTEXITCODE)" }
}
finally { Pop-Location }
}
finally { $lock.Dispose() }
