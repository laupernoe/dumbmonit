# Lista de bloqueio de faixa de IP {#ip-range-blocklist}

Cada endereço de uma faixa IPv4 pública, até um /24 (256 endereços),
verificado contra as listas de bloqueio DNS de [Reputação do servidor de e-mail](mail_blacklist.md#the-blocklists):
Spamhaus ZEN, Barracuda, SpamCop, Mailspike, UCEPROTECT Level 1, PSBL e
SpamRATS. A página do monitor diz quantos endereços estão listados, em quais
listas, e traz um link direto para a página de consulta e remoção de cada
listagem.

Útil para a faixa pública de um escritório, o bloco que um provedor de
hospedagem lhe deu ou os endereços de onde seus servidores enviam: a máquina
comprometida de um vizinho recebe uma listagem que um destinatário mais
rigoroso pode estender à faixa inteira.

## Limites {#limits}

- **No máximo um /24 por monitor.** Uma faixa maior é recusada com uma mensagem
  dizendo quantos endereços ela contém: divida-a em vários monitores /24. Cada
  endereço custa uma consulta a cada lista, e uma varredura de milhares de
  endereços parece abuso para elas.
- **Somente IPv4.** Dessas listas, só o Spamhaus ZEN responde para IPv6, e uma
  faixa IPv6 não pode ser percorrida endereço por endereço. Um único endereço
  IPv6 pode ser verificado com [Reputação do servidor de e-mail](mail_blacklist.md).
- **Somente endereços públicos.** Uma faixa que inclui um endereço privado,
  compartilhado (CGNAT) ou de documentação é recusada: as listas de bloqueio
  só conhecem a Internet pública.
- **As listas de domínio** (Spamhaus DBL, SURBL, URIBL) não se aplicam a uma
  faixa.

## Resolvedores e ritmo {#resolvers-and-pace}

Aplicam-se as mesmas regras de [Reputação do servidor de e-mail](mail_blacklist.md#resolvers-refusals-and-timeouts).
**Nenhuma chave de API é necessária.** Só o Spamhaus recusa resolvedores públicos: a
recusa dele (`127.255.255.254`) nunca é mostrada como listagem, a página do monitor
diz "O Spamhaus recusa este resolvedor" e as outras listas mantêm seus resultados.
Uma lista que não responde é desconhecida, nunca limpa. Para o Spamhaus também, use
um resolvedor recursivo seu, ou a chave Spamhaus DQS gratuita e opcional como
credencial.

As listas são as marcadas em **Listas de bloqueio** (por padrão as principais listas
de endereços) mais suas próprias zonas em **Listas personalizadas**; veja
[Suas próprias listas](mail_blacklist.md#your-own-lists). As listas de domínios não
se aplicam a uma faixa.

Um /24 leva perto de duas mil consultas. Elas são enviadas em **Consultas por
segundo** (10 por padrão), no máximo oito em andamento, e a faixa inteira é
verificada uma vez por **Intervalo de atualização** (6 horas por padrão); o
último resultado é mostrado de novo a cada verificação no meio-tempo. A
primeira verificação de uma faixa leva alguns minutos e roda em segundo plano.

## Verificar uma faixa de IP pública contra as listas de bloqueio {#check-a-public-ip-range-against-the-blocklists}

1. Informe um endereço IPv4 ou uma faixa CIDR até /24, por exemplo "198.51.100.0/24" (256 endereços). Uma faixa maior é recusada: divida-a em vários monitores /24. Faixas IPv6 são recusadas, já que só o Spamhaus responde para IPv6: verifique um único endereço IPv6 com Reputação do servidor de e-mail.

2. Nenhuma chave de API é necessária: deixe a credencial em Sem chave. As listas de bloqueio são consultadas pelo resolvedor DNS deste servidor, ou pelo que você definir em Resolvedor. Só o Spamhaus pode recusar, e só quando esse resolvedor é público (Google, Cloudflare, Quad9): o monitor mostra então "O Spamhaus recusa este resolvedor" e as outras listas continuam funcionando. Para o Spamhaus também, use um resolvedor recursivo seu, ou a chave Spamhaus DQS gratuita e opcional.

3. Marque em Listas de bloqueio as listas de endereços a consultar (as principais vêm marcadas por padrão) e adicione em Listas personalizadas as zonas DNSBL de qualquer outra lista. Um /24 leva perto de duas mil consultas com todas as listas de endereços marcadas, enviadas a 10 por segundo e repetidas a cada 6 horas por padrão: mantenha esse ritmo, as listas de bloqueio banem os resolvedores que as varrem.

4. A primeira verificação de uma faixa leva alguns minutos: a página do monitor diz que ela está em andamento e depois mostra quantos endereços estão listados, em quais listas, e um link de remoção para cada um.

!!! warning
    O Spamhaus PBL lista faixas de usuários finais que não deveriam enviar e-mail diretamente (conexões residenciais, pools dinâmicos): uma listagem ali é uma política, não uma denúncia de abuso, e só importa para um endereço que roda um servidor de e-mail.

## O que ele observa {#what-it-watches}

As mesmas métricas de [Reputação do servidor de e-mail](mail_blacklist.md#what-it-watches),
com o prefixo `dumbmonit_blacklist_`, sem as de DNS reverso, SPF e DMARC. Para
manter pequeno o número de séries, `check` só é escrita para um endereço que
está listado ou que uma lista não respondeu; `list_checked`, `list_listed` e
`list_unknown` contam todos os endereços por lista.

As [regras integradas](../alerting/rules.md#dns-blocklists) que se aplicam:
Listed on a major blocklist e Listed on a blocklist.

A página do monitor tem um link **Verificar também no MXToolbox**: para uma faixa,
ao lado de cada endereço listado, ele abre em uma nova aba a consulta pública de
listas de bloqueio do mxtoolbox.com. O DumbMonit não chama a API deles.

## Credenciais {#credentials}

Nenhuma necessária (o padrão), ou opcionalmente uma chave Spamhaus DQS.

Endereço: um endereço IPv4 ou uma faixa CIDR até /24. Opções: Resolvedor, Listas de
bloqueio (as principais marcadas em um monitor novo), Listas personalizadas,
Intervalo de atualização (6 horas), Consultas por segundo (10) e Timeout por
consulta (5 s).
