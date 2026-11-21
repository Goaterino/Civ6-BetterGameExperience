# Registro de alterações

[English](../CHANGELOG.md) · outros idiomas: veja o [registro em inglês](../CHANGELOG.md).

## BetterGameExperience 0.14.0

No multijogador, todos os jogadores humanos precisam estar na 0.14.0 para as correções
**[todos os jogadores]**: uma sala de espera que mistura 0.13.0 e 0.14.0 as desliga.

### Instalação e atualizações
- **Arquivos assinados**: o instalador e as duas DLLs do BetterGameExperience agora têm um
  certificado de assinatura de código (Certum, desenvolvedor de código aberto). O Windows mostra o
  editor nas propriedades do arquivo e no aviso do instalador, e os alertas do SmartScreen e dos
  antivírus devem ficar mais raros.

### Versões do jogo
- **Civilization VI 1.0.12.58**: o BetterGameExperience agora reconhece esta versão antiga do jogo, em
  DX12 e DX11. Cada versão do jogo usa seus próprios endereços, e um jogo cujos arquivos não
  correspondem à sua versão não instala nenhuma correção.
- **Versão do jogo não suportada**: um jogador cuja versão do jogo não é suportada pelo
  BetterGameExperience é anunciado no chat ao chegar (a mensagem dizia que a cópia dele não era a
  versão atual da Steam).

### Correções do multijogador [todos os jogadores]
- **Menos dessincronizações após recarregar**: uma partida carregada em uma máquina calculava de forma
  um pouco diferente da partida em andamento nas outras. As melhorias lembradas pelos jogadores eram
  reescritas ao carregar, os ouvintes de eventos rodavam em outra ordem, a IA mantinha para sempre as
  preferências do seu líder (Sejong: -75% de pontos de Grande Profeta) e as unidades dadas ao criar
  um distrito (bônus de dificuldade da IA, inícios em eras avançadas) eram criadas em outra ordem.
  Uma partida carregada agora calcula como uma em andamento. Contribuição de OzrielTheLost.
- **Os mesmos cálculos em todo processador**: alguns cálculos (exponenciais, potências) podiam
  arredondar de forma diferente em processadores com e sem instruções FMA3. Todas as máquinas agora
  os calculam do mesmo jeito. Contribuição de OzrielTheLost.
- **Troca de unidades**: trocar duas unidades podia deixar uma no meio do caminho, empilhada em um
  hexágono com outra unidade (um espadachim sobre um tigre agachado). As duas unidades agora concluem
  o movimento.

### Multijogador
- **Dessincronizações reparadas sem recarregar**: quando um objeto continua dessincronizado, o
  anfitrião agora envia todos os seus dados, e a partida do jogador o repara no lugar em vez de se
  afastar até um recarregamento completo. Para isso, só o anfitrião precisa da 0.14.0. Contribuição de
  OzrielTheLost.

### Congresso Mundial
- **Turno do congresso após um fim de turno forçado**: quando o fim de turno simultâneo forçava o fim
  do turno de um jogador, o turno do congresso podia terminar dois segundos depois de abrir, sem
  votação. O turno do congresso agora começa com o cronômetro cheio, a janela de votação reabre quando
  o turno terminou sem voto e o botão do congresso mostra a sessão atual.

### Carregamentos mais rápidos
- **Recarregamentos mais rápidos**: com os mesmos mods, um recarregamento mantém a etapa de
  configuração e deixa os textos dos mods como estão, a última fase do carregamento não espera mais
  entre as etapas e a espera da VSync também é removida no DX11. Medido em um mapa pequeno: de 13 s
  para 9 s. Configurações nas opções avançadas, ligadas por padrão. Contribuição de OzrielTheLost.

### Interface
- **Atalhos da tela de acordo**: na coluna "disponível", Shift+clique adiciona o máximo que o acordo
  aceita e Ctrl+clique adiciona dez vezes um clique, para recursos, ouro, ouro por turno e favor.
  Ligado por padrão. Contribuição de huchenlei.
- **Selo de desconto dos distritos**: depois de colocar um distrito com desconto em uma cidade, outra
  cidade já vista continuava mostrando o desconto, embora custasse o preço cheio.
- **Produção excedente**: o "+N" no estandarte da cidade continuava aparecendo depois que o turno o
  tinha gastado.

### Configurações
- **Novidades de interface das atualizações**: uma nova configuração no topo da seção Interface. Em
  "Desativado", as novidades de interface das próximas atualizações chegam desligadas, e as que você
  já tem mantêm o estado atual.

## BetterGameExperience 0.13.0

No multijogador, todos os jogadores humanos precisam estar na 0.13.0 para as correções
**[todos os jogadores]**: uma sala de espera que mistura 0.12.0 e 0.13.0 as desliga.

### Atualização do jogo 1.0.12.74
- **Atualização de Civilization VI de 19 de novembro (1.0.12.74)**: o BetterGameExperience agora
  reconhece o jogo atualizado, em DX12 e DX11. As versões anteriores não instalam nele nenhuma das
  suas correções. Contribuição de yodha.

### Correções do multijogador [todos os jogadores]
- **Jogador que cai**: o jogo entregava um jogador desconectado à IA em cada máquina separadamente,
  sem nenhuma mensagem entre elas, então as máquinas podiam jogar aquele turno de forma diferente e
  dessincronizar. Agora a vaga continua humana, e o anfitrião encerra o turno do jogador ausente
  quando todos os humanos conectados encerraram o seu. Um jogador expulso continua indo para a IA.
  Contribuição de yodha.
- **Recarregamento de todos**: quando a partida de um jogador deixa de bater, o anfitrião agora
  inicia o recarregamento e todos os jogadores carregam a mesma cópia da partida dele; o turno só
  começa depois que cada jogador a carregou e sua partida foi verificada. Um jogador que ainda difere
  é recarregado mais uma vez. Contribuição de yodha.

## BetterGameExperience 0.12.0

No multijogador, todos os jogadores humanos precisam estar na 0.12.0 para as correções
**[todos os jogadores]**: uma sala de espera que mistura 0.11.0 e 0.12.0 as desliga.

### Correções do multijogador [todos os jogadores]
- **Dessincronização após uma reconexão**: um jogador que se reconectava, ou era recarregado
  sozinho, rodava uma partida recém-carregada contra as partidas em andamento dos outros. As duas
  diferem em dados que a verificação de sincronia não cobre, então a partida voltava a divergir e
  recarregava esse jogador de novo e de novo. Agora, quando a partida de um jogador deixa de
  corresponder, o anfitrião diz o motivo no chat e todos recarregam na hora a partida do anfitrião;
  o anfitrião espera a verificação de cada jogador sobre a partida recarregada e então o turno
  começa para todos. Contribuição de yodha.
- **Reconexão no fim de um turno**: um jogador que se reconectava durante um fim de turno era
  comparado com o turno seguinte, o que causava um reparo inútil e depois uma recarga a cada dois
  turnos. Contribuição de yodha.
- **Remap do Multiplayer Helper**: após um remap, o anfitrião rodava a partida recém-gerada e todos
  os outros jogadores a mesma partida carregada do jogo salvo do anfitrião, e a IA fazia escolhas
  ligeiramente diferentes em cada lado (reparos de vários MB por turno em uma partida de 8
  jogadores). Agora o anfitrião também carrega o próprio jogo salvo, então todas as máquinas rodam a
  mesma partida. Custo: uma tela de carregamento a mais para o anfitrião, uma vez por remap.
  Contribuição de yodha.
- **Cada partida começa do zero**: alguns contadores do jogo passavam de uma partida para a
  seguinte na mesma sessão, e os objetos eram numerados de forma diferente dos outros jogadores.
  Contribuição de yodha.

### Multijogador
- **Carregamento duplo ao iniciar um jogo salvo**: o anfitrião não envia mais uma segunda cópia da
  partida aos jogadores que ainda estão carregando, o que fazia alguns carregarem duas vezes. Quando
  uma cópia ainda é recusada, o chat mostra "Carregamento duplo evitado". Contribuição de yodha.
- **Mensagens de rede danificadas**: uma mensagem que apagaria as configurações da partida agora
  também é recusada. O chat nomeia o jogador e o que foi evitado ("Congelamento da partida evitado",
  "Apagamento das configurações da partida evitado"). Contribuição de yodha.
- **Mensagens cortadas no fim de um carregamento**: uma mensagem em espera durante um carregamento
  não é mais colada a uma mensagem que ainda chega em pedaços. Contribuição de yodha.
- **Correções do BetterGameExperience decididas no carregamento**: a decisão de ligar as correções
  **[todos os jogadores]** é verificada de novo no carregamento e quando a tela de carregamento
  fecha, um novo anfitrião após uma migração a assume (jogadores que entravam após uma migração
  jogavam sem elas), e jogadores ignorados continuam ignorados após uma recarga. Contribuição de
  yodha.
- **Cópia do jogo desatualizada**: um jogador cujo jogo não é a versão atual da Steam é anunciado no
  chat ao chegar, já que as correções de rede do BetterGameExperience não podem funcionar no PC dele.
- **Janela "Partida pausada"**: não se acumula mais durante os carregamentos nem aparece sobre a
  tela de carregamento.
- **Configuração da partida**: os sinais de equipe não podem mais ser ligados ou desligados depois
  que a partida começa (a mudança não fazia nada ou deixava os sinais ativos), jogos salvos
  incluídos. No First/Last moves e os sinais de equipe não podem mais ser ligados em uma sala de
  espera onde um jogador não tem o BetterGameExperience ou a versão certa, mas ainda podem ser
  desligados. Contribuição de mariusbw.

### No First/Last moves
- A opção continua ativa depois de ligada. O anfitrião não espera mais um jogador que não ficou
  pronto por dois turnos seguidos, e avisa no chat. Um jogador que não recebe o sinal de início do
  anfitrião por 75 s começa o turno sozinho. Contribuição de yodha.
- **Chat mais discreto**: a mensagem de ativação ficou só "No First/Last moves ativo.", e as linhas
  de ordens retidas e de largada aparecem uma vez por partida em vez de a cada turno. O aviso de que
  o BetterGameExperience está ativo é enviado uma vez por sala de espera, e não a cada vez que a tela
  da sala é reaberta.

### Carregamentos mais rápidos
- Novas configurações na aba BetterGameExperience, todas ligadas por padrão, que encurtam os
  carregamentos sem mudar a partida: busca mais rápida dos arquivos gráficos (vários segundos a
  partir do menu principal), banco de dados de jogo e textos dos mods mantidos entre carregamentos
  quando os mods são os mesmos (cerca de 1,5 s cada), sem espera do VSync nas telas de carregamento
  (cerca de 2 s, só com o VSync ligado), uma única limpeza de Lua ao fechar a interface (cerca de 2 s
  antes de um remap), um primeiro carregamento mais leve para o anfitrião no remap (cerca de 10 s) e
  uma análise de gargalos mais rápida na geração do mapa (cerca de 2 s, mesmo resultado). O registro
  de gargalos da IA (AI_ChokePoint.csv, 12 MB) não é mais gravado por padrão.
- **Threads de tarefas**: threads ociosas desperdiçam menos tempo de processador enquanto esperam
  (configurações na mesma aba).

### Interface
- **Formação mais alta primeiro**: na lista de produção, o botão principal de uma unidade treina a
  formação mais alta que a cidade pode treinar (exército, senão corpo; no mar, armada, senão frota),
  e a unidade sozinha vai para o menu suspenso abaixo. As compras com ouro e fé não mudam. Ligado
  por padrão. Contribuição de huchenlei.
- **Aviso antecipado de habitações**: os estandartes das suas cidades mostram uma casa laranja
  quando falta uma habitação para o limite (crescimento -50 %) e a casa vermelha assim que a cidade o
  atinge (-75 %); o jogo só mostrava a casa vermelha acima do limite. Ligado por padrão.
  Contribuição de huchenlei.
- **Cartas de política**: os efeitos delas não somem mais pelo resto da partida quando a tela do
  governo é desenhada pela primeira vez num momento ruim (visto após recarregar uma partida); só
  voltavam após outra recarga.
- **Bandeiras das unidades**: unidades de combate com cargas (legião romana, toa, nau…) voltam a
  mostrar suas promoções em vez das cargas; construtores, engenheiros militares e as demais unidades
  civis e de apoio continuam mostrando suas cargas.

### Relatório de bug
- Registros que contêm tokens de login (FiraxisLive.log, EOS.log) ficam de fora, e registros longos
  mantêm o início e o fim. Contribuição de yodha.

## BetterGameExperience 0.11.0

No multijogador, todos os jogadores humanos precisam estar na 0.11.0 para as correções
**[todos os jogadores]**: uma sala de espera que mistura 0.10.0 e 0.11.0 as desliga.

### Correções do multijogador [todos os jogadores]
- **Bônus de produção de unidades que se desgastam**: remover um bônus percentual à produção de
  unidades (os +30 % aos construtores do Ilkum, ou o bônus de uma cidade a um tipo de unidade)
  tirava um pouco mais do que ele tinha adicionado, cerca de 0,4 % a cada vez, e a perda crescia a
  cada troca da carta. Remover um bônus agora tira exatamente o que ele adicionou. Os jogos salvos
  mantêm a perda já acumulada. Contribuição de Serianol.

### Multijogador
- **Partida travada logo depois de carregar**: um jogador com conexão lenta podia receber a partida
  uma segunda vez do anfitrião logo depois do fim do próprio carregamento, recarregá-la a partir do
  mapa e travar. Essa segunda cópia atrasada agora é recusada; a entrada, a reconexão e as
  ressincronizações funcionam como antes.
- **Travamento ao carregar um jogo salvo depois de outra partida**: a reconexão dos jogadores
  retransmitidos pelo anfitrião (0.10.0) podia ler a conexão da partida anterior, já fechada, e
  fechar o jogo durante o carregamento da seguinte. Contribuição de yodha.
- **Configuração da partida**: as opções de partida adicionadas pelo BetterGameExperience ficam
  acinzentadas quando o anfitrião está com o BetterGameExperience desligado.

### Sinais de equipe
- Novas imagens e sons para a roda, os marcadores e cada chamado, instalados com o
  BetterGameExperience. Eles só são adicionados a uma partida se o anfitrião ligar os sinais de
  equipe e todos os jogadores tiverem o BetterGameExperience: um jogador sem BGE ainda pode entrar na
  sala de espera. Contribuição de mariusbw.
- O **volume dos sinais de equipe** fica logo abaixo dos efeitos sonoros nas opções de áudio, tanto
  no menu principal quanto na partida, e os atalhos dos sinais continuam listados nas opções de
  teclas. Contribuição de mariusbw.

### Configurações
- **Aba BetterGameExperience**: ligar ou desligar o BetterGameExperience sempre pede para reiniciar
  o jogo. Voltar uma configuração ao valor anterior sem reabrir a tela é salvo de novo; antes era
  ignorado.

## BetterGameExperience 0.10.0

No multijogador, todos os jogadores humanos precisam estar na 0.10.0 para as correções
**[todos os jogadores]** e os sinais de equipe: uma sala de espera que mistura 0.9.x e 0.10.0 os
desliga.

### Correções do multijogador [todos os jogadores]
- **Jogadores presos em « não pronto »**: uma ação diplomática (embaixada, acordo) recebida bem no
  fim do turno podia fazer o anfitrião começar o turno seguinte cedo demais. Alguns jogadores então o
  começavam já marcados como concluídos e não conseguiam ficar prontos de novo. O turno seguinte
  agora espera a verificação do jogo, e a ação continua sendo aplicada.
- **Espectadores (Better Spectator Mod)**: a reformulação dos espectadores da 0.9.0 está suspensa
  até ser testada mais a fundo, porque as partidas com um espectador ficavam menos estáveis. O
  BetterGameExperience não muda mais o nível de civilização dos espectadores nem substitui os
  scripts do Better Spectator Mod, e a opção « Better Spectator Mod para espectadores » deixa de
  existir por enquanto. Os espectadores voltam a contar como civilizações principais para scripts,
  vitórias e pontuação, como com o Better Spectator Mod sozinho. O turismo e as votações do
  Congresso Mundial continuam a ignorá-los.
- **Mantido para os espectadores**: trocar o jogador observado continua sem dessincronizar a partida,
  e o turno do espectador continua terminando sozinho após uma pausa do MPH. As duas correções agora
  se aplicam aos scripts originais do Better Spectator Mod.

### Novidades
- **Sinais de equipe (BGE)**: marque um hexágono para sua equipe com **X**, ou segure **X** para uma
  roda de avisos (perigo, estou indo, ajuda, inimigo sumiu); **Alt+1–4** os enviam diretamente. Os
  colegas de equipe veem e ouvem o sinal com o nome de quem enviou, os espectadores veem as duas
  equipes. Desligado por padrão: o anfitrião marca **Sinais de equipe (BGE)** na configuração da
  partida. Atalhos nas opções de teclas do jogo, volume nas opções de áudio; `/mute nome` bloqueia o
  chat e os sinais de um jogador. Contribuição de mariusbw.

### Multijogador
- **Jogadores retransmitidos pelo anfitrião**: dois jogadores cuja conexão direta falhou ao entrar
  passam pelo anfitrião até o fim da partida, e ambos são desconectados se o anfitrião sair. Durante
  o início de turno simultâneo, o BetterGameExperience agora tenta conectá-los diretamente de novo
  (três tentativas no máximo) e avisa no chat. Contribuição de yodha.
- **Pings travados em 0 ms**: o ping de um jogador na lista de jogadores não fica mais congelado em
  0 ms depois de entrar; ele volta a ser medido a cada 5 segundos. Não é mais preciso sair e voltar
  para « consertá-lo », o que muitas vezes causava uma dessincronização. Contribuição de yodha.
- **Mensagens de rede danificadas**: a proteção adicionada na 0.9.0 agora também funciona com a
  versão DirectX 11 do jogo. Contribuição de yodha.
- **Jogadores sem BetterGameExperience**: o aviso na partida sobre um jogador que entrou sem
  BetterGameExperience não para mais num erro de script. Contribuição de yodha.

### Instalação e atualizações
- O instalador baixa e reinicia o jogo sem iniciar programas auxiliares ocultos, e traz informações
  de versão: os antivírus devem sinalizá-lo com menos frequência.

## BetterGameExperience 0.9.1

Compatível com a 0.9.0 no multijogador: jogadores na 0.9.0 e na 0.9.1 jogam juntos com todas as
correções.

### Correções da interface
- **Tela de relatórios**: as abas Rendimentos e Cidades voltam a funcionar. O cálculo do efeito dos
  cartões de política, carregado na mesma tela, substituía alguns dos seus dados e as quebrava. As
  abas Políticas e Cidades-estado não falham mais em uma linha com pontos de grandes personalidades,
  e ordenar uma coluna da aba Cidades não falha mais quando a aba Rendimentos não foi aberta antes.
- **Distritos com desconto**: o desconto mostrado agora é o que o jogo aplica. O jogo só atualiza o
  custo de um distrito em certos momentos (nova tecnologia ou cívico, nova cidade, distrito do mesmo
  tipo posicionado ou concluído), então a lista podia mostrar um desconto que o jogo só dava alguns
  turnos depois.

## BetterGameExperience 0.9.0

Inclui tudo da pré-release 0.8.1. No multijogador, todos os jogadores humanos precisam estar na
0.9.0 para as correções **[todos os jogadores]**: uma sala de espera que mistura 0.8.x e 0.9.0 as
desativa.

### Correções do multijogador [todos os jogadores]
- **Espectadores (Better Spectator Mod)**: não contam mais como civilizações principais e saem das
  listas de jogadores dos scripts, das vitórias e da pontuação. O BetterGameExperience inclui as
  suas próprias versões dos scripts do Better Spectator Mod, feitas para isso. Contribuição de
  banipouz.
- **Dessincronização causada por um espectador**: mudar o jogador que um espectador observa não
  dessincroniza mais a partida (o Better Spectator Mod o gravava nos dados da partida, só no
  computador do espectador).
- **Espectadores na faixa de líderes**: os jogadores não veem mais os espectadores na faixa do canto
  superior direito, onde apareciam como uma IA, com um retrato « ? », o nome e os rendimentos.
- **Turno do espectador após uma pausa**: após uma pausa do MPH, o turno do espectador volta a
  terminar sozinho, em vez de esperar que ele o passe à mão.

### Multijogador
- **Mensagens de rede danificadas**: uma mensagem danificada, vista quando um jogador se
  desconectava, podia congelar o jogo até faltar memória alguns minutos depois. Os valores
  impossíveis de uma mensagem assim agora são descartados e a partida continua; uma linha no chat
  avisa a todos. Uma configuração de partida ou de jogador danificada não é mais aplicada, em vez de
  apagar a configuração. Protege os jogadores que têm o BetterGameExperience. Contribuição de yodha.
- **No First/Last moves com um anfitrião espectador**: o turno agora começa assim que os jogadores
  estão prontos, em vez de retê-los até o prazo de segurança.

### Novidades
- **Progresso da pesquisa e do princípio**: no painel superior esquerdo, ao lado do nome da
  tecnologia e do princípio em andamento, **x/y** mostra a ciência (a cultura) já investida e o
  custo total, seguido de **(+z)** quando o jogo guarda ciência (cultura) excedente de uma
  tecnologia (um princípio) concluída com mais que o seu custo. Uma configuração permite desativá-lo.
- **Disable Advanced UI indicators (BGE)**: nova opção de partida, logo abaixo de No First/Last
  moves, desativada por padrão. Quando o anfitrião a marca, nenhum jogador vê o progresso da
  pesquisa e do princípio (**x/y (+z)**) nem a produção guardada das cidades, sejam quais forem as
  suas configurações. Ela é relida a cada partida: numa partida sem a opção, eles voltam a aparecer
  sem reiniciar o Civ VI.
- **Cancelar a produção pelo estandarte da cidade**: um clique direito no ícone de produção do
  estandarte de uma de suas cidades remove o item em andamento, como no painel de produção.
- **Distritos com desconto**: na lista de produção, um distrito com custo reduzido (você tem menos
  distritos desse tipo que a sua média) mostra o desconto, por exemplo **-40%**. Ele é atualizado
  assim que um distrito é posicionado, pois posicionar um pode acabar com o desconto. Os distritos
  que não contam na média, como o Thành do Vietnã, também são indicados.
- **Produção em reserva**: o estandarte da cidade, o painel da cidade e o painel de produção
  mostram a produção que a cidade guarda em reserva (excedente de um item concluído, ou um turno
  sem produzir; o jogo guarda no máximo um turno), somada à sua produção do turno seguinte. No
  estandarte de uma cidade sem produção, ela aparece como **+N**.
- **BGE Check**: numa sala de espera multijogador, o anfitrião tem um botão **BGE CHECK**, ao lado
  do MOD CHECK do MPH, que pergunta a cada jogador a sua versão do BetterGameExperience e escreve o
  resultado no chat: mesma versão, outra versão, instalado mas desativado ou sem resposta.
- **Mensagem de chegada**: ao abrir uma sala de espera com o BetterGameExperience ativado, o chat
  avisa, com a sua versão exata.

### Correções da interface
- **Painel da cidade depois de remover a produção**: o painel no canto inferior direito é atualizado
  ao remover o único item da fila, em vez de continuar a mostrá-lo.
- **Clique direito na produção atual**: agora também a remove pela aba Produção, não só com a fila
  aberta.
- **Sala de espera do MPH**: fora de um draft, a lista de jogadores sobe para o espaço vazio que o
  MPH reserva para o draft, e o « . » mostrado acima dela numa partida recarregada desaparece.
- **Tela de diplomacia**: depois de conhecer um jogador cuja equipe faltava na lista de equipes do
  jogo, os ícones de líderes falhavam a cada turno, e abrir a tela de diplomacia podia esconder toda
  a interface de vez. Os ícones voltam a funcionar e, se a tela de diplomacia não abrir, a interface
  volta.
- **Imhotep**: o seu efeito agora mostra o valor na velocidade da partida (87, ou 175 para uma
  maravilha antiga ou clássica, na velocidade Online), na tela de personalidades ilustres e na
  Civilopédia, em vez de 175 em todos os casos.
- **Pausa do MPH**: o menu MULTIPLAYER HELPER e a votação de remap agora aparecem acima da tela de
  pausa do MPH, com botões clicáveis.
- **Barra superior**: as comodidades médias por cidade são arredondadas para o inteiro mais próximo
  em vez de para baixo.
- **Relatar um bug (BGE)**: o jogo salvo agora é anexado ao relatório; só os salvamentos
  automáticos eram enviados.

### Configurações
- As novidades da interface (cancelar pela faixa da cidade, distritos com desconto, produção
  guardada, progresso da pesquisa e do princípio) e os scripts do Better Spectator Mod têm cada um
  uma configuração na aba BetterGameExperience, ativada por padrão.

## BetterGameExperience 0.8.0

No multijogador, todos os jogadores humanos precisam estar na 0.8.0. A correção dos acordos no fim
do turno só protege quem a tem: um único jogador ainda na 0.7.x pode trazer de volta o turno travado
para todos. No saguão, o anfitrião vê em vermelho quem tem outra versão, e um saguão que mistura 0.7.x
e 0.8.0 desativa as correções **[todos os jogadores]**.

### Correções do multijogador [todos os jogadores]
- **Turno em loop depois de um acordo entre humanos**: um acordo proposto ou aceito bem no fim do seu
  turno podia chegar ao anfitrião depois do fim do turno. O turno seguinte então recomeçava sem parar
  para alguns jogadores, era preciso voltar ao saguão e a partida muitas vezes dessincronizava. Um
  acordo assim não é mais enviado depois que o seu cronômetro acabou, nem quando nenhum humano ainda
  está com o turno ativo: uma linha no seu chat avisa. Os acordos com as IAs não mudam.

### Novidades
- Botão **Relatar um bug (BGE)** no menu Esc, abaixo de Opções. Descreva o problema: o BGE envia um
  jogo salvo, os últimos salvamentos automáticos e os registros do jogo (sem o seu nome de usuário do
  Windows), e uma linha do seu chat dá o número do relatório. Os outros jogadores veem no chat que um
  relatório está sendo enviado. Se o envio falhar, o zip é copiado para a sua Área de Trabalho.

### Correções da interface
- **Efeito das cartas de política**: Racionalismo, Grande Ópera, Simultaneum e Livre Mercado também
  contam os rendimentos fixos adicionados às construções que reforçam (governadores…), e não só o
  rendimento base delas. Os bônus das cidades-estado nessas construções ficam de fora, como no
  jogo, que não os multiplica.
- **Escolha das rotas comerciais**: numa rota doméstica, o ouro que a habilidade de Poundmaker dá à
  cidade de destino (+1 por acampamento e por pasto) agora é mostrado, com uma linha na dica que
  nomeia a cidade.
- **Barra superior**: o botão da população também mostra a média de comodidades líquidas por cidade
  (em verde a partir de +3, em vermelho abaixo de 0).
- **Menu principal**: a versão do BetterGameExperience aparece no canto inferior esquerdo, depois da
  do jogo e da do MPH (`… - MPH / … - BGE / 0.8.0`).

### Configurações
- Opção de partida **"No First/Last moves (BGE)"**: agora marcada por padrão. O anfitrião ainda pode
  desmarcá-la.

## BetterGameExperience 0.7.0

No multijogador, as correções **[todos os jogadores]** exigem que todos os jogadores humanos
estejam na 0.7.0: um saguão que mistura 0.6.x e 0.7.0 as desativa.

### Correções de jogo [todos os jogadores]
- **Unidades empilhadas**: duas unidades militares da mesma classe podem acabar no mesmo hexágono
  (unidade comprada numa cidade ocupada, movimento interrompido, teletransporte…). A unidade
  excedente não conta mais nos bônus de flanqueamento e de apoio: duas unidades empilhadas ao lado
  do alvo dão +2, não +4. A defesa das cidades não muda.

### Congresso Mundial
- **Removidas**: as correções da 0.6.x que impediam o fim do turno durante a votação (sem fim de turno
  automático, sem fim de turno forçado com uma lista de resoluções vazia, 240 s no cronômetro do
  MPH). O cronômetro e o fim de turno do jogo e do MPH voltam a valer durante a votação. O botão
  **Voltar à votação**, a tela de boas-vindas uma vez por sessão e o recálculo dos luxos continuam.

### Correções da interface
- **Tela de cidades-estado**: os emissários posicionados mas ainda não confirmados não se perdem mais
  quando a tela é atualizada (emissário de outro jogador, missão, guerra ou paz). Eles só são
  retirados se a cidade-estado for destruída ou se sua equipe declarar guerra a ela.
- **Tela de acordo**: o botão do ouro disponível volta a funcionar. Ele podia ficar impossível de clicar
  após um acordo anterior (bug do jogo).
- **Mapa inteiro desde o início** (ativado por padrão): o minimapa e a câmera cobrem o mapa todo
  desde o primeiro turno, sem precisar de marcadores nas bordas do mapa. As áreas inexploradas
  continuam ocultas. Puramente local.
- **Área de movimento de uma unidade sobre um distrito inimigo**: sem mais travadas ao selecioná-la
  (acampamento inimigo não concluído), as zonas de controle (hexágonos
  vermelhos) são exibidas e os hexágonos de água onde a unidade ainda não pode entrar (costa antes
  do embarque, oceano antes da Cartografia) não são mais oferecidos.

### Configurações
- Botão **Redefinir BGE** na aba BetterGameExperience: volta todas as configurações ao valor
  padrão.
- Nova configuração avançada de diagnóstico, desativada por padrão: **Medir o custo da área de
  movimento (em tempo)**.

## BetterGameExperience 0.6.1

Inclui tudo da pré-versão 0.6.0. No multijogador, as correções **[todos os jogadores]** exigem que
todos os jogadores humanos estejam na 0.6.0 ou na 0.6.1: um saguão que mistura 0.5.0 e 0.6.x as
desativa.

### Instalação e atualizações
- O instalador não mostra mais a opção de inicialização da Steam após instalar: ele avisa que está
  tudo pronto, e você pode fechá-lo. O aviso de atualização do jogo basta para se manter atualizado.
  Uma opção de inicialização já adicionada continua funcionando.
- O instalador agora se chama `BetterGameExperience_V<version>.exe`. Ao ser executado, ele apaga os
  instaladores antigos da mesma pasta (geralmente Downloads).

### Multijogador
- **"No First/Last moves (BGE)"** **[todos os jogadores]**: você fica na tela de processamento do
  turno, botão de fim de turno incluído, até o anfitrião iniciar o turno, em vez de ver seu turno
  aberto com as ordens recusadas.

### Congresso Mundial
- *(experimental)* **[todos os jogadores]** Enquanto você espera os outros jogadores, um botão
  **Voltar à votação** reabre a janela de votação com seus votos e a quantidade certa de Favor
  Diplomático.
- *(experimental)* O turno não termina mais contra a sua vontade durante a votação: sem fim de turno
  automático, sem fim de turno forçado com uma lista de resoluções vazia, e o cronômetro do MPH dá os
  240 s previstos para a votação.
- *(experimental)* **[todos os jogadores]** Os recursos de luxo afetados por uma proibição são
  recalculados a cada sessão.
- A tela de boas-vindas do Congresso Mundial e seu som aparecem só uma vez por sessão, em vez de
  voltar toda vez que outro jogador volta à votação.

### Correções da interface
- Com o Better Spectator Mod, a barra superior volta a mostrar os recursos estratégicos dos seus
  colegas de equipe e os recursos que um colega desbloqueia.

## BetterGameExperience 0.5.0: primeira versão pública

O BetterGameExperience (BGE) corrige e acelera o Civilization VI: Gathering Storm com duas DLLs colocadas ao lado
dos arquivos do jogo. **Nenhum arquivo do jogo é substituído**: nem a verificação de arquivos da Steam
nem as atualizações do jogo o removem. Funciona com DirectX 11 e DirectX 12, com BBG, Better Spectator
Mod e MPH.

Todas as alterações estão listadas abaixo, por categoria. As marcadas com **[todos os jogadores]**
mudam as regras da partida: só são ativadas quando todos os jogadores humanos da sala têm a mesma
versão do BetterGameExperience. Todo o resto afeta apenas o seu computador. Dá para jogar sem risco com quem
não tem o mod.

### Instalação e atualizações
- Instalador `BetterGameExperience.exe`: instalar, atualizar, reparar e desinstalar. Ele encontra o jogo nas
  suas bibliotecas da Steam e verifica o manifesto assinado e o SHA-256 de cada arquivo antes de copiar
  qualquer coisa.
- Dois arquivos são adicionados ao lado dos do jogo: `winmm.dll` e `BGD_GameCore_XP2.dll`. Instalações
  antigas (0.3.0 e anteriores) são limpas automaticamente. Um `winmm.dll` de outro mod nunca é
  sobrescrito.
- Aviso de atualização no menu principal, inclusive no do MPH. **Atualizar** fecha o jogo, instala a
  nova versão e oferece reiniciar o jogo.
- Caixa "Versão de teste" no instalador para testar as próximas versões, com volta à versão estável
  em um clique.
- Opção de inicialização da Steam opcional, que procura atualizações sempre que você inicia o jogo.
- Instalador em 11 idiomas, os do Better Balanced Game. Ele segue o idioma do Windows, e dá para
  escolher outro na parte de baixo da janela.

### Configurações
- Nova aba **BetterGameExperience** nas opções do jogo: um interruptor geral que desativa tudo sem
  desinstalar, uma configuração por recurso e opções avançadas ocultas por padrão. Ela avisa quando
  uma mudança precisa de reinício.
- Créditos para os autores dos mods de interface integrados.
- Todos os textos do jogo estão disponíveis nos 11 idiomas do Better Balanced Game: inglês, francês,
  alemão, espanhol, italiano, japonês, coreano, polonês, português do Brasil, russo e chinês
  simplificado.

### Multijogador
- **Verificação na sala**: cada computador informa a sua versão do BetterGameExperience, e o anfitrião
  decide por todos. Mensagens de chat com o prefixo `BGE -` dizem quem está sem o mod ou com outra
  versão. As reconexões são tratadas. Se alguém entrar sem o mod enquanto as correções **[todos os
  jogadores]** estão ativas, o anfitrião recebe um aviso.
- As mensagens de chat `BGE -` são enviadas em inglês, para que quem não tem o mod possa lê-las. Quem
  tem o mod as vê no idioma do próprio jogo.
- **Opção de partida "No First/Last moves (BGE)"** **[todos os jogadores]**:
  - Todos os jogadores começam o turno juntos. As ordens ficam retidas, e o cronômetro parado, até
    que todos terminem de carregar (no máximo 60 s).
  - O turno termina para todos quando o cronômetro do anfitrião acaba.
  - A antiga sincronização de turno do MPH é desativada.
- Partidas carregadas na sala do MPH: as cidades-estado não causam mais "Opções de jogador inválidas"
  nem o botão Pronto desativado.

### Correções de jogo [todos os jogadores]
- Hexágonos trabalhados: uma cidade não trabalha mais um hexágono 2/1 enquanto um 2/2 está livre (foco
  em comida e produção).
- Os espectadores do Better Spectator Mod não contam mais no turismo. Eles não podem votar no
  Congresso Mundial nem ser alvo de uma resolução. Continuam podendo ver a tela do Congresso.
- Unidades navais podem se teletransportar para uma cidade a partir de qualquer distrito naval
  (porto, parque aquático, canal…), e não só de um centro da cidade, assim como já podiam se
  teletransportar de uma cidade para qualquer distrito naval (chega de almirante preso num lago de
  um hexágono).

### Correções da interface
- Os botões de ataque (cidade, acampamento, ataque nuclear de um silo) são desenhados acima das
  bandeiras das unidades, onde dá para clicar neles.
- O ataque à distância de uma cidade não selecionada precisa de um clique, não dois.
- No modo de mira do ataque à distância de uma cidade ou de um distrito, clicar na bandeira de uma
  unidade inimiga ataca essa unidade.
- A câmera não centraliza mais ao clicar num botão de ataque (dá para reativar).
- O contador de turnos de produção agora inclui os bônus de governo: fascismo, autocracia e república
  mercante.
- Uma maravilha retirada da fila de produção após duas trocas é recolocada.
- A produção em andamento é atualizada corretamente ao passar para outra cidade.
- Os rendimentos do império na barra superior são atualizados após uma ação de governador ou uma
  grande personalidade.
- *(experimental)* Os atalhos de teclado não param mais de funcionar quando uma tela de diplomacia
  fecha o menu Esc, ou após uma janela dupla do Congresso.
- Esc agora fecha corretamente a tela de enviados: enviados pendentes não bloqueiam mais o fim do
  turno.
- *(experimental)* A lente do colono é redesenhada após a fundação de uma cidade, e os hexágonos perto
  demais de uma cidade aparecem como proibidos.
- Enter segue a sua tecla de "Próxima ação": dá para tirá-la de lá e usá-la em outro atalho.
  Shift+Enter continua forçando o fim do turno.
- A área de movimento aparece para uma unidade num distrito defensável inimigo.
- *(experimental)* Os marcadores do mapa não são mais reconstruídos quando nada mudou: menos
  travamentos no multijogador.

### Mods de interface integrados
Cada um pode ser desativado na aba BetterGameExperience. Se você também tiver o mod da Oficina, a versão
integrada ao BGE o substitui.
- **Cargas dos construtores** nas bandeiras das unidades (baseado em Better Builder Charges Tracking,
  de wltk).
- **Turnos restantes das rotas comerciais** nas bandeiras dos seus comerciantes (novo).
- Botão **Limpar notificações** (baseado em NotificationClear, de 336tarot).
- **Barra superior ampliada**: comida, produção, população e luxos, com o detalhe por cidade (baseado
  em TopPanel Extension Pro, de 号码菌 / Azukii).
- **Efeito dos cartões de política**: cada cartão mostra o seu ganho real para o seu império (baseado
  em Extended Policy Cards, de Aristos). O Better Report Screen não é mais necessário. Valores
  corrigidos para:
  - Raj, Almedina, Monasticismo e Prestígio civil;
  - Alistamento e Levée en Masse;
  - os cartões Pretório e Impostos Coloniais;
  - os pontos de grandes personalidades, agora mostrados como [ícone]+x.
- **Tela de relatórios melhorada** com as abas Acordos, Unidades, Políticas, Cidades-estado e Cidades
  (baseada em Better Report Screen, de Infixo), com as mesmas correções.

### Desempenho e conforto
- Passagem de turno mais rápida:
  - Durante uma cascata de declarações de guerra, os dados de cidades e de diplomacia são
    atualizados só uma vez. O turno medido ficou 2,4 s mais rápido (−82 % na cascata): muito menos
    lag quando cidades-estado trocam de lado numa guerra.
  - Dois caches aceleram o turno da IA: melhorias construíveis e rendimentos dos hexágonos.
  - Os resultados são idênticos aos do jogo base: sem risco com jogadores que não têm o mod.
- A animação dos ataques nucleares fica 20 vezes mais rápida com o combate rápido ativado: cerca de
  0,6 s em vez de 11 s. Apenas exibição.
- O limitador de quadros oferece 45, 75, 90 e 100 FPS.

### Limitações conhecidas
- O mod foi feito para a versão oficial atual do jogo. Após uma atualização do jogo, os recursos
  afetados se desativam sozinhos até que o BetterGameExperience seja atualizado.
- Um jogador que entra no meio da partida sem o mod, enquanto as correções **[todos os jogadores]**
  estão ativas, vai dessincronizar. O anfitrião recebe um aviso.
- As correções marcadas como *(experimental)* ainda não foram testadas numa partida real, ou o
  efeito delas ainda não foi totalmente medido.
- As traduções que não são em inglês nem em francês ainda não foram revisadas por falantes nativos.
  Correções são bem-vindas.
