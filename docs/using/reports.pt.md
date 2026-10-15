# Relatórios por email {#email-reports}

Um relatório é um email curto que diz à sua equipa como a infraestrutura se
comportou no último dia, semana ou mês, sem que ninguém precise de abrir o
DumbMonit. É semanal por predefinição; diário e mensal estão a um clique de
distância. Os relatórios reutilizam um **canal de notificação por email
(SMTP)**, por isso configure um primeiro (ver
[Canais de notificação](../notifications.md)).

Abra **Settings > Reports** para os configurar. Tudo aqui é só para
administradores, incluindo a leitura, porque a página lista os endereços dos
destinatários.

## O que um relatório contém {#what-a-report-contains}

| Secção | Conteúdo |
| --- | --- |
| **Título** | Disponibilidade global do período, número de incidentes e a variação em relação ao período anterior. |
| **Disponibilidade por grupo** | Uma linha por grupo, depois por monitor (no máximo os 40 menos disponíveis), calculada da mesma forma que nas [páginas de estado](status-pages.md): sondas pela sua taxa de sucesso, anfitriões e monitores pela frequência com que reportaram. Monitores sem histórico são mostrados como «sem dados» em vez de 100%. |
| **Incidentes** | Quando cada um começou, quanto tempo durou (ou que ainda está aberto) e em que monitor. No máximo 25. Os alertas informativos são deixados de fora. |
| **Monitores menos estáveis** | Os cinco monitores com a disponibilidade mais baixa ou mais incidentes. |
| **Comparação** | Disponibilidade e número de incidentes face ao período de igual duração imediatamente anterior. |

O email é feito de tabelas HTML simples com estilos em linha e sem imagem
remota, pelo que é apresentado da mesma forma em qualquer lugar e nada é
obtido quando é aberto. Uma versão em texto é enviada em conjunto. Todos os
nomes de monitor e mensagens são escapados.

## Definições {#settings}

| Definição | O que faz |
| --- | --- |
| **Name** | Mostrado apenas na lista. |
| **Enabled** | Desativado mantém as definições sem enviar nada. |
| **Frequency** | Diária, semanal (predefinição) ou mensal. |
| **Day** | O dia da semana para relatórios semanais, o dia do mês (1 a 28) para os mensais. |
| **Hour** e **Timezone** | Quando o relatório é enviado, num fuso horário IANA como `Europe/Lisbon`. A hora de verão é seguida. |
| **Recipients** | Até 20 endereços, em formato livre: não têm de ser utilizadores do DumbMonit. Cada destinatário recebe o seu próprio email, para que ninguém veja os outros. |
| **Email channel** | Qual canal SMTP o envia. Por predefinição, o primeiro ativado. Só o servidor de email do canal é usado: os seus próprios destinatários são ignorados. |

O período é uma janela deslizante que termina quando o relatório é enviado: as
últimas 24 horas, 7 dias ou 30 dias. O período anterior é a janela de igual
duração imediatamente anterior.

Clique no **i** junto ao título da secção para um resumo do conteúdo.

## Enviar uma pré-visualização {#send-a-preview}

**Send a preview** envia por email o relatório atual aos destinatários
guardados, de imediato, com `[Preview]` no assunto. Não altera nada no
calendário: o próximo relatório ainda é enviado a tempo. Uma pré-visualização
pode ser enviada uma vez a cada 30 segundos por relatório. Guarde primeiro as
suas alterações, já que a pré-visualização usa os destinatários guardados.

## Garantias de entrega {#delivery-guarantees}

- **No máximo uma vez.** A vaga é reservada na base de dados antes de o email
  ser construído, pelo que um reinício nunca envia o mesmo relatório duas
  vezes. Quando o envio falha, o erro é guardado no relatório e mostrado na
  lista, e a vaga não é repetida; a seguinte é enviada normalmente.
- **Sem recuperação.** Se o DumbMonit estava em baixo na hora agendada, o
  relatório é enviado quando volta apenas se isso ocorrer dentro de 3 horas da
  vaga. Mais tarde do que isso, espera pela vaga seguinte. Um relatório que
  cria ou reativa nunca envia uma vaga já passada.
- Podem existir no máximo 10 relatórios.
- No modo de demonstração nada é enviado.

Os relatórios estão incluídos nas [cópias de segurança](../install/backup.md):
o canal de email é associado pelo nome no restauro, e um relatório restaurado
começa do zero (a sua data do último envio não é transportada).

## API {#api}

Ver [API HTTP](../reference/api.md#email-reports).
