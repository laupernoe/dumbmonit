# Proxmox Datacenter Manager {#proxmox-datacenter-manager}

A console que federa vários clusters Proxmox VE e instâncias Proxmox
Backup Server: quais instâncias ela alcança, todo o acervo em um
piscar de olhos, as tarefas que falharam em qualquer lugar, e a própria
saúde da console.

!!! warning "Testado apenas contra uma instância de teste"

    Esta integração foi construída e verificada contra um Proxmox
    Datacenter Manager 1.1 instalado para esse fim, não contra um de
    produção. Toda chamada é uma leitura, e as formas vêm de um
    datacenter manager real, mas ninguém ainda o executou por semanas em
    um datacenter manager em funcionamento. Avise-nos o que não
    funcionar.

## Um dispositivo para o acervo, ou um dispositivo por cluster? {#one-device-for-the-estate-or-one-device-per-cluster}

Um dispositivo Datacenter Manager não é um substituto para dispositivos
[Proxmox VE](proxmox.md) e [Proxmox Backup Server](pbs.md). Ele responde
uma pergunta diferente.

| | Dispositivo Datacenter Manager | Um dispositivo por cluster e por servidor de backup |
|---|---|---|
| Dispositivos a criar | um | um por cluster, um por servidor de backup |
| Credenciais a gerenciar | um token, na console | um token por instância |
| Carga em cada cluster | nenhuma nossa: lemos o próprio cache da console | uma sondagem por intervalo, por cluster |
| O que você obtém | acessibilidade, versão, contagens de guest e nó, totais de CPU, memória e armazenamento, tarefas com falha | tudo isso mais guests um a um, snapshots, jobs de backup, HA, Ceph, replicação, discos e SMART, pools ZFS, o calendário de backup de 30 dias |
| O que você não pode obter | séries por guest, detalhe por datastore, discos, snapshots, logs de tarefa | um total único para todo o acervo |
| Falha principal | a console não consegue alcançar uma instância | o cluster não nos responde |

**Nossa recomendação.** Adicione o dispositivo Datacenter Manager quando
você já roda uma console e quer uma página que diz se o acervo está
completo — é a forma mais barata de notar que um site remoto caiu, e não
custa nada aos clusters. Adicione um dispositivo dedicado para todo
cluster e servidor de backup que você realmente opera: é ali que está a
profundidade, e é isso que diz *qual* VM parou ou *qual* backup falhou. Os
dois se sobrepõem sem conflitar; um backup com falha então gera um alerta
do dispositivo PBS (com o log da tarefa) e um da console (nomeando a
instância). Silencie a regra de tarefa da console se achar isso
redundante, ou a restrinja às instâncias para as quais você não tem
dispositivo dedicado.

Se você não roda nenhuma console, não instale uma só para monitoramento:
dispositivos dedicados veem estritamente mais.

## A página do dispositivo {#the-device-page}

Além dos gráficos genéricos, um dispositivo Datacenter Manager mostra
três painéis, lidos do que a sondagem armazenou — abrir a página não
consulta nem a console nem os clusters que ela gerencia:

* **Instâncias federadas** — uma faixa de totais do acervo (instâncias,
  nós online, guests rodando e parados, núcleos, memória, armazenamento),
  depois uma linha por instância federada. Instâncias que a console não
  consegue alcançar vêm primeiro na ordenação, depois as com tarefas com
  falha, depois as atrasadas na versão. Cada linha dá o produto (Proxmox
  VE ou Backup Server), a versão, contagens de nó e guest, uso de memória
  e armazenamento, o estado da assinatura, o endereço configurado, quando
  a console coletou dela por último — e, quando falhou, a mensagem que a
  própria console recebeu. Essa mensagem é a única coisa que explica a
  indisponibilidade, então é mostrada literalmente.
* **Falhas** — toda tarefa que falhou em todo o acervo nos últimos 14
  dias, mais recente primeiro, com a instância em que rodou, o tipo de
  tarefa, quando começou, quanto tempo levou e a mensagem de erro. O
  próprio log da tarefa permanece na instância: abra-o lá, ou adicione
  aquele cluster como seu próprio dispositivo, onde *Mostrar log* o
  busca.
* **Host da console** — a máquina que roda a console: CPU, memória,
  sistema de arquivos raiz, tempo de atividade e kernel, seus
  certificados com os dias restantes antes da expiração, os pacotes
  esperando por uma atualização, e o status de assinatura dos nós
  gerenciados. Todo o painel é opcional: um token sem `Sys.Audit` não lê
  nada disso e o painel avisa isso, em vez de mostrar zeros.

O histórico de tarefas é mantido em SQLite (`pdm_task_history`, 19 dias,
no máximo 5000 tarefas por dispositivo) então a lista de falhas sobrevive
a reinícios mesmo que cada sondagem só olhe a janela de tarefas.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_pdm_`.

| Família | Métricas | Rótulos |
|---|---|---|
| Acervo | `remotes_total`, `remotes_failed`, `nodes_online`, `nodes_offline`, `cpu_used_cores`, `cpu_total_cores`, `memory_used_bytes`, `memory_total_bytes`, `memory_used_percent`, `storage_used_bytes`, `storage_total_bytes`, `storage_used_percent`, `datastores_total` | |
| Guests do acervo | `guests_running`, `guests_stopped` | `guest_type` (`qemu`, `lxc`) |
| Instâncias | `remote_reachable` (1 a console obteve resposta, 0 não obteve), `remote_version_info` (série de presença, rótulo `version`), `remote_version_behind`, `remote_nodes_online`, `remote_nodes_offline`, `remote_guests_running`, `remote_guests_stopped`, `remote_cpu_used_cores`, `remote_cpu_total_cores`, `remote_cpu_used_percent`, `remote_memory_used_bytes`, `remote_memory_total_bytes`, `remote_memory_used_percent`, `remote_storage_used_bytes`, `remote_storage_total_bytes`, `remote_storage_used_percent`, `remote_datastores`, `remote_subscription_active`, `remote_updates_pending`, `remote_last_collection_age_seconds`, `remote_tasks_failed` | `remote`, `type` (`pve`, `pbs`) |
| Tarefas na janela | `tasks_total`, `tasks_failed`, `tasks_running` | |
| Host da console | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`, `node_load1`, `node_memory_used_bytes`, `node_memory_total_bytes`, `node_memory_used_percent`, `node_swap_used_bytes`, `node_swap_total_bytes`, `node_rootfs_used_bytes`, `node_rootfs_total_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `node_updates_pending`, `version_info` | |
| Certificados da console | `node_certificate_expiry_days` (negativo uma vez expirado) | `file` (`proxy.pem`) |
| Assinatura | `subscription_active` (rótulo `status`), `subscription_active_nodes`, `subscription_total_nodes` | |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Uma tarefa que termina com `WARNINGS: n` conta como bem sucedida. Só
`GET /version` falhando condena toda a sondagem; qualquer outra chamada
falhando é contada em `scrape_errors`, e uma instância que a console não
consegue alcançar é uma medição (`remote_reachable = 0`), não um erro de
coleta.

`remote_version_behind` compara as instâncias federadas **umas com as
outras**, produto por produto: é 1 quando outro cluster Proxmox VE — ou
outro servidor de backup — no mesmo acervo roda uma versão mais nova. Não
é uma comparação contra o que o Proxmox publica; uma sondagem não tem
nada a ver alcançando a internet. A série só existe para uma instância
cuja versão foi lida, então uma instância inacessível nunca passa como
atualizada.

Totais de armazenamento contam cada nome de armazenamento uma vez por
instância: um armazenamento compartilhado por cinco nós de um cluster é
contado uma vez, não cinco.

Regras integradas que se aplicam: Device unreachable, Federated instance
unreachable, Federated instance behind, Task failed on a federated
instance, Datacenter console disk almost full, Datacenter console
certificate expiring, Datacenter console updates pending. Notificações
nomeiam a instância envolvida.

## O que preparar no Proxmox Datacenter Manager {#what-to-prepare-in-proxmox-datacenter-manager}

Os passos abaixo são os que o aviso ao lado do formulário mostra. O
princípio: um usuário reservado para monitoramento, um token com um papel
somente leitura — nunca a conta com a qual você entra.

1. Na console, abra Configuration → Access Control → Users e clique em
   Add. Nomeie a conta como abaixo e deixe a senha vazia: o token é o que
   entra.

    ```
    dumbmonit@pdm
    ```

2. Ainda em Access Control, abra API Tokens → Add, escolha esse usuário e
   nomeie o token "monitor". A console mostra o secret uma vez: copie-o
   agora.

    ```
    dumbmonit@pdm!monitor
    ```

3. Dê tanto ao usuário quanto ao token o papel `Auditor` em `/` (o topo
   da árvore, então todo o acervo). `Auditor` carrega `System.Audit`,
   `Resource.Audit` e `Access.Audit`, e não pode alterar nada. Um token
   nunca tem mais direitos que seu usuário, então a permissão é concedida
   duas vezes: Permissions → Add → User Permission, depois de novo com
   API Token Permission.
4. Copie o id do token para o campo Token ID do DumbMonit e o secret para
   seu campo Secret.
5. No DumbMonit, informe o endereço da console, por exemplo "dc.lan" ou
   "dc.lan:8443".

!!! warning
    Não reutilize a conta com a qual você entra: um token exposto então
    controlaria todo cluster que a console federa de uma vez. O papel
    `Auditor` só pode ler. A console também usa um certificado
    autoassinado por padrão: se a conexão for recusada por esse motivo,
    marque "Aceitar um certificado não verificável" nas opções. Dois
    itens precisam de mais que `Auditor` e são simplesmente pulados sem
    ele: o status de coleta de métricas por instância, e o resumo de
    atualização das instâncias federadas.

Prefere a linha de comando? `proxmox-datacenter-manager-admin` gerencia
remotes, não usuários; crie a conta, o token e as duas permissões na
interface web.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de API (recomendado) | **Token ID**: usuário, realm e nome do token como a console os mostra, `dumbmonit@pdm!monitor`. **Secret**: o UUID mostrado uma vez quando o token foi criado. O DumbMonit os envia como o cabeçalho `PDMAPIToken=user@pdm!name:secret` que a console espera. |

Não há opção de nome de usuário / senha, e isso é deliberado: desde seu
primeiro lançamento a console retorna seu ticket de sessão apenas em um
cookie `HttpOnly` — o corpo JSON carrega um `ticket-info` inutilizável.
Um login não compraria nada sobre um token enquanto abriria uma sessão
conectada em cada sondagem. Tokens são o que o Proxmox recomenda para
automação de qualquer forma.

Direitos, somente leitura. O mais simples e suficiente: o papel embutido
`Auditor` em `/`, concedido ao usuário *e* ao token. Por endpoint:

| Endpoint | Precisa de |
|---|---|
| `GET /version` | qualquer usuário autenticado |
| `GET /remotes/remote` | retorna as instâncias que o chamador pode auditar |
| `GET /remotes/remote/{id}/version` | `Resource.Audit` em `/resource/{id}` |
| `GET /resources/status`, `GET /resources/list` | `Resource.Audit` em `/resource` |
| `GET /resources/subscription` | `Resource.Audit` em `/resource` |
| `GET /remotes/tasks/list` | `Resource.Audit` em `/resource/{remote}`, por instância |
| `GET /nodes/{node}/status` | `System.Audit` em `/system/status` |
| `GET /nodes/{node}/certificates/info`, `GET /nodes/{node}/subscription` | qualquer usuário autenticado |
| `GET /nodes/{node}/apt/update` | `System.Audit`; um 403 é tolerado: nenhuma série `node_updates_pending`, nenhum erro de coleta |
| `GET /remotes/metric-collection/status` | mais que `Auditor`; um 403 é tolerado: nenhuma série `remote_last_collection_age_seconds`, nenhum erro de coleta |
| `GET /remotes/updates/summary` | `Resource.Modify` — um privilégio de escrita. Desligado por padrão (`remote_updates`) por esse motivo |

Note que a console redige os tokens de acesso que mantém para suas
instâncias (`GET /remotes/remote` retorna um `token` vazio), e o
DumbMonit nunca lê esse campo de qualquer forma: nenhum segredo de
instância jamais passa pelo coletor, a visão armazenada ou a API.

Endereço: `10.0.0.40`, `dc.lan`, `dc.lan:8443`, `[fd00::2]` ou uma URL
completa. O esquema `https` e a porta 8443 são adicionados se faltando.

## Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `port` | Porta da API | `8443` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | O Proxmox Datacenter Manager vem com um certificado autoassinado por padrão: ative isso se a conexão for recusada por esse motivo. |
| `request_timeout_seconds` | Tempo limite por requisição (segundos) | `20` | Tempo permitido para cada chamada de API, de 1 a 120. A console retransmite chamadas para toda instância federada, então um site lento retém toda a resposta. |
| `node` | Nome do nó da console | `localhost` | Nome do nó que roda a console. Um Datacenter Manager tem um único nó e o chama de "localhost". |
| `task_lookback_hours` | Janela de tarefas (horas) | `24` | Tarefas mais antigas não são contadas entre as falhas, de 1 a 8760. |
| `max_age_seconds` | Idade de cache aceita (segundos) | `60` | Quão desatualizado o próprio inventário da console pode estar antes de consultar as instâncias federadas de novo, de 0 a 3600. Zero pede dados novos a cada sondagem, o que coloca todo cluster de volta na linha. |
| `remotes` | Instâncias monitoradas | *(vazio)* | Nomes das instâncias federadas a monitorar, separados por vírgulas. Vazio: toda instância. |
| `max_remotes` | Limite de versão de instância | `100` | Número máximo de instâncias perguntadas por sua versão em cada sondagem, de 1 a 1000. |
| `versions` | Ler a versão de cada instância | `true` | Uma chamada por instância federada, para reportar sua versão e marcar as atrasadas em relação a seus pares. Uma instância que não responde conta como inacessível. |
| `tasks` | Observar tarefas entre instâncias | `true` | Lê a lista de tarefas de toda instância federada para reportar as falhas. |
| `node_status` | Observar o host da console | `true` | Lê a CPU, memória, sistema de arquivos raiz, tempo de atividade, certificados e assinatura da máquina que roda a console. |
| `updates` | Contar atualizações pendentes | `true` | Lista os pacotes esperando por uma atualização na própria console. |
| `remote_updates` | Contar atualizações em cada instância | `false` | Lê o resumo de atualização da console para as instâncias federadas. Desligado por padrão: a console protege esse caminho com `Resource.Modify`, um privilégio de escrita que uma conta de monitoramento não tem motivo para ter. |

`max_age_seconds` é o que vale a pena entender. A console mantém seu
próprio inventário, atualizado por seu próprio ciclo de coleta. Pedir
dados novos a cada sondagem faz a console chamar todo cluster de novo,
além de seu ritmo normal — exatamente a carga que um único dispositivo de
console deveria evitar. Sessenta segundos ainda é mais fresco que o
intervalo de sondagem padrão.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de certificado | Certificado autoassinado: instale um confiável (ACME está embutido na console) ou ative `insecure_tls`. |
| Erro de autenticação | Secret colado em Token ID (ou o contrário), ou a permissão foi concedida ao usuário mas não ao token — um token nunca tem mais direitos que seu usuário, e as duas ACLs são entradas separadas. |
| Toda instância aparece como inacessível, mas a própria página da console está bem | O token tem `Auditor` no usuário mas não no token, ou só em `/system`: `Resource.Audit` em `/resource` é o que permite a console responder por suas instâncias. |
| Uma instância está inacessível e a mensagem menciona uma fingerprint | A console verifica o certificado de cada instância que gerencia; corrija isso na console (Remotes → a instância), não aqui. |
| Nenhuma série `remote_last_collection_age_seconds` | `GET /remotes/metric-collection/status` precisa de mais que `Auditor` e é pulado silenciosamente. Nada mais é afetado. |
| Nenhuma série `remote_updates_pending` | `remote_updates` está desligado por padrão porque esse caminho precisa de `Resource.Modify`. Ative apenas se você estiver dispostos a conceder um privilégio de escrita. |
| Nenhum painel de host da console | O token não tem `System.Audit` em `/system/status`, ou `node_status` está desligado. |
| "Federated instance behind" nunca dispara | Ela compara instâncias do mesmo produto entre si. Uma console que federa um único cluster Proxmox VE não tem nada com que compará-lo. |
| Totais do acervo são todos travessões | A console respondeu mas não contou nada — geralmente porque toda instância está inacessível. As linhas de instância dizem por quê. |
