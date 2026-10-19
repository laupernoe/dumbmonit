# Reputação do servidor de e-mail {#mail-server-reputation}

Um domínio de e-mail, ou o endereço público de um servidor de e-mail,
verificado contra as listas de bloqueio DNS que ainda decidem se um e-mail
é entregue: Spamhaus, Barracuda, SpamCop e algumas menores. Para cada lista
que registra um endereço, a página do monitor mostra o código de resposta da
lista, o motivo que ela publica e um link direto para sua página de consulta e
remoção.

**Nenhuma chave de API é necessária.** Deixe a credencial em "Sem chave (recomendado)":
as listas são consultadas pelo resolvedor DNS do servidor, ou pelo resolvedor que
você definir em **Resolvedor**. Só o Spamhaus pode recusar, e só quando esse
resolvedor é público; veja [Resolvedores, recusas e timeouts](#resolvers-refusals-and-timeouts).

Informe um domínio (`example.com`) e o DumbMonit verifica os endereços dos
seus hosts MX (registros A e AAAA), os endereços do próprio domínio e os
**Endereços extras** que você adicionar (um relay de saída que não é um MX).
O domínio também é verificado contra as listas de bloqueio de domínio. Informe
um endereço IP e só esse endereço é verificado. Para vigiar uma faixa inteira
de endereços, use [Lista de bloqueio de faixa de IP](ip_blacklist.md).

## Gmail, Outlook.com e Yahoo {#gmail-outlook-com-and-yahoo}

Os grandes provedores de caixa de correio **não publicam nenhuma lista de
bloqueio que possa ser consultada**: o que pensam de um remetente é interno a
eles. O DumbMonit não finge perguntar a eles. Ele verifica o que pesa na
decisão deles e aponta para suas ferramentas:

- **DNS reverso**: cada endereço de envio deve ter um registro PTR cujo nome
  resolva de volta para o mesmo endereço (DNS reverso confirmado, FCrDNS).
  Gmail, Outlook.com e Yahoo recusam ou mandam para o spam e-mails de
  endereços sem isso.
- **SPF e DMARC**: exatamente um registro SPF no domínio e um registro DMARC
  em `_dmarc.<domínio>`. Gmail e Yahoo exigem ambos de remetentes em massa.
- **As listas de bloqueio abaixo**, que os filtros deles também leem.

O DKIM não é verificado: o seletor é escolhido por cada remetente e não pode
ser adivinhado. A página do monitor traz links para:

| Provedor | Ferramentas |
|---|---|
| Gmail | [Postmaster Tools](https://postmaster.google.com/) (reputação de um domínio verificado), [diretrizes para remetentes](https://support.google.com/a/answer/81126), [formulário de problemas de entrega](https://support.google.com/mail/contact/gmail_bulk_sender_escalation) |
| Outlook.com / Hotmail | [SNDS](https://sendersupport.olc.protection.outlook.com/snds/) (dados de reputação por endereço), [políticas para remetentes](https://sendersupport.olc.protection.outlook.com/pm/), [formulário de suporte ao remetente](https://olcsupport.office.com/) (remoção de listas do Outlook.com), [portal de remoção do Microsoft 365](https://sender.office.com/) |
| Yahoo / AOL | [Sender Hub](https://senders.yahooinc.com/), [boas práticas](https://senders.yahooinc.com/best-practices/), [formulário de suporte ao remetente](https://senders.yahooinc.com/contact/) |

## As listas de bloqueio {#the-blocklists}

Verificadas em 16 de outubro de 2026. O SORBS fechou em junho de 2024 e não é
consultado. Cada lista é consultada por um registro `A` sob sua zona. O formulário
as mostra como uma lista de caixas chamada **Listas de bloqueio** (nome, se verifica
endereços ou o domínio, principal ou não, e uma breve descrição); as listas
**principais** vêm marcadas em um monitor novo. O id da primeira coluna é o que a
opção `lists` guarda e o nome que a API e as métricas dão à lista.

| Id | Lista | Zona | Verifica | IPv6 | Principal | Uso e remoção |
|---|---|---|---|---|---|---|
| `spamhaus_zen` | Spamhaus ZEN (SBL, CSS, XBL, PBL, DROP) | `zen.spamhaus.org` | endereços | sim | sim | Espelhos públicos gratuitos para baixo volume e uso não comercial; recusa resolvedores públicos e abertos. Remoção pelo [verificador do Spamhaus](https://check.spamhaus.org/). O PBL lista por política faixas de usuários finais: só remova um endereço dele se ele realmente roda um servidor de e-mail. |
| `barracuda` | Barracuda BRBL | `b.barracudacentral.org` | endereços | não | sim | Gratuita; a Barracuda pede que os servidores DNS que a consultam sejam registrados com uma conta gratuita em barracudacentral.org. [Pedido de remoção](https://www.barracudacentral.org/rbl/removal-request). |
| `spamcop` | SpamCop | `bl.spamcop.net` | endereços | não | sim | Gratuita. As listagens expiram sozinhas cerca de um dia depois que as denúncias param; a [página de consulta](https://www.spamcop.net/bl.shtml) mostra o motivo e oferece a remoção. |
| `mailspike` | Mailspike | `bl.mailspike.net` | endereços | não | não | Gratuita em baixo volume. [Consulta e remoção](https://mailspike.io/ip_verify). |
| `uceprotect_1` | UCEPROTECT Level 1 | `dnsbl-1.uceprotect.net` | endereços | não | não | Consulta gratuita. As listagens expiram após sete dias sem novo spam; a [página de verificação](https://www.uceprotect.net/en/rblcheck.php) também vende remoção expressa, que nunca é necessária. Poucos destinatários a usam. |
| `psbl` | PSBL (Passive Spam Block List) | `psbl.surriel.com` | endereços | não | não | Gratuita. Remoção por conta própria na [página de listagem](https://psbl.org/). |
| `spamrats` | SpamRATS (Dyna, NoPtr, Spam, Auth) | `all.spamrats.com` | endereços | não | não | Gratuita em baixo volume. O RATS-NoPtr lista endereços sem DNS reverso. [Consulta](https://www.spamrats.com/lookup.php) e [remoção](https://www.spamrats.com/removal.php). |
| `spamhaus_dbl` | Spamhaus DBL | `dbl.spamhaus.org` | o domínio | — | sim | Mesmos termos do ZEN. Remoção pelo [verificador do Spamhaus](https://check.spamhaus.org/). |
| `surbl` | SURBL (multi) | `multi.surbl.org` | o domínio | — | não | Gratuita em baixo volume; recusa resolvedores públicos e ocupados. [Consulta e remoção](https://www.surbl.org/surbl-analysis). |
| `uribl` | URIBL (multi) | `multi.uribl.com` | o domínio | — | não | Gratuita em baixo volume; recusa resolvedores públicos e ocupados. [Consulta](https://lookup.uribl.com/), [remoção](https://admin.uribl.com/). |

Uma listagem em uma lista **principal** dispara um alerta crítico; em uma das
outras, um aviso. Os endereços IPv6 do MX são verificados somente contra o
Spamhaus ZEN, a única lista aqui que responde para IPv6; as outras os mostram
como não cobertos.

### Suas próprias listas {#your-own-lists}

Em **Listas personalizadas**, no fim da lista de caixas, adicione as zonas DNSBL de
qualquer outra lista que você use ou que você mesmo mantenha (por exemplo
`dnsbl.example.org`). Cada uma tem um **Tipo**: `ip` (a zona é consultada para cada
endereço, como `1.2.0.192.dnsbl.example.org`) ou `domain` (para o domínio de e-mail),
e uma chave **Principal** para uma lista cuja listagem deve disparar o alerta
crítico. A zona precisa ser um nome de domínio válido, não um endereço IP, e não
pode repetir outra zona nem uma das listas acima; no máximo 20 listas
personalizadas. Qualquer resposta `127.x.x.x` conta como listado; `NXDOMAIN`, como
limpo.

O monitor consulta exatamente as listas marcadas mais as suas. Um monitor criado
antes dessas opções não tem nenhum ajuste e continua consultando todas as listas
acima. A opção `lists` guarda os ids marcados separados por vírgulas (`-` quando
nenhum está marcado) e `custom_lists` um array JSON como
`[{"zone": "dnsbl.example.org", "subject": "ip", "major": false}]`.

## Resolvedores, recusas e timeouts {#resolvers-refusals-and-timeouts}

As listas são consultadas por DNS, e a resposta delas depende de **qual
resolvedor pergunta**:

- **O Spamhaus recusa consultas que passam por resolvedores públicos** (Google
  `8.8.8.8`, Cloudflare `1.1.1.1`, Quad9), por resolvedores que encaminham
  para eles ou por resolvedores sem DNS reverso atribuível. Ele então responde
  `127.255.255.254` em vez de `NXDOMAIN` (`127.255.255.255` quando um
  resolvedor envia consultas demais, `127.255.255.252` para uma consulta mal
  formada ou uma chave DQS errada). Isso se parece com listagens, mas não é: o
  DumbMonit marca a lista do Spamhaus como **recusada** ("O Spamhaus recusa este
  resolvedor" na página do monitor), nunca como "listado", e as outras listas
  mantêm seus resultados. Só quando nenhuma outra lista selecionada respondeu é que
  o monitor mostra um **erro de configuração** (que não envia alerta de
  indisponibilidade) em vez de um resultado.
- **URIBL e SURBL respondem `127.0.0.1`** aos resolvedores que recusam. Essa
  lista é então mostrada como "Recusada" (desconhecido), não como listagem.
- **Uma lista que não responde a tempo é desconhecida, nunca limpa.** Só
  `NXDOMAIN` significa "não listado". Uma resposta fora de `127.0.0.0/8` (um
  resolvedor que reescreve `NXDOMAIN` no endereço de uma página de busca) também
  é desconhecida.

Então use **um resolvedor recursivo seu**: Unbound, Knot Resolver, BIND ou
PowerDNS Recursor resolvendo a partir dos servidores raiz (sem encaminhar para
um resolvedor público), e defina o endereço dele em **Resolvedor** se não for o
resolvedor do sistema. Pi-hole ou AdGuard Home na frente de um Unbound
funciona; na frente de `1.1.1.1`, não.

Ou, só se você quiser também o Spamhaus e não puder trocar de resolvedor, crie uma
conta gratuita no [Spamhaus Data Query Service](https://www.spamhaus.com/data-access/free-data-query-service/)
(uso não comercial, baixo volume) e informe a chave dela como credencial ("Chave
Spamhaus DQS (opcional)"): o Spamhaus é então consultado em
`<chave>.zen.dq.spamhaus.net` e `<chave>.dbl.dq.spamhaus.net`, que respondem por
qualquer resolvedor. A chave é guardada criptografada e nunca mais é exibida.

## Ritmo {#pace}

As listas de bloqueio limitam a frequência com que um resolvedor pode
consultá-las e banem resolvedores que as varrem. O DumbMonit consulta cada
lista uma vez por **Intervalo de atualização** (6 horas por padrão), no máximo
**Consultas por segundo** (10 por padrão) com oito consultas em andamento, e
mostra o último resultado de novo a cada verificação no meio-tempo, de modo que
as regras de alerta veem uma série contínua. A verificação roda em segundo
plano: o primeiro resultado aparece alguns segundos depois de o monitor ser
adicionado, e mudar o endereço, uma opção ou a chave inicia uma nova
verificação na hora. Uma verificação cujo resultado é um erro é repetida após
dez minutos.

## Verificar um servidor de e-mail contra as listas de bloqueio {#check-a-mail-server-against-the-blocklists}

1. Informe o domínio para o qual você envia e-mail, por exemplo "example.com": o DumbMonit verifica os endereços do MX e do próprio domínio. Ou informe o endereço IP público de onde o servidor envia. Se o e-mail sai por um relay que não é um MX, adicione o endereço dele em Endereços extras.

2. Nenhuma chave de API é necessária: deixe a credencial em Sem chave. Sem chave, as listas de bloqueio são consultadas pelo resolvedor DNS deste servidor, ou pelo que você definir em Resolvedor.

3. Marque em Listas de bloqueio as listas a consultar (as principais vêm marcadas por padrão) e adicione em Listas personalizadas as zonas DNSBL de qualquer outra lista que você use.

4. Só o Spamhaus pode recusar uma consulta, e só quando o resolvedor é público (Google, Cloudflare, Quad9) ou encaminha para um deles. O monitor mostra então "O Spamhaus recusa este resolvedor" e todas as outras listas continuam funcionando. Para ter também o Spamhaus, defina Resolvedor como um resolvedor recursivo seu (um Unbound que resolve a partir dos servidores raiz basta) ou escolha como credencial a chave Spamhaus DQS opcional: uma conta gratuita (uso não comercial, baixo volume) cuja chave faz o Spamhaus responder por qualquer resolvedor.

    ```
    https://www.spamhaus.com/data-access/free-data-query-service/
    ```

5. Para verificar a configuração, adicione um monitor para 127.0.0.2, o endereço de teste que toda lista de bloqueio registra: cada uma deve mostrá-lo como listado.

6. Gmail, Outlook.com e Yahoo não publicam nenhuma lista de bloqueio consultável. O DumbMonit verifica em vez disso o que eles ponderam (um DNS reverso que aponta de volta para o endereço, registros SPF e DMARC) e liga para as ferramentas de remetente deles na página do monitor.

!!! warning
    Uma resposta que o Spamhaus recusou (127.255.255.254, um resolvedor público) nunca é mostrada como listagem: essa lista fica marcada como recusada e as outras mantêm o resultado. Só quando nenhuma outra lista responde é que o monitor indica um erro de configuração. Uma lista de bloqueio que não responde é mostrada como desconhecida, nunca como limpa. As listas são consultadas a cada 6 horas por padrão: perguntar com mais frequência não adianta e faz os resolvedores serem banidos.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_blacklist_`.

| Métrica | O quê | Rótulos |
|---|---|---|
| `listed_major_lists` | listas principais (Spamhaus ZEN e DBL, Barracuda, SpamCop) que registram um endereço ou o domínio | |
| `listed_lists` | todas as listas que registram um endereço ou o domínio | |
| `addresses_checked` | endereços verificados | |
| `addresses_listed` | endereços listados em pelo menos uma lista | |
| `domain_listed` | 1 quando o domínio está em uma lista de bloqueio de domínio | |
| `unknown_checks` | verificações sem resposta (timeout, recusa) | |
| `check` | uma série por endereço (ou domínio) e lista: 1 listado, 0 limpo, −1 sem resposta, −2 recusado, −3 não coberto | `list`, `subject`, `state`, `code`, `meaning`, `reason` |
| `list_checked`, `list_listed`, `list_unknown` | por lista: assuntos consultados, listados, sem resposta | `list` |
| `list_info` | valor 1, uma por lista realmente consultada (inclusive as suas): o que a página do monitor mostra | `list`, `name`, `zone`, `subject`, `major` |
| `address_info` | valor 1, de onde vem cada endereço | `address`, `source` (`mx`, `domain`, `input`, `extra`), `mx` |
| `ptr_ok` | 1 quando o DNS reverso resolve de volta para o endereço | `address`, `ptr` |
| `mx_hosts` | hosts MX do domínio | |
| `spf_records` | registros `v=spf1` no domínio (1 é o correto) | |
| `dmarc_records` | registros `v=DMARC1` em `_dmarc.<domínio>` | |
| `dmarc_policy_info` | valor 1 | `policy` |
| `mail_auth_missing` | 0 a 2: SPF ausente ou duplicado, DMARC ausente | |
| `scan_age_seconds`, `scan_duration_seconds`, `scan_running` | a verificação exibida: sua idade, quanto levou, se uma nova está em andamento | |

As [regras integradas](../alerting/rules.md#dns-blocklists) que se aplicam:
Listed on a major blocklist, Listed on a blocklist, Mail server reverse DNS
mismatch, SPF or DMARC missing, além de Monitor unreachable quando o
resolvedor não responde a nenhuma das consultas.

## A página do monitor {#the-monitor-page}

O painel lê o que a sondagem guardou; abrir a página nunca consulta uma lista
de bloqueio. Ele mostra as contagens, depois as listas realmente consultadas (as
marcadas e as suas, cada uma com quantos assuntos ela registra e "O Spamhaus recusa
este resolvedor" quando for o caso), depois cada resultado com o código da lista, o
motivo que ela publica e links para suas páginas de consulta e remoção, depois o DNS
reverso de cada endereço, SPF e DMARC, e as ferramentas de remetente do Gmail,
Outlook.com e Yahoo.

A página também tem um link **Verificar também no MXToolbox**. Ele abre em uma nova
aba a consulta pública de listas de bloqueio do mxtoolbox.com para o endereço ou o
domínio monitorado; o DumbMonit não chama a API deles.

## Credenciais {#credentials}

Nenhuma necessária ("Sem chave (recomendado)", o padrão), ou opcionalmente uma
chave Spamhaus DQS (veja acima), que só importa para o Spamhaus.

Endereço: um domínio (`example.com`) ou um endereço IP público. Opções:
Resolvedor, Listas de bloqueio (as principais marcadas em um monitor novo; todas em
um criado antes), Listas personalizadas, Endereços extras, Intervalo de atualização
(6 horas, de 1 a 168), Consultas por segundo (10, de 1 a 50) e Timeout por consulta
(5 s, de 1 a 30). Endereços privados nunca são verificados: as listas de bloqueio só
conhecem a Internet pública. `127.0.0.2` é aceito como endereço de teste.
