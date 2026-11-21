# Sessão automática da 0.10.x (20/21-11-2026)

- **Build:** 0.10.4, `native-fdc87ae-6df4da2044b3`, launcher 0.2.61, instância `chestcavity-dev`.
- **Mundo:** plano, com comandos, `advance_time false`, de dia. Janela normal fora do ecrã (`scripts/sessao_jogo.ps1`).
- **Corrida:** `scripts/sessao.ps1`, os 7 cenários de `sessoes/`. O `input-log` não registou ações humanas.
- O trace não tem nenhum FAIL e o `latest.log` não tem erros do mod.

## Resultado

| Cenário | hzt | Trace | Conclusão |
|---|---|---|---|
| selftest | 9 PASS | `316 PASS, 0 FAIL, 0 INFO`: 24 corpos (normal, 11 sem um órgão, 12 com um órgão a mais), cavidade guardada, rejeição (contagem e efeito), peito 4x9, config | SIM |
| opener-mob | 50 PASS | `the opener on minecraft:zombie/cow/creeper`; `mob 37 minecraft:cow: bleeding` (3x), a vaca morre; o creeper mostra o `oak_log` | SIM |
| morte | 84 PASS | `player ... died (keeps its cavity: false): 1 organs leave the body`; com keepInventory não há linha de queda e o órgão volta | SIM |
| saque | 35 PASS | Wither: `Looting 3, cleaver false` larga órgãos; `Looting 30, cleaver true` larga o salvamento (`bone_meal`, `soul_sand`) | SIM |
| teclas | 26 PASS | `ability <nome>: none` para as 8, uma vez pelo comando e outra pela tecla (V/R) | PARCIAL: prova o caminho tecla/comando → servidor; o corpo de teste não tem nenhuma habilidade |
| comer | 38 PASS | com estômago `ate: food 9 -> 14`; sem estômago `14 -> 15` (o jogo tinha dado 19) | SIM |
| ovo | 25 PASS | `grew true`, `taken away: true`; `cc list` com 36 | SIM |

## Bugs que a sessão apanhou (corrigidos)

1. **`/cc put` não metia órgãos tirados de um corpo** (0.10.3). Estes órgãos guardam o dono nos dados, e por isso não são itens "plain": o `take_items` do Host não os vê e a resposta era "Essa pilha mudou". A correção tira a pilha exata da ranhura com `take_stacks`, no tick do servidor. Tem teste: `body/tests.rs`, `an_organ_with_its_owner_in_the_hand_goes_back_in_with_cc_put`.
2. **Cenários:** o `cc put` corria antes de o servidor ver a mão nova, logo a seguir ao `select` (agora há um `wait 10`). E o cenário do ovo não era repetível com o peito já aumentado: o `cc reset full` (op) volta aos 27 slots.

## Fica por cobrir

- Mob `paralysed` (sem espinha) e `drowning` (sem pulmões): o trace existe, mas nenhum cenário o provoca.
- Habilidades com efeito real: precisam de um corpo com os órgãos de cada habilidade.
- Rejeição com dano real ao longo do tempo, `keep_chest_cavity`, `keep_grown_cavity` e os relógios do Astro: estão na checklist como "Sessão (a acrescentar)".
