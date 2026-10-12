# Proxmox Mail Gateway

Gateway de e-mail: filas do Postfix e a idade da mensagem mais antiga
esperando, e-mail contado e filtrado hoje, tamanhos de quarentena, a idade
dos bancos de dados de assinatura de antivírus e antispam, serviços,
certificados e estado do cluster.

!!! warning "Testado apenas contra uma instância de teste"

    Esta integração foi construída e verificada contra um Proxmox Mail
    Gateway 9 instalado para esse fim, não contra um de produção. Toda
    chamada é uma leitura, e as formas vêm de um gateway de e-mail real,
    mas ninguém ainda o executou por semanas em um gateway de e-mail em
    funcionamento. Avise-nos o que não funcionar.

## A página do dispositivo

Além dos gráficos genéricos, um dispositivo Proxmox Mail Gateway mostra
três painéis, lidos do que a sondagem armazenou — abrir a página nunca
consulta o próprio gateway:

* **Filas de e-mail** — as quatro filas do Postfix na ordem em que um
  administrador de e-mail as lê: *incoming* (acabou de ser aceito),
  *active* (sendo entregue), *deferred* (entrega falhou, será tentada de
  novo) e *hold* (retida por uma regra). Cada uma mostra quantas mensagens
  contém, quantos domínios de destino, o mais movimentado desses
  domínios, e por quanto tempo a mensagem mais antiga está esperando. Uma
  fila que simplesmente tem e-mail nela é tráfego normal; uma fila
  mantendo uma mensagem por mais de quatro horas é marcada *Travada*.
* **E-mail filtrado hoje** — mensagens de entrada e saída, bytes, spam,
  vírus, devoluções, tempo médio de processamento, e o que foi recusado
  antes da entrega (greylisting, listas de bloqueio, SPF, pregreeting).
  Uma barra por fatia da janela recente desenha a curva de tráfego sem
  consultar o banco de dados de séries temporais. Depois as três
  quarentenas com sua contagem de mensagens e uso de disco, e os vírus
  pegos hoje pelo nome.
* **Gateway** — serviços parados primeiro, depois os membros do cluster e
  seu estado de sincronização, depois cada nó: tempo de atividade, CPU,
  memória, disco, desvio de relógio, os bancos de dados de assinatura com
  sua idade e veredito, certificados expirando dentro de duas semanas,
  atualizações pendentes e o status de assinatura.

Apenas contagens, nunca conteúdo. O DumbMonit lê quantas mensagens estão
em cada quarentena, nunca um assunto, um remetente, um destinatário ou um
corpo. Os endpoints de estatísticas por endereço (`/statistics/sender`,
`/statistics/receiver`, `/statistics/contact`) não são chamados de forma
alguma.

## Hoje, não as últimas vinte e quatro horas

`GET /statistics/mail` agrega por **dia local no gateway**: o PMG ignora
o fim da janela que você pede, só `starttime` escolhe o dia. O DumbMonit
portanto pede "agora", o que dá os totais desde a meia-noite — exatamente
o que o próprio painel do PMG mostra. Essas séries são gauges que voltam
a zero à meia-noite, não contadores: não as envolva em `rate()`. A curva
recente (`/statistics/recent`) lê a tabela bruta em vez disso e realmente
rola.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_pmg_`.

| Família | Métricas | Rótulos |
|---|---|---|
| Nó | `node_cpu_percent`, `node_iowait_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_used_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_clock_offset_seconds` (positivo quando o gateway está adiantado em relação ao DumbMonit), `node_insync`, `node_kernel_info`, `node_version_info` (a versão da API *deste* nó — em um cluster, o que ficou atrás da última atualização aparece aqui), `version_info` | `node` |
| Filas | `queue_messages`, `queue_domains`, `queue_oldest_age_seconds` | `queue` (`incoming`, `active`, `deferred`, `hold`) |
| E-mail hoje | `mail_count_in/out`, `mail_bytes_in/out`, `mail_spam_in/out`, `mail_virus_in/out`, `mail_bounces_in/out`, `mail_junk_in`, `mail_junk_out` (seus próprios usuários enviando lixo: o primeiro sinal de uma conta comprometida), `mail_junk_percent` (ausente em um dia sem e-mail de entrada), `mail_greylisted`, `mail_spf_rejects`, `mail_rbl_rejects`, `mail_pregreet_rejects`, `mail_avg_processing_seconds` | |
| Vazão | `mail_rate_in_per_minute`, `mail_rate_out_per_minute` (da última fatia completa da curva recente) | |
| Scores de spam | `spam_score_messages` | `level` (`0` a `10`; `10` agrega tudo acima) |
| Vírus | `virus_detections` (top dez do dia) | `virus` |
| Quarentenas | `quarantine_messages`, `quarantine_bytes`, `quarantine_avg_spam_level` | `kind` (`spam`, `virus`, `attachment`) |
| Serviços | `service_running` (1 ou 0; nenhuma série de forma alguma para uma unidade que não está instalada) | `node`, `service` |
| Assinaturas | `signature_age_seconds`, `signature_count`, `signature_update_available` | `node`, `family` (`virus`, `spam`), `database` (`main`, `daily`, `bytecode`, ou o canal SpamAssassin) |
| Certificados | `certificate_expires_in_seconds` | `node`, `certificate` |
| Cluster | `cluster_nodes`, `cluster_node_insync`, `cluster_node_healthy` (todos ausentes em um gateway autônomo) | `node`, `role` |
| Assinatura | `subscription_active` (rótulos carregam o status exato) | `node`, `status`, `level` |
| Atualizações | `node_updates_pending`, `node_updates_security_pending` | `node` |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

`queue_oldest_age_seconds` é um **limite inferior**. `qshape` reporta
faixas de idade (menos de 5 min, 5–10 min, 10–20 min, e assim por diante,
dobrando), então o DumbMonit mantém o piso da faixa ocupada mais alta:
uma mensagem reportada com "pelo menos 20 minutos" pode ter 39 minutos.
Errar para baixo significa que o alerta dispara mais tarde em vez de mais
cedo, nunca em uma mensagem que não está realmente atrasada.

`queue_domains` é exato em um único gateway. Em um cluster as filas por
nó são mescladas e a contagem é feita sobre os domínios mais movimentados
de cada nó, limitada a vinte por nó: além disso ela subconta.
Deduplicar vence somar aqui, já que dois nós de um cluster recebem
e-mail para os mesmos domínios.

Um gateway autônomo responde `/config/cluster/status` com uma lista
vazia. Isso é um gateway sem cluster, não um cluster degradado: nenhuma
série é publicada e nenhuma regra pode disparar.

Só o banco de dados **daily** do ClamAV é julgado por sua idade, tanto
pelo painel quanto pela regra. `main` é reconstruído uma ou duas vezes
por ano e `bytecode` quase não mais frequentemente: julgá-los por dois
dias imprimiria "desatualizado" o ano todo em um gateway perfeitamente
atual. Eles ainda mostram sua idade, versão e contagem de assinatura —
sem um veredito. Da mesma forma, um canal SpamAssassin que nunca foi
buscado não tem data alguma, e "nunca atualizado" não é "desatualizado":
nenhuma série, nenhum veredito, nenhum alerta.

O ClamAV escreve sua data de build em seu próprio formato dentro do
cabeçalho `.cvd` (`16 Dec 2025 23-18 +0000`, com um hífen onde todo mundo
coloca dois pontos) e o PMG o passa direto. Se esse formato algum dia
mudar, a idade simplesmente fica ausente em vez de errada por cinquenta
anos. A mesma regra se aplica em todo lugar: um valor que a API não
retorna produz uma métrica menos, nunca um zero fingindo ser uma medição.

Regras integradas que se aplicam: Device unreachable, Mail queue growing,
Mail stuck in the queue, Mail gateway service stopped, Virus signatures
out of date, Spam rules out of date, Quarantine filling up, Mail gateway
cluster degraded, Mail gateway certificate expiring, Mail gateway updates
pending. Notificações nomeiam a fila, serviço, banco de dados ou nó
envolvido.

## O que preparar no Proxmox Mail Gateway

Os passos abaixo são os que o aviso ao lado do formulário mostra. O
princípio: um usuário reservado para monitoramento, com um papel somente
leitura — nunca a conta com a qual você entra.

1. Na interface web: Configuration → User Management → Users → Add.
   Nomeie a conta como abaixo, dê a ela uma senha longa usada em nenhum
   outro lugar, e marque Enabled.

    ```
    dumbmonit@pmg
    ```

2. Defina seu Role para `Audit`. Esse é o mínimo exato somente leitura
   para tudo que o DumbMonit lê: status do nó, serviços, filas do
   postfix, estatísticas de e-mail, contagens de quarentena, idade dos
   bancos de dados ClamAV e SpamAssassin, status do cluster,
   certificados, assinatura e atualizações pendentes. `Audit` não pode
   alterar nada, liberar nada da quarentena ou ler nenhuma mensagem.

3. Prefere um shell? A mesma conta em um comando, depois defina a senha.

    ```
    pmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment "DumbMonit monitoring"
    ```

4. No DumbMonit, informe o endereço do gateway, por exemplo "mail.lan" ou
   "mail.lan:8006", depois o nome de usuário (realm incluído) e senha
   desta conta.

5. O Proxmox Mail Gateway não emite tokens de API, diferente do Proxmox
   VE e Proxmox Backup Server: o nome de usuário e senha são o caminho de
   entrada. O DumbMonit abre uma sessão de duas horas e a renova, em vez
   de entrar a cada medição.

!!! warning

    Não reutilize a conta com a qual você entra: o papel Audit acima só
    pode ler, e não pode liberar uma mensagem em quarentena ou alterar
    uma regra. O Proxmox Mail Gateway também usa um certificado
    autoassinado por padrão: se a conexão for recusada por esse motivo,
    marque "Aceitar um certificado não verificável" nas opções.

Endereço: `10.0.0.40`, `mail.lan`, `mail.lan:8006`, `[fd00::4]` ou uma URL
completa. O esquema `https` e a porta 8006 são adicionados se faltando.

## Opções

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `port` | Porta da API | `8006` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | O Proxmox Mail Gateway vem com um certificado autoassinado por padrão: ative isso se a conexão for recusada por esse motivo. |
| `request_timeout_seconds` | Tempo limite por requisição (segundos) | `15` | Tempo permitido para cada chamada de API, de 1 a 120. Ler uma fila executa um comando Postfix no gateway e pode levar alguns segundos. |
| `node` | Nó monitorado | *(vazio)* | Nome do nó do cluster a monitorar. Vazio: todo nó que o gateway lista. |
| `recent_hours` | Janela de tráfego (horas) | `12` | Quão longe no passado a curva de tráfego vai, de 1 a 24. Os totais diários sempre cobrem o dia atual. |
| `queues` | Observar as filas de e-mail | `true` | Lê as filas incoming, active, deferred e hold e a idade da mensagem mais antiga em cada uma. |
| `quarantine` | Contar as quarentenas | `true` | Lê quantas mensagens as quarentenas de spam e vírus contêm, e quanto disco usam. Apenas contagens. |
| `attachment_quarantine` | Também contar a quarentena de anexos | `false` | Essa quarentena não tem chamada de contagem: precisa ser listada para ser contada. Desligado por padrão; só o número de entradas é mantido. |
| `signatures` | Observar os bancos de dados de assinatura | `true` | Lê a idade dos bancos de dados de vírus do ClamAV e dos canais de regra do SpamAssassin. |
| `services` | Observar os serviços | `true` | Lê o estado de postfix, pmg-smtp-filter, pmgpolicy e as outras unidades de cada nó. |
| `certificates` | Observar os certificados | `true` | Lê os certificados que a interface web serve e avisa antes de expirarem. |
| `updates` | Contar atualizações pendentes | `true` | Lista os pacotes esperando por uma atualização em cada nó. |
| `subscription` | Ler a assinatura | `true` | Lê o status de assinatura de cada nó. |

## Erros comuns

| Sintoma | Causa provável |
|---|---|
| ClamAV `main` mostra uma idade enorme | Esperado: é reconstruído uma ou duas vezes por ano. Só `daily` carrega um veredito. |
| Erro de certificado | Certificado autoassinado: instale um confiável (ACME está embutido no PMG) ou ative `insecure_tls`. |
| Erro de autenticação | Realm errado no nome de usuário — precisa ler `dumbmonit@pmg`, não `dumbmonit` — ou a conta não está Enabled. Só `GET /version` falhando condena toda a sondagem; qualquer outra chamada falhando é contada em `scrape_errors`. |
| Permissões insuficientes | O papel da conta está abaixo de `Audit`. `quser` só vê seu próprio e-mail; `helpdesk` não pode ler o nó ou as estatísticas. |
| Toda métrica `mail_*` é zero | O gateway genuinamente não filtrou nada desde a meia-noite. Os totais cobrem o dia local atual no gateway e reiniciam em sua meia-noite, não na sua. |
| Nenhuma série `service_running` de forma alguma | A opção `services` está desligada, ou o papel não pode listá-los. Uma única unidade faltando na lista simplesmente não está instalada naquele nó (`chrony` em uma máquina usando `systemd-timesyncd`, por exemplo) e deliberadamente não é reportada como parada. |
| Nenhum `signature_age_seconds` para um canal SpamAssassin | Esse canal nunca foi atualizado, então o PMG não retorna data. Execute `sa-update` no gateway; um canal secundário que está configurado mas nunca buscado permanece sem data de propósito. |
| Nenhum banco de dados ClamAV de forma alguma | O ClamAV não está instalado ou `freshclam` nunca completou um download. O endpoint responde com uma lista vazia, que não é um erro. |
| Nenhuma métrica de cluster | O gateway é autônomo. `/config/cluster/status` responde uma lista vazia, e nenhuma regra de cluster pode disparar. |
| "Mail stuck in the queue" dispara tarde | `queue_oldest_age_seconds` é o piso da faixa de idade ocupada mais alta do `qshape`, então ele subreporta por design. |
| `quarantine_messages{kind="attachment"}` faltando | A opção `attachment_quarantine` está desligada: essa quarentena não tem endpoint de contagem, então contá-la significa listá-la. |
