# Modo parede e música {#wall-mode-and-music}

`/wall` mostra só o boletim, em tela cheia, para um ecrã que fica
sempre ligado numa sala: a frase, a janela meteorológica, três
leituras grandes e a lista *Needs you*, atualizadas a cada 20
segundos, com o ecrã mantido aceso. ++esc++ ou **Exit** volta à visão
geral. Abra-o a partir da paleta de comandos («Wall mode») ou
escrevendo o URL. `?theme=dark` ou `?theme=light` força um tema para
esse ecrã sem tocar na sua preferência guardada.

![Modo parede](../assets/screenshots/wall-light.png){ loading=lazy }

Cada problema à esquerda leva o ícone do produto do seu monitor (as
mesmas marcas das listas de monitores). O que está em baixo vem primeiro, depois
os avisos, depois o que um pai mascara, depois o que está em pausa ou
reconhecido; os problemas idênticos juntam-se numa linha com uma contagem
(*×5*). A lista nunca ultrapassa o seu espaço: num ecrã pequeno, ou com muitos
problemas, os mosaicos grandes encolhem para uma linha cada, os problemas
mascarados e em pausa resumem-se numa linha, e só depois o resto recolhe-se em
*N problemas a mais*, em vez de se sobreporem aos monitores de baixo. Os
pombos, o relógio e os painéis da cena afastam-se logo que ficariam sob o
texto.

## Notícias na parede {#news-on-the-wall}

Um administrador pode acrescentar **títulos** à parede em **Settings → Wall
news**. Vêm desativados por predefinição.

Os últimos títulos dos feeds escolhidos, só texto, cada um com a sua fonte,
atualizados de cinco em cinco minutos. Ficam onde a disposição da parede os põe
(a coluna direita por predefinição, ver [Disposição](#layout)): tantos quantos
couberem, com duas linhas no máximo cada um, parados, sem deslizar nem alternar.
Numa barra, um título fica numa só linha.

É o servidor que lê os feeds, não o navegador (os navegadores não podem ler
os feeds de outros sites). Segue no máximo três redirecionamentos, recusa
endereços internos, desiste ao fim de oito segundos ou 2 MB por feed, e
guarda a resposta em cache durante cinco minutos para todas as paredes. Lê
RSS 2.0, RSS 1.0 (RDF) e Atom.

### Escolher as fontes {#choosing-sources}

Escolha um **país** (pré-selecionado a partir da região e do idioma do seu
navegador), depois marque as suas fontes: até oito feeds no total. Para usar
qualquer outro feed, cole o seu endereço RSS ou Atom em **Add a custom feed**
e prima **Test**: o DumbMonit lê-o a sério e mostra o último título antes de
o acrescentar.

| País | Títulos |
|---|---|
| França | BFM TV, franceinfo, Le Monde, France 24 |
| Suíça | RTS Info, SRF News, Le Temps, NZZ |
| Bélgica | RTBF, VRT NWS, La DH |
| Alemanha | Tagesschau, ZDFheute, Spiegel, Deutsche Welle |
| Reino Unido | BBC News, The Guardian |
| Estados Unidos | NPR, PBS NewsHour, CBS News, The New York Times |
| Canadá | Radio-Canada, CBC News |
| Espanha | RTVE, 20minutos, El País, El Mundo |
| Itália | ANSA, RaiNews |
| Portugal | RTP Notícias, Público |
| Brasil | G1, Folha de S.Paulo |
| Rússia | Interfax, Meduza, Kommersant |
| China | CGTN, South China Morning Post, Xinhua |
| Internacional | Deutsche Welle, France 24, BBC News (world) |

O feed da BFM TV é `https://www.bfmtv.com/rss/news-24-7/`. A lista foi
verificada quando foi escrita; um site pode mudar o seu feed a qualquer
momento, e então a parede limita-se a ignorá-lo. Use **Test** na substituição
e acrescente-a como feed personalizado.

As imagens e o mosaico de vídeo em direto de versões anteriores foram retirados;
uma parede guardada com eles limita-se a deixar de os mostrar.

### API {#api}

`GET /api/wall/news` (o que as paredes consultam), `GET|PUT
/api/wall/news/settings` e `POST /api/wall/news/test` (admin), `GET
/api/wall/news/presets`.

## Widgets na parede {#widgets-on-the-wall}

Um administrador pode acrescentar **widgets** à parede em **Settings → Wall
display → Wall widgets**: o tempo, preços de criptomoedas, cotações de ações,
taxas de câmbio e as últimas vulnerabilidades críticas. Cada um vem desativado
por predefinição e ativa-se por si só.

O **tempo** é mantido simples: um ícone, a temperatura atual e o local. Sem
previsão, sem detalhes. Um leitor de ecrã ouve o local, o céu e a temperatura.

Todos os widgets ativados são mostrados **ao mesmo tempo**, pequenos e parados:
nada alterna, nada desliza. Os widgets falam mais baixo do que os monitores
(texto mais pequeno, tinta secundária): os problemas continuam a ser a razão de
ser da parede. Quando uma zona fica sem espaço, os seus widgets mostram menos em
vez de alternar: primeiro a forma curta (sem percentagens nem etiquetas), depois
menos entradas, com um discreto *+N* para o resto.

### Disposição {#layout}

Em **Settings → Wall display → Wall widgets → Layout**, cada widget (os títulos
incluídos) vai para uma de quatro zonas, pela ordem que escolher:

- **Top bar**: uma linha fina no topo da parede;
- **Bottom bar**: uma linha fina na base, como uma barra de estado;
- **Right column**: cartões leves sob o relógio;
- **Hidden**.

Mude a zona de um widget com o seu menu, e o lugar dele dentro da zona com as
setas para cima e para baixo (também pelo teclado). Por predefinição, o tempo, os
títulos e as vulnerabilidades ficam na coluna direita, e os preços e a utilização
de IA na barra inferior. Colocar um widget não o ativa: um que esteja desativado
não aparece em lado nenhum, e a disposição indica-o com *Off* ao lado. A
disposição é guardada no servidor e vale para todas as paredes.

As subidas e descidas levam sempre uma seta e um sinal (▲ +1.04 %, ▼ −0.57 %),
e não apenas uma cor. Ao passar o rato por cima, um widget indica a sua fonte;
um cartão da coluna direita diz quando os seus dados estão desatualizados.

| Widget | O que mostra | Fonte | Chave | O servidor consulta |
|---|---|---|---|---|
| Tempo | Um ícone e a temperatura atual, para um local | [Open-Meteo](https://open-meteo.com) (condições atuais e geocodificação) | nenhuma | a cada 15 min |
| Cripto | Preço e variação em 24 horas de até dez moedas, na moeda escolhida | API pública da [CoinGecko](https://www.coingecko.com) | nenhuma (chave Demo gratuita opcional) | a cada minuto, uma chamada |
| Ações | Último preço e variação do dia de até dez símbolos | [Finnhub](https://finnhub.io) | **chave gratuita obrigatória** | a cada 5 min, uma chamada por símbolo |
| Taxas de câmbio | Quanto custa uma unidade (ou 100 de uma moeda pequena) na moeda base, e a sua variação desde o dia anterior | [Frankfurter](https://frankfurter.dev) (taxas de referência dos bancos centrais) | nenhuma | a cada hora |
| Vulnerabilidades críticas | Os últimos CVE classificados como críticos (CVSS 9 ou mais) esta semana e/ou as últimas entradas do catálogo da CISA de vulnerabilidades ativamente exploradas, com pontuação e data | API 2.0 do [NVD](https://nvd.nist.gov), [CISA KEV](https://www.cisa.gov/known-exploited-vulnerabilities-catalog) | nenhuma | a cada hora |

**As fontes e os seus termos** (verificados em outubro de 2026):

- A **Open-Meteo** é gratuita sem chave para uso não comercial (até 10 000
  chamadas por dia) e os seus dados estão sob CC BY 4.0.
- A API sem chave da **CoinGecko** partilha um conjunto de cerca de 10 a 30
  chamadas por minuto por endereço e não se destina a consultas intensivas; o
  DumbMonit consulta uma vez por minuto para todas as paredes. Uma **chave
  Demo** gratuita (coingecko.com) dá à instância um limite de taxa próprio.
- As **ações** precisam de uma chave gratuita da **Finnhub**: nenhuma fonte de
  cotações sem chave é simultaneamente fiável e permitida (a Stooq passou a
  pedir uma chave, a Yahoo proíbe a sua API não documentada). O plano gratuito
  cobre ações cotadas nos EUA e permite 60 chamadas por minuto; as cotações
  podem ter atraso.
- A **Frankfurter** não tem chave nem quota; as taxas mudam uma vez por dia
  útil.
- O **NVD** permite 5 pedidos por 30 segundos sem chave; o DumbMonit faz um por
  hora. *This product uses the NVD API but is not endorsed or certified by the
  NVD.* O catálogo **CISA KEV** é um ficheiro JSON público.

**Tudo passa pelo servidor.** Nenhum ecrã fala com estes serviços: nem o seu
endereço nem os limites dos serviços dependem do número de ecrãs. Cada fonte
tem a sua própria cache. Quando uma fonte falha, o cartão mantém os últimos
valores até seis horas, marcados *Out of date*, e o servidor espera um minuto
antes de voltar a perguntar; sem nada para mostrar, o cartão diz
*Unavailable*. Uma parede nunca se estraga por causa de um widget.

**As chaves de API nunca voltam.** Ficam guardadas cifradas com o segredo da
instância; as Definições só mostram se há uma guardada. Escrever uma nova
substitui-a, **Remove the key** apaga-a.

### Escolher o que mostrar {#choosing-what-to-show}

- **Tempo:** escreva uma cidade e prima **Search** (geocodificação da
  Open-Meteo, no seu idioma), depois escolha o local (escolher outro
  substitui-o). Unidades: °C ou °F.
- **Cripto:** marque entre as moedas principais (BTC, ETH, SOL, XRP, BNB, ADA,
  DOGE, TRX, TON, AVAX, DOT, LINK, LTC, XLM, XMR, USDT, USDC) ou acrescente
  qualquer moeda pelo seu id da CoinGecko (a última parte do seu endereço em
  coingecko.com, como `avalanche-2`). Moeda de apresentação: USD, EUR, GBP,
  CHF, JPY e outras.
- **Ações:** escreva os símbolos, separados por vírgulas (`AAPL, MSFT, NVDA`),
  e cole a sua chave da Finnhub.
- **Taxas de câmbio:** escolha a moeda base e até dez outras entre as
  principais (USD, EUR, GBP, JPY, CHF, CNY, CAD, AUD…).
- **Vulnerabilidades críticas:** *Both* (críticas e exploradas; um CVE
  explorado que já esteja na lista do NVD leva a etiqueta *Exploited*),
  *Critical* (NVD) ou *Actively exploited* (CISA KEV), e quantas mostrar (1 a
  10).

#### Como obter uma chave de administrador {#ai-admin-keys}

Ambos os fornecedores pedem uma **chave de administrador**, diferente da chave de API habitual. Serve apenas para ler o consumo e os custos da organização. Mantenha-a secreta: cole-a em **Settings → Wall display → Wall widgets** e o DumbMonit guarda-a cifrada.

##### Claude (Anthropic) {#ai-admin-key-claude}

1. Inicie sessão na [Claude Console](https://platform.claude.com/settings/admin-keys) como membro com a função **admin** de uma **organização** (uma conta pessoal não a pode criar; crie primeiro uma organização em Settings → Organization).
2. Abra a [página Admin keys](https://platform.claude.com/settings/admin-keys), clique em **Create admin key**, dê-lhe um nome e escolha uma expiração.
3. Copie logo o segredo (`sk-ant-admin01-…`): só é mostrado uma vez.

##### OpenAI {#ai-admin-key-openai}

1. Inicie sessão na [plataforma OpenAI](https://platform.openai.com/settings/organization/admin-keys) como **proprietário** da organização.
2. Abra a [página Admin keys](https://platform.openai.com/settings/organization/admin-keys) (Organization settings → Admin keys) e clique em **Create new admin key**; a permissão *Read only* chega.
3. Copie logo a chave: só é mostrada uma vez.

As subscrições individuais (Claude Pro e Max, ChatGPT Plus) **não têm API de utilização**: não podem aparecer na parede.

### API {#widgets-api}

`GET /api/wall/widgets` (o que as paredes consultam a cada minuto: a disposição
`layout` e uma secção por widget ativado, com `data`, `updated`, `error`,
`needs_key`, `source`),
`GET|PUT /api/wall/widgets/settings` e `GET /api/wall/widgets/places?q=`
(admin), `GET /api/wall/widgets/catalog`. No `PUT`, `secrets.finnhub` e
`secrets.coingecko` definem uma chave, uma cadeia vazia apaga-a, e omiti-las
mantém-nas; `layout` aceita as listas `top`, `bottom`, `right` e `hidden` (um
identificador desconhecido é descartado, um widget em falta volta à sua zona
predefinida).

## Música na parede {#music-on-the-wall}

A parede também pode reproduzir música e mostrar o que está a tocar,
para que possa iniciar uma lista de reprodução do seu telefone,
ouvi-la pela TV, e vê-la do outro lado da sala. A música fica sempre
no canto inferior da janela meteorológica, sobre o céu: nunca tira
espaço ao boletim ou à lista *Needs you*, e desaparece quando nada
toca. Cada cartão colapsa numa pequena pílula (o ecrã lembra-se dela).

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

### API {#api_1}

| Rota | Quem | O quê |
|---|---|---|
| `GET /api/music/now` | qualquer sessão ou token | O que toca na conta Spotify (em cache alguns segundos) e a ligação que as paredes reproduzem |
| `PUT /api/music/link` `{"link": "…"}` | admin | Toda parede reproduz esta ligação |
| `DELETE /api/music/link` | admin | As paredes param o leitor incorporado |
| `GET /api/music/spotify` | qualquer sessão ou token | A conta ligada: estado, Client ID, data de reconexão — nunca um token |
| `POST /api/music/spotify/authorize` | sessão de admin | Inicia uma ligação: `{"client_id", "redirect_uri"}` → URL de aprovação do Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | sessão de admin | Termina uma ligação loopback com o endereço em que o navegador aterrou |
| `GET /api/music/spotify/callback` | o navegador do admin | O redirecionamento direto do Spotify (instalações HTTPS) |
| `DELETE /api/music/spotify` | admin | Desliga e esquece os tokens |
| `GET /api/music/spotify/token` | só sessão de navegador | Um token de acesso de curta duração para o leitor da parede |
| `GET /api/music/speaker` | qualquer sessão ou token | O altifalante: nome, Premium, os dispositivos Spotify Connect da conta, se o altifalante está listado, o que cada parede reportou |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Renomeia o altifalante (`null` volta a «DumbMonit Wall») |
| `POST /api/music/speaker/report` | só sessão de navegador | Uma parede reporta o seu altifalante (enviado a cada minuto); a resposta diz se o Spotify o lista |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | só sessão de navegador | Reproduz a conta nesse dispositivo, ou no dispositivo nomeado como o altifalante («Test sound», «Play here») |
