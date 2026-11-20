# Sessões do Chest Cavity

Depois de instalar a build na `chestcavity-dev`, arranca o jogo a partir da raiz do repo e corre o script sem pipe.

```powershell
pwsh -NoProfile -File scripts/sessao_jogo.ps1
pwsh -NoProfile -File scripts/sessao_jogo.ps1 -Mostrar
```

No jogo, cria ou abre um mundo plano descartável, com comandos permitidos, em sobrevivência. Os cenários usam a config padrão do mod e uma arena junto de `0 101 0`. Para o ovo, começa com uma cavidade de 27 slots. Guarda as teclas V e R nas associações padrão do mod.

```powershell
pwsh -NoProfile -File scripts/sessao_jogo.ps1 -Esconder
pwsh -NoProfile -File scripts/sessao.ps1
pwsh -NoProfile -File scripts/sessao.ps1 -Cenario opener-mob
```

O runner usa o `horizon-ctl` da build registada para a instância, chama o HZT com o `latest.log` e valida as linhas novas do trace. Mostra o código do HZT e o resultado de cada cenário, repõe as opções de foco e chat e avisa se o jogo registou ações humanas. O código final é o maior dos códigos dos cenários. As falhas do HZT guardam evidência em `local/hzt-falhas/`.

| Cenário | O que verifica |
| --- | --- |
| `selftest` | `cc selftest true` responde no chat com zero falhas. |
| `opener-mob` | O Opener abre o zombie e o creeper. Retira o coração do zombie, confirma a hemorragia numa vaca e encontra o tronco no creeper. |
| `morte` | O coração alheio cai no chão com `keep_inventory false` e reaparece no inventário após respawn com `keep_inventory true`. |
| `saque` | A espada com Looting III dá saque de órgãos. O cutelo com Looting III multiplica o nível por dez e transforma o saque. |
| `ovo` | O Opener gasta o ovo de dragão e o `cc list` passa a mostrar 36 slots. |
| `teclas` | As oito habilidades chegam pelo comando e pelos grupos das teclas V e R. |
| `comer` | O pão dá cinco pontos de comida com estômago e um sem ele. O défice é preparado em fácil, pois a exaustão em pacífico não baixa a barra. |
