# Proxmox Mail Gateway {#proxmox-mail-gateway}

Gateway de email: filas do Postfix e a idade da mensagem mais antiga à
espera, email contado e filtrado hoje, tamanhos de quarentena, a idade
das bases de dados de assinaturas de antivírus e antispam, serviços,
certificados e estado do cluster.

!!! warning "Testado apenas contra uma instância de teste"

    Esta integração foi construída e verificada contra um Proxmox Mail
    Gateway 9 instalado para esse fim, não contra um de produção. Toda
    chamada é uma leitura, e as formas vêm de um gateway de email real,
    mas ainda ninguém o executou durante semanas num gateway de email em
    produção. Diga-nos o que falha.

## A página do dispositivo {#the-device-page}

Além dos gráficos genéricos, um dispositivo Proxmox Mail Gateway mostra
três painéis, lidos do que a verificação guardou — abrir a página nunca
consulta o próprio gateway:

* **Mail queues** — as quatro filas do Postfix pela ordem em que um
  administrador de email as lê: *incoming* (acabadas de aceitar),
  *active* (a ser entregues), *deferred* (entrega falhou, será repetida)
  e *hold* (retidas por uma regra). Cada uma mostra quantas mensagens
  detém, quantos domínios de destino, o mais ocupado desses domínios, e
  há quanto tempo a mensagem mais antiga está à espera. Uma fila que
  apenas tem email nela é tráfego normal; uma fila a reter uma mensagem
  há mais de quatro horas é marcada *Stuck*.
* **Mail filtered today** — mensagens recebidas e enviadas, bytes, spam,
  vírus, devoluções, tempo médio de processamento, e o que foi recusado
  antes da entrega (greylisting, listas de bloqueio, SPF, pregreeting).
  Uma barra por fração da janela recente desenha a curva de tráfego sem
  consultar a base de dados de séries temporais. Depois as três
  quarentenas com a sua contagem de mensagens e uso de disco, e os vírus
  apanhados hoje pelo nome.
* **Gateway** — serviços parados primeiro, depois os membros do cluster
  e o seu estado de sincronização, depois cada nó: tempo de atividade,
  CPU, memória, disco, desvio de relógio, as bases de dados de
  assinaturas com a sua idade e veredicto, certificados a expirar dentro
  de duas semanas, atualizações pendentes e o estado de subscrição.

Só contagens, nunca conteúdo. O DumbMonit lê quantas mensagens estão em
cada quarentena, nunca um assunto, um remetente, um destinatário ou um
corpo. Os endpoints de estatísticas por endereço (`/statistics/sender`,
`/statistics/receiver`, `/statistics/contact`) não são chamados de todo.

## Hoje, não as últimas vinte e quatro horas {#today-not-the-last-twenty-four-hours}

`GET /statistics/mail` agrega pelo **dia local no gateway**: o PMG
ignora o fim da janela que pede, só `starttime` escolhe o dia. O
DumbMonit pede portanto «agora», o que dá os totais desde a meia-noite —
exatamente o que o próprio painel do PMG mostra. Essas séries são
medidores que voltam a zero à meia-noite, não contadores: não os
envolva em `rate()`. A curva recente (`/statistics/recent`) lê a tabela
em bruto em vez disso e realmente gira.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_pmg_`.

| Família | Métricas | Etiquetas |
|---|---|---|
| Nó | `node_cpu_percent`, `node_iowait_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_used_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_clock_offset_seconds` (positivo quando o gateway está adiantado em relação ao DumbMonit), `node_insync`, `node_kernel_info`, `node_version_info` (a versão de API *deste* nó — num cluster, o que ficou atrás da última atualização mostra-se aqui), `version_info` | `node` |
| Filas | `queue_messages`, `queue_domains`, `queue_oldest_age_seconds` | `queue` (`incoming`, `active`, `deferred`, `hold`) |
| Email hoje | `mail_count_in/out`, `mail_bytes_in/out`, `mail_spam_in/out`, `mail_virus_in/out`, `mail_bounces_in/out`, `mail_junk_in`, `mail_junk_out` (os seus próprios utilizadores a enviar lixo: o primeiro sinal de uma conta comprometida), `mail_junk_percent` (ausente num dia sem email recebido), `mail_greylisted`, `mail_spf_rejects`, `mail_rbl_rejects`, `mail_pregreet_rejects`, `mail_avg_processing_seconds` | |
| Caudal | `mail_rate_in_per_minute`, `mail_rate_out_per_minute` (da última fração completa da curva recente) | |
| Pontuações de spam | `spam_score_messages` | `level` (`0` a `10`; `10` agrega tudo acima) |
| Vírus | `virus_detections` (top dez do dia) | `virus` |
| Quarentenas | `quarantine_messages`, `quarantine_bytes`, `quarantine_avg_spam_level` | `kind` (`spam`, `virus`, `attachment`) |
| Serviços | `service_running` (1 ou 0; nenhuma série para uma unidade não instalada) | `node`, `service` |
| Assinaturas | `signature_age_seconds`, `signature_count`, `signature_update_available` | `node`, `family` (`virus`, `spam`), `database` (`main`, `daily`, `bytecode`, ou o canal SpamAssassin) |
| Certificados | `certificate_expires_in_seconds` | `node`, `certificate` |
| Cluster | `cluster_nodes`, `cluster_node_insync`, `cluster_node_healthy` (todas ausentes num gateway autónomo) | `node`, `role` |
| Subscrição | `subscription_active` (as etiquetas transportam o estado exato) | `node`, `status`, `level` |
| Atualizações | `node_updates_pending`, `node_updates_security_pending` | `node` |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

`queue_oldest_age_seconds` é um **limite inferior**. O `qshape` reporta
intervalos de idade (menos de 5 min, 5–10 min, 10–20 min, e assim por
diante, duplicando), por isso o DumbMonit mantém o mínimo do intervalo
ocupado mais alto: uma mensagem reportada a «pelo menos 20 minutos» pode
ter 39 minutos. Errar por baixo significa que o alerta dispara mais
tarde em vez de mais cedo, nunca numa mensagem que não está realmente
atrasada.

`queue_domains` é exato num único gateway. Num cluster as filas por nó
são fundidas e a contagem é feita sobre os domínios mais ocupados de
cada nó, limitada a vinte por nó: além disso subconta. Deduplica supera
somar aqui, já que dois nós de um cluster recebem email para os mesmos
domínios.

Um gateway autónomo responde a `/config/cluster/status` com uma lista
vazia. Isso é um gateway sem cluster, não um cluster degradado: nenhuma
série é publicada e nenhuma regra pode disparar.

Só a base de dados **daily** do ClamAV é julgada pela sua idade, tanto
pelo painel como pela regra. `main` é reconstruída uma ou duas vezes por
ano e `bytecode` pouco mais frequentemente: julgá-las por dois dias
imprimiria «desatualizado» o ano todo num gateway perfeitamente atual.
Continuam a mostrar a sua idade, versão e contagem de assinaturas — sem
veredicto. Da mesma forma, um canal SpamAssassin que nunca foi obtido
não tem nenhuma data, e «nunca atualizado» não é «desatualizado»:
nenhuma série, nenhum veredicto, nenhum alerta.

O ClamAV escreve a sua data de compilação no seu próprio formato dentro
do cabeçalho `.cvd` (`16 Dec 2025 23-18 +0000`, com um hífen onde todos
os outros colocam dois pontos) e o PMG transmite-o. Se esse formato
alguma vez mudar, a idade fica simplesmente ausente em vez de errada por
cinquenta anos. A mesma regra aplica-se em todo lado: um valor que a API
não devolve produz uma métrica a menos, nunca um zero a fingir ser uma
medição.

As regras incorporadas que se aplicam: Device unreachable, Mail queue
growing, Mail stuck in the queue, Mail gateway service stopped, Virus
signatures out of date, Spam rules out of date, Quarantine filling up,
Mail gateway cluster degraded, Mail gateway certificate expiring, Mail
gateway updates pending. As notificações nomeiam a fila, serviço, base
de dados ou nó em causa.

## O que preparar no Proxmox Mail Gateway {#what-to-prepare-in-proxmox-mail-gateway}

Os passos abaixo são os que o aviso junto ao formulário mostra. O
princípio: um utilizador reservado para monitorização, com uma função
só de leitura — nunca a conta com que inicia sessão.

1. Na interface web: Configuration → User Management → Users → Add. Dê o seguinte nome à conta, uma senha longa não usada em nenhum outro lugar, e marque Enabled.

    ```
    dumbmonit@pmg
    ```

2. Defina a sua Role como `Audit`. Esse é o mínimo exato só de leitura para tudo o que o DumbMonit lê: estado do nó, serviços, filas do postfix, estatísticas de email, contagens de quarentena, idade das bases de dados ClamAV e SpamAssassin, estado do cluster, certificados, subscrição e atualizações pendentes. `Audit` não pode alterar nada, não pode libertar nada da quarentena nem ler nenhuma mensagem.

3. Prefere uma shell? A mesma conta num comando, depois defina a senha.

    ```
    pmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment "DumbMonit monitoring"
    ```

4. No DumbMonit, introduza o endereço do gateway, por exemplo «mail.lan» ou «mail.lan:8006», depois o nome de utilizador desta conta (realm incluído) e a senha.

5. O Proxmox Mail Gateway não emite tokens de API, ao contrário do Proxmox VE e do Proxmox Backup Server: o nome de utilizador e a senha são a forma de entrar. O DumbMonit abre uma sessão de duas horas e renova-a, em vez de iniciar sessão em cada medição.

!!! warning

    Não reutilize a conta com que inicia sessão: a função Audit acima só pode ler, e não pode libertar uma mensagem em quarentena nem alterar uma regra. O Proxmox Mail Gateway também usa um certificado autoassinado por predefinição: se a ligação for recusada por esse motivo, marque «Accept an unverifiable certificate» nas opções.

Endereço: `10.0.0.40`, `mail.lan`, `mail.lan:8006`, `[fd00::4]` ou um URL
completo. O esquema `https` e a porta 8006 são adicionados se estiverem
em falta.

## Opções {#options}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | API port | `8006` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | O Proxmox Mail Gateway vem com um certificado autoassinado por predefinição: ative isto se a ligação for recusada por esse motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tempo permitido para cada chamada de API, de 1 a 120. Ler uma fila executa um comando Postfix no gateway e pode demorar alguns segundos. |
| `node` | Monitored node | *(vazio)* | Nome do nó do cluster a monitorizar. Vazio: todo nó que o gateway lista. |
| `recent_hours` | Traffic window (hours) | `12` | Até quando a curva de tráfego recua, de 1 a 24. Os totais diários cobrem sempre o dia atual. |
| `queues` | Watch the mail queues | `true` | Lê as filas incoming, active, deferred e hold e a idade da mensagem mais antiga em cada uma. |
| `quarantine` | Count the quarantines | `true` | Lê quantas mensagens as quarentenas de spam e vírus detêm, e quanto disco usam. Só contagens. |
| `attachment_quarantine` | Also count the attachment quarantine | `false` | Essa quarentena não tem chamada de contagem: tem de ser listada para ser contada. Desligada por predefinição; só o número de entradas é mantido. |
| `signatures` | Watch the signature databases | `true` | Lê a idade das bases de dados de vírus do ClamAV e dos canais de regras do SpamAssassin. |
| `services` | Watch the services | `true` | Lê o estado do postfix, pmg-smtp-filter, pmgpolicy e as outras unidades de cada nó. |
| `certificates` | Watch the certificates | `true` | Lê os certificados que a interface web serve e avisa antes de expirarem. |
| `updates` | Count pending updates | `true` | Lista os pacotes à espera de atualização em cada nó. |
| `subscription` | Read the subscription | `true` | Lê o estado de subscrição de cada nó. |

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| O ClamAV `main` mostra uma idade enorme | Esperado: é reconstruído uma ou duas vezes por ano. Só `daily` transporta um veredicto. |
| Erro de certificado | Certificado autoassinado: instale um de confiança (ACME está incorporado no PMG) ou ative `insecure_tls`. |
| Erro de autenticação | Realm errado no nome de utilizador — tem de ler `dumbmonit@pmg`, não `dumbmonit` — ou a conta não está Enabled. Só a falha de `GET /version` condena toda a verificação; qualquer outra chamada falhada é contada em `scrape_errors`. |
| Permissões insuficientes | A função da conta está abaixo de `Audit`. `quser` só vê o seu próprio email; `helpdesk` não pode ler o nó nem as estatísticas. |
| Toda métrica `mail_*` é zero | O gateway genuinamente não filtrou nada desde a meia-noite. Os totais cobrem o dia local atual no gateway e reiniciam à sua meia-noite, não à sua. |
| Nenhuma série `service_running` de todo | A opção `services` está desligada, ou a função não as pode listar. Uma única unidade em falta na lista simplesmente não está instalada nesse nó (`chrony` numa máquina que usa `systemd-timesyncd`, por exemplo) e deliberadamente não é reportada como parada. |
| Nenhum `signature_age_seconds` para um canal SpamAssassin | Esse canal nunca foi atualizado, por isso o PMG não devolve nenhuma data. Execute `sa-update` no gateway; um canal secundário configurado mas nunca obtido fica sem data de propósito. |
| Nenhuma base de dados ClamAV de todo | O ClamAV não está instalado ou o `freshclam` nunca completou um download. O endpoint responde com uma lista vazia, o que não é um erro. |
| Nenhuma métrica de cluster | O gateway é autónomo. `/config/cluster/status` responde uma lista vazia, e nenhuma regra de cluster pode disparar. |
| «Mail stuck in the queue» dispara tarde | `queue_oldest_age_seconds` é o mínimo do intervalo de idade ocupado mais alto do `qshape`, por isso subreporta por desenho. |
| `quarantine_messages{kind="attachment"}` em falta | A opção `attachment_quarantine` está desligada: essa quarentena não tem endpoint de contagem, por isso contá-la significa listá-la. |
