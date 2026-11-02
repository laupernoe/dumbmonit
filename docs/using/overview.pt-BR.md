# A página de visão geral

A página inicial responde a "está tudo bem?" em cerca de um segundo e depois
deixa você aprofundar conforme precisar.

## O banner de status {#the-bulletin}

Uma faixa no topo mostra um céu noturno fixo: estrelas que cintilam, uma lua
crescente e um pombo que o atravessa sem nunca pousar. O céu não muda com o
tempo; a frase ao seu pé diz como estão as coisas, em palavras. "Todos os 146
monitores estão ativos." quando nada precisa de você; caso contrário, "pve-02 está
fora do ar." ou "pve-02 merece uma olhada.", seguido, em voz mais baixa, do que o
sistema fez ("3 convidados retidos, alerta enviado."). Toque no pombo e ele dá uma
pirueta. Com `prefers-reduced-motion`, o pombo sai do céu e as estrelas ficam
paradas.

## O guia de primeira execução {#the-first-run-guide}

Em uma instância nova, a página abaixo do boletim não está vazia: é um guia
de três passos — adicione um monitor, conecte uma forma de ser avisado,
verifique que uma mensagem chega. Cada passo é um clique, cada um pode ser
dispensado por conta própria, e **Pular o guia** afasta todos os três. A
escolha é armazenada na instância, então um segundo navegador não reinicia
o guia. Veja
[Adicione seu primeiro monitor](../install/first-device.md#the-three-first-steps).

## Os quatro números {#the-figures}

Sob o banner há quatro números: **Monitores no ar** (sobre o total, com as contagens de fora do ar e em pausa), **Disponibilidade 30 d**, **Resposta p95** e **Alertas abertos** (críticos e de aviso). O intervalo de tempo do cabeçalho controla o p95 e o rio.

## O que precisa de atenção {#what-needs-attention}

A lista ao lado do rio só contém o que está quebrado agora, o pior primeiro, cada linha com a sua ação rápida: reconhecer ou adiar um alerta, ou silenciar um monitor (a pequena cruz). Sem nada quebrado, o pombo dorme e o cartão diz isso. No pé, a **próxima semana** em uma linha: o que expira nos próximos sete dias. **Ver tudo** abre os [Alertas](alerts.md).

## O rio de saúde {#the-health-river}

Uma faixa por monitor, uma célula por fatia do intervalo, os piores monitores no topo. A visão geral mostra os oito primeiros; **Ver todos os monitores** abre a aba [Lista](devices.md) de Monitores. Um estado é sempre escrito ao lado de uma forma (círculo ativo, losango degradado, quadrado fora do ar), nunca indicado só pela cor. Em um telefone, o rio abre com um toque.

## Dois gráficos {#charts}

Mais abaixo: o **tempo de resposta** de toda a frota no intervalo (p95 e mediana, com os monitores mais lentos) e os **alertas dos últimos sete dias** (disparados e resolvidos).

## Modo parede {#wall-mode}

`/wall` mostra um quadro para um monitor de sala: o veredito, o relógio, os cinco
números, o rio de saúde, os alertas abertos e quem está de plantão. Abra-o com o
ícone **Wall mode** no fundo do menu lateral ou com a paleta de comandos; ++esc++ ou
**Exit** volta ao console. Veja [Modo parede](wall.md).

## Instale-o em um celular {#install-it-on-a-phone}

A interface entrega um manifesto web, então um celular pode adicioná-la à
tela inicial: **Compartilhar → Adicionar à Tela de Início** no iOS,
**Instalar aplicativo** no menu do Chrome no Android. Ela então abre sem a
moldura do navegador, no tema para o qual o celular está configurado.

Deliberadamente não há **modo offline e nenhum service worker**. Uma tela de
monitoramento que mostra uma página em cache é pior que uma que diz não
conseguir alcançar o servidor: o ponto principal é que o que você lê está
atual. Sem um worker, um servidor atualizado também é servido imediatamente,
sem build desatualizado para limpar.

## Paleta de comandos {#command-palette}

++cmd+k++ no macOS, ++ctrl+k++ nos demais, abre a paleta de comandos: um
campo, resultados em três grupos.

| Grupo | Entradas |
|---|---|
| Pages | Visão geral, Monitores, Mapa, Alertas, Automação, Notificações, Explorador de métricas, Segurança, Páginas de status, Relatórios, Logs, Configurações, Modo parede, Documentação |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Todo monitor, com seu LED de estado; ++enter++ o abre |

## Navegação {#navigation}

Um **menu** estreito à esquerda lista quatro destinos: **Visão geral**, **Monitores**, **Alertas** (com a contagem do que precisa de você) e **Status**. No pé ficam as **Configurações**, o **modo parede**, a documentação, o GitHub e a sua conta. O que antes era uma página própria agora é uma aba dessas entradas. O cabeçalho indica onde você está, com o interruptor **AO VIVO** que pausa as atualizações do próprio console, o intervalo de tempo, a paleta e **Adicionar um monitor**. Em um telefone, o menu vira uma barra de abas inferior com o essencial e **Mais**.

- **Visão geral**: se está tudo bem e o que precisa de você.
- **Monitores**: cada fonte que você monitora, em quatro abas: **Lista**, **Mapa**, **Logs** e **Segurança**. Veja [Monitores](../monitors/index.md).
- **Alertas**: cinco abas: **Agora**, **Histórico**, **Regras**, **Automação** e **Plantões**. Veja a [página de alertas](alerts.md).
- **Status**: **Páginas** (páginas de status públicas) e **Relatórios**. Veja [Status](../status/index.md).
- **Configurações**: a sua conta, equipe e acesso, monitoramento, notificações, a parede e o sistema. Veja [Configurações](../settings/index.md).

A interface é escura e nada nela transmite status só pela cor: um indicador
sempre vem com uma palavra (Reporting, Unreachable, Advisory, Warning,
Suppressed by parent, Building up).
