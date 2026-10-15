# Notificações push (PWA) {#push-notifications-pwa}

O DumbMonit pode tocar no seu telemóvel ou computador por si próprio, como uma
app nativa, sem instalar uma app nem criar uma conta em lado nenhum: o próprio
serviço push do navegador (Google, Mozilla, Apple, Microsoft) transporta uma
mensagem cifrada ponta a ponta do seu servidor para o dispositivo, e a
notificação aparece mesmo com todos os separadores do DumbMonit fechados. Este é
o **Web Push** padrão; o site pode também ser instalado como app (uma
*progressive web app*, PWA) para abrir a partir de um ícone como qualquer outra.

Dois passos: ativar o push em cada dispositivo (esta página), depois decidir
que alertas vão para lá com um canal **Web Push**
([Canais de notificação](../notifications.md#web-push)).

## Antes de começar {#before-you-start}

- **HTTPS é obrigatório.** Os navegadores só oferecem push a uma origem segura.
  Ponha o DumbMonit atrás de um proxy inverso com certificado (Caddy, Traefik,
  Nginx Proxy Manager…) e abra-o por esse endereço. `http://localhost` também é
  aceite, o que só serve para testar no próprio servidor. Em `http://192.168.x.x`
  simples, a secção diz **Needs HTTPS**.
- **O servidor precisa de HTTPS de saída** para os serviços push:
  `fcm.googleapis.com` (Chrome, e Edge no Android), `*.notify.windows.com` (Edge
  no Windows), `updates.push.services.mozilla.com` (Firefox) e
  `*.push.apple.com` (Safari, iPhone, iPad). Nada entra: o seu DumbMonit não
  precisa de ser alcançável pela internet, o telemóvel só tem de o alcançar
  quando toca numa notificação.
- **Defina `DUMBMONIT_PUBLIC_URL`** para o endereço HTTPS que usa: é dado aos
  serviços push como contacto do remetente, e é o endereço para o qual apontam
  as ligações noutros canais.

## Ativar num telemóvel ou computador {#enable-on-a-phone-or-computer}

### Android, Windows, macOS, Linux {#android-windows-macos-linux}

1. Abra o DumbMonit no Chrome, Edge, Firefox ou Safari e inicie sessão.
2. Opcional: instale-o como app (barra de endereço → **Instalar**, ou o menu do
   navegador → **Adicionar ao ecrã principal**). As notificações funcionam de
   qualquer forma.
3. **Definições → Notificações push → Ativar neste dispositivo**, e permita
   notificações quando o navegador perguntar.
4. **Envie um teste.** Uma *DumbMonit — notificação de teste* deve aparecer em
   poucos segundos.

### iPhone e iPad {#iphone-and-ipad}

A Apple só dá push a sites adicionados ao Ecrã Principal (iOS e iPadOS 16.4 ou
posterior):

1. Abra o DumbMonit no **Safari**.
2. **Partilhar → Adicionar ao Ecrã Principal**, depois abra o DumbMonit **a
   partir do novo ícone**.
3. Inicie sessão, depois **Definições → Notificações push → Ativar neste
   dispositivo**, e permita notificações.
4. **Envie um teste.**

No Safari sem instalar, a secção diz **Install first**.

## Escolher que alertas chegam {#choose-which-alerts-arrive}

Em **Settings → Notifications & reports**, adicione um canal do tipo **Web Push**. Sem
definições, todos os dispositivos subscritos de todas as contas o recebem; liste
nomes de conta em **Contas** para visar apenas algumas pessoas. Aplica-se a
política habitual dos canais (gravidade mínima, horas de silêncio, filtro de
encaminhamento…), pelo que pode, por exemplo, reservar um telemóvel só para
alertas críticos.

Uma notificação lê-se `Critical · nas01` com um breve resumo; tocar nela abre a
página de **Alertas**. A resolução de um alerta substitui a sua notificação em
vez de acrescentar uma segunda.

## Dispositivos e contas {#devices-and-accounts}

Cada conta vê e gere os seus próprios dispositivos: os visualizadores também
podem ativar o push no seu telemóvel, o que não altera mais nada. Uma conta tem
no máximo 10 dispositivos (remova um antigo para adicionar outro), e um teste
pode ser enviado uma vez a cada 30 segundos. A lista mostra o navegador e o
sistema, o serviço push, quando o dispositivo foi adicionado e quando recebeu
uma mensagem pela última vez; **Remover** esquece-o (e, no dispositivo que está
a usar, cancela também a subscrição do navegador). Um dispositivo que o serviço
push reporta como desaparecido (a app foi removida, os dados do navegador
limpos) é apagado automaticamente no envio seguinte.

## Resolução de problemas {#troubleshooting}

| O que vê | O que fazer |
|---|---|
| **Needs HTTPS** | Abra o DumbMonit pelo seu endereço HTTPS. |
| **Install first** (iPhone, iPad) | Partilhar → Adicionar ao Ecrã Principal, abra a partir do ícone. |
| **Blocked** | As notificações foram recusadas para este site: permita-as nas definições do site do navegador (cadeado na barra de endereço, ou Definições do iOS → Notificações → DumbMonit), depois recarregue. |
| **Not supported** | O navegador não tem Web Push (versão antiga, alguns navegadores de privacidade): use um Chrome, Edge, Firefox ou Safari recente. |
| O teste diz *push service unreachable* | O servidor não consegue alcançar o serviço push: verifique o seu HTTPS de saída e o DNS. |
| O teste diz *refused the VAPID signature* | Geralmente um relógio do servidor errado; verifique o NTP no anfitrião. |
| Nada chega no Android quando o telemóvel dorme | A otimização da bateria pode atrasar o navegador: isente-o, ou defina a **Urgência** do canal para `high`. |

## Privacidade e segurança {#privacy-and-security}

- As mensagens são cifradas para cada dispositivo (RFC 8291) antes de saírem do
  servidor: o serviço push só vê um blob opaco, o dispositivo decifra-o.
- O servidor assina cada mensagem com o seu próprio par de chaves VAPID (RFC
  8292), gerado no primeiro arranque e guardado cifrado com o segredo da
  instância; a chave privada nunca é devolvida pela API. Uma
  [restauração](../install/backup.md) numa máquina nova gera um novo par de
  chaves: ative o push de novo em cada dispositivo.
- O endereço da subscrição vem do navegador, pelo que o servidor só aceita
  endereços HTTPS em IPs públicos: os endereços privados, loopback e link-local
  são recusados, no momento da subscrição e de novo em cada ligação.
- O service worker (`/sw.js`) só mostra notificações. Não guarda nada em cache e
  não intercepta nenhum pedido: a interface e a API vêm sempre do servidor,
  nunca de uma cópia desatualizada.
