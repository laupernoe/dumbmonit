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

## Os quatro números {#the-figures}

Sob o banner há quatro números: **Monitores ativos** (sobre o total, com as contagens de parados e em pausa), **Disponibilidade 30 d**, **Resposta p95** e **Alertas abertos** (críticos e de aviso). O intervalo de tempo do cabeçalho comanda o p95 e o rio.

## O que precisa de atenção {#what-needs-attention}

A lista ao lado do rio só contém o que está avariado agora, o pior primeiro, cada linha com a sua ação rápida: reconhecer ou adiar um alerta, ou silenciar um monitor (a pequena cruz). Sem nada avariado, o pombo dorme e o cartão di-lo. No fim, a **próxima semana** numa linha: o que expira nos próximos sete dias. **Ver tudo** abre os [Alertas](alerts.md).

## O rio de saúde {#the-health-river}

Uma faixa por monitor, uma célula por fatia do intervalo, os piores monitores no topo. A visão geral mostra os oito primeiros; **Ver todos os monitores** abre o separador [Lista](devices.md) de Monitores. Um estado escreve-se sempre ao lado de uma forma (círculo ativo, losango degradado, quadrado parado), nunca só pela cor. Num telefone, o rio abre-se com um toque.

## Dois gráficos {#charts}

Mais abaixo: o **tempo de resposta** de toda a frota no intervalo (p95 e mediana, com os monitores mais lentos) e os **alertas dos últimos sete dias** (disparados e resolvidos).

## Modo parede {#wall-mode}

`/wall` mostra um quadro para um monitor de sala: o veredicto, o relógio, os cinco
números, o rio de saúde, os alertas abertos e quem está de prevenção. Abra-o com
o ícone **Wall mode** no fundo do menu lateral ou com a paleta de comandos; ++esc++ ou
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
| Pages | Visão geral, Monitores, Mapa, Alertas, Automação, Notificações, Explorador de métricas, Segurança, Páginas de estado, Relatórios, Registos, Definições, Modo parede, Documentação |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Cada monitor, com o seu LED de estado; ++enter++ abre-o |

## Navegação {#navigation}

Um **menu** estreito à esquerda lista quatro destinos: **Visão geral**, **Monitores**, **Alertas** (com a contagem do que precisa de si) e **Estado**. No fundo estão as **Definições**, o **modo parede**, a documentação, o GitHub e a sua conta. O que antes era uma página própria é agora um separador destas entradas. O cabeçalho indica onde está, com o interruptor **AO VIVO** que pausa as atualizações da própria consola, o intervalo de tempo, a paleta e **Adicionar um monitor**. Num telefone, o menu passa a uma barra de separadores inferior com o essencial e **Mais**.

- **Visão geral**: se está tudo bem e o que precisa de si.
- **Monitores**: cada fonte que monitoriza, em quatro separadores: **Lista**, **Mapa**, **Registos** e **Segurança**. Veja [Monitores](../monitors/index.md).
- **Alertas**: cinco separadores: **Agora**, **Histórico**, **Regras**, **Automação** e **Plantões**. Veja a [página de alertas](alerts.md).
- **Estado**: **Páginas** (páginas de estado públicas) e **Relatórios**. Veja [Estado](../status/index.md).
- **Definições**: a sua conta, equipa e acesso, monitorização, notificações, a parede e o sistema. Veja [Definições](../settings/index.md).

A interface é escura e nada nela transmite estado só pela cor: um indicador vem
sempre com uma palavra (Reporting, Unreachable, Advisory, Warning, Suppressed by
parent, Building up).
