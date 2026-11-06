# Registro de alterações

[English](../CHANGELOG.md) · outros idiomas: veja o [registro em inglês](../CHANGELOG.md).

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
