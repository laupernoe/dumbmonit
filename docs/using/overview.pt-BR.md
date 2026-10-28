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

## Os cinco números e o rio de saúde {#the-figures}

Sob o banner, cinco números: **Monitors up** (sobre o total, com os fora do ar e
os pausados), **Availability 30 d**, **p95 response time**, **Open alerts**
(críticos e avisos) e **MTTR 30 d** (tempo médio de resolução). O intervalo de
tempo do cabeçalho controla o p95 e o rio.

O **rio de saúde** é uma faixa por monitor, uma célula por fatia do intervalo,
com os piores monitores no topo. Um estado é sempre escrito junto de uma forma
(círculo ativo, losango degradado, quadrado fora do ar), nunca só por cor.

## A tabela de monitores {#the-monitor-table}

A tabela lista cada monitor, o pior primeiro. Filtra por estado, tipo e texto,
permite adicionar colunas (endereço, pasta) ou remover outras e é controlada pelo
teclado: ++j++ / ++k++ movem, ++enter++ seleciona, ++o++ abre.

O monitor selecionado alimenta o **painel de contexto** à direita, como um
inspetor: o que precisa de atenção (com as ações sobre alertas, veja
[Página de alertas](alerts.md)), **Check now** e **Silence 1 h**, as previsões
mais próximas e quem está de plantão. ++i++ o mostra ou oculta; em uma tela mais
estreita ele abre como gaveta. A lista completa está na
[página de monitores](devices.md).

## Modo parede {#wall-mode}

`/wall` mostra um quadro para um monitor de sala: o veredito, o relógio, os cinco
números, o rio de saúde, os alertas abertos e quem está de plantão. Abra-o com o
ícone **Wall mode** da barra superior ou com a paleta de comandos; ++esc++ ou
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
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Todo monitor, com seu LED de estado; ++enter++ o abre |

## Navegação {#navigation}

Uma **barra de ícones** estreita à esquerda lista cada destino: Overview,
Monitors, Alerts (com a contagem do que precisa de você), Dashboards, Explore, o
mapa de topologia, Security, Logs, Status pages, Reports e Settings. No pé ficam a
documentação, o GitHub e sua conta. O cabeçalho diz onde você está, com o
interruptor **LIVE** que pausa as atualizações do próprio console, o intervalo de
tempo, a paleta, **Wall mode** e **New monitor**. Em um celular, a barra vira uma
barra de abas inferior com o essencial e **More**.

- **Overview**: está tudo bem, e o que precisa de você.
- **Monitors**: toda fonte que você monitora. Veja a
  [página de monitores](devices.md).
- **Alerts**: o que está disparando, janelas de manutenção, histórico e regras.
  Veja a [página de alertas](alerts.md).
- **Settings**: sua conta, usuários e login único, tokens, canais de
  notificação, aparência, sobre. Veja [Settings](settings.md).

A interface é escura e nada nela transmite status só pela cor: um indicador
sempre vem com uma palavra (Reporting, Unreachable, Advisory, Warning,
Suppressed by parent, Building up).
