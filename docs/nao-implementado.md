# Chest Cavity para o Horizon: o que falta (versão 0.7.0)

Estado a 11-11-2026. Fontes: o mod original (Chest Cavity, Tigereye504), o Chest Cavity Beyond e o Astro Adds, lidos em `vendor/`. Nada daqui foi ainda testado dentro do jogo: o que está em "Feito" compila, passa os testes e usa hooks que a documentação do Horizon descreve, mas só o jogo diz se se comporta como deve.

## Feito nesta versão

- **Línguas**: todo o texto do mod (nomes dos itens, descrições dos órgãos, mensagens, comandos) está em inglês e português (`pt_pt` e `pt_br`), conforme a língua do Minecraft; as outras línguas veem inglês.
- **Mobs**: o que se tira do peito de um mob muda o mob.
- **Abrir o peito**: no ar abre o teu; com botão direito num mob abre o dele (órgãos próprios de cada espécie, guardados no mob). Custa 4 de dano a quem é aberto, salvo `ease_of_access` (agora há o pseudo-órgão das portas e alçapões). Em criativo o jogo não deixa magoar o jogador.
- **Comer**: digestão, nutrição, carnívoro/herbívoro, comida podre, resistência (cansaço) e metabolismo mais rápido. Náusea e fome com estômago ou intestinos maus.
- **Respirar**: pulmões, capacidade pulmonar e guelras, com as fórmulas do original.
- **Defesa**: ossos, fogo, impacto, salto (queda), frio e fogo do Beyond, esquiva de flechas (teleporte).
- **Corpo**: coração (e sangramento), rins (veneno com a duração do original), nervos, brilho, sorte, força, velocidade, alcance, recuo, leveza (gravidade).
- **Ambiente**: fotossíntese, cristalossíntese, hidroalergia, hidrofobia.
- **Golpes**: lançamento; veneno (soco com a mão vazia).
- **Habilidades ativas** (tecla **V** utilitárias, **R** ataques, ou `/cc ability <nome>`): reparar com ferro, pastar, seda, piromancia, ghastly, bombas de dragão, cuspo forte, explosão creepy.
- **Voo** (órgãos de voo do Astro): voo como em criativo.
- **Drops**: só órgãos verdadeiros (já não saem terra, paus nem pólvora).
- **Tooltip** de cada órgão com o que melhora, o que piora, as consequências e as habilidades; separador criativo com tudo.

50 dos 74 scores de órgãos têm agora efeito real (eram 15). 240 dos 254 órgãos têm pelo menos um efeito real; 194 têm todos.

## Falta

### Habilidades ativas
- Sopro de dragão (`dragon_breath`): nuvem de dano ao fundo do olhar.
- Balas de shulker (`shulker_bullets`): bala teleguiada ao ser vivo mais perto.
- Fornalha interior (`furnace_powered`) e o efeito Furnace Power; os itens internos `cud` e `furnace_power`.
- Exalar (`buoyant`), flutuação (`buoyant`, `neutral_water_buoyant`).
- Teclas próprias por habilidade (o original tem 12, sem tecla por omissão) e teclas configuráveis (aqui V e R são fixas).
- Disparar um projétil de cada vez, de 5 em 5 ticks (aqui saem todos juntos) e o recuo ao disparar (o Horizon não empurra jogadores).
- Cooldowns como efeitos de poção visíveis: aqui são temporizadores do mod e perdem-se ao sair do jogo.
- Ruminar com o efeito Ruminating (aqui a erva é comida de uma vez); `eat_the_rich` (Astro: comer o bloco).
- Seda: lançar o fio ao chão (aqui vai para o inventário); seda ao ordenhar a vaca e ao tosquiar a ovelha.
- Explosão creepy: a nuvem com os efeitos de poção de quem explode; o creeper que só explode com o órgão.
- Voo do Astro como o original (alternar com `.` ou V, velocidade ×5).
- Beyond: o sistema de habilidades próprio (ecrã com V, tecla C, 54 teclas), teleporte do apêndice de Ender, lasers de guardião, caveira de Wither, balas de vento, onda sónica, fermentação de flores, purga do fígado enfeitiçado, jukebox portátil, habilidades automáticas dos mobs.

### Efeitos passivos
- `detoxification`, `buff_purging`, `withered`: duração dos efeitos de poção recebidos.
- `iron_gut` (comida sem efeitos maus) e o rotgut a tirar a Fome da comida podre.
- `crystallization`: o End Crystal cura quem tem o órgão.
- Beyond: `water_allergy` e `water_weakness`, `lava_swim_speed`, `strider_kidney`, o fogo que se apaga com imunidade, a curva logarítmica de `defense`, `strength`, `speed` e `nerves` (o port usa as lineares do original), o lado-efeito de `scale`.
- Poção de água a rebentar perto de quem tem hidroalergia ou hidrofobia.
- Encurtar a distância de queda com `lightweight` e `buoyant`.
- Metabolismo mais lento que o normal (só o mais rápido está feito).
- Priões da comida humana; efeitos especiais ao comer certos órgãos (coração de dragão, carne tóxica, alienígena).
- Pânico, ressonância, shrieking, explosão em cacho (Astro: reações ao ser atingido).

### Rejeição e compatibilidade de órgãos
- Órgãos ligados ao dono, implantes alheios rejeitados (2 de dano a cada 600/score ticks, efeito `organ_rejection`).
- Dadores universais, o encantamento O Negative, a maldição Malpractice.
- Largar os órgãos implantados ao morrer; órgãos de emergência (`insertWelfareOrgans`); órgãos vitais ao reaparecer (Beyond).
- Precisa de dados por pilha (o port guarda só id e quantidade).

### Cavidades e mobs
- Só se pode abrir um mob fraco e sem peitoral (vida ≤ 20 ou ≤ 50 %, `ease_of_access` salta isto). O port abre qualquer mob.
- Estado "aberta" e cavidade preguiçosa: o original só aplica os efeitos depois de a cavidade ser aberta.
- Os mobs cujo peito foi aberto vivem pelos órgãos que têm (sem coração sangram até morrer, sem espinha não se mexem, sem pulmões afogam-se, sem rins envenenam-se, vida, força e velocidade pelos músculos e coração). Falta o resto: o creeper sem apêndice ainda explode, o wither larga sempre a estrela, e os mixins de Enderman, Shulker, Blaze, Ghast, Breeze, Warden, Guardian, Llama, golem de neve, aranha de caverna, golem de ferro e strider. Um mob que nunca foi aberto não muda.
- Ranhuras proibidas por tipo (o importador ignora `forbiddenSlots`). Os órgãos excecionais por tipo (o tronco do creeper vale vida, defesa e nervos) já estão.
- Tamanhos de cavidade do Beyond (27 a 54 ranhuras), conversão de órgãos ao transformar um mob.
- Partes do dragão: o clique numa parte devia abrir o dragão.
- Abrir com um dispensador (Astro).

### Loot e encantamentos
- Saque e os encantamentos Surgical, Tomophobia e Malpractice; o cutelo (saque ×10) e as receitas de salvamento com múltiplos.
- Só dar órgãos aleatórios a cavidades nunca abertas.
- Órgãos nos baús de estruturas (pirâmide do deserto).
- Os 8 encantamentos do Beyond (Teleoperation, Advanced Surgery, Safe Surgery, Prudent Surgery, Hydraulic Clamp, Anesthesia Surgery, Postoperative Suture, Primal Reversion).

### Astro Adds
- Regrowth, autofagia, caça-tumores, infeção sculk e tumores (precisam de dados por pilha e temporizadores sobre a cavidade).

### Itens, receitas e configuração
- Receita da glândula de veneno (glândula + poção) e as glândulas de veneno e de alquimia a guardar efeitos.
- Analisador biológico e tooltips detalhados com SHIFT (Beyond).
- Os valores de comida do original (as comidas usam as da carne vanilla).
- Ficheiros de configuração: `HEART_HP`, `BONE_DEFENSE`, cooldowns, etc. estão fixos no código.
- Comandos do Beyond (`/chestcavity <alvos> row_3..row_6`, `resetorgans`, `attributes`).
- Avanços de receitas.
- Tipos de dano `cc_heartbleed`, `cc_organ_rejection`, `open_chest` e `organ_loss` (o port usa os genéricos).

## Aproximações (existem, mas não são iguais)

- Respiração: não distingue correr nem chuva (o original divide a recuperação por 4 a correr e soma a respiração aquática à chuva).
- `launching`: não limita a 4 blocos nem desconta a resistência a recuo do alvo.
- `fire_resistance` e `frost_resistance` do Beyond: abaixo do limite o dano baixa proporcionalmente (o original diz só "atenua").
- Veneno: sempre Veneno 10 s, não os efeitos guardados na glândula.
- Voo: voo de criativo (duplo salto), não o voo do Astro.
- Metabolismo e comer: medidos a cada tick sobre a barra de comida; o original altera o `HungerManager`. A saturação ganha pode diferir uma casa decimal.
- Heartbleed e outros danos usam `minecraft:generic` e não tipos próprios.
- Os projéteis são invocados com NBT de `Motion` e `power`; se o Horizon ler outros nomes, saem parados.
