# A página de resumo

A página inicial responde a «está tudo bem?» em cerca de um segundo e depois
deixa-o aprofundar conforme precisar.

## O banner de estado {#the-bulletin}

Uma faixa no topo mostra um céu noturno fixo: estrelas que cintilam, uma lua
crescente e um pombo que o atravessa sem nunca pousar. O céu não muda com o
tempo; a frase ao seu pé diz como estão as coisas, por palavras. «Os 146
monitores estão ativos.» quando nada precisa de si; caso contrário, «pve-02 está
em baixo.» ou «pve-02 merece uma olhadela.», seguido, em voz mais baixa, do que o
sistema fez («3 convidados retidos, alerta enviado.»). Toque no pombo e ele faz
uma pirueta. Com `prefers-reduced-motion`, o pombo sai do céu e as estrelas ficam
paradas.

## O guia de primeira utilização {#the-first-run-guide}

Numa instância nova, a página sob o boletim não está vazia: é um guia em
três passos — adicionar um monitor, ligar uma forma de ser avisado,
verificar que chega uma mensagem. Cada passo é um clique, cada um pode ser
dispensado por si só, e **Skip the guide** arreda os três. A escolha é
guardada na instância, por isso um segundo navegador não recomeça o guia.
Ver [Adicionar o primeiro monitor](../install/first-device.md#the-three-first-steps).

## Os cinco números e o rio de saúde {#the-figures}

Sob o banner, cinco números: **Monitors up** (sobre o total, com os em baixo e os
em pausa), **Availability 30 d**, **p95 response time**, **Open alerts** (críticos
e avisos) e **MTTR 30 d** (tempo médio de resolução). O intervalo de tempo do
cabeçalho controla o p95 e o rio.

O **rio de saúde** é uma faixa por monitor, uma célula por fatia do intervalo,
com os piores monitores no topo. Um estado é sempre escrito junto de uma forma
(círculo ativo, losango degradado, quadrado em baixo), nunca só por cor.

## A tabela de monitores {#the-monitor-table}

A tabela lista cada monitor, o pior primeiro. Filtra por estado, tipo e texto,
permite acrescentar colunas (endereço, pasta) ou remover outras e controla-se
pelo teclado: ++j++ / ++k++ movem, ++enter++ seleciona, ++o++ abre.

O monitor selecionado alimenta o **painel de contexto** à direita, como um
inspetor: o que precisa de atenção (com as ações sobre alertas, ver
[Página de alertas](alerts.md)), **Check now** e **Silence 1 h**, as previsões
mais próximas e quem está de prevenção. ++i++ mostra-o ou oculta-o; num ecrã mais
estreito abre-se como gaveta. A lista completa está na
[página de monitores](devices.md).

## Modo parede {#wall-mode}

`/wall` mostra um quadro para um monitor de sala: o veredicto, o relógio, os cinco
números, o rio de saúde, os alertas abertos e quem está de prevenção. Abra-o com
o ícone **Wall mode** da barra superior ou com a paleta de comandos; ++esc++ ou
**Exit** volta à consola. Ver [Modo parede](wall.md).

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
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Cada monitor, com o seu LED de estado; ++enter++ abre-o |

## Navegação {#navigation}

Uma **barra de ícones** estreita à esquerda lista cada destino: Overview,
Monitors, Alerts (com a contagem do que precisa de si), Dashboards, Explore, o
mapa de topologia, Security, Logs, Status pages, Reports e Settings. No fundo
ficam a documentação, o GitHub e a sua conta. O cabeçalho diz onde está, com o
interruptor **LIVE** que pausa as atualizações da própria consola, o intervalo de
tempo, a paleta, **Wall mode** e **New monitor**. Num telefone, a barra torna-se
uma barra de separadores inferior com o essencial e **More**.

- **Overview**: está tudo bem, e o que precisa de si.
- **Monitors**: cada fonte que monitoriza. Ver a
  [página de monitores](devices.md).
- **Alerts**: o que está ativo, janelas de manutenção, histórico e regras. Ver a
  [página de alertas](alerts.md).
- **Settings**: a sua conta, utilizadores e sign-on único, tokens, canais de
  notificação, aparência, sobre. Ver [Settings](settings.md).

A interface é escura e nada nela transmite estado só pela cor: um indicador vem
sempre com uma palavra (Reporting, Unreachable, Advisory, Warning, Suppressed by
parent, Building up).
