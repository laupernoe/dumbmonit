# Notificações push (PWA) {#push-notifications-pwa}

O DumbMonit pode fazer o seu telefone ou computador tocar diretamente, como
uma app nativa faria, sem instalar nenhuma app nem criar conta nenhuma: o
próprio serviço push do navegador (Google, Mozilla, Apple, Microsoft)
transporta uma mensagem encriptada de ponta a ponta do seu servidor para o
dispositivo, e a notificação aparece mesmo com todos os separadores do
DumbMonit fechados. Isto é o **Web Push** padrão; o site também pode ser
instalado como uma app (uma *progressive web app*, PWA) para que abra a
partir de um ícone como qualquer outra.

Dois passos: ative o push em cada dispositivo (esta página), depois decida
quais alertas vão para lá com um canal **Web Push**
([Canais de notificação](../notifications.md#web-push)).

## Antes de começar {#before-you-start}

- **HTTPS é obrigatório.** Os navegadores só oferecem push a uma origem
  segura. Coloque o DumbMonit atrás de um proxy inverso com um certificado
  (Caddy, Traefik, Nginx Proxy Manager…) e abra-o através desse endereço.
  `http://localhost` também é aceite, o que só é útil para testar no
  próprio servidor. Em `http://192.168.x.x` simples, a secção diz **Needs
  HTTPS**.
- **O servidor precisa de HTTPS de saída** para os serviços push:
  `fcm.googleapis.com` (Chrome, e Edge no Android), `*.notify.windows.com`
  (Edge no Windows), `updates.push.services.mozilla.com` (Firefox) e
  `*.push.apple.com` (Safari, iPhone, iPad). Nada entra: o seu DumbMonit
  não precisa de estar acessível a partir da internet, o telefone só
  precisa de o alcançar quando toca numa notificação.
- **Defina `DUMBMONIT_PUBLIC_URL`** para o endereço HTTPS que usa: é dado aos
  serviços push como o contacto do remetente, e é o endereço para onde
  apontam as ligações noutros canais.

## Ativar num telefone ou computador {#enable-on-a-phone-or-computer}

### Android, Windows, macOS, Linux {#android-windows-macos-linux}

1. Abra o DumbMonit no Chrome, Edge, Firefox ou Safari e inicie sessão.
2. Opcional: instale-o como uma app (barra de endereço → **Install**, ou o menu do navegador → **Add to Home screen**). As notificações funcionam de qualquer forma.
3. **Settings → Push notifications → Enable on this device**, e permita notificações quando o navegador perguntar.
4. **Send a test.** Uma notificação *DumbMonit — test notification* deve aparecer em poucos segundos.

### iPhone e iPad {#iphone-and-ipad}

A Apple só oferece push a sites adicionados ao ecrã principal (iOS e iPadOS
16.4 ou posterior):

1. Abra o DumbMonit no **Safari**.
2. **Share → Add to Home Screen**, depois abra o DumbMonit **a partir do novo ícone**.
3. Inicie sessão, depois **Settings → Push notifications → Enable on this device**, e permita notificações.
4. **Send a test.**

No Safari sem instalar, a secção diz **Install first**.

## Escolher quais alertas chegam {#choose-which-alerts-arrive}

Em **Alerts → Notifications**, adicione um canal do tipo **Web Push**. Sem
nenhuma definição, todo dispositivo subscrito de toda conta recebe-o; liste
nomes de conta em **Accounts** para visar apenas algumas pessoas. A política
de canal habitual aplica-se (severidade mínima, horas de silêncio, filtro de
encaminhamento…), por isso pode, por exemplo, reservar um telefone só para
alertas críticos.

Uma notificação lê-se «Critical · nas01» com um resumo curto; tocar nela
abre a página **Alerts**. A resolução de um alerta substitui a sua
notificação em vez de adicionar uma segunda.

## Dispositivos e contas {#devices-and-accounts}

Cada conta vê e gere os seus próprios dispositivos: os leitores também podem
ativar o push no seu telefone, isso não altera mais nada. A lista mostra o
navegador e o sistema, o serviço push, quando o dispositivo foi adicionado
e quando recebeu uma mensagem por último; **Remove** esquece-o (e, no
dispositivo que está a usar, também cancela a subscrição do navegador). Um
dispositivo que o serviço push reporta como desaparecido (a app foi
removida, os dados do navegador foram limpos) é eliminado automaticamente
no envio seguinte.

## Resolução de problemas {#troubleshooting}

| O que vê | O que fazer |
|---|---|
| **Needs HTTPS** | Abra o DumbMonit através do seu endereço HTTPS. |
| **Install first** (iPhone, iPad) | Share → Add to Home Screen, abra a partir do ícone. |
| **Blocked** | As notificações foram recusadas para este site: permita-as nas definições do site do navegador (cadeado na barra de endereço, ou iOS Settings → Notifications → DumbMonit), depois recarregue. |
| **Not supported** | O navegador não tem Web Push (versão antiga, alguns navegadores de privacidade): use um Chrome, Edge, Firefox ou Safari recente. |
| O teste diz *push service unreachable* | O servidor não consegue alcançar o serviço push: verifique o seu HTTPS de saída e o DNS. |
| O teste diz *refused the VAPID signature* | Geralmente um relógio do servidor errado; verifique o NTP no anfitrião. |
| Nada chega no Android quando o telefone dorme | A otimização de bateria pode atrasar o navegador: isente-o, ou defina o **Urgency** do canal para `high`. |

## Privacidade e segurança {#privacy-and-security}

- As mensagens são encriptadas para cada dispositivo (RFC 8291) antes de
  saírem do servidor: o serviço push só vê um blob opaco, o dispositivo
  desencripta-o.
- O servidor assina cada mensagem com o seu próprio par de chaves VAPID
  (RFC 8292), gerado no primeiro arranque e guardado encriptado com o
  segredo da instância; a chave privada nunca é devolvida pela API. Um
  [restauro](../install/backup.md) numa máquina nova gera um novo par de
  chaves: ative o push de novo em cada dispositivo.
- O endereço de subscrição vem do navegador, por isso o servidor só aceita
  endereços HTTPS em IPs públicos: endereços privados, loopback e
  link-local são recusados, no momento da subscrição e de novo em cada
  ligação.
- O service worker (`/sw.js`) só mostra notificações. Não armazena nada em
  cache e não intercepta nenhum pedido: a interface e a API vêm sempre do
  servidor, nunca de uma cópia obsoleta.
