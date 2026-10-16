# Synology DSM {#synology-dsm}

Um NAS Synology através da API web do DSM: volumes, pools de
armazenamento, saúde dos discos, temperatura, tarefas Hyper Backup,
tarefas Active Backup for Business, e Synology Drive e Photos. A
primeira razão para monitorizar um NAS são os seus discos: um disco
que aquece ou cujo estado SMART degrada anuncia uma falha muito antes
de um volume cair.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_synology_`.

| Família | Métricas | Etiquetas |
|---|---|---|
| Sistema | `system_info` (modelo, versão DSM, firmware…), `uptime_seconds`, `temperature_celsius`, `temperature_warning`, `system_crashed`, `system_need_repair`, `storage_health` | `model`, `dsm_version`, `firmware` |
| CPU e memória | `cpu_usage_percent`, `cpu_user/system/other_percent`, `cpu_cores`, `memory_total/used/available/installed_bytes`, `memory_usage_percent`, `swap_usage_percent` | |
| Rede | `network_rx_bytes_per_second`, `network_tx_bytes_per_second` | `interface` |
| Volumes | `volume_status`, `volume_total/used/available_bytes`, `volume_used_percent`, `volume_used_warning_percent`, `volume_used_critical_percent` | `volume`, `fs_type`, `raid_type` |
| Pools de armazenamento e caches SSD | `pool_status`, `pool_failed_disks`, `pool_total/used_bytes`, e os mesmos quatro sob `ssd_cache_` | `pool` ou `ssd_cache`, `name`, `raid_type` |
| Discos | `disk_status`, `disk_smart_status`, `disk_temperature_celsius`, `disk_size_bytes`, `disk_bad_sector_exceeded`, `disk_life_below_threshold`, `disk_remaining_life_percent` (só SSD: a vida que a unidade diz ter restante), `disk_unc_count` (o contador de setores ilegíveis do DSM, o que verifica contra o seu limiar de setores defeituosos), `disk_info` | `disk`, `name`; em `disk_info`: `model`, `serial`, `vendor`, `firmware`, `type`, `ssd` |
| Hyper Backup | `backup_tasks`, `backup_running`, `backup_last_result`, `backup_last_run_age_seconds`, `backup_last_success_age_seconds`, `backup_next_run_in_seconds` | por tarefa |
| Synology Drive | `drive_up`, `drive_team_folders`, `drive_team_folder_info`, `drive_connections` (ver abaixo) | `drive_team_folder_info`: `name` |
| Synology Photos | `photos_installed` (ver abaixo) | |
| Definições de segurança | `update_available`, `admin_account_enabled`, `autoblock_enabled`, `firewall_enabled` (`0`/`1`; lidos de hora a hora, só com conta de administrador, opcionais: ausentes se o DSM recusar) | |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

As métricas do Active Backup for Business são a excepção: têm o
prefixo `dumbmonit_abb_` (ver abaixo).

`volume_status`, `disk_status` e `disk_smart_status` transportam a
própria palavra do DSM numa etiqueta (`status`, `smart_status`) e um
valor numa escala: `0` normal, `1` atenção (um estado transitório
como `repairing` ou `checking`, ou uma palavra que o DSM introduziu e
que o DumbMonit ainda não conhece), `2` crítico (`degrade`,
`crashed`, `failed`…). O desconhecido conta deliberadamente como `1`,
nunca `0`.

Uma falha parcial continua a ser uma verificação bem-sucedida: se o
inventário de armazenamento falha, o estado do sistema e a utilização
continuam a ser publicados e a falha é contada em `scrape_errors`. Só
uma falha na chamada de descoberta de API ou na ligação condena toda a
verificação.

!!! note "O que as linhas de disco não conseguem mostrar"
    As contagens de setores realocados vivem na tabela de atributos
    SMART por disco, que o DSM só serve através de uma chamada não
    documentada cuja forma o DumbMonit não conseguiu confirmar. As
    linhas mostram portanto o que `load_info` dá: o veredicto de
    setores defeituosos do DSM, o seu contador de setores ilegíveis (e
    o seu crescimento), e a vida restante do SSD.

!!! note "O que muda entre versões do DSM"
    A mesma chamada não responde aos mesmos campos em todo DSM, e o
    DumbMonit lê ambas as formas em vez de assumir uma:

    * `disk_bad_sector_exceeded` vem de `exceed_bad_sector_thr` até ao
      DSM 7.1 e de `sb_days_left_critical` — a estimativa do DSM dos
      dias restantes antes de a unidade atingir o seu limiar de
      setores defeituosos — no DSM 7.2 e posterior, onde o primeiro
      campo desapareceu. O mesmo para `disk_life_below_threshold` e
      `remain_life_danger`;
    * `disk_remaining_life_percent` é um número simples em DSM mais
      antigos e um objeto que transporta o valor e se o DSM o
      confia no DSM 7.2 e posterior. Um valor que o DSM marca como não
      confiável não é publicado;
    * `volume_used_warning_percent` e `volume_used_critical_percent`
      repetem os limiares definidos no DSM. O DSM 7.4 já não os
      devolve, por isso num NAS recente essas duas séries simplesmente
      não existem — nada é inventado no seu lugar.

## A página do monitor {#the-device-page}

![Um monitor Synology: num olhar, volumes, discos e Active Backup](../assets/screenshots/device-synology-light.png){ loading=lazy }

Um monitor Synology recebe o seu próprio painel acima dos
gráficos:

* **At a glance** — CPU, memória (usada do total), temperatura e
  tempo de atividade, com o próprio veredicto de armazenamento do DSM
  como uma placa (saudável, atenção, avariado, reparação necessária);
* **Volumes** — uma linha por volume: barra de utilização, usado do
  total, tipo RAID e sistema de ficheiros, o estado do volume na
  palavra do DSM (normal, degradado, avariado, a reparar…);
* **Storage pools** — uma linha por pool: tipo RAID, o estado da pool
  na palavra do DSM, quantos dos seus discos o DSM reporta como
  falhados, e quanto dela já está entregue a volumes. É aqui que vive
  a redundância: um RAID 5 que perde um disco mantém o volume por
  cima dele em `normal` enquanto reconstrói, por isso a linha do
  volume não diz nada e esta diz tudo. Uma pool totalmente alocada aos
  seus volumes é o estado final normal, não um aviso, por isso este
  valor é uma frase em vez de um medidor colorido;
* **SSD cache** — uma linha por cache: o seu tipo RAID, o seu estado e
  o seu tamanho. A secção está ausente num NAS sem cache. O
  `load_info` do DSM não transporta nenhum modo de cache ou taxa de
  acerto, por isso nenhum é mostrado;
* **Disks** — uma linha por compartimento: modelo, tamanho,
  temperatura, veredicto SMART, veredicto de setores defeituosos do
  DSM, o contador de setores ilegíveis e, num SSD, a vida restante
  como uma pequena barra;
* **Active Backup for Business** — as tarefas, depois todo
  dispositivo com cópia de segurança com o seu ritmo e uma faixa de
  trinta dias (ver abaixo).

Tudo vem do que a verificação guardou: abrir a página nunca consulta o
NAS.

### Regras incorporadas para o NAS {#built-in-rules-for-the-nas}

| Regra | Severidade | Dispara quando |
|---|---|---|
| Synology disk SMART warning | advisory | O DSM já não reporta o estado SMART do disco como `normal`, durante 15 minutos. |
| Synology disk failed | warning | O DSM reporta o disco como avariado ou falhado, durante 5 minutos. |
| Synology disk bad sectors | warning | A contagem de setores defeituosos excede o limiar definido no DSM, durante 15 minutos. |
| Synology disk bad sectors growing | advisory | Novos setores ilegíveis apareceram nas últimas 24 horas (`delta(…disk_unc_count[24h]) > 0`). |
| Synology SSD wearing out | advisory | Um SSD tem menos de 10% da sua vida nominal restante, durante uma hora. |
| Synology volume degraded | warning | Um volume está degradado ou avariado, durante 5 minutos. |
| Synology storage pool degraded | warning | Uma pool de armazenamento ou cache SSD está degradada ou avariada, durante 5 minutos. Um RAID 5 que perde um disco mostra-se aqui primeiro: o DSM mantém o volume por cima dele em `normal` enquanto reconstrói. |
| Synology volume almost full | advisory | Um volume está 90% cheio ou mais, durante 15 minutos. |
| Synology temperature high | advisory | Um disco está acima de 55 °C, ou o DSM levantou o seu aviso de temperatura, durante 15 minutos. |
| Synology memory high | advisory | Uso de memória acima de 95% durante 15 minutos. |

## Active Backup for Business {#active-backup-for-business}

O Active Backup for Business (ABB) faz cópia de segurança de PCs,
servidores físicos, máquinas virtuais e servidores de ficheiros para
o NAS. Se o pacote estiver instalado e em execução, o DumbMonit lê as
suas tarefas em cada verificação; se não estiver, o NAS simplesmente
não anuncia a API e nada é pedido. A opção `abb` desliga toda a
família.

O mesmo se aplica ao Hyper Backup: o DSM só lista as APIs de um
pacote enquanto esse pacote corre, por isso um NAS cujo Hyper Backup
está parado — ou que só recebe cópias de segurança através do Hyper
Backup Vault — não publica nenhuma métrica `backup_*` e também não
reporta nenhum erro. O DumbMonit pergunta ao NAS que APIs tem antes de
chamar qualquer uma delas, por isso um pacote em falta é silêncio,
nunca uma verificação falhada.

| Métrica | Valor | Etiquetas |
|---|---|---|
| `dumbmonit_abb_tasks` | Número de tarefas. | |
| `dumbmonit_abb_task_last_status` | Última execução: `1` sucesso, `0` falhada, `2` em execução, `-1` desconhecido (nunca correu, cancelada, sem cópia de segurança). Um sucesso *parcial* conta como falhado: pelo menos um dispositivo não teve cópia de segurança. | `task`, `task_id`, `source_type`, `result` |
| `dumbmonit_abb_task_last_success_seconds` | Idade da última execução bem-sucedida, em segundos. Ausente para uma tarefa que nunca teve sucesso. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_enabled` | `1` se a tarefa tem um calendário e não está pausada, `0` se só corre manualmente. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_device_count` | Dispositivos ligados à tarefa. | `task`, `task_id`, `source_type` |

`task` é o nome da tarefa como mostrado no ABB; `source_type` é `pc`,
`vm`, `physical_server`, `file_server` ou `nas`; `result` é o
resultado em bruto da última execução (`success`, `partial_success`,
`fail`, `cancel`, `no_backup`, `running`, `none`).

A idade vem do histórico de tarefas filtrado por execuções de cópia
de segurança bem-sucedidas, não do «último resultado» da tarefa:
depois de uma purga noturna de versões, o último resultado é a purga,
não a cópia de segurança. Isto custa um pedido por tarefa, limitado a
vinte tarefas.

Regras incorporadas em tarefas:

| Regra | Severidade | Dispara quando |
|---|---|---|
| Active Backup task failed | warning | A última execução falhou, ou foi um sucesso parcial, durante 10 minutos. A notificação nomeia a tarefa. |
| Active Backup too old | advisory | Nenhuma execução bem-sucedida há mais de dois dias. |
| Active Backup task disabled | info | A tarefa não tem calendário, ou a sua cópia de segurança contínua está pausada, durante uma hora. |

### Dispositivos e o seu ritmo {#devices-and-their-rhythm}

Uma regra ao nível de tarefa é certa para um servidor e errada para um
portátil: as pessoas não deixam os seus portáteis ligados, e «sem
cópia de segurança há 24 horas» é segunda-feira de manhã para todo PC
de escritório. O DumbMonit também monitoriza portanto o ABB
**dispositivo a dispositivo**, contra os próprios hábitos do
dispositivo.

A cada verificação, o coletor lê as execuções por dispositivo que o
ABB mantém (`SYNO.ActiveBackup.Overview` `list_device_transfer_size`,
uma chamada para todos os dispositivos, só o que mudou desde a
verificação anterior) e copia-as para a sua própria base de dados —
limitada a noventa dias por NAS — para que o modelo sobreviva a um
reinício sem reler um mês de histórico. Depois, para cada dispositivo,
olha para os últimos trinta dias.

| Métrica | Valor | Etiquetas |
|---|---|---|
| `dumbmonit_abb_device_state` | `0` ok, `1` idle, `2` learning, `3` running, `4` overdue, `5` failing, `6` never. | `device`, `device_id`, `task`, `task_id`, `state` |
| `dumbmonit_abb_device_overdue` | `1` enquanto o dispositivo está em atraso, `0` caso contrário. | `device`, `device_id`, `task`, `task_id` |
| `dumbmonit_abb_device_consecutive_failures` | Tentativas falhadas consecutivas desde o último sucesso (uma execução cancelada não conta como nenhuma das duas). | mesmas |
| `dumbmonit_abb_device_last_success_seconds` | Idade da última execução bem-sucedida deste dispositivo. Ausente se nunca teve sucesso. | mesmas |
| `dumbmonit_abb_device_typical_interval_seconds` | Intervalo mediano entre dois sucessos, dias de descanso excluídos. Ausente enquanto a aprender. | mesmas |

#### Como «em atraso» é julgado {#how-overdue-is-judged}

Em palavras simples, para cada dispositivo:

1. **Em que dias está ativo?** Um dia da semana em que o dispositivo
   tentou pelo menos uma cópia de segurança nos últimos trinta dias —
   com sucesso ou não; uma falha prova que a máquina estava ligada
   tanto quanto um sucesso — é um *dia ativo*. Uma vez existindo pelo
   menos duas semanas de histórico, um dia da semana sem nenhuma
   tentativa é um *dia de descanso*. Um portátil usado de
   segunda-feira a sexta-feira recebe sábado e domingo como dias de
   descanso; um servidor não tem nenhum.
2. **Com que frequência faz cópia de segurança?** Os intervalos entre
   sucessos consecutivos são coletados em *tempo ativo*: os dias de
   descanso completos que um intervalo contém não são contados
   (sexta-feira à noite até segunda-feira à noite é um dia de tempo
   ativo, não três). A sua mediana é o *intervalo típico* do
   dispositivo; o seu percentil 90 é o que «uma semana lenta» parece.
3. **Quanto tempo é permitido?** A margem é
   `max(1.5 × intervalo p90, 2 × intervalo típico, 36 h)`. Um desktop
   noturno recebe 48 h; uma máquina com cópia de segurança a cada
   três dias recebe cerca de seis; ninguém recebe menos do que um dia
   e meio. Enquanto existirem menos de dois intervalos utilizáveis
   (um dispositivo adicionado esta semana), a margem é 72 h —
   suficiente para atravessar um fim de semana sem um falso alarme.
4. **Quanto tempo ativo passou?** O tempo desde o último sucesso,
   menos os dias de descanso completos que contém. Um portátil com a
   última cópia de segurança na sexta-feira às 20:00 tem, na
   segunda-feira às 08:00, doze horas de tempo ativo atrás de si, não
   sessenta.
5. **O veredicto**, por esta ordem: *running* (uma cópia de segurança
   está em curso); *failing* (as duas ou mais últimas tentativas
   falharam — um sucesso parcial é uma falha, uma execução cancelada
   não é nenhuma das duas); *never* (nenhum sucesso registado);
   *overdue* (o tempo ativo excede a margem); *idle* (hoje é um dos
   seus dias de descanso: desligado, como habitual); *learning*
   (ritmo ainda não conhecido); *ok*.

Só *overdue* gera `dumbmonit_abb_device_overdue`, e só *failing*
alimenta a regra de falha. Um portátil inativo num domingo não gera
nada; o mesmo portátil ainda silencioso na terça-feira à noite gera.

Os dias e horas são expressos na hora local do servidor (`TZ` do
contentor), que é onde a pessoa que lê a página vive. A página do
dispositivo mostra, por dispositivo: o estado numa palavra, o último
sucesso e último resultado, o ritmo em palavras («dias de semana,
cerca das 20:00», «a cada ~3 dias, às Seg, Qua, Sex, cerca das
03:00», «ainda a aprender (2 execuções em 30 dias)») e uma faixa de
trinta dias — turquesa para um dia com sucesso, vermelho para uma
falha, âmbar para uma execução cancelada, cinza para nenhuma
execução.

Regras incorporadas em dispositivos:

| Regra | Severidade | Dispara quando |
|---|---|---|
| Active Backup device overdue | advisory | `dumbmonit_abb_device_overdue > 0` durante 30 minutos. |
| Active Backup device failing | warning | Duas ou mais tentativas falhadas consecutivas, durante 10 minutos. |

!!! warning "Permissões"
    O ABB só responde a uma conta no grupo `administrators` com o
    pacote permitido nas suas permissões de aplicação, ou uma conta a
    quem o pacote foi delegado (DSM 7: Active Backup for Business →
    Settings → Privileges). Uma conta a quem a aplicação é negada não
    pode ler as tarefas ABB: a chamada é recusada (código 105) e
    contada em `scrape_errors`. Ou permita ou delegue o pacote à conta
    de monitorização, ou desmarque «Watch Active Backup for Business»
    (`abb = false`) para parar de perguntar.

!!! note "Uma API não documentada"
    O Synology não documenta a API web do ABB. O DumbMonit segue a
    forma registada por projetos da comunidade
    (`SYNO.ActiveBackup.Task` `list`, `SYNO.ActiveBackup.Log`
    `list_result` e `SYNO.ActiveBackup.Overview`
    `list_device_transfer_size`, todas versão 1) e descobre o seu
    caminho através de `SYNO.API.Info`. Um DSM cujas respostas
    diferem produz métricas em falta, nunca uma verificação falhada;
    um ABB demasiado antigo para oferecer a vista geral por
    dispositivo mantém as suas métricas de tarefa e simplesmente não
    tem ritmo de dispositivo.

    A versão 1 é deliberada. O DSM 7.4 anuncia estas APIs como
    versões 1 a 2, mas só a versão 1 transporta os métodos de
    leitura: pedidas na versão 2 todas respondem «the requested
    method does not exist», o que parece uma diferença de firmware e
    não é. A mesma resposta também volta quando o token CSRF está em
    falta, por isso o DumbMonit pede um ao DSM no início de sessão e
    envia-o em cada chamada.

## Synology Drive {#synology-drive}

O DSM não documenta a API web do Synology Drive. O DumbMonit lê três
chamadas cujo nome e forma vêm da biblioteca da comunidade
`N4S4/synology-api`, que as chama dos mesmos ecrãs que a consola de
administração do Synology Drive, e segue a mesma regra que o Active
Backup for Business: uma API que um NAS não anuncia é um pacote que
não está instalado, nunca uma falha.

| Métrica | Valor | Etiquetas |
|---|---|---|
| `dumbmonit_synology_drive_up` | `1` enquanto o serviço responde a `get_status`. Ausente se o pacote não está instalado. | |
| `dumbmonit_synology_drive_team_folders` | Número de pastas de equipa. | |
| `dumbmonit_synology_drive_team_folder_info` | Valor `1` por pasta de equipa (no máximo 32). | `name` |
| `dumbmonit_synology_drive_connections` | Número de ligações ativas (clientes desktop e móveis atualmente sincronizados). | |

!!! note "Porque as ligações são só uma contagem"
    A página «Client list» da consola de administração mostra o nome
    do dispositivo, utilizador, tipo de aplicação, estado online,
    endereço IP e localização de cada ligação, mas isso é uma
    descrição da página, não dos campos JSON da API: nem o Synology
    nem nenhum projeto da comunidade documenta como
    `SYNO.SynologyDrive.Connection` realmente nomeia os seus campos.
    Em vez de adivinhar o nome de um dispositivo ou a data de última
    sincronização a partir de uma chave não confirmada, o DumbMonit
    só publica a contagem de ligações. Os dispositivos individuais
    vão ligar-se à mesma tabela de [dispositivos cliente](../alerting/rules.md)
    usada pelo Immich, Proxmox Backup Server e Veeam uma vez essa
    forma confirmada a partir da resposta de um NAS real.

A opção `drive` (ativada por predefinição) desliga toda a família.

## Synology Photos {#synology-photos}

O Synology Photos vem com uma API de navegação bem documentada, mas
nem o Synology nem as bibliotecas da comunidade expõem uma lista a
nível de administrador de dispositivos de cópia de segurança móvel ou
a sua última cópia de segurança — a app móvel mostra o seu próprio
estado de cópia de segurança localmente, e nada o centraliza do lado
do NAS através de uma API. O DumbMonit portanto só reporta que o
pacote está instalado: `dumbmonit_synology_photos_installed` (valor
`1`), lido do catálogo de API sem nenhum pedido extra ao NAS — o mesmo
catálogo `SYNO.API.Info` que já responde para todo outro pacote. A
opção `photos` (ativada por predefinição) desliga-o.

## O que preparar no NAS {#what-to-prepare-on-the-nas}

Os passos abaixo são os que o aviso junto ao formulário mostra. O
princípio: uma conta reservada para monitorização, despida de tudo o
que a API do DSM não precisa — nunca a conta com que inicia sessão.

1. No DSM, abra Control Panel → User & Group → User e clique em Create. Dê o seguinte nome à conta e dê-lhe uma senha longa não usada em nenhum outro lugar.

    ```
    dumbmonit
    ```

2. Junte-se a grupos: marque administrators. O DSM só responde às chamadas de armazenamento, volume e SMART de disco a esse grupo; sem ele o NAS mostra-se vivo mas não diz nada sobre os seus discos. Os dois passos seguintes retomam tudo o resto.
3. Atribua permissões de pasta partilhada: No access em toda pasta partilhada. Atribua permissões de aplicação: Deny tudo exceto DSM, mais Active Backup for Business se quiser as suas tarefas lidas. Ignore as páginas de quota e limite de velocidade.
4. A verificação em dois passos tem de ficar desligada para esta conta: nenhum monitor automatizado pode escrever um código de uso único. Se Control Panel → Security → Account a impuser, restrinja a regra a grupos a que este utilizador não pertence, ou isente-a.
5. Os pacotes de cópia de segurança só são lidos enquanto estão instalados e em execução: o DSM não anuncia de todo a API de um pacote parado, por isso as tarefas Hyper Backup ficam então silenciosas em vez de falhar, e um NAS que só recebe cópias de segurança (Hyper Backup Vault) não tem nenhuma tarefa própria para mostrar. O Active Backup for Business também só responde a uma conta autorizada a usá-lo (Active Backup for Business → Settings → Privileges); caso contrário desmarque a opção «Watch Active Backup for Business» do DumbMonit para parar de perguntar.
6. No DumbMonit, introduza o endereço do NAS (HTTPS, porta 5001 por predefinição), depois o nome de utilizador e senha desta conta.

!!! warning
    O grupo administrators é exigido pela API de armazenamento do DSM, não pelo DumbMonit. É por isso que esta conta não recebe nenhuma pasta partilhada, nenhuma aplicação e uma senha não usada em nenhum outro lugar: pode ler o NAS, não tocar nos seus ficheiros.

!!! note "Que direitos dão que métricas"
    A API do DSM não precisa dos mesmos direitos para toda chamada. O
    estado do sistema (modelo, versão, temperatura, tempo de
    atividade) responde a qualquer conta. CPU, memória, volumes e,
    sobretudo, estado SMART de disco exigem o grupo
    `administrators`. No DSM 7, a delegação «System monitoring»
    (Control Panel → User & Group → Delegation) basta para utilização
    mas não para o inventário de armazenamento. Uma conta sem
    direitos particulares dá portanto um NAS que está «vivo», mas
    nada sobre os seus discos.

Documentação do fornecedor: [Criar um utilizador no DSM](https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/file_user_create).

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Conta DSM | **User name**: a conta criada acima. **Password**: a sua senha. A verificação em dois passos tem de estar desligada para esta conta: nenhum monitor automatizado pode escrever um código de uso único. |

Endereço: `192.168.1.30`, `nas.lan` ou `nas.lan:5001`. Sem porta, 5001
é usada via HTTPS e 5000 via HTTP.

## Opções {#options}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `scheme` | Protocol | `https` | HTTPS adequa-se a um NAS de fábrica. HTTP só se o DSM escutar só em texto simples. Escolhas: `https`, `http`. |
| `port` | DSM port | *(vazio)* | Usada se o endereço não der uma porta. Vazio: 5001 via HTTPS, 5000 via HTTP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Um NAS vem com um certificado autoassinado por predefinição: ative isto se a ligação for recusada por esse motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tempo permitido para cada chamada de API, de 1 a 120. O inventário de armazenamento pode despertar discos em suspensão. |
| `abb` | Watch Active Backup for Business | `true` | Lê as tarefas Active Backup for Business. Precisa do pacote instalado e de uma conta autorizada a usá-lo; um NAS sem o pacote é simplesmente ignorado. |
| `drive` | Watch Synology Drive | `true` | Lê se o serviço Synology Drive responde, as suas pastas de equipa e a sua contagem de ligações ativas. Um NAS sem o pacote é simplesmente ignorado. |
| `photos` | Watch Synology Photos | `true` | Mostra se o pacote Synology Photos está instalado, sem nenhum pedido extra ao NAS. O Synology não expõe uma lista de dispositivos de cópia de segurança móvel através de nenhuma API. |

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de autenticação | Senha errada, ou verificação em dois passos exigida para esta conta. O DSM responde HTTP 200 mesmo em falha; o código de erro no corpo JSON é o que conta, e o DumbMonit lê-o. |
| Erro de certificado | Certificado autoassinado: ative `insecure_tls`. Isto é um erro de configuração, nunca «inacessível»: um NAS que responde não se mostra como desligado. |
| Nenhuma métrica de disco ou volume | A conta não está no grupo `administrators` (ver a nota acima). |
| Nenhuma métrica `dumbmonit_abb_*`, `scrape_errors` em 1 | A conta não pode usar o Active Backup for Business: delegue-lhe o pacote, ou desative a opção `abb`. Sem o pacote instalado não há nenhum erro e nenhuma métrica. |
| Métricas de tarefa mas nenhuma linha de monitor na página | O NAS não anuncia `SYNO.ActiveBackup.Overview` (ABB mais antigo), ou o monitor é sondado através de um agente relay, que mantém o histórico em memória do seu lado: as métricas existem, o calendário não. |
| Um portátil mostra «Overdue» no dia seguinte a ser adicionado | Menos de três sucessos: o modelo ainda está a aprender e permite 72 horas entre cópias de segurança; estabiliza-se uma vez conhecidos duas semanas de hábitos. |
| Os discos despertam em cada verificação | O inventário de armazenamento desperta discos em suspensão. Alargue o intervalo de verificação. |
