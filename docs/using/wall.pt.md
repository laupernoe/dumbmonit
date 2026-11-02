# Modo parede {#wall-mode-and-music}

`/wall` mostra o quadro próprio da consola, em ecrã inteiro, para um monitor que
fica ligado numa sala. Atualiza-se sozinho e mantém o ecrã acordado.

De cima para baixo:

- o **veredicto** em poucas palavras («All good», «3 problems») com a data e o
  relógio, e quando foi atualizado pela última vez;
- os **cinco números** do resumo: monitores ativos, disponibilidade a 30 dias,
  tempo de resposta p95, alertas abertos e tempo médio de resolução;
- o **rio de saúde**, a grande imagem do quadro: uma faixa por monitor, os piores
  no topo;
- os **alertas abertos**, os mais graves primeiro;
- **quem está de prevenção** agora, com a escala e o fim do turno.

Sem widgets: fundo preto, a tipografia da consola, estados escritos junto de uma
forma e nunca só por cor. A [música](#music-on-the-wall), depois de configurada,
fica numa pequena doca ao fundo da coluna da direita.

## Abrir e sair

Abra-o com o ícone **Wall mode** no fundo do menu lateral, a partir da paleta de
comandos («Wall mode») ou escrevendo o URL. ++esc++ ou **Exit** volta à consola.
Uma conta limitada a algumas equipas ou clientes não tem o modo parede, porque
mostra toda a frota.

## Fundo {#backdrop}

A parede partilha o aspeto escuro da consola. O botão **Fundo** (em cima à
direita, ao lado de Exit) pode desenhar uma das cidades à noite das
[páginas de estado](status-pages.md) por trás do veredicto e do relógio: Paris,
Nova Iorque, Tóquio ou Dubai, ou *Auto* para a mais próxima do fuso horário do
navegador. Está desligado por predefinição e é lembrado por navegador;
`?city=paris` (ou `newyork`, `tokyo`, `dubai`, `none`) numa ligação escolhe-o só
para esse ecrã. O céu segue o painel: estrelas quando está tudo bem, nuvens com
um aviso, trovoada com um alerta. Num telemóvel, ou com o separador escondido, a
cidade fica parada. Um `zermatt` guardado ou numa ligação vai agora dar ao
Dubai.

## Música na parede {#music-on-the-wall}

A parede também pode reproduzir música e mostrar o que está a tocar: inicia uma
playlist no telemóvel, ouve-a na TV e vê-a do outro lado da sala. A música fica
numa pequena doca ao fundo da coluna da direita, sob o relógio e a prevenção:
nunca tira espaço ao veredicto nem aos problemas, e desaparece quando nada toca
ou nada está configurado. Mostra a capa, o título, o artista, o dispositivo que
toca e uma fina barra de progresso; quando a conta ligada o permite, um botão de
**reproduzir / pausa** pausa ou retoma a conta onde quer que toque (qualquer
sessão iniciada pode, contas de leitura incluídas, como em *Reproduzir aqui*). O
leitor de uma ligação partilhada e as mensagens do altifalante recolhem-se numa
pequena pastilha (o ecrã lembra-se). O botão **Música** da parede, ao lado de
Fundo e Sair, aparece quando a música está configurada.

Há duas formas, e pode usar ambas.

| | Spotify Connect | Uma ligação enviada para as paredes |
|---|---|---|
| Iniciar a partir de | a app Spotify no seu telefone (**Devices → DumbMonit Wall**) | qualquer ecrã DumbMonit, incluindo o seu telefone (**Play on the wall**) |
| Serviços | Spotify | Spotify, Deezer, YouTube e YouTube Music |
| Precisa de | Spotify Premium, uma app Spotify Developer própria, o telefone na mesma conta Spotify, a parede servida via HTTPS (ou localhost) num navegador com DRM | nada; sem sessão iniciada na parede, o Spotify e o Deezer reproduzem pré-visualizações de 30 segundos |
| Mostra o que toca | sim, onde quer que toque: a parede, a app Spotify de uma TV, um altifalante cast | o próprio leitor do serviço |

Ligar o Spotify e enviar ligações são ações de administrador; uma
conta de leitor na parede reproduz e mostra tudo.

### Spotify Connect: a parede torna-se um altifalante {#spotify-connect-the-wall-becomes-a-speaker}

O objetivo: no seu telefone, abra o Spotify, toque no ícone de
dispositivos, escolha a TV — a música toca na TV enquanto a parede
mostra o que está a tocar. A parede faz isto com o **Web Playback
SDK** do Spotify: o navegador da parede regista-se como um
dispositivo Spotify Connect. Isso só funciona quando **todas** estas
condições se verificam, e a parede diz-lhe no seu próprio ecrã qual
está em falta:

| Precisa de | Porquê | Quando falta |
|---|---|---|
| **Spotify Premium** na conta ligada | O Spotify só deixa contas Premium usarem altifalantes de navegador | a parede mostra *Spotify Premium is required*; Settings mostra *Premium: No* |
| **O telefone com sessão iniciada na mesma conta Spotify** | um altifalante de navegador só é listado para a conta cujo token detém — não é descoberto na rede como um altifalante inteligente, e outros membros de um plano familiar não o veem | o telefone simplesmente não lista a parede |
| **HTTPS** (ou `http://localhost` no próprio ecrã) | os navegadores só permitem media protegida (DRM) numa página segura | a parede mostra *opened over plain HTTP* |
| **Um navegador cujo DRM funciona** (Widevine, PlayReady ou FairPlay) | o Spotify transmite áudio encriptado | a parede mostra *This browser cannot play Spotify* |
| **Um toque ou tecla na parede por carregamento de página** | os navegadores mantêm uma página silenciosa até alguém interagir com ela | a parede mostra *Tap to enable sound* |

Quais navegadores funcionam: Chrome, Edge e Firefox num computador
(Firefox: permita *Play DRM-controlled content*), Safari num Mac,
Chromium com Widevine num Raspberry Pi (`libwidevinecdm0`). A maioria
dos **navegadores de smart TV** (Samsung Tizen, LG webOS),
**navegadores kiosk** e muitos **builds Chromium para Linux** vêm sem
um Widevine utilizável: não podem ser um altifalante Spotify,
independentemente do que o DumbMonit faça. Para essa TV, ver
[Quando o navegador da TV não consegue reproduzir Spotify](#when-the-tvs-browser-cannot-play-spotify).

#### Configurar {#set-it-up}

1. **Crie uma app Spotify.** Abra o [Spotify Developer Dashboard](https://developer.spotify.com/dashboard), crie uma app (qualquer nome), marque **Web API** e **Web Playback SDK**, e adicione o URI de redirecionamento que **Settings → Wall music** mostra:
    - DumbMonit servido via HTTPS: `https://<o seu endereço>/api/music/spotify/callback`. O Spotify volta diretamente ao DumbMonit depois de aprovar.
    - DumbMonit servido via HTTP simples: `http://127.0.0.1:8888/callback`. O Spotify só aceita `http://` para um endereço loopback (e não `localhost`), desde 2025. Nada responde nesse endereço: depois de aprovar, o seu navegador mostra uma página de erro cujo endereço transporta o código — copie-o e cole-o de volta no DumbMonit.
2. **Ligue.** Em **Settings → Wall music**, cole o **Client ID** da app (não há segredo: o DumbMonit usa Authorization Code com PKCE) e escolha **Connect Spotify**, depois aprove no Spotify **com a conta que o seu telefone usa**.
3. **Na TV**, abra o `/wall` do DumbMonit via HTTPS, com sessão iniciada (uma conta de leitor basta), e toque no ecrã ou pressione **OK** no controlo remoto uma vez: qualquer gesto desbloqueia o som. Um navegador iniciado com `--autoplay-policy=no-user-gesture-required` (um kiosk) não precisa de toque.
4. **No seu telefone**, na app Spotify, toque no ícone de dispositivos e escolha **DumbMonit Wall**. O som sai da TV, e a parede mostra a capa, título e progresso.

**Settings → Wall music → Speaker** mostra se tudo se verifica, sem ir
até à TV: a conta, Premium, se o Spotify lista o altifalante entre os
dispositivos da conta agora mesmo (e que dispositivos lista), e toda
parede que reportou nos últimos quinze minutos — o seu navegador
(«Chrome 130 on Linux»), se está pronta, e porque não. **Test sound**
faz a conta reproduzir no altifalante (retoma o que tocou por último);
cada parede pronta também tem o seu próprio **Test sound**. Na própria
parede, o painel **Music** tem **Play here**, que move o que estiver a
tocar na conta para este ecrã.

**O nome.** Renomeie o altifalante em **Settings → Wall music →
Speaker** («Living room TV»); toda parede capta o novo nome dentro de
segundos. Para nomear uma única parede, adicione `?speaker=Kitchen` ao
seu endereço — o ecrã lembra-se dele (`?speaker=` esquece-o).

**Uma parede fica ligada durante dias.** A parede pede ao DumbMonit um
token fresco quando o do Spotify expira (a cada hora); quando o
dispositivo cai (Wi-Fi, uma placa de rede em suspensão) e não volta
dentro de trinta segundos, a parede regista-se de novo; uma falha
passageira é repetida por si só (15 s, 30 s, … depois a cada cinco
minutos); e se o Spotify deixar de listar a parede enquanto ela pensa
que está pronta, regista-se de novo. Só o que a espera não consegue
resolver — sem DRM, sem Premium — para de vez, até **Try again**.

O DumbMonit mantém o token de atualização do Spotify encriptado com o
segredo da instância e nunca o envia a um navegador. A parede só
recebe um token de acesso de curta duração para o seu leitor, e só
através de uma sessão com sessão iniciada (um token de API não pode
obter um). **Disconnect** esquece a conta; para também remover o
DumbMonit da sua conta Spotify, use
[spotify.com/account/apps](https://www.spotify.com/account/apps/).

Também bom saber:

- Uma app Spotify pessoal (*development mode*) pertence a uma conta
  Premium e admite até cinco contas: adicione outras em **User
  Management** no painel.
- **Volte a ligar a cada seis meses.** O Spotify termina uma ligação
  seis meses depois de ter sido aprovada; Settings mostra a data, e a
  parede diz isso quando acontece.

**Now playing funciona sem nada disso.** O servidor do DumbMonit
pergunta ao Spotify o que está a tocar na conta a cada poucos segundos
(um pedido para todas as paredes) e a parede mostra a capa, título,
artista, progresso e o dispositivo em que toca — a própria parede, a
app Spotify de uma smart TV, um Chromecast, o seu portátil — mesmo via
HTTP simples e num navegador que não consegue reproduzir Spotify. Para
usar só isso, desligue **Use this display as a Spotify speaker** no
painel Music da parede.

#### Quando o navegador da TV não consegue reproduzir Spotify {#when-the-tvs-browser-cannot-play-spotify}

Escolha o que corresponde à sua configuração — em todos os casos a
parede continua a mostrar o que toca, porque *now playing* lê a
conta, não o altifalante:

- **A TV tem uma app Spotify, ou um Chromecast / Google TV / Fire TV
  stick.** Escolha-o na lista de dispositivos do Spotify; a parede
  mostra «Playing on Living room TV». Esta é a opção mais simples e
  fiável.
- **Um pequeno computador conduz a TV** (um Raspberry Pi, um mini PC,
  um portátil antigo). Abra a parede no Chrome ou Chromium com
  Widevine nele: torna-se o altifalante como descrito acima.
- **Quer um altifalante que está sempre lá, mesmo sem navegador.**
  Execute um recetor Spotify Connect baseado em
  [librespot](https://github.com/librespot-org/librespot) (MIT) — por
  exemplo [raspotify](https://github.com/dtcooper/raspotify) ou
  spotifyd — **na máquina cuja saída de áudio alcança a TV ou o
  altifalante**. Aparece na lista de dispositivos do telefone para
  todos na rede.

O DumbMonit não traz um recetor desses, de propósito: o contentor
DumbMonit geralmente corre num servidor num armário, não na máquina
ligada à TV, por isso o seu som saíria da caixa errada; um recetor
precisa da placa de som do anfitrião e de rede do anfitrião (DNS
multicast) que um contentor de monitorização não deveria ter; e o
librespot depende de protocolos Spotify não oficiais que se avariam de
tempos a tempos. Um recetor instalado na máquina certa, junto ao
DumbMonit, faz o trabalho melhor.

### Uma ligação enviada para toda parede {#a-link-sent-to-every-wall}

Em **Settings → Wall music** (ou o próprio painel **Music** da
parede), cole uma ligação Spotify, Deezer ou YouTube para uma faixa,
álbum, lista de reprodução, episódio ou vídeo e escolha **Play on the
wall**. Toda parede reproduz-a no leitor incorporado oficial do
serviço; uma ligação enviada enquanto uma parede está aberta começa
por si só onde o serviço o permite (YouTube, Deezer) uma vez a parede
tocada. **Stop** remove-a de toda parede.

O Deezer e o YouTube não têm equivalente ao Spotify Connect que uma
página web possa usar — o Deezer já não aceita novas apps de
programador e a sua API nunca expôs o que está a tocar — por isso uma
ligação é a forma para eles. Os leitores incorporados reproduzem
pré-visualizações de 30 segundos a menos que o navegador da parede
tenha sessão iniciada no serviço.

### Privacidade {#privacy}

Nada de terceiros carrega na parede até o Spotify estar ligado ou uma
ligação definida. A política de segurança de conteúdo da página
permite exatamente os três leitores incorporados, o script de leitor
do Spotify (`sdk.scdn.co/spotify-player.js`) e a sua frame, e cobre
de `i.scdn.co` — nada mais, e nada fora da interface web.

A política de permissões (*Permissions-Policy*) da interface só concede a
reprodução automática e os conteúdos protegidos (DRM) a esses leitores; as
páginas de estado públicas não recebem nenhum.

### API {#api_1}

| Rota | Quem | O quê |
|---|---|---|
| `GET /api/music/now` | qualquer sessão ou token | O que toca na conta Spotify (em cache alguns segundos) e a ligação que as paredes reproduzem |
| `PUT /api/music/link` `{"link": "…"}` | admin | Toda parede reproduz esta ligação |
| `DELETE /api/music/link` | admin | As paredes param o leitor incorporado |
| `GET /api/music/spotify` | qualquer sessão ou token | A conta ligada: estado, Client ID, data de reconexão — nunca um token |
| `POST /api/music/spotify/authorize` | sessão de admin | Inicia uma ligação: `{"client_id", "redirect_uri"}` → URL de aprovação do Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | sessão de admin | Termina uma ligação loopback com o endereço em que o navegador aterrou |
| `GET /api/music/spotify/callback` | o navegador do admin | O redirecionamento direto do Spotify (instalações HTTPS): entrega o código à página de definições, que conclui a ligação com a sua sessão |
| `DELETE /api/music/spotify` | admin | Desliga e esquece os tokens |
| `GET /api/music/spotify/token` | só sessão de navegador | Um token de acesso de curta duração para o leitor da parede |
| `GET /api/music/speaker` | qualquer sessão ou token | O altifalante: nome, Premium, os dispositivos Spotify Connect da conta, se o altifalante está listado, o que cada parede reportou |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Renomeia o altifalante (`null` volta a «DumbMonit Wall») |
| `POST /api/music/speaker/report` | só sessão de navegador | Uma parede reporta o seu altifalante (enviado a cada minuto); a resposta diz se o Spotify o lista |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | só sessão de navegador | Reproduz a conta nesse dispositivo, ou no dispositivo nomeado como o altifalante («Test sound», «Play here») |
| `POST /api/music/spotify/playback` `{"playing": false}` | só sessão de navegador | Pausa (`false`) ou retoma (`true`) a conta onde quer que toque; `can_control` em `GET /api/music/now` diz se é permitido |
