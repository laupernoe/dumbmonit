# Configuração {#configuration}

Tudo passa por variáveis de ambiente; nenhuma é obrigatória. Não há
nenhum ficheiro de configuração para montar.

## Servidor {#server}

| Variável | Predefinição | Função |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Endereço de escuta da API e da interface web, dentro do contentor. |
| `DUMBMONIT_DATA_DIR` | `/data` | Diretório persistente: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (VictoriaMetrics incorporado) e `webchange/` (capturas de ecrã de alterações de site). |
| `DUMBMONIT_VM_URL` | *(não definida)* | URL base de um VictoriaMetrics externo (`http://host:8428`). Quando definida, o incorporado não é iniciado; ver [abaixo](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(gerado)* | Segredo da instância. Se vazio, lido de `<diretório de dados>/secret.key`, gerado no primeiro arranque. Encripta credenciais de dispositivo, segredos de canal e tokens de agente com AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Máximo de verificações simultâneas, todos os coletores combinados. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Tempo limite rígido de uma verificação. Os monitores de serviço têm a sua própria opção `timeout_seconds`, mais curta. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Período das escritas em lote para a VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Número de amostras em buffer que aciona uma escrita antes do período passar. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Threads do runtime assíncrono. O servidor passa o tempo à espera da rede: quatro cobrem um homelab, suba só para centenas de dispositivos. |
| `DUMBMONIT_DB_POOL` | `4` | Máximo de ligações SQLite abertas. Cada uma custa uma thread e uma cache de página de 2 MB; ligações inativas fecham depois de cinco minutos. |
| `DUMBMONIT_LOG` | `info` | Filtro de registo, sintaxe `tracing`: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Diretório dos binários de agente servidos sob `/download/`. Vazio ou em falta: o comando de instalação falha com `404`, o servidor continua a correr. |
| `DUMBMONIT_RESET_PASSWORD` | *(desligado)* | `1`, `true`, `yes` ou `on`: limpa a senha e toda sessão no arranque. A interface mostra então `/setup` de novo. Remova-a depois. |
| `DUMBMONIT_COOKIE_SECURE` | *(desligado)* | `1` para definir o atributo `Secure` no cookie de sessão. Só atrás de HTTPS: via HTTP simples o navegador nunca devolveria o cookie. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(vazio)* | Proxies inversos cujo `X-Forwarded-For` é confiado, como endereços ou intervalos CIDR separados por vírgulas (`10.0.0.5, 172.16.0.0/12`). O limitador de taxa de início de sessão e o registo de segurança veem então o endereço real do cliente em vez do do proxy; sem isto, cada visitante atrás do proxy partilha um contentor. O cabeçalho é ignorado de qualquer outro endereço, por isso um cliente não pode escolher o seu próprio contentor. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(desligado)* | `true` para criar uma conta para uma identidade desconhecida no seu primeiro sign-on único. Desligado, só contas existentes (já ligadas, ou correspondidas por um email verificado pelo fornecedor) podem iniciar sessão. Lido só enquanto nenhuma definição de sign-on único está guardada na interface. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(desligado)* | `1` para aceitar um emissor OpenID Connect em `http://` simples. Desligado, o ecrã de definições recusa tudo exceto `https://`: um emissor em texto simples entrega o código de autorização e o segredo do cliente a quem escuta na rede. Só para um fornecedor de teste no loopback. |
| `DUMBMONIT_METRICS_PUBLIC` | *(desligado)* | `1` para servir `/metrics`, `/federate` e `/prometheus/api/v1/…` sem um token de API. Quem conseguir alcançar a porta lê então toda medição de todo dispositivo e o estado da instância; só a defina onde outra coisa já restringe o acesso à porta. Ver [Métricas](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Período de avaliação de alertas. Valores abaixo de 10 são subidos para 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Retenção do histórico de alertas, em dias. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Cópias de segurança locais agendadas da base de dados. `0`, `false`, `no` ou `off` para parar de as escrever. Ver [Cópia de segurança e restauro](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` para parar de verificar diariamente a versão mais recente no GitHub (instâncias air-gapped). Ver [Atualizar](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<diretório de dados>/backups` | Onde as cópias de segurança agendadas são escritas. Aponte-o para um segundo volume para sobreviver à perda do primeiro. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Horas entre duas cópias de segurança, 1 a 8760. A primeira acontece um intervalo depois do arranque, não no arranque. |
| `DUMBMONIT_DEMO` | *(desligado)* | `1` para uma demonstração pública, só de leitura, com um património fictício fixo, início de sessão `demo` / `demo`. **Recria a base de dados a cada arranque**: nunca num volume de dados real. Ver [Executar uma instância de demonstração](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quantas cópias de segurança são mantidas, 1 a 365. As mais antigas são removidas com o seu `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(não definida)* | Chromium headless usado para as capturas de ecrã de [alterações de site](../devices/webchange.md#screenshots), como `http://host:9222` (a sua porta DevTools). O ficheiro Compose traz um sob o perfil `screenshots`: `DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d`. Não definida: só comparação de texto. |

A retenção da linha de base (60 dias) e o período de aprendizagem de 14
dias não são configuráveis.

### Antigos nomes `EZYMONIT_*` {#former-ezymonit_-names}

O DumbMonit chamava-se EzyMonit até setembro de 2026. Toda variável
`DUMBMONIT_*` também é lida sob o seu antigo nome `EZYMONIT_*`, como
alternativa, com um aviso no arranque por variável ainda definida dessa
forma (`EZYMONIT_X is deprecated, use DUMBMONIT_X`). O antigo ficheiro
de base de dados `/data/ezymonit.db` é renomeado para `dumbmonit.db` no
arranque, e os tokens de agente `ezym_…` continuam válidos (os novos
são `dmon_…`). O cookie de sessão é agora `dumbmonit_session`: o
primeiro arranque depois da renomeação termina a sessão de todos uma
vez.

### VictoriaMetrics incorporado {#embedded-victoriametrics}

A imagem vem com o binário VictoriaMetrics. A menos que
`DUMBMONIT_VM_URL` esteja definida, o servidor inicia-o como um
processo filho no arranque, encaminha as suas linhas de registo para o
registo do servidor, reinicia-o com um backoff se morrer, e para-o no
encerramento (`SIGTERM`, 20 s de graça, depois kill; o
`stop_grace_period` do ficheiro Compose deixa espaço para isso).

| Variável | Predefinição | Função |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Caminho do binário VictoriaMetrics a iniciar. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Endereço de escuta, dentro do contentor. Loopback por predefinição: só o servidor lhe fala. A camada de desenvolvimento define `0.0.0.0:8428` e publica a porta. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retenção, na sintaxe de `-retentionPeriod`: um número de meses, ou uma duração como `30d` ou `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Orçamento para as caches do VictoriaMetrics (`-memory.allowedBytes`). Sem orçamento o VM dimensiona-as para 60% da RAM do anfitrião e a sua memória residente cresce durante dias. Algumas dezenas de dispositivos cabem em 256 MB; suba-o para centenas de anfitriões. |

Os seus dados vivem em `<diretório de dados>/vm`, no mesmo volume que a
base de dados. Três flags são fixas pelo servidor e não configuráveis:
`-search.maxUniqueTimeseries=100000` (uma consulta descontrolada não
pode esgotar o orçamento), `-search.maxConcurrentRequests=4` (uma
interface mais o motor de alertas) e `-dedup.minScrapeInterval=10s`
(pontos duplicados com menos de dez segundos de distância são
colapsados; nada se perde à cadência de 30 s dos coletores).

Com `DUMBMONIT_VM_URL` definida, nada disto se aplica: o servidor usa a
instância que nomear e o incorporado nunca é iniciado. `GET /api/health`
diz em que caso está (`victoria.embedded`).

## Variáveis a nível de Compose {#compose-level-variables}

Estas são lidas pelo `docker-compose.yml`, não pelo servidor:

| Variável | Predefinição | Função |
|---|---|---|
| `DUMBMONIT_PORT` | `8080` | Porta do anfitrião publicada para a interface. |
| `DUMBMONIT_VM_RETENTION`, `DUMBMONIT_VM_MEMORY` | `12`, `256MB` | Passadas ao servidor; ver acima. |
| `DUMBMONIT_VM_PORT` | `8428` | Porta do anfitrião para o VictoriaMetrics incorporado, só na camada de desenvolvimento. |

## Portas {#ports}

| Porta | Contentor | Propósito |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Interface web, API, instaladores de agente e descargas, ingestão de agente. A única porta a publicar. |
| `8428/tcp` | `dumbmonit` | API HTTP do VictoriaMetrics incorporado, no loopback do contentor. Publicada só pela camada de desenvolvimento. |

Saída: SNMP (UDP 161 por predefinição) para dispositivos, HTTPS para
Proxmox, PBS, Synology e serviços de notificação, mais o que os seus
monitores de serviço visarem. O ping ICMP precisa do sysctl
`net.ipv4.ping_group_range` definido no ficheiro Compose (sem
capacidade).

## Volumes e ficheiros {#volumes-and-files}

Um volume, `dumbmonit-data`, montado em `/data`. O seu nome é fixo no
ficheiro Compose, qualquer que seja o nome do projeto.

| Caminho (no contentor) | Volume | Conteúdo |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Dispositivos, credenciais (encriptadas), regras, estado e histórico de alertas, linhas de base, silêncios, canais, tokens de agente, hash de senha, sessões. |
| `/data/secret.key` | `dumbmonit-data` | O segredo da instância. **Faça cópia de segurança dele.** Sem ele, as credenciais encriptadas são irrecuperáveis e o servidor recusa arrancar. |
| `/data/backups/` | `dumbmonit-data` | Cópias de segurança locais agendadas: `dumbmonit-<timestamp>.db` (uma cópia SQLite em funcionamento) e o `.key` correspondente. Diária por predefinição, sete mantidas. |
| `/data/vm/` | `dumbmonit-data` | Séries temporais do VictoriaMetrics incorporado, 12 meses por predefinição. Vazio quando `DUMBMONIT_VM_URL` está definida. |
| `/agents/` | imagem | Binários de agente. Monte outro diretório e defina `DUMBMONIT_AGENT_DIR` para distribuir os seus próprios. |
| `/etc/ssl/certs/ca-certificates.crt` | imagem | Raízes TLS para HTTPS de saída. Monte o seu próprio conjunto aqui para confiar numa autoridade privada. |

## Repor a senha {#resetting-the-password}

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
# abra a interface: /setup pede uma nova senha
docker compose up -d dumbmonit   # inicie de novo sem a variável
```

A reposição limpa a senha e toda sessão; dispositivos, regras e canais
não são tocados.

## Agente {#agent}

O agente tem as suas próprias variáveis e ficheiro de configuração: ver
[Agente Linux, macOS, FreeBSD e Windows](../devices/agent.md#configuration-file).

### Âmbito de coleta {#collection-scope}

O que o agente mede, e portanto quantas séries uma máquina produz. As
predefinições mantêm um anfitrião com dezenas de contentores em
algumas centenas de séries; cada chave é uma entrada de `agent.yaml`,
substituível pela sua variável de ambiente.

| Chave | Variável | Predefinição | Função |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Interfaces de rede deixadas de fora: interfaces de contentor e VM, o loopback. Uma lista de nomes exatos ou expressões regulares (não ancoradas; qualquer coisa com um metacarácter é uma regex). O valor substitui a predefinição; uma lista vazia mantém toda interface. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(vazio)* | Lista de permissão, mesma sintaxe. Quando definida, só estas interfaces são reportadas e `interfaces_ignore` não é consultada. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Pontos de montagem deixados de fora do uso de sistema de ficheiros e I/O de disco, mesma sintaxe. Unidades removíveis sob `/run/media/` permanecem. Sistemas de ficheiros pseudo (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) são sempre ignorados, qualquer que seja o caminho. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Uma série `cpu_core_usage_percent` por núcleo, além do uso global. `cpu_count` é reportado de qualquer forma. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Contentores detalhados por ciclo, os em execução primeiro. Além disso, os contentores só são contados (`container_count`, `container_running_count` mantêm-se exatos) e `container_series_skipped` diz quantos. `0` conta sem detalhar. |

No ambiente, as listas são separadas por vírgulas (`^veth, ^br-,
docker0`); em YAML, ou uma lista ou uma cadeia separada por vírgulas.
Os contadores de I/O de disco (`disk_read_bytes`, `disk_written_bytes`)
transportam só a etiqueta `device`: uma série por dispositivo de
bloco, não importa quantas vezes está montado.
