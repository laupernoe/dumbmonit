# Reputação do servidor de correio {#mail-server-reputation}

Um domínio de correio, ou o endereço público de um servidor de correio,
verificado contra as listas de bloqueio DNS que ainda decidem se o correio é
entregue: Spamhaus, Barracuda, SpamCop e algumas mais pequenas. Por cada lista
que lista um endereço, a página do monitor dá o código de resposta da lista, o
motivo publicado e uma ligação direta para a sua página de consulta e remoção.

Introduza um domínio (`example.com`) e o DumbMonit verifica os endereços dos
seus anfitriões MX (registos A e AAAA), os endereços do próprio domínio, e
quaisquer **Extra addresses** que acrescente (um relay de saída que não é um
MX). O domínio é também verificado contra as listas de bloqueio de domínios.
Introduza um endereço IP e só esse endereço é verificado. Para vigiar todo um
intervalo de endereços, use [Lista de bloqueio de intervalo de IP](ip_blacklist.md).

## Gmail, Outlook.com e Yahoo {#gmail-outlook-com-and-yahoo}

Os grandes fornecedores de caixas de correio não publicam **nenhuma lista de
bloqueio que possa ser consultada**: o que pensam de um remetente é interno.
O DumbMonit não finge perguntar-lhes. Verifica o que pesa na decisão deles, e
liga às suas ferramentas:

- **DNS inverso**: cada endereço de envio deve ter um registo PTR cujo nome
  resolva de volta para o mesmo endereço (DNS inverso confirmado por
  resolução direta, FCrDNS). O Gmail, o Outlook.com e o Yahoo recusam ou
  enviam para lixo o correio de endereços que não o tenham.
- **SPF e DMARC**: exatamente um registo SPF no domínio, e um registo DMARC em
  `_dmarc.<domain>`. O Gmail e o Yahoo exigem ambos aos remetentes em massa.
- **As listas de bloqueio abaixo**, que os seus filtros também lêem.

O DKIM não é verificado: o seu seletor é escolhido por cada remetente e não
pode ser adivinhado. A página do monitor liga a:

| Fornecedor | Ferramentas |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/) (reputação de um domínio verificado), [diretrizes para remetentes](https://support.google.com/a/answer/81126), [formulário de problemas de entrega](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/) (dados de reputação por endereço), [políticas para remetentes](https://sendersupport.olc.protection.outlook.com/pm/), [formulário de apoio a remetentes](https://olcsupport.office.com/) (remoção de listagem no Outlook.com), [portal de remoção do Microsoft 365](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/), [boas práticas](https://senders.yahooinc.com/best-practices/), [formulário de apoio a remetentes](https://senders.yahooinc.com/contact/) |

## As listas de bloqueio {#the-blocklists}

Verificadas a 16 de outubro de 2026. A SORBS fechou em junho de 2024 e não é
consultada. A cada lista é pedido um registo `A` sob a sua zona; uma lista é
identificada pelo id da primeira coluna, que a opção **Lists** aceita para
restringir a verificação.

| Id | Lista | Zona | Verifica | IPv6 | Principal | Uso e remoção |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | endereços | sim | sim | Espelhos públicos gratuitos para baixo volume e uso não comercial; recusa resolvedores públicos e abertos. Remoção no [verificador da Spamhaus](https://check.spamhaus.org/). A PBL lista intervalos de utilizadores finais por política: só remova um endereço dela se ele executar realmente um servidor de correio. |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | endereços | não | sim | Gratuita; a Barracuda pede que os servidores DNS que a consultam estejam registados numa conta gratuita em barracudacentral.org. [Pedido de remoção](https://www.barracudacentral.org/rbl/removal-request). |
| `spamcop` | SpamCop | `bl.spamcop.net` | endereços | não | sim | Gratuita. As listagens expiram sozinhas cerca de um dia depois de as denúncias cessarem; a [página de consulta](https://www.spamcop.net/bl.shtml) mostra o motivo e oferece a remoção. |
| `mailspike` | Mailspike | `bl.mailspike.net` | endereços | não | não | Gratuita a baixo volume. [Consulta e remoção](https://mailspike.io/ip_verify). |
| `uceprotect_1` | UCEPROTECT Level 1 | `dnsbl-1.uceprotect.net` | endereços | não | não | Consulta gratuita. As listagens expiram após sete dias sem novo spam; a [página de verificação](https://www.uceprotect.net/en/rblcheck.php) também vende remoção expresso, que nunca é necessária. Poucos destinatários a usam. |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | endereços | não | não | Gratuita. Remoção em autosserviço na [página de listagem](https://psbl.org/). |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | endereços | não | não | Gratuita a baixo volume. A RATS-NoPtr lista endereços sem DNS inverso. [Consulta](https://www.spamrats.com/lookup.php) e [remoção](https://www.spamrats.com/removal.php). |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | o domínio | — | sim | Mesmos termos que a ZEN. Remoção no [verificador da Spamhaus](https://check.spamhaus.org/). |
| `surbl` | SURBL (multi) | `multi.surbl.org` | o domínio | — | não | Gratuita a baixo volume; recusa resolvedores públicos e ocupados. [Consulta e remoção](https://www.surbl.org/surbl-analysis). |
| `uribl` | URIBL (multi) | `multi.uribl.com` | o domínio | — | não | Gratuita a baixo volume; recusa resolvedores públicos e ocupados. [Consulta](https://lookup.uribl.com/), [remoção](https://admin.uribl.com/). |

Uma listagem numa lista **principal** dispara um alerta crítico; uma nas
outras listas, um alerta de aviso. Os endereços IPv6 do MX são verificados
apenas contra a Spamhaus ZEN, a única lista aqui que responde para IPv6; as
outras mostram-nos como não cobertos.

## Resolvedores, recusas e tempos limite {#resolvers-refusals-and-timeouts}

As listas são consultadas por DNS, e a forma como respondem depende de **que
resolvedor pergunta**:

- **A Spamhaus recusa consultas que chegam por resolvedores públicos** (Google
  `8.8.8.8`, Cloudflare `1.1.1.1`, Quad9), por resolvedores que reencaminham
  para eles, ou por resolvedores sem DNS inverso atribuível. Responde então
  `127.255.255.254` em vez de `NXDOMAIN` (`127.255.255.255` quando um
  resolvedor envia demasiadas consultas, `127.255.255.252` para uma consulta
  malformada ou uma chave DQS errada). Isto parece listagens e não é: o
  DumbMonit mostra a verificação como **erro de configuração**, nunca como
  «listado», e não envia alerta de indisponibilidade.
- **A URIBL e a SURBL respondem `127.0.0.1`** aos resolvedores que recusam.
  Essa lista é então mostrada como «Refused» (desconhecida), não como
  listagem.
- **Uma lista que não responde a tempo é desconhecida, nunca limpa.** Só
  `NXDOMAIN` significa «não listado». Uma resposta fora de `127.0.0.0/8` (um
  resolvedor que reescreve `NXDOMAIN` no endereço de uma página de pesquisa) é
  também desconhecida.

Por isso use **um resolvedor recursivo seu**: Unbound, Knot Resolver, BIND ou
PowerDNS Recursor a resolver a partir dos servidores raiz (sem reencaminhar
para um resolvedor público), e defina o seu endereço em **Resolver** se não
for o resolvedor do sistema. Pi-hole ou AdGuard Home à frente do Unbound
funciona; à frente de `1.1.1.1` não.

Ou crie uma conta gratuita do [Spamhaus Data Query Service](https://www.spamhaus.com/data-access/free-data-query-service/)
(uso não comercial, baixo volume) e introduza a sua chave como credencial: a
Spamhaus é então consultada em `<key>.zen.dq.spamhaus.net` e
`<key>.dbl.dq.spamhaus.net`, que respondem através de qualquer resolvedor. A
chave é guardada cifrada e nunca é mostrada de novo.

## Ritmo {#pace}

As listas de bloqueio limitam a frequência com que um resolvedor pode
perguntar, e banem os resolvedores que as varrem. O DumbMonit pergunta a cada
lista uma vez por **Refresh interval** (6 horas por predefinição), no máximo
**Queries per second** (10 por predefinição) com oito consultas em simultâneo,
e mostra de novo o último resultado a cada verificação intermédia, para que as
regras de alerta vejam uma série contínua. Uma verificação decorre em segundo
plano: o primeiro resultado aparece alguns segundos depois de o monitor ser
acrescentado, e mudar o endereço, uma opção ou a chave inicia logo uma nova
verificação. Uma verificação cujo resultado é um erro é repetida passados dez
minutos.

## Verificar um servidor de correio contra as listas de bloqueio {#check-a-mail-server-against-the-blocklists}

1. Introduza o domínio para o qual envia correio, por exemplo "example.com": o DumbMonit verifica os endereços do seu MX e do próprio domínio. Ou introduza o endereço IP público de onde o servidor envia. Se o correio sai por um relay que não é um MX, acrescente o seu endereço em Extra addresses.

2. Use um resolvedor recursivo seu. A Spamhaus recusa consultas que chegam por resolvedores públicos (Google, Cloudflare, Quad9) ou por um resolvedor que reencaminha para eles, e outras listas limitam-nas: um Unbound que resolva a partir dos servidores raiz chega. Defina o seu endereço em Resolver se não for o resolvedor do sistema.

3. Ou crie uma conta gratuita do Spamhaus Data Query Service (uso não comercial, baixo volume) e cole a sua chave como credencial: a Spamhaus responde então através de qualquer resolvedor.

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

4. Para verificar a configuração, acrescente um monitor para 127.0.0.2, o endereço de teste que todas as listas de bloqueio listam: cada uma deve mostrá-lo como listado.

5. O Gmail, o Outlook.com e o Yahoo não publicam nenhuma lista de bloqueio que possa ser consultada. O DumbMonit verifica antes o que eles ponderam (um DNS inverso que aponta de volta para o endereço, registos SPF e DMARC) e liga às suas ferramentas para remetentes na página do monitor.

!!! warning
    Uma resposta que a Spamhaus recusou (127.255.255.254, um resolvedor público) é mostrada como erro de configuração, nunca como listagem, e uma lista de bloqueio que não responde é mostrada como desconhecida, nunca como limpa. As listas são consultadas de 6 em 6 horas por predefinição: perguntar com mais frequência não ganha nada e leva ao banimento dos resolvedores.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_blacklist_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `listed_major_lists` | listas principais (Spamhaus ZEN e DBL, Barracuda, SpamCop) que listam um endereço ou o domínio | |
| `listed_lists` | todas as listas que listam um endereço ou o domínio | |
| `addresses_checked` | endereços verificados | |
| `addresses_listed` | endereços listados em pelo menos uma lista | |
| `domain_listed` | 1 quando o domínio está numa lista de bloqueio de domínios | |
| `unknown_checks` | verificações sem resposta (tempo limite, recusa) | |
| `check` | uma série por endereço (ou domínio) e lista: 1 listado, 0 limpo, −1 sem resposta, −2 recusado, −3 não coberto | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | por lista: sujeitos consultados, listados, sem resposta | `list` |
| `address_info` | valor 1, de onde vem cada endereço | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 1 quando o DNS inverso resolve de volta para o endereço | `address`, `ptr` |
| `mx_hosts` | anfitriões MX do domínio | |
| `spf_records` | registos `v=spf1` no domínio (1 é o correto) | |
| `dmarc_records` | registos `v=DMARC1` em `_dmarc.<domain>` | |
| `dmarc_policy_info` | valor 1 | `policy` |
| `mail_auth_missing` | 0 a 2: SPF em falta ou duplicado, DMARC em falta | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | a verificação mostrada: a sua idade, quanto demorou, se está a decorrer uma nova | |

As [regras integradas](../alerting/rules.md#dns-blocklists) que se aplicam:
Listed on a major blocklist, Listed on a blocklist, Mail server reverse DNS
mismatch, SPF or DMARC missing, mais Monitor unreachable quando o resolvedor
não responde a nenhuma das consultas.

## A página do monitor {#the-monitor-page}

O painel lê o que a sondagem guardou; abrir a página nunca consulta uma lista
de bloqueio. Mostra as contagens, depois cada lista com quantos sujeitos lista,
depois cada resultado com o código da lista, o motivo publicado e ligações
para as suas páginas de consulta e remoção, depois o DNS inverso de cada
endereço, SPF e DMARC, e as ferramentas para remetentes do Gmail, Outlook.com
e Yahoo.

## Credenciais {#credentials}

Nenhuma, ou uma chave Spamhaus DQS (veja acima).

Endereço: um domínio (`example.com`) ou um endereço IP público. Opções:
Resolver, Lists (todas por predefinição), Extra addresses, Refresh interval (6
horas, de 1 a 168), Queries per second (10, de 1 a 50) e Timeout per query (5
s, de 1 a 30). Os endereços privados nunca são verificados: as listas de
bloqueio só conhecem a Internet pública. `127.0.0.2` é aceite como endereço de
teste.
