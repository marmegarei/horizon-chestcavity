param(
    [ValidateSet('todos', 'selftest', 'opener-mob', 'morte', 'saque', 'ovo', 'teclas', 'comer')]
    [string]$Cenario = 'todos',
    [ValidatePattern('^[a-zA-Z0-9_-]+$')]
    [string]$Instancia = 'chestcavity-dev',
    [string]$Socket = (Join-Path $env:TEMP 'cc-sessao.json')
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$repo = Split-Path $PSScriptRoot -Parent
$Socket = [IO.Path]::GetFullPath($Socket)
$launcherHome = Join-Path $env:APPDATA 'horizon-launcher'
$instances = (Get-Content -LiteralPath (Join-Path $launcherHome 'instances.json') -Raw |
    ConvertFrom-Json -AsHashtable)['instances']
$version = $instances[$Instancia].version
if (-not $version) { throw "A instância $Instancia não tem uma versão no launcher." }
$ctl = Join-Path $launcherHome "versions\$version\horizon-ctl.exe"
$hzt = Join-Path $env:USERPROFILE 'projetos\horizon-test\hzt.py'
$instanceDir = Join-Path $env:APPDATA "horizon\instances\$Instancia"
$log = Join-Path $instanceDir 'logs\latest.log'
$trace = Join-Path $instanceDir 'config\chestcavity-trace.log'
foreach ($file in @($ctl, $hzt, $Socket)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Falta $file." }
}
Get-Command python -ErrorAction Stop | Out-Null

function Invoke-Ctl {
    $json = & $ctl --socket $Socket @args
    $code = $LASTEXITCODE
    if ($code -ne 0) { throw "horizon-ctl $($args[0]) terminou com código $code." }
    $reply = $json | ConvertFrom-Json
    if ($null -eq $reply -or $reply.PSObject.Properties['error']) {
        throw "Resposta de erro de horizon-ctl $($args[0])."
    }
    $reply
}

function Read-Trace {
    if (Test-Path -LiteralPath $trace -PathType Leaf) {
        try { return [string](Get-Content -LiteralPath $trace -Raw -Encoding utf8) }
        catch [System.Management.Automation.ItemNotFoundException] { } # Rotação do trace.
    }
    return ''
}

function Test-Trace([string]$Nome, [string]$Antes) {
    $depois = Read-Trace
    # O mod acrescenta linhas ao trace ou roda o ficheiro quando passa de 200 KB.
    $novo = if ($depois.StartsWith($Antes, [StringComparison]::Ordinal)) {
        $depois.Substring($Antes.Length)
    } else { $depois }
    $linhas = @($novo -split '\r?\n' | Where-Object { $_ -match '^\d+ ' })

    $esperados = switch ($Nome) {
        'opener-mob' {
            'the opener on minecraft:zombie \d+ by '
            'the opener on minecraft:creeper \d+ by '
            'mob \d+ minecraft:cow: bleeding$'
        }
        'saque' {
            'minecraft:wither \d+ died to Some\(\d+\) \(Looting 3, cleaver false\): '
            'minecraft:wither \d+ died to Some\(\d+\) \(Looting 30, cleaver true\): .*minecraft:(?:bone_meal|soul_sand)'
        }
        'ovo' {
            'the opener on the dragon egg at \[0, 101, 2\] by [0-9a-f]+: grew true$'
            'the dragon egg at \[0, 101, 2\] taken away: true$'
        }
    }
    foreach ($pattern in $esperados) {
        if (-not ($linhas -cmatch "^\d+ $pattern")) { "Falta no trace novo o padrão $pattern" }
    }

    switch ($Nome) {
        'morte' {
            $mortes = @($linhas -cmatch '^\d+ player [0-9a-f]+ died \(keeps its cavity: false\): \d+ organs leave the body$')
            if ($mortes.Count -ne 1 -or $mortes[0] -cnotmatch ': 1 organs leave the body$') {
                'Esperava uma linha de morte com um órgão largado e nenhuma linha na morte com keep_inventory.'
            }
        }
        'teclas' {
            foreach ($ability in @('creepy', 'dragon_bombs', 'forceful_spit', 'ghastly', 'pyromancy', 'grazing', 'iron_repair', 'silk')) {
                $registos = @($linhas -cmatch "^\d+ ability ${ability}: (ok|resting|none)$")
                if ($registos.Count -ne 2) { "Esperava dois registos de $ability, um do comando e um da tecla." }
            }
        }
        'comer' {
            $refeicoes = @($linhas | ForEach-Object {
                if ($_ -cmatch '^\d+ ate: food (\d+) -> (\d+) \(the game gave \d+\)$') {
                    [int]$Matches[2] - [int]$Matches[1]
                }
            })
            if ($refeicoes.Count -ne 2 -or $refeicoes[0] -ne 5 -or $refeicoes[1] -ne 1) {
                'Esperava duas refeições de pão, com ganho de 5 e 1 pontos de comida, por esta ordem.'
            }
        }
    }
}

$cenarios = if ($Cenario -eq 'todos') {
    @('selftest', 'opener-mob', 'morte', 'saque', 'teclas', 'comer', 'ovo')
} else { @($Cenario) }
$result = 0
$inputStart = (Invoke-Ctl input-log --origin user --limit 1).last
Push-Location -LiteralPath $repo
try {
    foreach ($nome in $cenarios) {
        Write-Host "=== $nome ==="
        $options = (Invoke-Ctl options-get pauseOnLostFocus chatVisibility).options
        $antes = if ($nome -ne 'selftest') { Read-Trace } else { '' }
        $scenarioPath = Join-Path $repo "sessoes\$nome.hzt"
        $code = 1
        try {
            & python -I $hzt $scenarioPath --ctl $ctl --socket $Socket --log $log
            $hztCode = $LASTEXITCODE
            $code = $hztCode
            if ($code -eq 0 -and $nome -ne 'selftest') {
                $falhas = @(Test-Trace $nome $antes)
                foreach ($falha in $falhas) { Write-Warning $falha }
                if ($falhas.Count -gt 0) { $code = 1 }
            }
        } finally {
            foreach ($key in @('chatVisibility', 'pauseOnLostFocus')) {
                try {
                    $value = $options.$key.value
                    if ($value -is [bool]) { $value = ([string]$value).ToLowerInvariant() }
                    Invoke-Ctl options-set $key $value | Out-Null
                } catch {
                    Write-Warning "Falhou a reposição da opção $key."
                    if ($code -eq 0) { $code = 1 }
                }
            }
        }
        Write-Host "$nome.hzt terminou com código $hztCode. Resultado do cenário $code."
        $result = [Math]::Max($result, $code)
    }
} finally {
    Pop-Location
    try {
        $input = Invoke-Ctl input-log --origin user --since $inputStart
        $inputCount = @($input.events).Count
        if ($inputCount -gt 0) { Write-Warning "O jogo registou $inputCount ações humanas durante a sessão." }
    } catch {
        Write-Warning 'Não foi possível verificar se houve ações humanas durante a sessão.'
    }
}
exit $result
