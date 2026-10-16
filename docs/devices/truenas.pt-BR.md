# TrueNAS {#truenas}

Servidor de armazenamento ZFS (TrueNAS SCALE, agora TrueNAS Community
Edition): a saúde de cada pool e o disco que falhou, scrubs e
resilvers, uso de pool e dataset contra suas cotas, snapshots e
replicação, temperatura de disco e autotestes SMART, os alertas que o
próprio TrueNAS dispara, serviços e a máquina.

!!! warning "Testado apenas contra uma instância de teste"

    Esta integração foi construída e verificada contra o TrueNAS
    25.04.2.6, instalado a partir da ISO oficial em uma máquina virtual
    para esse fim — um pool RAIDZ1, datasets, tarefas de snapshot e
    replicação, depois o mesmo pool com um disco colocado offline — não
    contra um NAS que vem guardando as fotos de uma família por anos. Uma
    máquina virtual não tem temperaturas de disco nem resultados SMART,
    então esses seguem o código-fonte do TrueNAS em vez de uma resposta
    observada. Toda chamada é uma leitura. Avise-nos o que não funcionar.

## A falha para a qual isso existe {#the-failure-this-is-for}

Um vdev mirror ou RAIDZ que perde um disco continua servindo os dados. O
compartilhamento permanece montado, backups continuam rodando, a
interface web de toda outra ferramenta permanece verde — e o pool agora
não tem redundância alguma. O segundo disco leva o pool.
`truenas_pool_healthy` cai para 0 no primeiro, a regra **NAS pool
degraded** dispara, e a página do monitor nomeia o disco.

## A página do monitor {#the-device-page}

Além dos gráficos genéricos, um monitor TrueNAS mostra três painéis,
lidos do que a sondagem armazenou — abrir a página nunca consulta o
próprio NAS:

* **Pools e discos** — cada pool com seu estado em uma palavra, a própria
  frase do ZFS quando algo está errado, quão cheio está, seu layout
  (mirror, RAIDZ, cache, log), suas contagens de erro de leitura,
  escrita e checksum, e acima de tudo o nome de qualquer disco que não
  esteja `ONLINE`. Um scrub ou resilver em andamento mostra seu
  progresso. Depois todo disco: modelo, número de série, tipo, tamanho,
  pool, temperatura e o resultado de seu último autoteste SMART.
* **Datasets e proteção** — cada dataset com seu uso contra sua cota, sua
  contagem de snapshot e quando um job de snapshot o cobriu por último;
  o último scrub de cada pool e se está atrasado; toda tarefa de
  replicação e snapshot periódico com seu estado, sua última execução, o
  último snapshot que tratou e, quando falhou, a própria frase de erro
  do TrueNAS.
* **Saúde do NAS** — os alertas que o próprio TrueNAS disparou, mais
  severo primeiro, depois os serviços parados, depois a máquina: versão,
  tempo de atividade, carga, memória, CPU.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_truenas_`.

| Família | Métricas | Rótulos |
|---|---|---|
| Identidade | `version_info` (a versão, sem o prefixo `TrueNAS-SCALE-` que versões antigas colocavam na frente) | `version` |
| Máquina | `uptime_seconds`, `load1/5/15`, `memory_total_bytes`, `cpu_count` | |
| Pools | `pool_healthy` (o próprio veredito do ZFS: 0 em `DEGRADED` como em `FAULTED`), `pool_warning` (resilvering, recursos não ativados), `pool_status_info`, `pool_size/allocated/free_bytes`, `pool_used_percent`, `pool_fragmentation_percent` | `pool`, e `status` para a série info |
| Monitores do pool | `pool_device_errors` (acumulado desde o último `zpool clear`), `pool_devices_unhealthy` (ambos ausentes quando o pool não pode ser aberto) | `pool`, `kind` (`read`, `write`, `checksum`) |
| Scrubs | `pool_scan_running`, `pool_scan_percent` (só enquanto em execução), `pool_last_scrub_age_seconds`, `pool_last_scrub_errors`, `pool_scrub_threshold_days` | `pool`, `function` (`SCRUB`, `RESILVER`) |
| Datasets | `dataset_used_bytes`, `dataset_available_bytes`, `dataset_quota_bytes` e `dataset_quota_used_percent` (ausente sem uma cota), `dataset_snapshots`, `dataset_snapshot_age_seconds` (idade do último snapshot feito por uma tarefa periódica que cobre o dataset; ausente quando nenhuma o faz), `dataset_locked` (apenas datasets criptografados) | `dataset`, `pool` |
| Snapshots | `snapshots` (total) | |
| Discos | `disk_temperature_celsius`, `disk_size_bytes`, `disk_smart_failed` (ausente para um disco nunca testado) | `disk`, `serial` |
| Alertas | `alerts` — o número de alertas ativos por nível, zero incluído | `level` (`INFO` … `EMERGENCY`) |
| Tarefas | `replication_error`, `replication_last_run_age_seconds`, `snapshot_task_error`, `snapshot_task_last_run_age_seconds` (apenas tarefas ativadas) | `task` |
| Serviços | `service_running` (apenas serviços configurados para iniciar com o NAS) | `service` |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Algumas coisas que o TrueNAS faz e vale a pena conhecer:

* **O último scrub vive apenas no registro de varredura do pool.** Não há
  histórico de scrub na API, e um resilver sobrescreve o registro de
  varredura: depois de um resilver, a data do último scrub é
  simplesmente desconhecida até o próximo terminar, e
  `pool_last_scrub_age_seconds` fica ausente em vez de errada.
* **Um pool que não pode ser aberto** — exportado, ou com muitos discos
  faltando — ainda aparece, sem layout e sem tamanhos. Seu
  `pool_healthy` é 0; as séries de monitor e tamanho ficam ausentes
  em vez de zero.
* **Uma cota não definida não é uma cota de zero.** O TrueNAS responde
  `parsed: null` para "sem cota" e o DumbMonit não publica série de cota
  para esse dataset. `quota` tem precedência sobre `refquota` quando
  ambos estão definidos.
* **Contagens de snapshot vêm do próprio cache do ZFS**
  (`snapshots_count`), e o total da contagem rápida de snapshot do
  TrueNAS — nenhum snapshot é jamais listado. O total é maior que a soma
  das contagens por dataset: inclui os snapshots dos datasets internos
  do TrueNAS, que a lista de datasets escondem.
* **A idade do snapshot mais recente de um dataset** é a última execução
  da tarefa de snapshot periódico que o cobre, diretamente ou através de
  um pai recursivo. O TrueNAS não pode dar a data do snapshot mais
  recente sem listá-los todos, então um dataset que nenhuma tarefa cobre
  não tem idade — o que já é algo que vale a pena saber.
* **Temperaturas são lidas do cache do TrueNAS** (`only_cached`),
  atualizado pelo TrueNAS a cada cinco minutos ou mais. Um disco
  dormindo nunca é despertado, e um disco sem leitura não tem série.
* **Nada é jamais iniciado.** Nenhum scrub, nenhum teste SMART, nenhuma
  verificação de atualização, nenhuma execução de replicação. A
  sondagem lê.
* **Caminhos que mudaram entre versões são opcionais.** `zfs/snapshot`
  se tornou `pool/snapshot` na 25.10 e ambos são tentados;
  `smart/test/results` desapareceu na 25.10 e é então pulado em
  silêncio.

Regras integradas que se aplicam: Monitor unreachable, NAS pool degraded,
NAS disk errors, NAS pool almost full, NAS scrub found errors, NAS scrub
overdue, NAS disk failed its SMART test, NAS disk too hot, NAS dataset
near its quota, NAS replication failed, NAS snapshot task failed, NAS
snapshots stale, TrueNAS alert raised, NAS service stopped. Notificações
nomeiam o pool, disco, dataset, tarefa ou serviço envolvido.

## O que preparar no TrueNAS {#what-to-prepare-in-truenas}

Os passos abaixo são os que o aviso ao lado do formulário mostra. Uma
coisa a saber primeiro: pela API REST que o DumbMonit usa, **o TrueNAS
não tem chave somente leitura**. Seus papéis somente leitura só foram
conectados em sua API WebSocket mais nova; uma chave cujo usuário é um
Read-Only Administrator é recusada, com um `403` vazio, em toda chamada
REST. Isso foi verificado no TrueNAS 25.04.2.6: uma chave Read-Only
Administrator recebeu `403` em todos os 55 caminhos REST testados, e a
mesma chave funcionou pela API WebSocket. A chave precisa pertencer a um
usuário com o privilégio Local Administrator — então ela recebe um
usuário e um grupo próprios, e nada mais.

1. Na interface web: Credentials → Groups → Add. Nomeie o grupo como
   abaixo e dê a ele o privilégio Local Administrator.

    ```
    dumbmonit
    ```

2. Por que Local Administrator e não Read-Only Administrator: o TrueNAS
   conectou seus papéis somente leitura apenas em sua API WebSocket. Na
   API REST que o DumbMonit usa, uma chave somente leitura é recusada em
   toda chamada única. O próprio DumbMonit sempre só lê: nunca inicia um
   scrub, um teste SMART, uma atualização ou uma replicação.

3. Credentials → Users → Add. Nomeie a conta como abaixo, faça dumbmonit
   seu grupo primário, e deixe acesso shell, sudo e SSH desligados:
   nenhum deles é necessário.

    ```
    dumbmonit
    ```

4. Crie a chave: Credentials → Users → API Keys → Add, escolha o usuário
   dumbmonit e dê à chave um nome. O TrueNAS mostra a chave apenas uma
   vez: copie-a. No TrueNAS 24.10 o caminho é o menu do usuário no canto
   superior direito → API Keys → Add, e uma chave não pertence a nenhum
   usuário.

5. No DumbMonit, informe o endereço do NAS, por exemplo "nas.lan", e cole
   a chave. Mantenha HTTPS: o TrueNAS revoga uma chave que recebe uma vez
   via HTTP simples de outra máquina.

!!! warning

    Esta chave pode alterar qualquer coisa no NAS: o TrueNAS não oferece
    chave somente leitura via sua API REST. Trate-a como a senha de uma
    conta que pode apagar seus pools, mantenha-a só para o DumbMonit e
    revogue-a da mesma página se ela for exposta. O TrueNAS também vem
    com um certificado autoassinado: se a conexão for recusada por esse
    motivo, marque "Aceitar um certificado não verificável" nas opções.

Endereço: `192.168.1.20`, `nas.lan`, `nas.lan:8443`, `[fd00::20]` ou uma
URL completa. O esquema `https` e a porta 443 são adicionados se
faltando.

O TrueNAS anunciou que a API REST será removida no TrueNAS 26, a favor da
API WebSocket. Até então, REST é o que toda versão até a 25.10 serve.

## Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `scheme` | Protocolo | `https` | Mantenha HTTPS: o TrueNAS revoga uma chave de API que recebe via HTTP simples de outra máquina. |
| `port` | Porta da interface web | `443` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | O TrueNAS vem com um certificado autoassinado por padrão. |
| `request_timeout_seconds` | Tempo limite por requisição (segundos) | `20` | Tempo permitido para cada chamada de API, de 1 a 120. |
| `datasets` | Observar os datasets | `true` | Uso contra cota, contagem de snapshot, última mudança de snapshot. |
| `disks` | Observar os discos | `true` | Inventário e temperaturas em cache; um disco dormindo nunca é despertado. |
| `smart` | Ler os resultados dos testes SMART | `true` | O último autoteste de cada disco; nenhum é jamais iniciado. |
| `alerts` | Ler os próprios alertas do TrueNAS | `true` | A lista de alertas que o TrueNAS mantém, alertas descartados deixados de fora. |
| `tasks` | Observar tarefas de replicação e snapshot | `true` | Estado, última execução e último snapshot de cada tarefa. |
| `services` | Observar os serviços | `true` | Serviços configurados para iniciar com o NAS, e se estão rodando. |

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de autenticação mencionando Local Administrator | O usuário da chave não é um administrador completo — tipicamente um Read-Only Administrator. O TrueNAS recusa tal chave em toda chamada REST, com um `403` vazio. |
| Erro de autenticação "Invalid API key" | A chave foi digitada errada, excluída, expirou — ou revogada pelo TrueNAS porque foi enviada uma vez via HTTP simples de outra máquina. Crie uma nova e mantenha HTTPS. |
| Erro de certificado | Certificado autoassinado: instale um confiável (ACME está embutido) ou ative `insecure_tls`. |
| Nenhuma temperatura de disco | O NAS roda em uma máquina virtual, ou o TrueNAS ainda não sondou os discos desde que iniciou. Leituras vêm de seu cache. |
| Nenhuma série SMART | Nenhum autoteste jamais rodou nesse disco, ou o NAS roda TrueNAS 25.10, que removeu essa chamada. Agende testes SMART em Data Protection. |
| Nenhum `pool_last_scrub_age_seconds` | A última varredura daquele pool foi um resilver, que apaga o registro do último scrub, ou o pool nunca foi escaneado. |
| Um dataset não tem série de cota | Ele não tem cota. Uma cota não definida não é uma cota de zero. |
| "NAS pool degraded" e a página não mostra nome de disco | O pool não pode ser aberto de forma alguma (exportado, ou muitos discos faltando): o TrueNAS então não reporta layout. |
| `scrape_errors` acima de zero | Uma chamada opcional falhou por um motivo que não é um caminho faltando — um tempo esgotado, geralmente. As outras métricas não são afetadas. |
