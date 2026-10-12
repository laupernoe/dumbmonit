# A página de visão geral {#the-overview-page}

A página inicial responde «está tudo bem?» em cerca de um segundo, depois
deixa-o explorar em detalhe quando quiser.

![A visão geral](../assets/screenshots/overview-light.png){ loading=lazy }

## O boletim {#the-bulletin}

A faixa no topo é um boletim meteorológico para a sua rede. Uma frase diz o
estado do céu: «Clear skies.» quando nada precisa de si, caso contrário uma
contagem do que precisa («9 advisories, 1 unreachable.»). Abaixo, placas
contam dispositivos a reportar e inacessíveis, com a hora da última
verificação, e três leituras: **Reporting**, **Needs you**, **Forecasts**.

A janela meteorológica à direita acompanha a rede: céu limpo, nublado,
encoberto ou tempestade, dependendo de quantos alertas estão ativos e quão
severos são, com o pombo a voar. Sob `prefers-reduced-motion`, tudo fica
imóvel.

A única ação principal da página é **Add a device**.

## O guia de primeira utilização {#the-first-run-guide}

Numa instância nova, a página sob o boletim não está vazia: é um guia em
três passos — adicionar um dispositivo, ligar uma forma de ser avisado,
verificar que chega uma mensagem. Cada passo é um clique, cada um pode ser
dispensado por si só, e **Skip the guide** arreda os três. A escolha é
guardada na instância, por isso um segundo navegador não recomeça o guia.
Ver [Adicionar o primeiro dispositivo](../install/first-device.md#the-three-first-steps).

## Needs you {#needs-you}

A lista do que precisa de si agora, o mais severo primeiro: dispositivos
inacessíveis, depois alertas ativos por severidade. Cada linha nomeia a
regra, o dispositivo, o valor atual, há quanto tempo dura, e se agrupa
várias séries («×4» para quatro sistemas de ficheiros no mesmo dispositivo).
Quatro ações: **Ack** (confirmar este alerta por 1 h, 4 h, 24 h ou até
resolver — os lembretes pausam, a resolução continua a ser notificada),
**Snooze** (ocultar só este alerta por 1 h, 8 h, 1 dia ou até resolver),
**Ignore** (parar esta regra de alguma vez alertar neste dispositivo de
novo, reversivelmente), e **Open device**.

Os alertas confirmados e adiados movem-se para o seu próprio grupo mais
silencioso sob a lista e saem da contagem *Needs you*. Um alerta ignorado
desaparece completamente — não é «conhecido», está desligado para esse
dispositivo. Ver [Livrar-se de um alerta](alerts.md#getting-rid-of-an-alert).

## Resumo de dispositivos {#devices-summary}

Uma linha: quantos dispositivos, quantos a reportar, quantos inacessíveis,
e uma ligação para a página completa Devices. A visão geral nunca lista
dispositivos: o bastidor vive em [Devices](devices.md).

## Previsões e últimas 24 horas {#forecasts-and-last-24-hours}

**Forecasts** mostra o que é previsto em vez de observado: a anomalia da
linha de base sazonal, as extrapolações «cheio dentro de quatro dias», a
estimativa de enchimento do PBS, e regras cujo nome anuncia uma previsão.
Enquanto uma linha de base está a aprender, as suas entradas dizem-o.

**Last 24 hours** lista as transições ativas do dia, com uma ligação para o
histórico completo na página Alerts.

## Modo parede {#wall-mode}

`/wall` mostra só o boletim, em tela cheia, para um monitor de sala: a
frase, as contagens e a lista *Needs you*, atualizadas a cada 20 segundos,
com o ecrã mantido aceso. ++esc++ ou **Exit** volta à visão geral. Abra-o a
partir da paleta de comandos («Wall mode») ou escrevendo o URL. Também pode
reproduzir música iniciada a partir do seu telefone: ver
[Modo parede e música](wall.md).

![Modo parede](../assets/screenshots/wall-light.png){ loading=lazy }

## Instalar num telefone {#install-it-on-a-phone}

A interface vem com um manifesto web, por isso um telefone pode adicioná-la
ao ecrã principal: **Share → Add to Home Screen** no iOS, **Install app**
no menu do Chrome no Android. Depois abre sem o chrome do navegador, no
tema que o telefone tem definido.

Não há, de propósito, **nenhum modo offline nem service worker**. Um ecrã
de monitorização que mostra uma página em cache é pior do que um que diz
que não consegue alcançar o servidor: o objetivo é que o que lê esteja
atual. Sem worker, um servidor atualizado também é servido de imediato, sem
nenhuma versão obsoleta para limpar.

## Paleta de comandos {#command-palette}

++cmd+k++ no macOS, ++ctrl+k++ nos restantes, abre a paleta de comandos: um
campo, resultados em três grupos.

| Grupo | Entradas |
|---|---|
| Pages | Overview, Devices, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a device, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident, Toggle theme |
| Devices | Todos os dispositivos, com o seu LED de estado; ++enter++ abre-o |

## Navegação {#navigation}

A barra superior tem Overview, Devices, Alerts (com a contagem do que
precisa de si), Status e Settings, o atalho da paleta e o alternador de
tema. Num telefone, torna-se uma barra de separadores inferior com as
mesmas cinco entradas.

- **Overview** — o boletim: está tudo bem, e o que precisa de si.
- **Devices** — o bastidor; cada fonte que monitoriza. Ver [Página de dispositivos](devices.md).
- **Alerts** — o que está ativo, manutenção agendada, as regras, e os
  canais e política de notificação. Ver [Página de alertas](alerts.md).
- **Status** — as páginas de estado públicas e os seus anúncios. Ver
  [Páginas de estado](status-pages.md).
- **Settings** — a sua conta, utilizadores e sign-on único, tokens de
  agente e assistente, aparência, sobre. Ver [Settings](settings.md). Nada na interface transmite estado só pela
cor: uma placa ou um LED vêm sempre com uma palavra (Reporting, Unreachable,
Advisory, Warning, Suppressed by parent, Building up).
