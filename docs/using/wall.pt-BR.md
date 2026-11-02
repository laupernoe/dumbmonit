# Modo parede {#wall-mode-and-music}

`/wall` mostra o quadro próprio do console, em tela cheia, para um monitor que
fica ligado em uma sala. Atualiza-se sozinho e mantém a tela acordada.

De cima para baixo:

- o **veredito** em poucas palavras ("All good", "3 problems") com a data e o
  relógio, e quando foi atualizado pela última vez;
- os **cinco números** da visão geral: monitores ativos, disponibilidade em 30
  dias, tempo de resposta p95, alertas abertos e tempo médio de resolução;
- o **rio de saúde**, a grande imagem do quadro: uma faixa por monitor, os piores
  no topo;
- os **alertas abertos**, os mais graves primeiro;
- **quem está de plantão** agora, com a escala e o fim do turno.

Sem widgets: fundo preto, a tipografia do console, estados escritos junto de uma
forma e nunca só por cor. A [música](#music-on-the-wall), depois de configurada,
fica em um pequeno dock no pé da coluna da direita.

## Abrir e sair

Abra-o com o ícone **Wall mode** no fundo do menu lateral, pela paleta de comandos
("Wall mode") ou digitando a URL. ++esc++ ou **Exit** volta ao console. Uma conta
limitada a algumas equipes ou clientes não tem o modo parede, porque ele mostra
toda a frota.

## Fundo {#backdrop}

A parede compartilha a aparência escura do console. O botão **Fundo** (no alto
à direita, ao lado de Exit) pode desenhar uma das cidades à noite das
[páginas de status](status-pages.md) atrás do veredito e do relógio: Paris,
Nova York, Tóquio ou Dubai, ou *Auto* para a mais próxima do fuso horário do
navegador. Ele vem desligado e é lembrado por navegador; `?city=paris` (ou
`newyork`, `tokyo`, `dubai`, `none`) em um link escolhe só para aquela tela. O
céu acompanha o painel: estrelas quando está tudo bem, nuvens com um aviso,
tempestade com um alerta. No celular, ou com a aba oculta, a cidade fica
parada. Um `zermatt` salvo ou em um link agora cai em Dubai.

## Música na parede {#music-on-the-wall}

A parede também pode tocar música e mostrar o que está tocando: você inicia uma
playlist no celular, ouve na TV e vê do outro lado da sala. A música fica em um
pequeno dock no pé da coluna da direita, sob o relógio e o plantão: nunca tira
espaço do veredito nem dos problemas, e some quando nada toca ou nada está
configurado. Mostra a capa, o título, o artista, o dispositivo que toca e uma
fina barra de progresso; quando a conta conectada permite, um botão de **tocar /
pausar** pausa ou retoma a conta onde quer que ela toque (qualquer sessão
conectada pode, contas de leitura incluídas, como em *Tocar aqui*). O player de
um link compartilhado e as mensagens do alto-falante se recolhem em uma pequena
pílula (a tela se lembra). O botão **Música** da parede, ao lado de Fundo e
Sair, aparece quando a música está configurada.

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

A política de permissões (*Permissions-Policy*) da interface só concede a
reprodução automática e a mídia protegida (DRM) a esses players; as páginas de
status públicas não recebem nenhuma das duas.

### API {#api}

| Rota | Quem | O que |
|---|---|---|
| `GET /api/music/now` | qualquer sessão ou token | O que toca na conta Spotify (em cache por alguns segundos) e o link que as paredes tocam |
| `PUT /api/music/link` `{"link": "…"}` | admin | Toda parede toca este link |
| `DELETE /api/music/link` | admin | As paredes param o player embutido |
| `GET /api/music/spotify` | qualquer sessão ou token | A conta conectada: status, Client ID, data de reconexão — nunca um token |
| `POST /api/music/spotify/authorize` | sessão admin | Inicia uma conexão: `{"client_id", "redirect_uri"}` → URL de aprovação do Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | sessão admin | Termina uma conexão loopback com o endereço em que o navegador caiu |
| `GET /api/music/spotify/callback` | o navegador do admin | O redirecionamento direto do Spotify (instalações HTTPS): entrega o código à página de configurações, que conclui a conexão com a própria sessão |
| `DELETE /api/music/spotify` | admin | Desconecta e esquece os tokens |
| `GET /api/music/spotify/token` | apenas sessão de navegador | Um token de acesso de curta duração para o player da parede |
| `GET /api/music/speaker` | qualquer sessão ou token | O alto-falante: nome, Premium, os dispositivos Spotify Connect da conta, se o alto-falante está listado, o que cada parede reportou |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Renomeia o alto-falante (`null` volta para "DumbMonit Wall") |
| `POST /api/music/speaker/report` | apenas sessão de navegador | Uma parede reporta seu alto-falante (enviado a cada minuto); a resposta diz se o Spotify o lista |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | apenas sessão de navegador | Toca a conta naquele dispositivo, ou no dispositivo nomeado como o alto-falante ("Testar som", "Tocar aqui") |
| `POST /api/music/spotify/playback` `{"playing": false}` | apenas sessão de navegador | Pausa (`false`) ou retoma (`true`) a conta onde quer que ela toque; `can_control` em `GET /api/music/now` diz se é permitido |
