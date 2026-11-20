<#
.SYNOPSIS
Abre a instância de teste numa janela normal de 854×480 e move-a para fora do ecrã.

.DESCRIPTION
Corre sem pipe. Usa -Mostrar ou -Esconder para mover a janela da sessão já aberta.
#>
param(
    [ValidatePattern('^[a-zA-Z0-9_-]+$')]
    [string]$Instancia = 'chestcavity-dev',
    [string]$Socket = (Join-Path $env:TEMP 'cc-sessao.json'),
    [switch]$Mostrar,
    [switch]$Esconder
)

$ErrorActionPreference = 'Stop'
$Socket = [IO.Path]::GetFullPath($Socket)
$socketPath = [regex]::Escape($Socket)
$socketArgument = '(?:^|\s)--control-socket(?:\s+|=)(?:"' + $socketPath + '"|' + $socketPath + ')(?:\s|$)'

if (-not ('ChestCavity.Janela' -as [type])) {
    Add-Type -Namespace ChestCavity -Name Janela -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
'@
}

function Move-Janela([int]$X, [int]$Y) {
    $ids = @(Get-CimInstance Win32_Process -Filter "Name='horizon-client.exe'" |
        Where-Object { $_.CommandLine -match $socketArgument } |
        ForEach-Object ProcessId)
    if ($ids.Count -eq 0) { return $false }

    $processo = Get-Process -Id $ids -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if (-not $processo) { return $false }

    # SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE
    return [ChestCavity.Janela]::SetWindowPos($processo.MainWindowHandle, [IntPtr]::Zero, $X, $Y, 0, 0, 0x15)
}

if ($Mostrar -and $Esconder) { throw 'Escolhe -Mostrar ou -Esconder.' }
if ($Mostrar -or $Esconder) {
    $moveu = if ($Mostrar) { Move-Janela 40 40 } else { Move-Janela -4000 100 }
    if (-not $moveu) { throw "Não foi possível mover a janela da sessão (socket $Socket)." }
    if ($Mostrar) { 'Janela de volta ao ecrã.' } else { 'Janela fora do ecrã.' }
    return
}

if (Get-Process horizon-client -ErrorAction SilentlyContinue) {
    throw 'Já há um horizon-client aberto. Fecha-o antes de arrancar a sessão.'
}

$launcher = Join-Path $env:USERPROFILE 'Desktop\aprender\launcher-0.2.61\horizon-launcher.exe'
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) { throw "Falta o launcher em $launcher." }
if (Test-Path -LiteralPath $Socket -PathType Leaf) { Remove-Item -LiteralPath $Socket }

$repo = Split-Path $PSScriptRoot -Parent
$local = Join-Path $repo 'local'
[IO.Directory]::CreateDirectory($local) | Out-Null
$log = Join-Path $local "play-$Instancia.log"
$gameArgs = @('play', '--instance', $Instancia, '--arg=--control-socket', ('--arg="{0}"' -f $Socket),
    '--arg=--width', '--arg=854', '--arg=--height', '--arg=480')
Start-Process -FilePath $launcher -ArgumentList $gameArgs -WindowStyle Hidden `
    -RedirectStandardOutput $log -RedirectStandardError "$log.err"

$espera = [Diagnostics.Stopwatch]::StartNew()
while (-not (Move-Janela -4000 100)) {
    if ($espera.Elapsed.TotalSeconds -ge 300) {
        throw "A janela da sessão não apareceu em 5 min. Consulta $log e $log.err."
    }
    Start-Sleep -Milliseconds 250
}
"Jogo aberto com a janela fora do ecrã. Socket em $Socket."
