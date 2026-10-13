# Notificações push (PWA) {#push-notifications-pwa}

O DumbMonit pode tocar no seu celular ou computador diretamente, como um
aplicativo nativo faria, sem instalar um app ou criar uma conta em lugar
nenhum: o próprio serviço de push do navegador (Google, Mozilla, Apple,
Microsoft) carrega uma mensagem criptografada de ponta a ponta do seu
servidor até o dispositivo, e a notificação aparece mesmo com todas as abas
do DumbMonit fechadas. Isso é o **Web Push** padrão; o site também pode ser
instalado como um aplicativo (um *progressive web app*, PWA) para abrir a
partir de um ícone como qualquer outro.

Dois passos: ative o push em cada dispositivo (esta página), depois decida
quais alertas vão para lá com um canal **Web Push**
([Canais de notificação](../notifications.md#web-push)).

## Antes de começar {#before-you-start}

- **HTTPS é obrigatório.** Navegadores só oferecem push a uma origem segura.
  Coloque o DumbMonit detrás de um proxy reverso com um certificado (Caddy,
  Traefik, Nginx Proxy Manager…) e abra-o através desse endereço.
  `http://localhost` também é aceito, o que só é útil para testar no próprio
  servidor. Em um simples `http://192.168.x.x`, a seção diz **Precisa de
  HTTPS**.
- **O servidor precisa de HTTPS de saída** para os serviços de push:
  `fcm.googleapis.com` (Chrome, e Edge no Android), `*.notify.windows.com`
  (Edge no Windows), `updates.push.services.mozilla.com` (Firefox) e
  `*.push.apple.com` (Safari, iPhone, iPad). Nada entra: seu DumbMonit não
  precisa ser acessível pela internet, o celular só precisa alcançá-lo
  quando você tocar em uma notificação.
- **Defina `DUMBMONIT_PUBLIC_URL`** para o endereço HTTPS que você usa: ele é
  dado aos serviços de push como o contato do remetente, e é o endereço para
  o qual os links em outros canais apontam.

## Ativar em um celular ou computador {#enable-on-a-phone-or-computer}

### Android, Windows, macOS, Linux {#android-windows-macos-linux}

1. Abra o DumbMonit no Chrome, Edge, Firefox ou Safari e entre.
2. Opcional: instale-o como um aplicativo (barra de endereço → **Instalar**,
   ou o menu do navegador → **Adicionar à tela inicial**). As notificações
   funcionam de qualquer forma.
3. **Configurações → Notificações push → Ativar neste dispositivo**, e
   permita as notificações quando o navegador perguntar.
4. **Envie um teste.** Uma *DumbMonit — notificação de teste* deve aparecer
   em poucos segundos.

### iPhone e iPad {#iphone-and-ipad}

A Apple só oferece push a sites adicionados à Tela de Início (iOS e iPadOS
16.4 ou posteriores):

1. Abra o DumbMonit no **Safari**.
2. **Compartilhar → Adicionar à Tela de Início**, depois abra o DumbMonit
   **a partir do novo ícone**.
3. Entre, depois **Configurações → Notificações push → Ativar neste
   dispositivo**, e permita as notificações.
4. **Envie um teste.**

No Safari sem instalar, a seção diz **Instale primeiro**.

## Escolher quais alertas chegam {#choose-which-alerts-arrive}

Em **Alertas → Notificações**, adicione um canal do tipo **Web Push**. Sem
nenhuma configuração, todo dispositivo inscrito de toda conta o recebe;
liste nomes de conta em **Contas** para direcionar a apenas algumas pessoas.
A política de canal habitual se aplica (severidade mínima, horas de
silêncio, filtro de roteamento…), então você pode, por exemplo, manter um
celular apenas para alertas críticos.

Uma notificação lê `Critical · nas01` com um resumo curto; tocar nela abre a
página **Alertas**. A resolução de um alerta substitui sua notificação em
vez de adicionar uma segunda.

## Dispositivos e contas {#devices-and-accounts}

Cada conta vê e gerencia seus próprios dispositivos: visualizadores também
podem ativar o push em seu celular, isso não muda mais nada. A lista mostra
o navegador e o sistema, o serviço de push, quando o dispositivo foi
adicionado e quando recebeu uma mensagem por último; **Remover** o esquece
(e, no dispositivo que você está usando, também cancela a inscrição do
navegador). Um dispositivo que o serviço de push reporta como desaparecido
(o app foi removido, os dados do navegador foram limpos) é excluído
automaticamente no próximo envio.

## Solução de problemas {#troubleshooting}

| O que você vê | O que fazer |
|---|---|
| **Precisa de HTTPS** | Abra o DumbMonit através de seu endereço HTTPS. |
| **Instale primeiro** (iPhone, iPad) | Compartilhar → Adicionar à Tela de Início, abra a partir do ícone. |
| **Bloqueado** | Notificações foram recusadas para este site: permita-as nas configurações do site do navegador (cadeado na barra de endereço, ou iOS Configurações → Notificações → DumbMonit), depois recarregue. |
| **Não suportado** | O navegador não tem Web Push (versão antiga, alguns navegadores de privacidade): use um Chrome, Edge, Firefox ou Safari recente. |
| O teste diz *serviço de push inacessível* | O servidor não consegue alcançar o serviço de push: verifique seu HTTPS de saída e DNS. |
| O teste diz *assinatura VAPID recusada* | Geralmente um relógio do servidor errado; verifique o NTP no host. |
| Nada chega no Android quando o celular dorme | A otimização de bateria pode atrasar o navegador: isente-o, ou defina a **Urgência** do canal para `high`. |

## Privacidade e segurança {#privacy-and-security}

- Mensagens são criptografadas para cada dispositivo (RFC 8291) antes de
  deixarem o servidor: o serviço de push só vê um blob opaco, o dispositivo
  o descriptografa.
- O servidor assina cada mensagem com seu próprio par de chaves VAPID
  (RFC 8292), gerado na primeira inicialização e armazenado criptografado
  com o segredo da instância; a chave privada nunca é retornada pela API.
  Uma [restauração](../install/backup.md) em uma máquina nova gera um novo
  par de chaves: ative o push novamente em cada dispositivo.
- O endereço de assinatura vem do navegador, então o servidor só aceita
  endereços HTTPS em IPs públicos: endereços privados, loopback e
  link-local são recusados, no momento da assinatura e novamente em cada
  conexão.
- O service worker (`/sw.js`) só mostra notificações. Ele não armazena em
  cache nada e não intercepta nenhuma requisição: a interface e a API
  sempre vêm do servidor, nunca de uma cópia desatualizada.
