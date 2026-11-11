# Chest Cavity para o Horizon: o que falta (versão 0.8.1)

Estado a 11-11-2026. Fontes: o mod original (Chest Cavity, Tigereye504), o Chest Cavity Beyond e o Astro Adds, lidos em `vendor/`. Nada daqui foi ainda testado dentro do jogo: o que está em "Feito" compila, passa os testes e usa hooks que a documentação do Horizon descreve, mas só o jogo diz se se comporta como deve.

## Feito

- **Línguas**: todo o texto do mod (nomes dos itens, descrições dos órgãos, mensagens, comandos) está em inglês e português (`pt_pt` e `pt_br`), conforme a língua do Minecraft; as outras línguas veem inglês.
- **Abrir o peito**: no ar abre o teu; com botão direito num mob abre o dele (órgãos próprios de cada espécie, guardados no mob). Custa 4 de dano a quem é aberto, salvo `ease_of_access` (as portas e alçapões contam como órgãos). Um mob só se abre se estiver fraco (vida ≤ 20 ou ≤ 50 % da máxima) e sem peitoral; `ease_of_access` dispensa só o estar fraco. Em criativo o jogo não deixa magoar o jogador.
- **Mobs**: o que se tira do peito de um mob muda o mob. Sem coração sangra até morrer, sem espinha não se mexe, sem pulmões afoga-se, sem rins envenena-se; vida, força e velocidade seguem os músculos e o coração. O creeper sem o apêndice não explode, o wither sem a estrela não a larga.
- **Rejeição e compatibilidade**: cada órgão nasce no corpo de quem o tem (o uuid do jogador ou do mob) e guarda-o; noutro corpo é rejeitado (2 de dano a cada 600/n ticks para n órgãos alheios, com o efeito visível **Rejeição de Órgãos** a mostrar o tempo até ao próximo golpe). Os órgãos que um mob larga de saque servem a qualquer corpo, e 10 % dos órgãos do peito de um mob são dadores universais (escolhidos pelo uuid do mob, sempre os mesmos). O tooltip diz se o órgão serve só o corpo de onde veio.
- **Morte**: o jogador larga os órgãos que não são seus (os próprios perdem-se com o corpo e o peito volta ao normal ao reaparecer); com `keep_chest_cavity` o peito mantém-se e recebe coração, pulmão, espinha ou carne podre se lhe faltarem. Com `keepInventory` o jogo não avisa o mod da queda, por isso os órgãos alheios voltam ao inventário ao reaparecer. Um mob aberto larga só o que não é seu, e a tabela de saque já não lhe junta órgãos.
- **Saque**: o nível de Looting da arma conta (`0,1 + 0,04 × nível`, vezes o multiplicador de cada espécie; chefes largam sempre 3 ou mais). Um **cutelo** (os seis do mod) multiplica o Looting por 10 e desfaz os órgãos largados pelas receitas de salvamento do original (carne, farinha de osso, pólvora, ferro, enchidos). Os mesmos salvamentos existem como receitas de crafting (`required` órgãos juntos); o cutelo de netherite faz-se na bancada de ferraria.
- **Configuração** (`config/chestcavity.json`, criado ao abrir o mundo): `organ_rejection`, `rejection_rate`, `rejection_damage`, `keep_chest_cavity`, `opener_absolute_health`, `opener_fractional_health`, `universal_donor_rate`, `looting_boost`, `cooldown_scale`.
- **Comer**: digestão, nutrição, carnívoro/herbívoro, comida podre, resistência (cansaço) e metabolismo mais rápido. Náusea e fome com estômago ou intestinos maus. Intestino de ferro (comida sem efeitos maus) e rotgut (a comida podre deixa de dar Fome).
- **Poções**: desintoxicação (efeitos maus duram mais ou menos), purga de bónus e ossos secos contra o Wither.
- **Pânico** (Astro): ao levar dano o corpo ganha Velocidade (a força do efeito do original, +60 % por nível) durante o tempo do original, e só volta a acontecer depois de acabar.
- **Respirar**: pulmões, capacidade pulmonar e guelras, com as fórmulas do original.
- **Defesa**: ossos, fogo, impacto, salto (queda), frio e fogo do Beyond, esquiva de flechas (teleporte).
- **Corpo**: coração (e sangramento), rins (veneno com a duração do original), nervos, brilho, sorte, força, velocidade, alcance, recuo, leveza (gravidade).
- **Ambiente**: fotossíntese, cristalossíntese e cristalização (o End Crystal cura), hidroalergia, hidrofobia, fraqueza na água (menos dano e menos velocidade a minerar).
- **Golpes**: lançamento; veneno (soco com a mão vazia).
- **Habilidades ativas** (tecla **V** utilitárias, **R** ataques, ou `/cc ability <nome>`): reparar com ferro, pastar, seda, piromancia, ghastly, bombas de dragão, cuspo forte, explosão creepy.
- **Voo** (órgãos de voo do Astro): voo como em criativo.
- **Drops**: só órgãos verdadeiros (já não saem terra, paus nem pólvora).
- **Tooltip** de cada órgão com o que melhora, o que piora, as consequências e as habilidades; separador criativo com tudo.

60 dos 74 scores de órgãos têm efeito real (no 0.7.0 eram 50). 249 dos 254 órgãos têm pelo menos um efeito real; 232 têm todos.

## Falta

### Habilidades ativas
- Sopro de dragão (`dragon_breath`): nuvem de dano ao fundo do olhar.
- Balas de shulker (`shulker_bullets`): bala teleguiada ao ser vivo mais perto.
- Fornalha interior (`furnace_powered`) e o efeito Furnace Power; os itens internos `cud` e `furnace_power`.
- Exalar (`buoyant`), flutuação (`buoyant`, `neutral_water_buoyant`).
- Teclas próprias por habilidade (o original tem 12, sem tecla por omissão) e teclas configuráveis: o port já lê `key.chestcavity.<habilidade>` de `horizon-mods.txt`, mas V e R continuam a ser as únicas com tecla por omissão.
- Disparar um projétil de cada vez, de 5 em 5 ticks (aqui saem todos juntos) e o recuo ao disparar (o Horizon não empurra jogadores).
- Cooldowns como efeitos de poção visíveis: aqui são temporizadores do mod e perdem-se ao sair do jogo.
- Ruminar com o efeito Ruminating (aqui a erva é comida de uma vez); `eat_the_rich` (Astro: comer o bloco).
- Seda: lançar o fio ao chão (aqui vai para o inventário); seda ao ordenhar a vaca e ao tosquiar a ovelha.
- Explosão creepy: a nuvem com os efeitos de poção de quem explode.
- Voo do Astro como o original (alternar com `.` ou V, velocidade ×5).
- Beyond: o sistema de habilidades próprio (ecrã com V, tecla C, 54 teclas), teleporte do apêndice de Ender, lasers de guardião, caveira de Wither, balas de vento, onda sónica, fermentação de flores, purga do fígado enfeitiçado, jukebox portátil, habilidades automáticas dos mobs.

### Efeitos passivos
- Beyond: `lava_swim_speed`, `strider_kidney`, o fogo que se apaga com imunidade, a curva logarítmica de `defense`, `strength`, `speed` e `nerves` (o port usa as lineares do original), o lado-efeito de `scale`.
- Poção de água a rebentar perto de quem tem hidroalergia ou hidrofobia.
- Encurtar a distância de queda com `lightweight` e `buoyant`.
- Metabolismo mais lento que o normal (só o mais rápido está feito).
- Priões da comida humana; efeitos especiais ao comer certos órgãos (coração de dragão, carne tóxica, alienígena).
- Astro, reações ao ser atingido: `resonance`, `shrieking` e `cluster_explode` (explosões que não magoam o dono; o Horizon não dá explosões a mods, só se conseguem invocando um creeper, que magoa o dono).

### Rejeição e compatibilidade de órgãos
- O encantamento O Negative e a maldição Malpractice: o Horizon só lê encantamentos de `data/minecraft`, um mod de conteúdo não os pode juntar. Também sem Surgical e Tomophobia (ver abaixo).
- Ao morrer o original dá ao corpo um id de compatibilidade novo (os órgãos próprios que levavas no inventário passam a alheios); aqui o dono é sempre o teu uuid.
- Órgãos vitais ao reaparecer (Beyond).

### Cavidades e mobs
- Estado "aberta" e cavidade preguiçosa: o original só aplica os efeitos depois de a cavidade ser aberta. Aqui o jogador tem sempre a sua, e um mob que nunca foi aberto não muda.
- Os mixins do Beyond que mudam a IA de Enderman, Shulker, Blaze, Ghast, Breeze, Warden, Guardian, Llama, golem de neve, aranha de caverna, golem de ferro e strider: o Horizon não expõe os objetivos dos mobs.
- Ranhuras proibidas por tipo (o importador ignora `forbiddenSlots`). Os órgãos excecionais por tipo (o tronco do creeper vale vida, defesa e nervos) já estão.
- Tamanhos de cavidade do Beyond (27 a 54 ranhuras), conversão de órgãos ao transformar um mob.
- Partes do dragão: o clique numa parte devia abrir o dragão.
- Abrir com um dispensador (Astro).

### Loot e encantamentos
- Os encantamentos Surgical, Tomophobia e Malpractice (Horizon não os carrega, ver acima) e os 8 do Beyond (Teleoperation, Advanced Surgery, Safe Surgery, Prudent Surgery, Hydraulic Clamp, Anesthesia Surgery, Postoperative Suture, Primal Reversion).
- Órgãos nos baús de estruturas (pirâmide do deserto).

### Astro Adds
- Regrowth, autofagia, caça-tumores, infeção sculk e tumores (precisam de dados por pilha e temporizadores sobre a cavidade).

### Itens, receitas e configuração
- Receita da glândula de veneno (glândula + poção) e as glândulas de veneno e de alquimia a guardar efeitos: o Horizon não dá a grelha de crafting a um mod.
- Analisador biológico e tooltips detalhados com SHIFT (Beyond); as receitas do Beyond que usam as suas etiquetas (`bone_meal_rib`, `bone_meal_spine`, `rotten`, jukebox portátil).
- Os valores de comida do original (as comidas usam as da carne vanilla).
- Da configuração só estão as nove chaves acima: `HEART_HP`, `BONE_DEFENSE` e os valores de cada órgão estão fixos no código.
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
- Receitas de salvamento: exatamente `required` órgãos de cada vez (o original aceita qualquer múltiplo). O cutelo reconhece-se pelo nome (`chestcavity:*_cleaver`) e não pela etiqueta.
- Saque: os mobs largam órgãos ao morrer de qualquer causa (o jogo só diz o jogador que matou se se lembra do golpe; sem ele o Looting é 0), desde que a tabela de saque do mob tenha rolado.
- Rejeição: o efeito mostra o tempo até ao golpe mas o golpe é dado pelo mod, não pelo efeito (o Horizon ainda não dá comportamento aos efeitos de mods).
