# Relatórios por e-mail {#email-reports}

Um relatório é um e-mail curto que conta à sua equipe como a infraestrutura se
comportou no último dia, semana ou mês, sem que ninguém precise abrir o
DumbMonit. Por padrão é semanal; diário e mensal estão a um clique de
distância. Os relatórios reaproveitam um **canal de notificação por e-mail
(SMTP)**, então configure um primeiro (veja
[Canais de notificação](../notifications.md)).

Abra **Configurações > Relatórios** para configurá-los. Tudo aqui é restrito a
administradores, incluindo a leitura, porque a página lista os endereços dos
destinatários.

## O que um relatório contém {#what-a-report-contains}

| Seção | Conteúdo |
| --- | --- |
| **Resumo** | Disponibilidade geral do período, o número de incidentes, e a variação em relação ao período anterior. |
| **Disponibilidade por grupo** | Uma linha por grupo, depois por monitor (no máximo os 40 menos disponíveis), calculada da mesma forma que nas [páginas de status](status-pages.md): sondagens pela taxa de sucesso, hosts e monitores por quão frequentemente reportaram. Monitores sem histórico são mostrados como "sem dados" em vez de 100%. |
| **Incidentes** | Quando cada um começou, quanto tempo durou (ou que ainda está em aberto) e em qual monitor. Até 25. Alertas informativos são deixados de fora. |
| **Monitores menos estáveis** | Os cinco monitores com a menor disponibilidade ou o maior número de incidentes. |
| **Comparação** | Disponibilidade e número de incidentes em relação ao período de mesma duração imediatamente anterior. |

O e-mail é feito de tabelas HTML simples com estilos inline e sem nenhuma
imagem remota, então é renderizado da mesma forma em qualquer lugar e nada é
buscado quando é aberto. Uma versão em texto é enviada junto. Todo nome de
monitor e mensagem é escapado.

## Configurações {#settings}

| Configuração | O que faz |
| --- | --- |
| **Nome** | Mostrado apenas na lista. |
| **Ativado** | Desligado mantém as configurações sem enviar nada. |
| **Frequência** | Diária, semanal (padrão) ou mensal. |
| **Dia** | O dia da semana para relatórios semanais, o dia do mês (1 a 28) para os mensais. |
| **Hora** e **Fuso horário** | Quando o relatório é enviado, em um fuso horário IANA como `Europe/Paris`. O horário de verão é seguido. |
| **Destinatários** | Até 20 endereços, de forma livre: não precisam ser usuários do DumbMonit. Cada destinatário recebe seu próprio e-mail, então ninguém vê os outros. |
| **Canal de e-mail** | Qual canal SMTP o envia. Por padrão, o primeiro ativado. Apenas o servidor de e-mail do canal é usado: seus próprios destinatários são ignorados. |

O período é uma janela móvel que termina quando o relatório é enviado: as
últimas 24 horas, 7 dias ou 30 dias. O período anterior é a janela de mesma
duração imediatamente antes.

Clique no **i** ao lado do título da seção para um lembrete do conteúdo.

## Enviar uma pré-visualização {#send-a-preview}

**Enviar pré-visualização** envia por e-mail o relatório atual aos
destinatários salvos agora mesmo, com `[Preview]` no assunto. Isso não muda
nada na programação: o próximo relatório ainda sai no horário certo. Uma
pré-visualização pode ser enviada uma vez a cada 30 segundos por relatório.
Salve suas alterações primeiro, já que a pré-visualização usa os destinatários
salvos.

## Garantias de entrega {#delivery-guarantees}

- **No máximo uma vez.** O horário é reservado no banco de dados antes do
  e-mail ser montado, então um reinício nunca envia o mesmo relatório duas
  vezes. Quando o envio falha, o erro fica registrado no relatório e é
  mostrado na lista, e o horário não é repetido: o próximo sai normalmente.
- **Sem recuperação atrasada.** Se o DumbMonit estava fora do ar no horário
  programado, o relatório é enviado quando ele volta somente se isso estiver
  dentro de 3 horas do horário previsto. Depois disso, ele espera o próximo
  horário. Um relatório que você cria ou reativa nunca envia um horário que já
  passou.
- No máximo 10 relatórios podem existir.
- No modo demo nada é enviado.

Os relatórios estão incluídos nos [backups](../install/backup.md): o canal de
e-mail é correspondido pelo nome na restauração, e um relatório restaurado
começa do zero (sua data do último envio não é preservada).

## API {#api}

Veja [API HTTP](../reference/api.md#email-reports).
