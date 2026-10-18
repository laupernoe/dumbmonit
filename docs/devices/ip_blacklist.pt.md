# Lista de bloqueio de intervalo de IP {#ip-range-blocklist}

Todos os endereços de um intervalo IPv4 público, até um /24 (256 endereços),
verificados contra as listas de bloqueio DNS de [Reputação do servidor de correio](mail_blacklist.md#the-blocklists):
Spamhaus ZEN, Barracuda, SpamCop, Mailspike, UCEPROTECT Level 1, PSBL e
SpamRATS. A página do monitor diz quantos endereços estão listados, em que
listas, e dá uma ligação direta para a página de consulta e remoção de cada
listagem.

Útil para o intervalo público de um escritório, o bloco que um fornecedor de
alojamento lhe deu, ou os endereços de onde os seus servidores enviam: a
máquina comprometida de um vizinho obtém uma listagem que um destinatário mais
rigoroso pode estender a todo o intervalo.

## Limites {#limits}

- **No máximo um /24 por monitor.** Um intervalo maior é recusado com uma
  mensagem a indicar quantos endereços contém: divida-o em vários monitores
  /24. Cada endereço custa uma consulta a cada lista, e uma varredura de
  milhares de endereços parece abuso para elas.
- **Apenas IPv4.** Destas listas, só a Spamhaus ZEN responde para IPv6, e um
  intervalo IPv6 não pode ser percorrido endereço a endereço. Um único
  endereço IPv6 pode ser verificado com [Reputação do servidor de correio](mail_blacklist.md).
- **Apenas endereços públicos.** Um intervalo que inclua um endereço privado,
  partilhado (CGNAT) ou de documentação é recusado: as listas de bloqueio só
  conhecem a Internet pública.
- **As listas de domínio** (Spamhaus DBL, SURBL, URIBL) não se aplicam a um
  intervalo.

## Resolvedores e ritmo {#resolvers-and-pace}

Aplicam-se as mesmas regras de [Reputação do servidor de correio](mail_blacklist.md#resolvers-refusals-and-timeouts).
A Spamhaus recusa resolvedores públicos: a sua recusa (`127.255.255.254`) é
mostrada como erro de configuração, nunca como listagem. Uma lista que não
responde é desconhecida, nunca limpa. Use um resolvedor recursivo seu, ou uma
chave gratuita Spamhaus DQS como credencial.

Um /24 custa perto de duas mil consultas. São enviadas a **Queries per
second** (10 por predefinição), no máximo oito em simultâneo, e todo o
intervalo é verificado uma vez por **Refresh interval** (6 horas por
predefinição); o último resultado é mostrado de novo a cada verificação
intermédia. A primeira verificação de um intervalo demora alguns minutos e
decorre em segundo plano.

## Verificar um intervalo de IP público contra as listas de bloqueio {#check-a-public-ip-range-against-the-blocklists}

1. Introduza um endereço IPv4 ou um intervalo CIDR até /24, por exemplo "198.51.100.0/24" (256 endereços). Um intervalo maior é recusado: divida-o em vários monitores /24. Os intervalos IPv6 são recusados, uma vez que só a Spamhaus responde para IPv6: verifique um único endereço IPv6 com Reputação do servidor de correio.

2. Use um resolvedor recursivo seu, como para Reputação do servidor de correio: a Spamhaus recusa resolvedores públicos, ou cole uma chave gratuita Spamhaus DQS como credencial. Um /24 custa perto de duas mil consultas, enviadas a 10 por segundo e repetidas a cada 6 horas por predefinição: mantenha esse ritmo, as listas de bloqueio banem os resolvedores que as varrem.

3. A primeira verificação de um intervalo demora alguns minutos: a página do monitor diz que está em curso, depois mostra quantos endereços estão listados, em que listas, e uma ligação de remoção para cada um.

!!! warning
    A Spamhaus PBL lista intervalos de utilizadores finais que não devem enviar correio diretamente (ligações domésticas, conjuntos dinâmicos): uma listagem aí é uma política, não uma denúncia de abuso, e só importa para um endereço que execute um servidor de correio.

## O que monitoriza {#what-it-watches}

As mesmas métricas de [Reputação do servidor de correio](mail_blacklist.md#what-it-watches),
com o prefixo `dumbmonit_blacklist_`, sem as de DNS inverso, SPF e DMARC.
Para manter pequeno o número de séries, `check` só é escrita para um endereço
que esteja listado ou a que uma lista não tenha respondido; `list_checked`,
`list_listed` e `list_unknown` contam todos os endereços por lista.

As [regras integradas](../alerting/rules.md#dns-blocklists) que se aplicam:
Listed on a major blocklist e Listed on a blocklist.

## Credenciais {#credentials}

Nenhuma, ou uma chave Spamhaus DQS.

Endereço: um endereço IPv4 ou um intervalo CIDR até /24. Opções: Resolver,
Lists (todas as listas de endereços por predefinição), Refresh interval (6
horas), Queries per second (10) e Timeout per query (5 s).
