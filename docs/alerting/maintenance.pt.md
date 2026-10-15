# Janelas de manutenção {#maintenance-windows}

Uma janela de manutenção (um *silêncio* na API) suspende as notificações de
um dispositivo, ou de tudo, durante um período. A avaliação continua por
baixo: quando a janela termina, um alerta ainda ativo é notificado uma vez,
em vez de aparecer do nada depois de duas horas.

## Onde {#where}

- **Alerts → Maintenance**: a lista de janelas, com *Active now* ou
  *Scheduled*, e o formulário para adicionar uma.
- **Página do dispositivo → Silence 1 h / Silence until…**: uma janela
  pontual rápida para todo o dispositivo.
- ++ctrl+k++ → *Schedule maintenance*.

**Snooze**, num alerta do separador *Now* ou numa página de dispositivo, é
uma versão mais pequena do mesmo mecanismo: uma janela pontual limitada às
etiquetas exatas desse alerta em vez de todo o dispositivo, para que uma
regra irmã continue a falar. Também aparece aqui, chamada `Snooze · …`. Ver
[Livrar-se de um alerta](../using/alerts.md#getting-rid-of-an-alert).

## Janela pontual {#one-off-window}

| Campo | Significado |
|---|---|
| Name | Mostrado na lista e no histórico. |
| Device | Deixe vazio para cobrir todos os dispositivos. |
| Start, End | Horas absolutas. A janela inclui o início e exclui o fim. |
| Comment | Opcional. Por que razão a janela existe. |

As janelas pontuais são eliminadas depois de terminarem.

## Janela semanal {#weekly-window}

| Campo | Significado |
|---|---|
| Days | Quais dias da semana (segunda a domingo). |
| From, To | Hora local no fuso horário da janela. Se *To* for igual ou anterior a *From*, a janela estende-se até à meia-noite («23:00 → 01:00» pertence ao dia em que começa). |
| Time zone | Um nome IANA como `Europe/Lisbon`. Pré-preenchido com o fuso do seu navegador. |

## Janela mensal {#monthly-window}

| Campo | Significado |
|---|---|
| On a weekday | *First*, *Second*, …, *Fifth* ou *Last*, mais o dia — «o primeiro domingo», «a última sexta-feira». |
| On a date | Um ou mais dias do mês, de 1 a 31, separados por vírgulas («1, 15»). |
| From | Hora local de início, no fuso horário da janela. |
| For | Quanto tempo a janela dura, como tempo real decorrido. Uma janela mensal pode durar mais do que um dia. |
| Time zone | Como acima. |

Um mês que não contém a ocorrência é saltado em vez de aproximado: não há
dia 31 em fevereiro, nem um quinto domingo em abril de 2026, por isso a
janela simplesmente não abre nesse mês. A lista mostra a próxima ocorrência
calculada pelo servidor, para que a possa verificar num olhar.

O DumbMonit não tem expressões cron, de propósito: uma expressão cron nomeia
*instantes*, e uma janela de manutenção precisa de uma *duração*. Dias do mês
e «o n-ésimo dia da semana» cobrem os calendários que as pessoas realmente
mantêm, e lêem-se numa linha.

## Fusos horários e hora de verão {#time-zones-and-daylight-saving}

Uma janela recorrente abre à sua hora local do relógio de parede e depois
corre pela sua duração em tempo real. Ambas as partes importam:

- **A hora local não deriva.** «Todos os domingos às 02:00 em
  `Europe/Lisbon`» é 01:00 UTC no inverno e 00:00 UTC no verão — a janela
  segue a mudança de hora em vez de deslizar uma hora duas vezes por ano.
- **A duração não se estica.** Na noite em que o relógio recua, 02:00 a
  04:00 no relógio duraria três horas reais; a janela continua a durar duas.
  Na noite em que o relógio avança e as 02:00 não existem, a janela abre
  assim que o relógio retoma.

Uma janela guardada antes de existirem fusos horários mantém o seu deslocamento
UTC fixo e comporta-se exatamente como antes. Volte a guardá-la com um fuso
para que siga as mudanças de hora. Um nome de fuso que o servidor não
conhece é recusado ao guardar, com um exemplo.

## Correspondências {#matchers}

Uma janela também pode ser limitada por correspondências exatas de etiquetas
(`matchers` na API): todas têm de corresponder às etiquetas do alerta, por
exemplo `tag_role = lab` e `__name__ = dumbmonit_up`. Uma janela sem
dispositivo e sem correspondência silencia toda a instância. Janelas
sobrepostas não são problema; a primeira que cobre o momento explica o
silêncio.

## Como se mostra um alerta silenciado {#what-a-silenced-alert-looks-like}

O alerta mantém a sua fase e mostra `silenced: true`; o histórico regista a
transição com `notified: false` e o motivo. Desativar uma janela
(`enabled: false`) mantém-a na lista sem efeito.

## Numa página de estado pública {#on-a-public-status-page}

Um dispositivo coberto por uma janela de manutenção é mostrado como
**Maintenance** em qualquer [página de estado](../using/status-pages.md) que
o liste, em vez de vermelho. Se nada mais na página estiver em baixo ou
degradado, o estado geral da página também lê *Maintenance*. Só as janelas
que nomeiam um dispositivo — ou cujas correspondências são etiquetas a nível
de dispositivo — aparecem desta forma; uma janela limitada a uma métrica
continua a ser uma questão de alerta.
