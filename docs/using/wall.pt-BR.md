# Modo parede e música

`/wall` mostra apenas o boletim, em tela cheia, para uma tela que fica
ligada em uma sala: a frase, a janela do tempo, três leituras grandes e a
lista *Precisa de você*, atualizadas a cada 20 segundos, com a tela
mantida acordada. ++esc++ ou **Sair** volta à visão geral. Abra-o a partir
da paleta de comandos ("Modo parede") ou digitando a URL. `?theme=dark` ou
`?theme=light` força um tema para essa tela sem alterar sua preferência
salva.

![Modo parede](../assets/screenshots/wall-light.png){ loading=lazy }

## Música na parede

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

### Spotify Connect: a parede se torna um alto-falante

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

#### Configure

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

#### Quando o navegador da TV não consegue tocar o Spotify

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

### Um link enviado para toda parede

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

### Privacidade

Nada de terceiros carrega na parede até o Spotify ser conectado ou um link
ser definido. A política de segurança de conteúdo da página permite
exatamente os três players embutidos, o script do player do Spotify
(`sdk.scdn.co/spotify-player.js`) e seu frame, e cobre a partir de
`i.scdn.co` — nada mais, e nada fora da interface web.

### API

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
