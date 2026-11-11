# Registro de alterações

[English](../CHANGELOG.md) · outros idiomas: veja o [registro em inglês](../CHANGELOG.md).

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
