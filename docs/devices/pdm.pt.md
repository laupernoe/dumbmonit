# Proxmox Datacenter Manager {#proxmox-datacenter-manager}

A consola que federa vários clusters Proxmox VE e instâncias Proxmox
Backup Server: quais instâncias alcança, todo o património num olhar, as
tarefas que falharam em qualquer lugar, e a própria saúde da consola.

!!! warning "Testado apenas contra uma instância de teste"

    Esta integração foi construída e verificada contra um Proxmox
    Datacenter Manager 1.1 instalado para esse fim, não contra um de
    produção. Toda chamada é uma leitura, e as formas vêm de um
    datacenter manager real, mas ainda ninguém o executou durante
    semanas num datacenter manager em produção. Diga-nos o que falha.

## Um dispositivo para o património, ou um dispositivo por cluster? {#one-device-for-the-estate-or-one-device-per-cluster}

Um dispositivo Datacenter Manager não é um substituto para dispositivos
[Proxmox VE](proxmox.md) e [Proxmox Backup Server](pbs.md). Responde a
uma pergunta diferente.

| | Dispositivo Datacenter Manager | Um dispositivo por cluster e por servidor de cópias de segurança |
|---|---|---|
| Dispositivos a criar | um | um por cluster, um por servidor de cópias de segurança |
| Credenciais a gerir | um token, na consola | um token por instância |
| Carga em cada cluster | nenhuma nossa: lemos a própria cache da consola | uma verificação por intervalo, por cluster |
| O que obtém | acessibilidade, versão, contagens de convidados e nós, totais de CPU, memória e armazenamento, tarefas falhadas | tudo isso mais convidados um a um, snapshots, tarefas de cópia de segurança, HA, Ceph, replicação, discos e SMART, pools ZFS, o calendário de cópias de segurança de 30 dias |
| O que não pode obter | séries por convidado, detalhe por datastore, discos, snapshots, registos de tarefa | um total único à escala do património |
| Falha principal | a consola não consegue alcançar uma instância | o cluster não nos responde |

**A nossa recomendação.** Adicione o dispositivo Datacenter Manager
quando já executa uma consola e quer uma página que diga se o património
está completo — é a forma mais barata de notar que um site remoto caiu, e
não custa nada aos clusters. Adicione um dispositivo dedicado para cada
cluster e servidor de cópias de segurança que realmente opera: é aí que
está a profundidade, e é isso que diz *qual* VM parou ou *qual* cópia de
segurança falhou. Os dois sobrepõem-se sem entrar em conflito; uma cópia
de segurança falhada gera então um alerta do dispositivo PBS (com o
registo da tarefa) e um da consola (nomeando a instância). Silencie a
regra de tarefas da consola se achar isso redundante, ou limite-a às
instâncias para as quais não tem nenhum dispositivo dedicado.

Se não executa nenhuma consola de todo, não instale uma só para
monitorização: os dispositivos dedicados veem estritamente mais.

## A página do dispositivo {#the-device-page}

Além dos gráficos genéricos, um dispositivo Datacenter Manager mostra
três painéis, lidos do que a verificação guardou — abrir a página não
consulta nem a consola nem os clusters que gere:

* **Federated instances** — uma faixa de totais do património
  (instâncias, nós online, convidados em execução e parados, núcleos,
  memória, armazenamento), depois uma linha por instância federada. As
  instâncias que a consola não consegue alcançar ordenam-se primeiro,
  depois as com tarefas falhadas, depois as atrasadas em versão. Cada
  linha dá o produto (Proxmox VE ou Backup Server), a versão, contagens
  de nós e convidados, uso de memória e armazenamento, o estado da
  subscrição, o endereço configurado, quando a consola coletou dela por
  último — e, quando falhou, a mensagem que a própria consola recebeu.
  Essa mensagem é a única coisa que explica a indisponibilidade, por
  isso é mostrada literalmente.
* **Failures** — cada tarefa que falhou em todo o património nos
  últimos 14 dias, a mais recente primeiro, com a instância em que
  correu, o tipo de tarefa, quando começou, quanto demorou e a mensagem
  de erro. O próprio registo da tarefa fica na instância: abra-o aí, ou
  adicione esse cluster como o seu próprio dispositivo, onde *Show log* o
  obtém.
* **Console host** — a máquina que executa a consola: CPU, memória,
  sistema de ficheiros raiz, tempo de atividade e kernel, os seus
  certificados com os dias restantes até expirar, os pacotes à espera de
  atualização, e o estado de subscrição dos nós geridos. O painel
  inteiro é opcional: um token sem `Sys.Audit` não lê nada dele e o
  painel diz isso, em vez de mostrar zeros.

O histórico de tarefas é guardado em SQLite (`pdm_task_history`, 19
dias, no máximo 5000 tarefas por dispositivo) para que a lista de falhas
sobreviva a reinícios ainda que cada verificação só olhe para a janela de
tarefas.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_pdm_`.

| Família | Métricas | Etiquetas |
|---|---|---|
| Património | `remotes_total`, `remotes_failed`, `nodes_online`, `nodes_offline`, `cpu_used_cores`, `cpu_total_cores`, `memory_used_bytes`, `memory_total_bytes`, `memory_used_percent`, `storage_used_bytes`, `storage_total_bytes`, `storage_used_percent`, `datastores_total` | |
| Convidados do património | `guests_running`, `guests_stopped` | `guest_type` (`qemu`, `lxc`) |
| Instâncias | `remote_reachable` (1 a consola obteve uma resposta, 0 não obteve), `remote_version_info` (série de presença, etiqueta `version`), `remote_version_behind`, `remote_nodes_online`, `remote_nodes_offline`, `remote_guests_running`, `remote_guests_stopped`, `remote_cpu_used_cores`, `remote_cpu_total_cores`, `remote_cpu_used_percent`, `remote_memory_used_bytes`, `remote_memory_total_bytes`, `remote_memory_used_percent`, `remote_storage_used_bytes`, `remote_storage_total_bytes`, `remote_storage_used_percent`, `remote_datastores`, `remote_subscription_active`, `remote_updates_pending`, `remote_last_collection_age_seconds`, `remote_tasks_failed` | `remote`, `type` (`pve`, `pbs`) |
| Tarefas na janela | `tasks_total`, `tasks_failed`, `tasks_running` | |
| Anfitrião da consola | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`, `node_load1`, `node_memory_used_bytes`, `node_memory_total_bytes`, `node_memory_used_percent`, `node_swap_used_bytes`, `node_swap_total_bytes`, `node_rootfs_used_bytes`, `node_rootfs_total_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `node_updates_pending`, `version_info` | |
| Certificados da consola | `node_certificate_expiry_days` (negativo depois de expirado) | `file` (`proxy.pem`) |
| Subscrição | `subscription_active` (etiqueta `status`), `subscription_active_nodes`, `subscription_total_nodes` | |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Uma tarefa que termina em `WARNINGS: n` conta como bem-sucedida. Só a
falha de `GET /version` condena toda a verificação; qualquer outra
chamada falhada é contada em `scrape_errors`, e uma instância que a
consola não consegue alcançar é uma medição (`remote_reachable = 0`),
não um erro de coleta.

`remote_version_behind` compara as instâncias federadas **entre si**,
produto a produto: é 1 quando outro cluster Proxmox VE — ou outro
servidor de cópias de segurança — no mesmo património corre uma versão
mais recente. Não é uma comparação contra o que o Proxmox publica; uma
verificação não tem motivo para contactar a internet. A série só existe
para uma instância cuja versão foi lida, por isso uma instância
inacessível nunca passa por atualizada.

Os totais de armazenamento contam cada nome de armazenamento uma vez por
instância: um armazenamento partilhado por cinco nós de um cluster é
contado uma vez, não cinco.

As regras incorporadas que se aplicam: Device unreachable, Federated
instance unreachable, Federated instance behind, Task failed on a
federated instance, Datacenter console disk almost full, Datacenter
console certificate expiring, Datacenter console updates pending. As
notificações nomeiam a instância em causa.

## O que preparar no Proxmox Datacenter Manager {#what-to-prepare-in-proxmox-datacenter-manager}

Os passos abaixo são os que o aviso junto ao formulário mostra. O
princípio: um utilizador reservado para monitorização, um token com uma
função só de leitura — nunca a conta com que inicia sessão.

1. Na consola, abra Configuration → Access Control → Users e clique em Add. Dê o seguinte nome à conta e deixe a senha vazia: o token é que inicia sessão.

    ```
    dumbmonit@pdm
    ```

2. Ainda em Access Control, abra API Tokens → Add, escolha esse utilizador e dê ao token o nome «monitor». A consola mostra o segredo uma vez: copie-o agora.

    ```
    dumbmonit@pdm!monitor
    ```

3. Dê tanto ao utilizador como ao token a função `Auditor` em `/` (o topo da árvore, logo todo o património). `Auditor` transporta `System.Audit`, `Resource.Audit` e `Access.Audit`, e não pode alterar nada. Um token nunca tem mais direitos do que o seu utilizador, por isso a permissão é concedida duas vezes: Permissions → Add → User Permission, depois de novo com API Token Permission.
4. Copie o id do token para o campo Token ID do DumbMonit e o segredo para o seu campo Secret.
5. No DumbMonit, introduza o endereço da consola, por exemplo «dc.lan» ou «dc.lan:8443».

!!! warning
    Não reutilize a conta com que inicia sessão: um token exposto controlaria então todo cluster que a consola federa de uma vez. A função `Auditor` só pode ler. A consola também usa um certificado autoassinado por predefinição: se a ligação for recusada por esse motivo, marque «Accept an unverifiable certificate» nas opções. Dois itens precisam de mais do que `Auditor` e são simplesmente ignorados sem isso: o estado de coleta de métricas por instância, e o resumo de atualizações das instâncias federadas.

Prefere a linha de comandos? `proxmox-datacenter-manager-admin` gere
remotes, não utilizadores; crie a conta, o token e as duas permissões na
interface web.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de API (recomendado) | **Token ID**: utilizador, realm e nome do token como a consola os mostra, `dumbmonit@pdm!monitor`. **Secret**: o UUID mostrado uma vez quando o token foi criado. O DumbMonit envia-os como o cabeçalho `PDMAPIToken=user@pdm!name:secret` que a consola espera. |

Não há opção de nome de utilizador / senha, e isso é deliberado: desde a
sua primeira versão a consola só devolve o seu bilhete de sessão num
cookie `HttpOnly` — o corpo JSON transporta um `ticket-info` inutilizável.
Um início de sessão não compraria nada em relação a um token enquanto
abriria uma sessão registada em cada verificação. Os tokens são o que o
Proxmox recomenda para automação de qualquer forma.

Direitos, só de leitura. Mais simples e suficiente: a função incorporada
`Auditor` em `/`, concedida ao utilizador *e* ao token. Por endpoint:

| Endpoint | Precisa de |
|---|---|
| `GET /version` | qualquer utilizador autenticado |
| `GET /remotes/remote` | devolve as instâncias que o chamador pode auditar |
| `GET /remotes/remote/{id}/version` | `Resource.Audit` em `/resource/{id}` |
| `GET /resources/status`, `GET /resources/list` | `Resource.Audit` em `/resource` |
| `GET /resources/subscription` | `Resource.Audit` em `/resource` |
| `GET /remotes/tasks/list` | `Resource.Audit` em `/resource/{remote}`, por instância |
| `GET /nodes/{node}/status` | `System.Audit` em `/system/status` |
| `GET /nodes/{node}/certificates/info`, `GET /nodes/{node}/subscription` | qualquer utilizador autenticado |
| `GET /nodes/{node}/apt/update` | `System.Audit`; um 403 é tolerado: nenhuma série `node_updates_pending`, nenhum erro de coleta |
| `GET /remotes/metric-collection/status` | mais do que `Auditor`; um 403 é tolerado: nenhuma série `remote_last_collection_age_seconds`, nenhum erro de coleta |
| `GET /remotes/updates/summary` | `Resource.Modify` — um privilégio de escrita. Desligado por predefinição (`remote_updates`) por essa razão |

Note que a consola oculta os tokens de acesso que detém para as suas
instâncias (`GET /remotes/remote` devolve um `token` vazio), e o
DumbMonit nunca lê esse campo de qualquer forma: nenhum segredo de
instância passa alguma vez pelo coletor, pela vista guardada ou pela API.

Endereço: `10.0.0.40`, `dc.lan`, `dc.lan:8443`, `[fd00::2]` ou um URL
completo. O esquema `https` e a porta 8443 são adicionados se estiverem
em falta.

## Opções {#options}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | API port | `8443` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | O Proxmox Datacenter Manager vem com um certificado autoassinado por predefinição: ative isto se a ligação for recusada por esse motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `20` | Tempo permitido para cada chamada de API, de 1 a 120. A consola reencaminha chamadas para cada instância federada, por isso um site lento retém toda a resposta. |
| `node` | Console node name | `localhost` | Nome do nó que executa a consola. Um Datacenter Manager tem um único nó e chama-lhe «localhost». |
| `task_lookback_hours` | Task window (hours) | `24` | Tarefas mais antigas não são contadas entre as falhas, de 1 a 8760. |
| `max_age_seconds` | Accepted cache age (seconds) | `60` | Quão obsoleto o próprio inventário da consola pode ser antes de consultar de novo as instâncias federadas, de 0 a 3600. Zero pede dados frescos em cada verificação, o que coloca todo cluster de novo na linha. |
| `remotes` | Monitored instances | *(vazio)* | Nomes das instâncias federadas a monitorizar, separados por vírgulas. Vazio: todas as instâncias. |
| `max_remotes` | Instance version limit | `100` | Número máximo de instâncias a que se pergunta a versão em cada verificação, de 1 a 1000. |
| `versions` | Read each instance version | `true` | Uma chamada por instância federada, para reportar a sua versão e marcar as que ficaram atrás dos seus pares. Uma instância que não responde conta como inacessível. |
| `tasks` | Watch tasks across instances | `true` | Lê a lista de tarefas de cada instância federada para reportar as falhas. |
| `node_status` | Watch the console host | `true` | Lê a CPU, memória, sistema de ficheiros raiz, tempo de atividade, certificados e subscrição da máquina que executa a consola. |
| `updates` | Count pending updates | `true` | Lista os pacotes à espera de atualização na própria consola. |
| `remote_updates` | Count updates on each instance | `false` | Lê o resumo de atualizações da consola para as instâncias federadas. Desligado por predefinição: a consola protege esse caminho com `Resource.Modify`, um privilégio de escrita que uma conta de monitorização não tem motivo para ter. |

`max_age_seconds` é a que vale a pena entender. A consola mantém o seu
próprio inventário, atualizado pelo seu próprio ciclo de coleta. Pedir
dados frescos em cada verificação faz a consola chamar cada cluster de
novo, por cima do seu ritmo normal — exatamente a carga que um único
dispositivo de consola deveria evitar. Sessenta segundos ainda é mais
fresco do que o intervalo de verificação predefinido.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de certificado | Certificado autoassinado: instale um de confiança (ACME está incorporado na consola) ou ative `insecure_tls`. |
| Erro de autenticação | O segredo colado em Token ID (ou o inverso), ou a permissão foi concedida ao utilizador mas não ao token — um token nunca tem mais direitos do que o seu utilizador, e as duas ACLs são entradas separadas. |
| Toda instância mostra-se inacessível, mas a própria página da consola está bem | O token tem `Auditor` no utilizador mas não no token, ou só em `/system`: `Resource.Audit` em `/resource` é o que deixa a consola responder pelas suas instâncias. |
| Uma instância está inacessível e a mensagem menciona uma impressão digital | A consola verifica o certificado de cada instância que gere; corrija-o na consola (Remotes → a instância), não aqui. |
| Nenhuma série `remote_last_collection_age_seconds` | `GET /remotes/metric-collection/status` precisa de mais do que `Auditor` e é ignorado silenciosamente. Nada mais é afetado. |
| Nenhuma série `remote_updates_pending` | `remote_updates` está desligado por predefinição porque esse caminho precisa de `Resource.Modify`. Ative-o só se estiver disposto a conceder um privilégio de escrita. |
| Nenhum painel de anfitrião da consola | O token não tem `System.Audit` em `/system/status`, ou `node_status` está desligado. |
| «Federated instance behind» nunca dispara | Compara instâncias do mesmo produto entre si. Uma consola que federa um único cluster Proxmox VE não tem nada com que o comparar. |
| Os totais do património são todos travessões | A consola respondeu mas não contou nada — geralmente porque toda instância está inacessível. As linhas de instância dizem porquê. |
