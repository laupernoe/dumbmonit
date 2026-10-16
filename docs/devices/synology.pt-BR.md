# Synology DSM {#synology-dsm}

NAS Synology através da API web do DSM: volumes, pools de armazenamento,
saúde de disco, temperatura, tarefas Hyper Backup, tarefas Active Backup
for Business, e Synology Drive e Photos. O primeiro motivo para
monitorar um NAS são seus discos: um disco que esquenta ou cujo status
SMART degrada anuncia uma falha muito antes de um volume cair.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_synology_`.

| Família | Métricas | Rótulos |
|---|---|---|
| Sistema | `system_info` (modelo, versão DSM, firmware…), `uptime_seconds`, `temperature_celsius`, `temperature_warning`, `system_crashed`, `system_need_repair`, `storage_health` | `model`, `dsm_version`, `firmware` |
| CPU e memória | `cpu_usage_percent`, `cpu_user/system/other_percent`, `cpu_cores`, `memory_total/used/available/installed_bytes`, `memory_usage_percent`, `swap_usage_percent` | |
| Rede | `network_rx_bytes_per_second`, `network_tx_bytes_per_second` | `interface` |
| Volumes | `volume_status`, `volume_total/used/available_bytes`, `volume_used_percent`, `volume_used_warning_percent`, `volume_used_critical_percent` | `volume`, `fs_type`, `raid_type` |
| Pools de armazenamento e caches SSD | `pool_status`, `pool_failed_disks`, `pool_total/used_bytes`, e os mesmos quatro sob `ssd_cache_` | `pool` ou `ssd_cache`, `name`, `raid_type` |
| Discos | `disk_status`, `disk_smart_status`, `disk_temperature_celsius`, `disk_size_bytes`, `disk_bad_sector_exceeded`, `disk_life_below_threshold`, `disk_remaining_life_percent` (apenas SSD: a vida que o drive diz que lhe resta), `disk_unc_count` (o contador de setores ilegíveis do DSM, o que ele verifica contra seu limite de setores defeituosos), `disk_info` | `disk`, `name`; em `disk_info`: `model`, `serial`, `vendor`, `firmware`, `type`, `ssd` |
| Hyper Backup | `backup_tasks`, `backup_running`, `backup_last_result`, `backup_last_run_age_seconds`, `backup_last_success_age_seconds`, `backup_next_run_in_seconds` | por tarefa |
| Synology Drive | `drive_up`, `drive_team_folders`, `drive_team_folder_info`, `drive_connections` (veja abaixo) | `drive_team_folder_info`: `name` |
| Synology Photos | `photos_installed` (veja abaixo) | |
| Configurações de segurança | `update_available`, `admin_account_enabled`, `autoblock_enabled`, `firewall_enabled` (`0`/`1`; lidos a cada hora, só com conta de administrador, opcionais: ausentes se o DSM recusar) | |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Métricas do Active Backup for Business são a exceção: têm o prefixo
`dumbmonit_abb_` (veja abaixo).

`volume_status`, `disk_status` e `disk_smart_status` carregam a própria
palavra do DSM em um rótulo (`status`, `smart_status`) e um valor em uma
escala: `0` normal, `1` atenção (um estado transitório como
`repairing` ou `checking`, ou uma palavra que o DSM introduziu e que o
DumbMonit ainda não conhece), `2` crítico (`degrade`, `crashed`,
`failed`…). O desconhecido deliberadamente conta como `1`, nunca `0`.

Uma falha parcial permanece uma sondagem bem sucedida: se o inventário
de armazenamento falha, o estado do sistema e a utilização ainda são
publicados e a falha é contada em `scrape_errors`. Só uma falha na
chamada de descoberta de API ou na conexão condena toda a sondagem.

!!! note "O que as linhas de disco não podem mostrar"
    Contagens de setores realocados vivem na tabela de atributos SMART
    por disco, que o DSM só serve através de uma chamada não documentada
    cuja forma o DumbMonit não conseguiu confirmar. As linhas portanto
    mostram o que `load_info` dá: o veredito de setor defeituoso do DSM,
    seu contador de setores ilegíveis (e seu crescimento), e a vida
    restante do SSD.

!!! note "O que muda entre versões do DSM"
    A mesma chamada não responde os mesmos campos em todo DSM, e o
    DumbMonit lê ambas as formas em vez de assumir uma:

    * `disk_bad_sector_exceeded` vem de `exceed_bad_sector_thr` até o
      DSM 7.1 e de `sb_days_left_critical` — a estimativa do DSM dos
      dias restantes antes do drive atingir seu limite de setores
      defeituosos — no DSM 7.2 e posteriores, onde o primeiro campo
      desapareceu. O mesmo para `disk_life_below_threshold` e
      `remain_life_danger`;
    * `disk_remaining_life_percent` é um número simples no DSM mais
      antigo e um objeto carregando o valor e se o DSM confia nele no
      DSM 7.2 e posteriores. Um valor que o DSM marca como não confiável
      não é publicado;
    * `volume_used_warning_percent` e `volume_used_critical_percent`
      repetem os limites definidos no DSM. O DSM 7.4 não os retorna
      mais, então em um NAS recente essas duas séries simplesmente não
      existem — nada é inventado no lugar.

## A página do monitor {#the-device-page}

![Um monitor Synology: visão geral, volumes, discos e Active Backup](../assets/screenshots/device-synology-light.png){ loading=lazy }

Um monitor Synology recebe seu próprio painel acima dos gráficos:

* **Visão geral** — CPU, memória (usada do total), temperatura e tempo
  de atividade, com o próprio veredito de armazenamento do DSM como uma
  placa (saudável, atenção, travado, reparo necessário);
* **Volumes** — uma linha por volume: barra de uso, usado do total,
  tipo RAID e sistema de arquivos, o estado do volume na palavra do DSM
  (normal, degradado, travado, reparando…);
* **Pools de armazenamento** — uma linha por pool: tipo RAID, o estado
  do pool na palavra do DSM, quantos de seus discos o DSM reporta como
  falhados, e quanto dele já está entregue a volumes. É aqui que a
  redundância vive: um RAID 5 que perde um disco mantém o volume acima
  dele em `normal` enquanto reconstrói, então a linha do volume não diz
  nada e esta diz tudo. Um pool totalmente alocado a seus volumes é o
  estado final normal, não um aviso, então este número é uma frase em
  vez de um medidor colorido;
* **Cache SSD** — uma linha por cache: seu tipo RAID, seu estado e seu
  tamanho. A seção está ausente em um NAS sem cache. O `load_info` do
  DSM não carrega modo de cache nem taxa de acerto, então nenhum dos
  dois é mostrado;
* **Discos** — uma linha por baia: modelo, tamanho, temperatura,
  veredito SMART, veredito de setor defeituoso do DSM, o contador de
  setores ilegíveis e, em um SSD, a vida restante como uma barra
  pequena;
* **Active Backup for Business** — as tarefas, depois todo monitor
  com backup com seu ritmo e uma faixa de trinta dias (veja abaixo).

Tudo vem do que a sondagem armazenou: abrir a página nunca consulta o
NAS.

### Regras integradas para o NAS {#built-in-rules-for-the-nas}

| Regra | Severidade | Dispara quando |
|---|---|---|
| Synology disk SMART warning | aviso | O DSM não reporta mais o status SMART do disco como `normal`, por 15 minutos. |
| Synology disk failed | alerta | O DSM reporta o disco como travado ou falhado, por 5 minutos. |
| Synology disk bad sectors | alerta | A contagem de setores defeituosos excede o limite definido no DSM, por 15 minutos. |
| Synology disk bad sectors growing | aviso | Novos setores ilegíveis apareceram nas últimas 24 horas (`delta(…disk_unc_count[24h]) > 0`). |
| Synology SSD wearing out | aviso | Um SSD tem menos de 10% de sua vida nominal restante, por uma hora. |
| Synology volume degraded | alerta | Um volume está degradado ou travado, por 5 minutos. |
| Synology storage pool degraded | alerta | Um pool de armazenamento ou cache SSD está degradado ou travado, por 5 minutos. Um RAID 5 que perde um disco aparece aqui primeiro: o DSM mantém o volume acima dele em `normal` enquanto reconstrói. |
| Synology volume almost full | aviso | Um volume está 90% cheio ou mais, por 15 minutos. |
| Synology temperature high | aviso | Um disco está acima de 55 °C, ou o DSM disparou seu aviso de temperatura, por 15 minutos. |
| Synology memory high | aviso | Uso de memória acima de 95% por 15 minutos. |

## Active Backup for Business {#active-backup-for-business}

O Active Backup for Business (ABB) faz backup de PCs, servidores
físicos, máquinas virtuais e servidores de arquivos para o NAS. Se o
pacote está instalado e rodando, o DumbMonit lê suas tarefas em toda
sondagem; se não está, o NAS simplesmente não anuncia a API e nada é
perguntado. A opção `abb` desliga toda a família.

O mesmo vale para o Hyper Backup: o DSM só lista as APIs de um pacote
enquanto esse pacote roda, então um NAS cujo Hyper Backup está parado —
ou que só recebe backups via Hyper Backup Vault — não publica métrica
`backup_*` e também não reporta erro. O DumbMonit pergunta ao NAS quais
APIs ele tem antes de chamar qualquer uma delas, então um pacote
faltando é silêncio, nunca uma sondagem falhada.

| Métrica | Valor | Rótulos |
|---|---|---|
| `dumbmonit_abb_tasks` | Número de tarefas. | |
| `dumbmonit_abb_task_last_status` | Última execução: `1` sucesso, `0` falha, `2` em execução, `-1` desconhecido (nunca rodou, cancelada, sem backup). Um sucesso *parcial* conta como falha: pelo menos um monitor não teve backup. | `task`, `task_id`, `source_type`, `result` |
| `dumbmonit_abb_task_last_success_seconds` | Idade da última execução bem sucedida de backup, em segundos. Ausente para uma tarefa que nunca teve sucesso. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_enabled` | `1` se a tarefa tem uma programação e não está pausada, `0` se só roda manualmente. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_device_count` | Monitores conectados à tarefa. | `task`, `task_id`, `source_type` |

`task` é o nome da tarefa como mostrado no ABB; `source_type` é `pc`,
`vm`, `physical_server`, `file_server` ou `nas`; `result` é o resultado
bruto da última execução (`success`, `partial_success`, `fail`,
`cancel`, `no_backup`, `running`, `none`).

A idade é tirada do histórico de tarefas filtrado em execuções de
backup bem sucedidas, não do "último resultado" da tarefa: depois de uma
purga noturna de versões, o último resultado é a purga, não o backup.
Isso custa uma requisição por tarefa, limitado a vinte tarefas.

Regras integradas sobre tarefas:

| Regra | Severidade | Dispara quando |
|---|---|---|
| Active Backup task failed | alerta | A última execução falhou, ou foi um sucesso parcial, por 10 minutos. A notificação nomeia a tarefa. |
| Active Backup too old | aviso | Nenhuma execução bem sucedida por mais de dois dias. |
| Active Backup task disabled | info | A tarefa não tem programação, ou seu backup contínuo está pausado, por uma hora. |

### Monitores e seu ritmo {#devices-and-their-rhythm}

Uma regra no nível de tarefa é certa para um servidor e errada para um
laptop: as pessoas não deixam seus laptops ligados, e "nenhum backup por
24 horas" é segunda-feira de manhã para todo PC de escritório. O
DumbMonit portanto também observa o ABB **monitor por monitor**,
contra os próprios hábitos do monitor.

A cada sondagem, o coletor lê as execuções por monitor que o ABB
mantém (`SYNO.ActiveBackup.Overview` `list_device_transfer_size`, uma
chamada para todos os monitores, só o que mudou desde a sondagem
anterior) e as copia para seu próprio banco de dados — limitado a
noventa dias por NAS — então o modelo sobrevive a um reinício sem
reler um mês de histórico. Depois, para cada monitor, olha os
últimos trinta dias.

| Métrica | Valor | Rótulos |
|---|---|---|
| `dumbmonit_abb_device_state` | `0` ok, `1` idle, `2` aprendendo, `3` em execução, `4` atrasado, `5` falhando, `6` nunca. | `device`, `device_id`, `task`, `task_id`, `state` |
| `dumbmonit_abb_device_overdue` | `1` enquanto o monitor está atrasado, `0` caso contrário. | `device`, `device_id`, `task`, `task_id` |
| `dumbmonit_abb_device_consecutive_failures` | Tentativas falhadas em sequência desde o último sucesso (uma execução cancelada não conta como nenhum dos dois). | mesmos |
| `dumbmonit_abb_device_last_success_seconds` | Idade da última execução bem sucedida deste monitor. Ausente se nunca teve sucesso. | mesmos |
| `dumbmonit_abb_device_typical_interval_seconds` | Intervalo mediano entre dois sucessos, dias de descanso excluídos. Ausente enquanto aprendendo. | mesmos |

#### Como "atrasado" é julgado {#how-overdue-is-judged}

Em palavras simples, para cada monitor:

1. **Em quais dias ele está?** Um dia da semana no qual o monitor
   tentou pelo menos um backup nos últimos trinta dias — com sucesso ou
   não; uma falha prova que a máquina estava ligada tanto quanto um
   sucesso — é um *dia ativo*. Uma vez que existem ao menos duas semanas
   de histórico, um dia da semana sem tentativa alguma é um *dia de
   descanso*. Um laptop usado de segunda a sexta recebe sábado e domingo
   como dias de descanso; um servidor não tem nenhum.
2. **Com que frequência faz backup?** Os intervalos entre sucessos
   consecutivos são coletados em *tempo ativo*: os dias de descanso
   completos que um intervalo contém não são contados (sexta à noite até
   segunda à noite é um dia de tempo ativo, não três). Sua mediana é o
   *intervalo típico* do monitor; seu percentil 90 é como é "uma
   semana lenta".
3. **Quanto tempo é permitido?** A tolerância é
   `max(1.5 × gap p90, 2 × intervalo típico, 36 h)`. Um desktop noturno
   recebe 48 h; uma máquina com backup a cada três dias recebe cerca de
   seis; ninguém recebe menos que um dia e meio. Enquanto existirem
   menos de dois intervalos utilizáveis (um monitor adicionado essa
   semana), a tolerância é 72 h — suficiente para atravessar um fim de
   semana sem um alarme falso.
4. **Quanto tempo ativo passou?** O tempo desde o último sucesso, menos
   os dias de descanso completos que contém. Um laptop com último
   backup sexta às 20:00 tem, na segunda às 08:00, doze horas de tempo
   ativo atrás dele, não sessenta.
5. **O veredito**, nesta ordem: *em execução* (um backup está em
   andamento); *falhando* (as duas últimas ou mais tentativas falharam —
   um sucesso parcial é uma falha, uma execução cancelada não é
   nenhuma das duas); *nunca* (nenhum sucesso registrado); *atrasado*
   (o tempo ativo excede a tolerância); *idle* (hoje é um de seus dias
   de descanso: desligado, como habitual); *aprendendo* (ritmo ainda não
   conhecido); *ok*.

Só *atrasado* dispara `dumbmonit_abb_device_overdue`, e só *falhando*
alimenta a regra de falha. Um laptop idle em um domingo não dispara
nada; o mesmo laptop ainda silencioso na terça à noite dispara.

Dias e horas são expressos no horário local do servidor (`TZ` do
contêiner), que é onde a pessoa lendo a página vive. A página do
monitor mostra, por monitor: o estado em uma palavra, o último
sucesso e último resultado, o ritmo em palavras ("dias de semana, por
volta das 20:00", "a cada ~3 dias, seg, qua, sex, por volta das 03:00",
"ainda aprendendo (2 execuções em 30 dias)") e uma faixa de trinta dias
— verde-água para um dia com sucesso, vermelho para uma falha, amarelo
para uma execução cancelada, cinza para nenhuma execução.

Regras integradas sobre monitores:

| Regra | Severidade | Dispara quando |
|---|---|---|
| Active Backup device overdue | aviso | `dumbmonit_abb_device_overdue > 0` por 30 minutos. |
| Active Backup device failing | alerta | Duas ou mais tentativas falhadas em sequência, por 10 minutos. |

!!! warning "Permissões"
    O ABB só responde a uma conta no grupo `administrators` com o pacote
    permitido em suas permissões de aplicativo, ou uma conta à qual o
    pacote foi delegado (DSM 7: Active Backup for Business → Settings →
    Privileges). Uma conta a que o aplicativo é negado não pode ler
    tarefas ABB: a chamada é recusada (código 105) e contada em
    `scrape_errors`. Ou permita ou delegue o pacote à conta de
    monitoramento, ou desmarque "Observar Active Backup for Business"
    (`abb = false`) para parar de pedir.

!!! note "Uma API não documentada"
    A Synology não documenta a API web do ABB. O DumbMonit segue a
    forma registrada por projetos da comunidade
    (`SYNO.ActiveBackup.Task` `list`, `SYNO.ActiveBackup.Log`
    `list_result` e `SYNO.ActiveBackup.Overview`
    `list_device_transfer_size`, todos versão 1) e descobre seu caminho
    através de `SYNO.API.Info`. Um DSM cujas respostas diferem produz
    métricas faltando, nunca uma sondagem falhada; um ABB muito antigo
    para oferecer a visão geral por monitor mantém suas métricas de
    tarefa e simplesmente não tem ritmo de monitor.

    A versão 1 é deliberada. O DSM 7.4 anuncia essas APIs como versões 1
    a 2, mas só a versão 1 carrega os métodos de leitura: pedidos na
    versão 2 todos respondem "the requested method does not exist", o
    que parece uma diferença de firmware e não é. A mesma resposta
    também volta quando o token CSRF está faltando, então o DumbMonit
    pede um ao DSM no login e o envia em toda chamada.

## Synology Drive {#synology-drive}

O DSM não documenta a API web do Synology Drive. O DumbMonit lê três
chamadas cujo nome e forma vêm da biblioteca comunitária
`N4S4/synology-api`, que as chama das mesmas telas que o console de
administração do Synology Drive, e segue a mesma regra do Active Backup
for Business: uma API que um NAS não anuncia é um pacote que não está
instalado, nunca uma falha.

| Métrica | Valor | Rótulos |
|---|---|---|
| `dumbmonit_synology_drive_up` | `1` enquanto o serviço responde `get_status`. Ausente se o pacote não está instalado. | |
| `dumbmonit_synology_drive_team_folders` | Número de pastas de equipe. | |
| `dumbmonit_synology_drive_team_folder_info` | Valor `1` por pasta de equipe (no máximo 32). | `name` |
| `dumbmonit_synology_drive_connections` | Número de conexões ativas (clientes desktop e mobile atualmente sincronizados). | |

!!! note "Por que as conexões são apenas uma contagem"
    A página "Client list" do console de administração mostra o nome do
    monitor, usuário, tipo de aplicativo, status online, endereço IP
    e localização de cada conexão, mas isso é uma descrição da página,
    não dos campos JSON da API: nem a Synology nem nenhum projeto da
    comunidade documenta o que `SYNO.SynologyDrive.Connection` realmente
    nomeia seus campos. Em vez de adivinhar o nome de um monitor ou
    a data da última sincronização de uma chave não confirmada, o
    DumbMonit publica apenas a contagem de conexões. Monitores
    individuais se conectarão à mesma tabela de
    [dispositivos clientes](../alerting/rules.md) usada por Immich,
    Proxmox Backup Server e Veeam uma vez que essa forma seja confirmada
    a partir da resposta de um NAS real.

A opção `drive` (ativada por padrão) desliga toda a família.

## Synology Photos {#synology-photos}

O Synology Photos entrega uma API de navegação bem documentada, mas nem
a Synology nem as bibliotecas da comunidade expõem uma lista em nível
de administração de monitores de backup móvel ou seu último backup —
o aplicativo móvel mostra seu próprio status de backup localmente, e
nada o centraliza do lado do NAS através de uma API. O DumbMonit
portanto só reporta que o pacote está instalado:
`dumbmonit_synology_photos_installed` (valor `1`), lido do catálogo de
API sem nenhuma requisição extra ao NAS — o mesmo catálogo
`SYNO.API.Info` já responde para todo outro pacote. A opção `photos`
(ativada por padrão) a desliga.

## O que preparar no NAS {#what-to-prepare-on-the-nas}

Os passos abaixo são os que o aviso ao lado do formulário mostra. O
princípio: uma conta reservada para monitoramento, despida de tudo que
a API do DSM não precisa — nunca a conta com a qual você entra.

1. No DSM, abra Control Panel → User & Group → User e clique em Create.
   Nomeie a conta como abaixo e dê a ela uma senha longa usada em
   nenhum outro lugar.

    ```
    dumbmonit
    ```

2. Entre em grupos: marque administrators. O DSM só responde às
   chamadas de armazenamento, volume e SMART de disco a esse grupo; sem
   ele o NAS aparece vivo mas não diz nada sobre seus discos. Os
   próximos dois passos tomam de volta tudo mais.
3. Atribua permissões de pasta compartilhada: Sem acesso em toda pasta
   compartilhada. Atribua permissões de aplicativo: Negue tudo exceto
   DSM, mais Active Backup for Business se você quer que suas tarefas
   sejam lidas. Pule as páginas de cota e limite de velocidade.
4. A verificação em duas etapas precisa permanecer desligada para esta
   conta: nenhum monitor automatizado pode digitar um código único. Se
   Control Panel → Security → Account a impõe, restrinja a regra a
   grupos aos quais este usuário não pertence, ou o isente.
5. Pacotes de backup só são lidos enquanto estão instalados e rodando:
   o DSM não anuncia de forma alguma a API de um pacote parado, então as
   tarefas Hyper Backup ficam silenciosas em vez de falhar, e um NAS
   que só recebe backups (Hyper Backup Vault) não tem tarefas próprias
   para mostrar. O Active Backup for Business adicionalmente só responde
   a uma conta autorizada a usá-lo (Active Backup for Business →
   Settings → Privileges); caso contrário desmarque a opção "Observar
   Active Backup for Business" do DumbMonit para parar de perguntar.
6. No DumbMonit, informe o endereço do NAS (HTTPS, porta 5001 por
   padrão), depois o nome de usuário e senha desta conta.

!!! warning
    O grupo administrators é exigido pela API de armazenamento do DSM,
    não pelo DumbMonit. É por isso que esta conta não recebe nenhuma
    pasta compartilhada, nenhum aplicativo e uma senha usada em nenhum
    outro lugar: ela pode ler o NAS, não tocar seus arquivos.

!!! note "Quais direitos dão quais métricas"
    A API do DSM não precisa dos mesmos direitos para toda chamada.
    Status do sistema (modelo, versão, temperatura, tempo de atividade)
    responde a qualquer conta. CPU, memória, volumes e, acima de tudo,
    status SMART de disco exigem o grupo `administrators`. No DSM 7, a
    delegação "System monitoring" (Control Panel → User & Group →
    Delegation) é suficiente para utilização mas não para o inventário
    de armazenamento. Uma conta sem direitos particulares portanto dá
    um NAS que está "vivo", mas nada sobre seus discos.

Documentação do fornecedor: [Create a user in DSM](https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/file_user_create).

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Conta DSM | **Nome de usuário**: a conta criada acima. **Senha**: sua senha. A verificação em duas etapas precisa estar desligada para esta conta: nenhum monitor automatizado pode digitar um código único. |

Endereço: `192.168.1.30`, `nas.lan` ou `nas.lan:5001`. Sem uma porta,
5001 é usada via HTTPS e 5000 via HTTP.

## Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `scheme` | Protocolo | `https` | HTTPS se encaixa em um NAS novo. HTTP apenas se o DSM escuta só em texto claro. Escolhas: `https`, `http`. |
| `port` | Porta DSM | *(vazio)* | Usada se o endereço não der uma porta. Vazio: 5001 via HTTPS, 5000 via HTTP. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Um NAS vem com um certificado autoassinado por padrão: ative isso se a conexão for recusada por esse motivo. |
| `request_timeout_seconds` | Tempo limite por requisição (segundos) | `15` | Tempo permitido para cada chamada de API, de 1 a 120. O inventário de armazenamento pode despertar discos dormindo. |
| `abb` | Observar Active Backup for Business | `true` | Lê as tarefas do Active Backup for Business. Precisa do pacote instalado e uma conta autorizada a usá-lo; um NAS sem o pacote é simplesmente pulado. |
| `drive` | Observar Synology Drive | `true` | Lê se o serviço Synology Drive responde, suas pastas de equipe e sua contagem de conexões ativas. Um NAS sem o pacote é simplesmente pulado. |
| `photos` | Observar Synology Photos | `true` | Mostra se o pacote Synology Photos está instalado, sem nenhuma requisição extra ao NAS. A Synology não expõe uma lista de dispositivos de backup móvel através de nenhuma API. |

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de autenticação | Senha errada, ou verificação em duas etapas exigida para esta conta. O DSM responde HTTP 200 mesmo em falha; o código de erro no corpo JSON é o que conta, e o DumbMonit o lê. |
| Erro de certificado | Certificado autoassinado: ative `insecure_tls`. Isso é um erro de configuração, nunca "inacessível": um NAS que responde não é mostrado como desligado. |
| Nenhuma métrica de disco ou volume | A conta não está no grupo `administrators` (veja a nota acima). |
| Nenhuma métrica `dumbmonit_abb_*`, `scrape_errors` em 1 | A conta não pode usar o Active Backup for Business: delegue o pacote a ela, ou desative a opção `abb`. Sem o pacote instalado não há erro nem métrica. |
| Métricas de tarefa mas nenhuma linha de monitor na página | O NAS não anuncia `SYNO.ActiveBackup.Overview` (ABB mais antigo), ou o monitor é sondado através de um agente relay, que mantém o histórico em memória de seu lado: as métricas existem, o calendário não. |
| Um laptop mostra "Atrasado" no dia depois de ser adicionado | Menos de três sucessos: o modelo ainda está aprendendo e permite 72 horas entre backups; se estabiliza uma vez que duas semanas de hábitos são conhecidas. |
| Discos ligam a cada sondagem | O inventário de armazenamento desperta discos dormindo. Aumente o intervalo de verificação. |
