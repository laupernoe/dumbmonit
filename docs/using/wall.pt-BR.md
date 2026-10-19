# Modo parede e música {#wall-mode-and-music}

`/wall` mostra apenas o boletim, em tela cheia, para uma tela que fica
ligada em uma sala: a frase, a janela do tempo, três leituras grandes e a
lista *Precisa de você*, atualizadas a cada 20 segundos, com a tela
mantida acordada. ++esc++ ou **Sair** volta à visão geral. Abra-o a partir
da paleta de comandos ("Modo parede") ou digitando a URL. `?theme=dark` ou
`?theme=light` força um tema para essa tela sem alterar sua preferência
salva.

![Modo parede](../assets/screenshots/wall-light.png){ loading=lazy }

Cada problema à esquerda traz o ícone de produto do seu monitor (as
mesmas marcas das listas de monitores). A lista nunca ultrapassa seu
espaço: em uma tela pequena, ou com muitos problemas, os cartões grandes
encolhem para uma linha cada e depois se dobram em *E mais N*, em vez de
se sobrepor aos monitores abaixo.

## Notícias na parede {#news-on-the-wall}

Um administrador pode adicionar **manchetes** e um **canal ao vivo** à
parede em **Configurações → Notícias do mural**. Ambos vêm desativados.

- **Faixa de manchetes.** Uma faixa na parte inferior da parede rola as
  manchetes mais recentes, cada uma com sua fonte e idade, e é atualizada a
  cada cinco minutos. Com *movimento reduzido* ativado no sistema da tela,
  nada desliza: uma manchete por vez aparece e some suavemente.
- **Vídeo ao vivo.** Um canal ao vivo sem som em um cartão, na coluna da
  direita sob o relógio (ou no meio da cena), pequeno, médio ou grande. O
  cartão encolhe para o espaço que sobra; nunca cobre o status.
- **Carrossel de imagens.** Os artigos mais recentes dos mesmos feeds que
  trazem uma imagem, um de cada vez na coluna da direita, sob o relógio, cada um por oito
  segundos, com transição suave, e com a fonte, a idade e a manchete sobre a
  imagem. Com *movimento reduzido*, as imagens trocam sem transição.

Quem lê os feeds é o servidor, não o navegador (navegadores não podem ler
feeds de outros sites). Ele segue no máximo três redirecionamentos, recusa
endereços internos, desiste após oito segundos ou 2 MB por feed e guarda a
resposta em cache por cinco minutos para todas as paredes. Lê RSS 2.0,
RSS 1.0 (RDF) e Atom.

A imagem de um artigo vem do feed: a maior imagem `media:content`, um
`media:thumbnail`, um `enclosure` de imagem (BFM TV), um `link rel="enclosure"`
do Atom ou, na falta, o primeiro `<img>` do conteúdo do artigo (Korben).
Quando o feed não tem nenhuma e o carrossel está ativado, o servidor lê uma
vez o início da página do artigo (512 KB no máximo, com a mesma proteção de
endereços) e pega seu `og:image`; a resposta é guardada, então uma página é
lida uma única vez. A parede nunca carrega uma imagem do site de notícias: o
servidor a retransmite em `/api/wall/news/image/<id>`, só para artigos que ele
publicou, com no máximo 6 MB, e somente quando o arquivo é realmente uma
imagem PNG, JPEG, GIF, WebP ou AVIF (nunca SVG). O endereço e os cookies da
tela ficam em casa, e a política de segurança de conteúdo da parede continua
permitindo imagens apenas do DumbMonit.

### Escolher as fontes {#choosing-sources}

Escolha um **país** (pré-selecionado a partir da região e do idioma do seu
navegador) e marque suas fontes: até oito feeds no total. Para usar
qualquer outro feed, cole o endereço RSS ou Atom dele em **Adicionar um feed
personalizado** e pressione **Testar**: o DumbMonit o lê de verdade e mostra a
manchete mais recente antes de você adicioná-lo.

Para o vídeo, escolha um dos canais do país ou **Outro link…** e cole um
endereço normal do YouTube, Dailymotion ou Twitch (um vídeo, uma live ou a
página `/live` de um canal do YouTube); o DumbMonit o converte no player
incorporável. Só esses três players podem ser exibidos. Uma live é oferecida
como predefinição apenas quando o canal tem uma oficial e estável.

| País | Manchetes | Vídeo ao vivo |
|---|---|---|
| França | BFM TV, franceinfo, Le Monde, France 24 | BFM TV, franceinfo, France 24, Euronews |
| Suíça | RTS Info, SRF News, Le Temps, NZZ | nenhum |
| Bélgica | RTBF, VRT NWS, La DH | nenhum |
| Alemanha | Tagesschau, ZDFheute, Spiegel, Deutsche Welle | DW News |
| Reino Unido | BBC News, The Guardian | Sky News |
| Estados Unidos | NPR, PBS NewsHour, CBS News, The New York Times | ABC News Live |
| Canadá | Radio-Canada, CBC News | CBC News |
| Espanha | RTVE, 20minutos, El País, El Mundo | nenhum |
| Itália | ANSA, RaiNews | nenhum |
| Portugal | RTP Notícias, Público | nenhum |
| Brasil | G1, Folha de S.Paulo | CNN Brasil |
| Rússia | Interfax, Meduza, Kommersant | nenhum |
| China | CGTN, South China Morning Post, Xinhua | CGTN |
| Internacional | Deutsche Welle, France 24, BBC News (world) | DW News, France 24 English, Euronews English |

O feed da BFM TV é `https://www.bfmtv.com/rss/news-24-7/`. A lista foi
verificada quando foi escrita; um site pode mudar seu feed a qualquer
momento, e então a faixa simplesmente o ignora. Use **Testar** no substituto
e adicione-o como feed personalizado.

A política de segurança de conteúdo da parede só deixa a página incorporar
players do YouTube (`youtube-nocookie.com`), Dailymotion, Twitch, Spotify e
Deezer.

### API {#api}

`GET /api/wall/news` (o que as paredes consultam), `GET|PUT
/api/wall/news/settings` e `POST /api/wall/news/test` (admin),
`GET /api/wall/news/presets`,
`GET /api/wall/news/image/{id}` (a imagem de um artigo, retransmitida).

## Widgets na parede {#widgets-on-the-wall}

Um administrador pode adicionar **widgets** à parede em **Configurações →
Exibição do mural → Widgets do mural**: o tempo, preços de criptomoedas,
cotações de ações, taxas de câmbio e as últimas vulnerabilidades críticas.
Cada um vem desativado e é ativado separadamente.

O **tempo** é simples: um ícone e a temperatura atual, para um lugar,
logo abaixo do relógio. Sem previsão, sem detalhes. Um leitor de tela lê o
lugar, o céu e a temperatura; passar o mouse mostra as mesmas palavras e a
fonte (Open-Meteo).

Os outros widgets dividem um cartão na coluna da direita, sob o relógio: um
widget por vez, cada um mantido pelo número de segundos escolhido (20 por
padrão), e então o próximo aparece suavemente. Listas longas são divididas em
páginas de cinco. Pontos sob o cartão mostram onde a rodada está. Com
*movimento reduzido*, os cartões trocam sem transição.

Altas e quedas sempre trazem uma seta e um sinal (▲ +1.04 %, ▼ −0.57 %), não
apenas uma cor. Cada cartão cita sua fonte, discretamente, e quão recente ele é.

| Widget | O que mostra | Fonte | Chave | O servidor consulta |
|---|---|---|---|---|
| Tempo | Um ícone e a temperatura atual, para um lugar, sob o relógio | [Open-Meteo](https://open-meteo.com) (condições atuais e geocodificação) | nenhuma | a cada 15 min |
| Cripto | Preço e variação em 24 horas de até dez moedas, na moeda escolhida | API pública do [CoinGecko](https://www.coingecko.com) | nenhuma (chave Demo gratuita opcional) | a cada minuto, uma chamada |
| Ações | Último preço e variação do dia de até dez símbolos | [Finnhub](https://finnhub.io) | **chave gratuita obrigatória** | a cada 5 min, uma chamada por símbolo |
| Taxas de câmbio | Quanto custa uma unidade (ou 100 de uma moeda pequena) na moeda base, e sua variação desde o dia anterior | [Frankfurter](https://frankfurter.dev) (taxas de referência dos bancos centrais) | nenhuma | a cada hora |
| Vulnerabilidades críticas | As últimas CVEs classificadas como críticas (CVSS 9 ou mais) nesta semana e/ou as últimas entradas do catálogo da CISA de vulnerabilidades ativamente exploradas, com pontuação e data | API 2.0 do [NVD](https://nvd.nist.gov), [CISA KEV](https://www.cisa.gov/known-exploited-vulnerabilities-catalog) | nenhuma | a cada hora |

**As fontes e seus termos** (verificados em outubro de 2026):

- **Open-Meteo** é gratuito sem chave para uso não comercial (até 10.000
  chamadas por dia) e seus dados estão sob CC BY 4.0.
- A API do **CoinGecko** sem chave compartilha um conjunto de cerca de 10 a 30
  chamadas por minuto por endereço e não é feita para consultas intensas; o
  DumbMonit consulta uma vez por minuto para todas as paredes. Uma **chave
  Demo** gratuita (coingecko.com) dá à instância um limite de requisições
  próprio.
- **Ações** precisam de uma chave gratuita do **Finnhub**: nenhuma fonte de
  cotações sem chave é ao mesmo tempo confiável e permitida (o Stooq agora
  pede uma chave, o Yahoo proíbe sua API não documentada). O plano gratuito
  cobre ações listadas nos EUA e permite 60 chamadas por minuto; as cotações
  podem ter atraso.
- **Frankfurter** não tem chave nem cota; as taxas mudam uma vez por dia útil.
- **NVD** permite 5 requisições por 30 segundos sem chave; o DumbMonit faz uma
  por hora. *Este produto usa a API do NVD, mas não é endossado nem certificado
  pelo NVD.* O catálogo **CISA KEV** é um arquivo JSON público.

**Tudo passa pelo servidor.** Nenhuma tela fala com esses serviços: nem o
endereço dela nem os limites dos serviços dependem do número de telas. Cada
fonte tem seu próprio cache. Quando uma fonte falha, o cartão mantém seus
últimos números por até seis horas, marcados como *Desatualizado*, e o
servidor espera um minuto antes de tentar de novo; sem nada para mostrar, o
cartão diz *Indisponível*. Uma parede nunca quebra por causa de um widget.

**As chaves de API nunca voltam.** Elas são guardadas criptografadas com o
segredo da instância; as Configurações só mostram se há uma salva. Digitar uma
nova a substitui, **Remover a chave** a apaga.

### Escolher o que mostrar {#choosing-what-to-show}

- **Tempo:** digite uma cidade e pressione **Buscar** (geocodificação do
  Open-Meteo, no seu idioma), e então escolha o lugar (escolher outro
  substitui o anterior). Unidades: °C ou °F.
- **Cripto:** marque entre as principais moedas (BTC, ETH, SOL, XRP, BNB, ADA,
  DOGE, TRX, TON, AVAX, DOT, LINK, LTC, XLM, XMR, USDT, USDC) ou adicione
  qualquer moeda pelo id do CoinGecko (a última parte do endereço dela em
  coingecko.com, como `avalanche-2`). Moeda de exibição: USD, EUR, GBP, CHF,
  JPY e outras.
- **Ações:** digite os símbolos, separados por vírgulas (`AAPL, MSFT, NVDA`),
  e cole sua chave do Finnhub.
- **Taxas de câmbio:** escolha a moeda base e até dez outras entre as
  principais (USD, EUR, GBP, JPY, CHF, CNY, CAD, AUD…).
- **Vulnerabilidades críticas:** *Ambas* (críticas e exploradas; uma CVE
  explorada que já está na lista do NVD recebe a marca *Explorada*),
  *Críticas* (NVD) ou *Ativamente exploradas* (CISA KEV), e quantas mostrar
  (de 1 a 10).

#### Como obter uma chave de administrador {#ai-admin-keys}

Os dois provedores pedem uma **chave de administrador**, diferente da chave de API habitual. Ela serve apenas para ler o consumo e os custos da organização. Mantenha-a em segredo: cole-a em **Settings → Wall display → Wall widgets** e o DumbMonit a guarda criptografada.

##### Claude (Anthropic) {#ai-admin-key-claude}

1. Entre no [Claude Console](https://platform.claude.com/settings/admin-keys) como membro com a função **admin** de uma **organização** (uma conta pessoal não pode criar uma; crie primeiro uma organização em Settings → Organization).
2. Abra a [página Admin keys](https://platform.claude.com/settings/admin-keys), clique em **Create admin key**, dê um nome e escolha uma expiração.
3. Copie imediatamente o segredo (`sk-ant-admin01-…`): ele só é mostrado uma vez.

##### OpenAI {#ai-admin-key-openai}

1. Entre na [plataforma OpenAI](https://platform.openai.com/settings/organization/admin-keys) como **proprietário** (owner) da organização.
2. Abra a [página Admin keys](https://platform.openai.com/settings/organization/admin-keys) (Organization settings → Admin keys) e clique em **Create new admin key**; a permissão *Read only* basta.
3. Copie imediatamente a chave: ela só é mostrada uma vez.

As assinaturas individuais (Claude Pro e Max, ChatGPT Plus) **não têm API de uso**: não podem aparecer na parede.

### API {#widgets-api}

`GET /api/wall/widgets` (o que as paredes consultam a cada minuto: uma seção
por widget ativado, com `data`, `updated`, `error`, `needs_key`, `source`),
`GET|PUT /api/wall/widgets/settings` e `GET /api/wall/widgets/places?q=`
(admin), `GET /api/wall/widgets/catalog`. No `PUT`, `secrets.finnhub` e
`secrets.coingecko` definem uma chave, uma string vazia a apaga, e omiti-los
as mantém.

## Música na parede {#music-on-the-wall}

A parede também pode tocar música e mostrar o que está tocando, para que
você possa iniciar uma playlist do seu celular, ouvi-la pela TV, e vê-la
do outro lado da sala. A música sempre fica no canto inferior da janela do
tempo, sobre o céu: ela nunca toma espaço do boletim ou da lista *Precisa
de você*, e desaparece quando nada toca. Cada cartão se recolhe em uma
pequena pílula (a tela se lembra disso).

Existem duas formas, e você pode usar ambas.

| | Spotify Connect | Um link enviado para as paredes |
|---|---|---|
| Inicie a partir de | o app Spotify no seu celular (**Dispositivos → DumbMonit Wall**) | qualquer tela DumbMonit, seu celular incluído (**Tocar na parede**) |
| Serviços | Spotify | Spotify, Deezer, YouTube e YouTube Music |
| Precisa de | Spotify Premium, um app Spotify Developer próprio, o celular na mesma conta Spotify, a parede servida via HTTPS (ou localhost) em um navegador com DRM | nada; sem estar conectado na parede, Spotify e Deezer tocam prévias de 30 segundos |
| Mostra o que toca | sim, onde quer que toque: a parede, o app Spotify de uma TV, um alto-falante cast | o próprio player do serviço |

Conectar o Spotify e enviar links são ações de administrador; uma conta de
visualizador na parede toca e mostra tudo.

### Spotify Connect: a parede se torna um alto-falante {#spotify-connect-the-wall-becomes-a-speaker}

O objetivo: no seu celular, abra o Spotify, toque no ícone de
dispositivos, escolha a TV — a música toca na TV enquanto a parede mostra
o que está tocando. A parede faz isso com o **Web Playback SDK** do
Spotify: o navegador da parede se registra como um dispositivo Spotify
Connect. Isso só funciona quando **todos** estes se cumprem, e a parede
diz em sua própria tela qual está faltando:

| Precisa de | Por quê | Quando está faltando |
|---|---|---|
| **Spotify Premium** na conta conectada | o Spotify só deixa contas Premium usarem alto-falantes de navegador | a parede mostra *Spotify Premium é necessário*; Configurações mostra *Premium: Não* |
| **O celular conectado à mesma conta Spotify** | um alto-falante de navegador só é listado para a conta cujo token ele mantém — ele não é descoberto na rede como um alto-falante inteligente, e outros membros de um plano Family não o veem | o celular simplesmente não lista a parede |
| **HTTPS** (ou `http://localhost` na própria tela) | navegadores só permitem mídia protegida (DRM) em uma página segura | a parede mostra *aberto via HTTP simples* |
| **Um navegador cujo DRM funciona** (Widevine, PlayReady ou FairPlay) | o Spotify transmite áudio criptografado | a parede mostra *Este navegador não pode tocar o Spotify* |
| **Um toque ou pressionar de tecla na parede por carregamento de página** | navegadores mantêm uma página silenciosa até alguém interagir com ela | a parede mostra *Toque para ativar o som* |

Quais navegadores funcionam: Chrome, Edge e Firefox em um computador
(Firefox: permita *Reproduzir conteúdo protegido por DRM*), Safari em um
Mac, Chromium com Widevine em um Raspberry Pi (`libwidevinecdm0`). A
maioria dos **navegadores de smart TV** (Samsung Tizen, LG webOS),
**navegadores kiosk** e muitos **builds Linux do Chromium** vêm sem um
Widevine usável: eles não podem ser um alto-falante Spotify, o que quer
que o DumbMonit faça. Para tal TV, veja
[Quando o navegador da TV não consegue tocar o Spotify](#when-the-tvs-browser-cannot-play-spotify).

#### Configure {#set-it-up}

1. **Crie um app Spotify.** Abra o
   [Spotify Developer Dashboard](https://developer.spotify.com/dashboard),
   crie um app (qualquer nome), marque **Web API** e **Web Playback SDK**,
   e adicione a URI de redirecionamento que **Configurações → Música da
   parede** mostra:
    - DumbMonit servido via HTTPS:
      `https://<seu endereço>/api/music/spotify/callback`. O Spotify volta
      direto ao DumbMonit depois que você aprova.
    - DumbMonit servido via HTTP simples: `http://127.0.0.1:8888/callback`.
      O Spotify só aceita `http://` para um endereço loopback (e não
      `localhost`), desde 2025. Nada responde nesse endereço: depois que
      você aprova, seu navegador mostra uma página de erro cujo endereço
      carrega o código — copie-o e cole-o de volta no DumbMonit.
2. **Conecte.** Em **Configurações → Música da parede**, cole o **Client
   ID** do app (não há segredo: o DumbMonit usa Authorization Code com
   PKCE) e escolha **Conectar Spotify**, depois aprove no Spotify **com a
   conta que seu celular usa**.
3. **Na TV**, abra o `/wall` do DumbMonit via HTTPS, conectado (uma conta
   de visualizador é suficiente), e toque a tela ou pressione **OK** no
   controle remoto uma vez: qualquer gesto destrava o som. Um navegador
   iniciado com `--autoplay-policy=no-user-gesture-required` (um kiosk)
   não precisa de toque.
4. **No seu celular**, no app Spotify, toque no ícone de dispositivos e
   escolha **DumbMonit Wall**. O som sai da TV, e a parede mostra a capa,
   título e progresso.

**Configurações → Música da parede → Alto-falante** mostra se tudo se
cumpre, sem andar até a TV: a conta, Premium, se o Spotify lista o
alto-falante entre os dispositivos da conta agora (e quais dispositivos
lista), e toda parede que reportou nos últimos quinze minutos — seu
navegador ("Chrome 130 no Linux"), se está pronta, e por que não. **Testar
som** faz a conta tocar no alto-falante (ela retoma o que tocou por
último); cada parede pronta também tem seu próprio **Testar som**. Na
própria parede, o painel **Música** tem **Tocar aqui**, que move o que
quer que toque na conta para esta tela.

**O nome.** Renomeie o alto-falante em **Configurações → Música da parede
→ Alto-falante** ("TV da sala"); toda parede pega o novo nome em
segundos. Para nomear apenas uma parede, adicione `?speaker=Cozinha` ao
seu endereço — a tela se lembra disso (`?speaker=` o esquece).

**Uma parede fica ligada por dias.** A parede pede ao DumbMonit um token
novo quando o do Spotify expira (a cada hora); quando o dispositivo cai
(Wi-Fi, uma placa de rede dormindo) e não volta dentro de trinta segundos,
a parede o registra de novo; uma falha passageira é tentada de novo por
conta própria (15 s, 30 s, … depois a cada cinco minutos); e se o Spotify
parar de listar a parede enquanto ela pensa estar pronta, ela se registra
de novo. Só o que esperar não pode corrigir — nenhum DRM, nenhum Premium
— para de vez, até **Tentar novamente**.

O DumbMonit mantém o token de atualização do Spotify criptografado com o
segredo da instância e nunca o envia para um navegador. A parede só
recebe um token de acesso de curta duração para seu player, e apenas
através de uma sessão conectada (um token de API não pode obter um).
**Desconectar** esquece a conta; para também remover o DumbMonit de sua
conta Spotify, use
[spotify.com/account/apps](https://www.spotify.com/account/apps/).

Também bom saber:

- Um app Spotify pessoal (*modo de desenvolvimento*) pertence a uma conta
  Premium e admite até cinco contas: adicione outras em **User
  Management** no painel.
- **Reconecte a cada seis meses.** O Spotify termina uma conexão seis
  meses depois de ser aprovada; Configurações mostra a data, e a parede
  avisa isso quando acontece.

**Tocando agora funciona sem nada disso.** O servidor do DumbMonit pergunta
ao Spotify o que está tocando na conta a cada poucos segundos (uma
requisição para todas as paredes) e a parede mostra a capa, título,
artista, progresso e o dispositivo em que toca — a própria parede, o app
Spotify de uma smart TV, um Chromecast, seu laptop — mesmo via HTTP simples
e em um navegador que não pode tocar o Spotify. Para usar só isso,
desligue **Usar esta tela como um alto-falante Spotify** no painel Música
da parede.

#### Quando o navegador da TV não consegue tocar o Spotify {#when-the-tvs-browser-cannot-play-spotify}

Escolha o que corresponde à sua configuração — em todo caso a parede
continua mostrando o que toca, porque *tocando agora* lê a conta, não o
alto-falante:

- **A TV tem um app Spotify, ou um Chromecast / Google TV / Fire TV
  stick.** Escolha-o na lista de dispositivos do Spotify; a parede mostra
  "Tocando na TV da sala". Esta é a opção mais simples e confiável.
- **Um pequeno computador conduz a TV** (um Raspberry Pi, um mini PC, um
  laptop antigo). Abra a parede no Chrome ou Chromium com Widevine nele:
  ele se torna o alto-falante como descrito acima.
- **Você quer um alto-falante que está sempre lá, mesmo sem navegador.**
  Execute um receptor Spotify Connect baseado em
  [librespot](https://github.com/librespot-org/librespot) (MIT) — por
  exemplo [raspotify](https://github.com/dtcooper/raspotify) ou
  spotifyd — **na máquina cuja saída de áudio alcança a TV ou o
  alto-falante**. Ele aparece na lista de dispositivos do celular para
  todos na rede.

O DumbMonit não entrega tal receptor ele mesmo, de propósito: o contêiner
DumbMonit geralmente roda em um servidor em um armário, não na máquina
conectada à TV, então seu som sairia da caixa errada; um receptor precisa
da placa de som do host e rede do host (multicast DNS) que um contêiner de
monitoramento não deveria ter; e o librespot depende de protocolos Spotify
não oficiais que quebram de tempos em tempos. Um receptor instalado na
máquina certa, ao lado do DumbMonit, faz o trabalho melhor.

### Um link enviado para toda parede {#a-link-sent-to-every-wall}

Em **Configurações → Música da parede** (ou o próprio painel **Música** da
parede), cole um link do Spotify, Deezer ou YouTube para uma faixa, álbum,
playlist, episódio ou vídeo e escolha **Tocar na parede**. Toda parede o
toca no player embutido oficial do serviço; um link enviado enquanto uma
parede está aberta começa por conta própria onde o serviço permite
(YouTube, Deezer) uma vez que a parede foi tocada. **Parar** o remove de
toda parede.

Deezer e YouTube não têm equivalente ao Spotify Connect que uma página web
possa usar — o Deezer não aceita mais novos apps de desenvolvedor e sua
API nunca expôs o que está tocando — então um link é a forma para eles. Os
players embutidos tocam prévias de 30 segundos a menos que o navegador da
parede esteja conectado ao serviço.

### Privacidade {#privacy}

Nada de terceiros carrega na parede até o Spotify ser conectado ou um link
ser definido. A política de segurança de conteúdo da página permite
exatamente os três players embutidos, o script do player do Spotify
(`sdk.scdn.co/spotify-player.js`) e seu frame, e cobre a partir de
`i.scdn.co` — nada mais, e nada fora da interface web.

### API {#api}

| Rota | Quem | O que |
|---|---|---|
| `GET /api/music/now` | qualquer sessão ou token | O que toca na conta Spotify (em cache por alguns segundos) e o link que as paredes tocam |
| `PUT /api/music/link` `{"link": "…"}` | admin | Toda parede toca este link |
| `DELETE /api/music/link` | admin | As paredes param o player embutido |
| `GET /api/music/spotify` | qualquer sessão ou token | A conta conectada: status, Client ID, data de reconexão — nunca um token |
| `POST /api/music/spotify/authorize` | sessão admin | Inicia uma conexão: `{"client_id", "redirect_uri"}` → URL de aprovação do Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | sessão admin | Termina uma conexão loopback com o endereço em que o navegador caiu |
| `GET /api/music/spotify/callback` | o navegador do admin | O redirecionamento direto do Spotify (instalações HTTPS) |
| `DELETE /api/music/spotify` | admin | Desconecta e esquece os tokens |
| `GET /api/music/spotify/token` | apenas sessão de navegador | Um token de acesso de curta duração para o player da parede |
| `GET /api/music/speaker` | qualquer sessão ou token | O alto-falante: nome, Premium, os dispositivos Spotify Connect da conta, se o alto-falante está listado, o que cada parede reportou |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Renomeia o alto-falante (`null` volta para "DumbMonit Wall") |
| `POST /api/music/speaker/report` | apenas sessão de navegador | Uma parede reporta seu alto-falante (enviado a cada minuto); a resposta diz se o Spotify o lista |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | apenas sessão de navegador | Toca a conta naquele dispositivo, ou no dispositivo nomeado como o alto-falante ("Testar som", "Tocar aqui") |
