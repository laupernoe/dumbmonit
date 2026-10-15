# Janelas de manutenção {#maintenance-windows}

Uma janela de manutenção (um *silêncio* na API) interrompe notificações para
um monitor, ou para tudo, durante um período. A avaliação continua por
baixo: quando a janela termina, um alerta que ainda está ativo é notificado
uma vez, em vez de aparecer do nada depois de duas horas.

## Onde {#where}

- **Alertas → Manutenção**: a lista de janelas, com *Ativa agora*
  ou *Programada*, e o formulário para adicionar uma.
- **Página do monitor → Silenciar 1 h / Silenciar até…**: uma janela
  pontual rápida para o monitor inteiro.
- ++ctrl+k++ → *Programar manutenção*.

**Soneca**, em um alerta na aba *Agora* ou em uma página de
monitor, é uma versão menor do mesmo mecanismo: uma janela pontual
restrita aos rótulos exatos daquele alerta em vez do monitor inteiro, de
modo que uma regra irmã continua falando. Ela também aparece aqui, nomeada
`Snooze · …`. Veja
[Se livrando de um alerta](../using/alerts.md#getting-rid-of-an-alert).

## Janela pontual {#one-off-window}

| Campo | Significado |
|---|---|
| Nome | Mostrado na lista e no histórico. |
| Monitor | Deixe vazio para cobrir todos os monitores. |
| Início, Fim | Horários absolutos. A janela inclui o início e exclui o fim. |
| Comentário | Opcional. Por que a janela existe. |

Janelas pontuais são removidas assim que terminam.

## Janela semanal {#weekly-window}

| Campo | Significado |
|---|---|
| Dias | Quais dias da semana (segunda a domingo). |
| De, Até | Hora local no fuso horário da janela. Se *Até* for igual a ou
anterior a *De*, a janela ultrapassa a meia-noite ("23:00 → 01:00" pertence
ao dia em que começa). |
| Fuso horário | Um nome IANA como `Europe/Paris`. Pré-preenchido com o fuso
do seu navegador. |

## Janela mensal {#monthly-window}

| Campo | Significado |
|---|---|
| Em um dia da semana | *Primeiro*, *Segundo*, …, *Quinto* ou *Último*, mais o
dia — "o primeiro domingo", "a última sexta". |
| Em uma data | Um ou mais dias do mês, de 1 a 31, separados por vírgulas
("1, 15"). |
| De | Hora local de início, no fuso horário da janela. |
| Por | Quanto tempo a janela dura, como tempo real transcorrido. Uma janela
mensal pode durar mais que um dia. |
| Fuso horário | Como acima. |

Um mês que não contém a ocorrência é pulado em vez de aproximado: não há
dia 31 em fevereiro, e nenhum quinto domingo em abril de 2026, então a janela
simplesmente não abre naquele mês. A lista mostra a próxima ocorrência que o
servidor calculou, para que você possa verificá-la rapidamente.

O DumbMonit não tem expressões cron, de propósito: uma expressão cron nomeia
*instantes*, e uma janela de manutenção precisa de uma *duração*. Dias do mês
e "o n-ésimo dia da semana" cobrem os calendários que as pessoas realmente
mantêm, e se leem de volta em uma linha.

## Fusos horários e horário de verão {#time-zones-and-daylight-saving}

Uma janela recorrente abre em sua hora local do relógio de parede e então
roda por sua duração em tempo real. Ambas as partes importam:

- **A hora local não desvia.** "Todo domingo às 02:00 em `Europe/Paris`" é
  01:00 UTC no inverno e 00:00 UTC no verão — a janela segue a mudança do
  relógio em vez de deslizar uma hora duas vezes por ano.
- **A duração não se estica.** Na noite em que o relógio volta, 02:00 às
  04:00 no relógio durariam três horas reais; a janela ainda dura duas. Na
  noite em que o relógio avança e 02:00 não existe, a janela abre assim que
  o relógio retoma.

Uma janela salva antes dos fusos horários existirem mantém seu deslocamento
UTC fixo e se comporta exatamente como antes. Salve-a novamente com um fuso
para que ela siga as mudanças do relógio. Um nome de fuso que o servidor não
conhece é recusado ao salvar, com um exemplo.

## Correspondências {#matchers}

Uma janela também pode ser limitada por correspondências exatas de rótulos
(`matchers` na API): todas precisam corresponder aos rótulos do alerta, por
exemplo `tag_role = lab` e `__name__ = dumbmonit_up`. Uma janela sem
monitor e sem correspondência silencia toda a instância. Janelas que se
sobrepõem funcionam bem; a primeira que cobre o momento explica o silêncio.

## Como um alerta silenciado aparece {#what-a-silenced-alert-looks-like}

O alerta mantém sua fase e mostra `silenced: true`; o histórico registra a
transição com `notified: false` e o motivo. Desativar uma janela
(`enabled: false`) a mantém na lista sem efeito.

## Em uma página de status pública {#on-a-public-status-page}

Um monitor coberto por uma janela de manutenção é mostrado como
**Manutenção** em qualquer [página de status](../using/status-pages.md) que o
liste, em vez de vermelho. Se nada mais na página estiver fora do ar ou
degradado, o estado geral da página também lê *Manutenção*. Apenas janelas
que nomeiam um monitor — ou cujas correspondências são rótulos no nível
do monitor — aparecem desse jeito; uma janela limitada a uma métrica
permanece uma questão de alerta.
