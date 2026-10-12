# Página de alertas {#alerts-page}

![A página Alerts: o que precisa de si, as regras, a manutenção agendada e o histórico](../assets/screenshots/alerts-light.png){ loading=lazy }

Tudo sobre alertas numa página, atrás de cinco separadores. O separador está
no URL (`/alerts#rules`), por isso uma vista pode ser ligada. **Schedule
maintenance**, no canto superior direito, abre o formulário para uma janela
a partir de qualquer separador.

| Separador | O que contém |
|---|---|
| **Now** | A lista em direto *Needs you*, agrupada por dispositivo: placa de severidade, motivo, desde quando, e *Ack*, *Snooze*, *Ignore* e *Open device* em cada um. Os alertas confirmados e adiados ficam cada um no seu próprio grupo silencioso no fundo. O emblema no separador, e em Alerts na barra superior, é a contagem do que ainda precisa de si. |
| **Scheduled** | Janelas de manutenção, *Active now* ou *Scheduled*, pontuais, semanais ou mensais, cada uma com a próxima ocorrência. Ver [Janelas de manutenção](../alerting/maintenance.md). |
| **Rules** | Todas as regras com a sua severidade, uma marca *Built-in* e, se algum dispositivo a ignora, *Ignored on N devices*; ativar, editar em linha, eliminar as suas próprias, criar uma regra de limiar. Ver [Regras](../alerting/rules.md). |
| **Notifications** | Onde os alertas o alcançam: os canais e a política de notificação. Detalhes abaixo. |
| **History** | As últimas 200 transições, cada uma nomeando o dispositivo, a regra e o que aconteceu; as resolvidas podem ser limpas. |

O mesmo modelo de verdade alimenta o boletim de visão geral e esta página,
por isso os dois estão sempre de acordo sobre o que precisa de si.

## Livrar-se de um alerta {#getting-rid-of-an-alert}

Um alerta ativo não é binário — «resolver» ou «viver com o ruído para
sempre». Quatro ações, para quatro situações diferentes, abertas a
administradores e operadores (um leitor vê o estado mas não pode alterá-lo):

- **Acknowledge** («estou a tratar disso») é para um alerta em que está
  ativamente a trabalhar: «sei, para de me lembrar durante 4 h». O menu
  oferece 1 h, 4 h, 24 h ou *até resolver*, mais uma nota opcional para quem
  ler o cartão depois de si. O alerta continua a disparar e a ser avaliado;
  só os seus lembretes e escalonamentos pausam — incluindo o escalonamento
  para um segundo canal. Continua a ser informado quando resolve, e a
  confirmação apaga-se nesse momento — um alerta que volta mais tarde
  notifica de novo. O cartão move-se da lista principal para um grupo mais
  silencioso *Acknowledged*, lê-se *Acked by someone until a time*, e
  oferece **Un-ack**.
- **Snooze** é para «não agora, verifico mais tarde» sem afirmar que está a
  fazer algo sobre isso: 1 h, 8 h, 1 dia ou *até resolver*. Ao contrário de
  Ack, funciona abrindo uma janela de manutenção limitada às etiquetas
  exatas desse alerta — uma regra irmã no mesmo dispositivo continua a
  falar. O cartão move-se para um grupo silencioso *Snoozed* mostrando o
  tempo restante, e oferece **Unsnooze**. Os lembretes pausam da mesma
  forma que os de uma confirmação.
- **Ignore** («não me alerte sobre isto outra vez») é para uma regra que
  nunca fará sentido neste dispositivo: desativa essa regra só para esse
  dispositivo (uma substituição por dispositivo — ver
  [Substituições por dispositivo](../alerting/rules.md#per-device-overrides))
  e o alerta desaparece de vez, de imediato, não apenas silenciado. É
  reversível: **Stop ignoring** na página do dispositivo, ou a partir da
  própria regra em Alerts → Rules, onde se lê *Ignored on N devices*.
- **Clear** é para o histórico, não para um alerta ativo: uma vez uma
  transição resolvida, *Clear* (ou *Clear all resolved*) retira-a da vista
  padrão de History. A linha permanece na base de dados — nada é eliminado
  — e *Show cleared* traz-a de volta.

As páginas de estado públicas ignoram confirmações e adiamentos: um alerta
confirmado ou adiado continua a ser um alerta para o mundo exterior. Uma
regra ignorada, por outro lado, nunca produziu um alerta, para começar.

## Notificações {#notifications}

Os canais e a política vivem aqui — sob Alerts, não Settings — porque
decidem quem ouve um alerta, o que é uma questão de alertas. Dois painéis:

**Channels.** A lista de canais, cada um com o seu tipo, *Enabled* ou
*Disabled*, quando enviou algo por último, e o seu último erro se existir.
**Add channel** abre um formulário construído a partir da descrição do
servidor do tipo escolhido: definições (visíveis) e segredos (só de
escrita). **Send test** envia uma mensagem de teste e mostra o resultado em
linha. As **Delivery options** de cada canal detêm a sua severidade mínima,
se ouve resoluções, um intervalo mínimo por alerta, e as suas **quiet
hours** (semanais, no seu fuso horário: só os alertas de nível Warning
passam, o resto espera por um resumo).

**Only some alerts.** As mesmas Delivery options detêm o **filtro de
encaminhamento** do canal: quais alertas quer, por etiqueta de dispositivo
(`site=cellar`), por tipo de dispositivo (`proxmox`), ou por regra.
Condições no mesmo campo lêem-se como *ou*, campos diferentes como *e*, e
uma linha *Except* ganha sempre. Sob o editor, uma pré-visualização lista
quais dos seus dispositivos o filtro seleciona agora mesmo, e uma frase diz
o conjunto de volta — *este canal recebe avisos e acima de dispositivos
etiquetados site=cellar, exceto dispositivos etiquetados role=lab*. Um canal
sem filtro continua a receber tudo, o que é o que todo canal existente faz.

Editar um canal e deixar o segredo vazio mantém o guardado. Os tipos de
canal e os seus campos estão documentados em
[Canais de notificação](../notifications.md).

As regras de alerta incorporadas notificam todos os canais ativados; uma
regra também pode ser limitada a canais específicos no seu editor.

**Notification policy.** Os controlos globais que mantêm as notificações
poucas: a janela de lote, o limite por canal por hora, a deteção de
oscilação (flap) em *More options*, o URL público usado pelas ligações
«Open in DumbMonit», e a linha de **escalation** — *se ninguém confirmar*
dentro de um prazo, *também avisar* outro canal, uma vez. O prazo conta a
partir do momento em que a primeira mensagem realmente saiu, e confirmar o
alerta (ou a sua resolução) para o salto. Ver
[Política de notificação](../alerting/notifications.md).

Ligações: `/alerts#notifications` abre o separador,
`/alerts#notifications-policy` desloca até ao painel de política. As antigas
ligações de Settings (`/settings#notifications`,
`/settings#notifications-policy`) redirecionam para aqui.

## Operadores e leitores {#operators-and-viewers}

Um **operador** trata dos alertas sem tocar na configuração: confirma,
adia, ignora uma regra para um dispositivo, agenda ou elimina janelas de
manutenção e limpa o histórico resolvido. As regras, canais e a política de
notificação ficam só de leitura para eles, marcados *Operator — config is
admin only*. Ignorar é a única substituição por dispositivo que um operador
pode fazer; os limiares continuam a ser decisão de um administrador, e um
*Ignore* ou *Stop ignoring* de um operador mantém qualquer limiar que um
administrador tenha definido para esse dispositivo.

Um leitor vê todos os separadores mas nenhum controlo: a página mostra
*Viewer — read only* onde um administrador encontraria os botões.
