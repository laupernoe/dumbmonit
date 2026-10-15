# Configuração {#configuration}

Tudo passa por variáveis de ambiente; nenhuma é obrigatória. Não há arquivo
de configuração para montar.

## Servidor {#server}

| Variável | Padrão | Função |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Endereço de escuta da API e da interface web, dentro do contêiner. |
| `DUMBMONIT_DATA_DIR` | `/data` | Diretório persistente: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (VictoriaMetrics embutido) e `webchange/` (capturas de tela de mudanças no site). |
| `DUMBMONIT_VM_URL` | *(não definido)* | URL base de um VictoriaMetrics externo (`http://host:8428`). Quando definido, o embutido não é iniciado; veja [abaixo](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(gerado)* | Segredo da instância. Se vazio, lido de `<diretório de dados>/secret.key`, gerado na primeira inicialização. Criptografa credenciais de monitor, segredos de canal e tokens de agente com AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Máximo de sondagens simultâneas, todos os coletores combinados. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Tempo limite rígido de uma sondagem. Monitores de serviço têm sua própria opção `timeout_seconds`, mais curta. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Período das escritas em lote para o VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Número de amostras em buffer que aciona uma escrita antes do período decorrer. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Threads do runtime assíncrono. O servidor gasta seu tempo esperando pela rede: quatro cobrem um homelab, aumente apenas para centenas de monitores. |
| `DUMBMONIT_DB_POOL` | `4` | Máximo de conexões SQLite abertas. Cada uma custa uma thread e um cache de página de 2 MB; conexões ociosas fecham após cinco minutos. |
| `DUMBMONIT_LOG` | `info` | Filtro de log, sintaxe `tracing`: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Diretório dos binários de agente servidos sob `/download/`. Vazio ou faltando: o comando de instalação falha com `404`, o servidor ainda roda. |
| `DUMBMONIT_RESET_PASSWORD` | *(desligado)* | `1`, `true`, `yes` ou `on`: limpa a senha e toda sessão na inicialização. A interface então mostra `/setup` de novo. Remova-a depois. |
| `DUMBMONIT_COOKIE_SECURE` | *(desligado)* | `1` para definir o atributo `Secure` no cookie de sessão. Apenas detrás de HTTPS: em HTTP simples o navegador nunca enviaria o cookie de volta. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(vazio)* | Proxies reversos cujo `X-Forwarded-For` é confiável, como endereços ou faixas CIDR separadas por vírgulas (`10.0.0.5, 172.16.0.0/12`). O limitador de taxa de login e o log de segurança então veem o endereço real do cliente em vez do do proxy; sem isso, todo visitante detrás do proxy compartilha um balde. O cabeçalho é ignorado de qualquer outro endereço, então um cliente não pode escolher seu próprio balde. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(desligado)* | `true` para criar uma conta para uma identidade desconhecida em seu primeiro login único. Desligado, apenas contas existentes (já vinculadas, ou correspondidas por um e-mail verificado pelo provedor) podem entrar. Lido apenas enquanto nenhuma configuração de login único está salva na interface. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(desligado)* | `1` para aceitar um emissor OpenID Connect em `http://` simples. Desligado, a tela de configurações recusa qualquer coisa além de `https://`: um emissor em texto claro entrega o código de autorização e o segredo do cliente a quem quer que esteja escutando na rede. Apenas para um provedor de teste no loopback. |
| `DUMBMONIT_METRICS_PUBLIC` | *(desligado)* | `1` para servir `/metrics`, `/federate` e `/prometheus/api/v1/…` sem um token de API. Qualquer um que alcance a porta então lê toda medição de todo monitor e o estado da instância; defina apenas onde algo mais já restringe o acesso à porta. Veja [Métricas](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Período de avaliação de alerta. Valores abaixo de 10 são elevados a 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Retenção do histórico de alertas, em dias. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Backups locais programados do banco de dados. `0`, `false`, `no` ou `off` para parar de escrevê-los. Veja [Backup e restauração](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` para parar a verificação diária do último lançamento no GitHub (instâncias isoladas). Veja [Atualização](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<diretório de dados>/backups` | Onde os backups programados são escritos. Aponte para um segundo volume para sobreviver à perda do primeiro. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Horas entre dois backups, 1 a 8760. O primeiro acontece um intervalo depois da inicialização, não na inicialização. |
| `DUMBMONIT_DEMO` | *(desligado)* | `1` para uma demo pública, somente leitura, com um acervo fictício fixo, login `demo` / `demo`. **Recria o banco de dados a cada inicialização**: nunca em um volume de dados real. Veja [Executar uma instância de demonstração](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quantos backups são mantidos, 1 a 365. Os mais antigos são removidos com seu `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(não definido)* | Chromium headless usado para as capturas de tela de [mudanças no site](../devices/webchange.md#screenshots), como `http://host:9222` (sua porta DevTools). Adicione um serviço `chromedp/headless-shell` ao seu arquivo Compose, como mostrado em [Capturas de tela](../devices/webchange.md#screenshots). Não definido: apenas comparação de texto. |

A retenção da linha de base (60 dias) e o período de aprendizado de 14 dias
não são configuráveis.

### Antigos nomes `EZYMONIT_*` {#former-ezymonit_-names}

O DumbMonit se chamava EzyMonit até setembro de 2026. Toda variável
`DUMBMONIT_*` também é lida sob seu antigo nome `EZYMONIT_*`, como
fallback, com um aviso na inicialização por variável ainda definida assim
(`EZYMONIT_X is deprecated, use DUMBMONIT_X`). O antigo arquivo de banco de
dados `/data/ezymonit.db` é renomeado para `dumbmonit.db` na
inicialização, e tokens de agente `ezym_…` permanecem válidos (os novos são
`dmon_…`). O cookie de sessão agora é `dumbmonit_session`: a primeira
inicialização após a renomeação desconecta todos uma vez.

### VictoriaMetrics embutido {#embedded-victoriametrics}

A imagem entrega o binário do VictoriaMetrics. A menos que
`DUMBMONIT_VM_URL` esteja definido, o servidor o inicia como um processo
filho na inicialização, encaminha suas linhas de log para o log do
servidor, o reinicia com um backoff se morrer, e o para no encerramento
(`SIGTERM`, 20 s de graça, depois kill; o `stop_grace_period` do arquivo
Compose deixa espaço para isso).

| Variável | Padrão | Função |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Caminho do binário VictoriaMetrics a iniciar. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Endereço de escuta, dentro do contêiner. Loopback por padrão: só o servidor fala com ele. A camada de desenvolvimento define `0.0.0.0:8428` e publica a porta. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retenção, na sintaxe de `-retentionPeriod`: um número de meses, ou uma duração como `30d` ou `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Orçamento para os caches do VictoriaMetrics (`-memory.allowedBytes`). Sem um orçamento o VM os dimensiona para 60% da RAM do host e sua memória residente cresce por dias. Algumas dezenas de monitores cabem em 256 MB; aumente para centenas de hosts. |

Seus dados vivem em `<diretório de dados>/vm`, no mesmo volume do banco de
dados. Três flags são fixas pelo servidor e não configuráveis:
`-search.maxUniqueTimeseries=100000` (uma consulta descontrolada não pode
esgotar o orçamento), `-search.maxConcurrentRequests=4` (uma interface
mais o motor de alertas) e `-dedup.minScrapeInterval=10s` (pontos
duplicados mais próximos que dez segundos são colapsados; nada se perde no
ritmo de 30 s dos coletores).

Com `DUMBMONIT_VM_URL` definido, nada disso se aplica: o servidor usa a
instância que você nomeia e o embutido nunca é iniciado. `GET /api/health`
diz em qual caso você está (`victoria.embedded`).

## Configurações em nível Compose {#compose-level-variables}

O `docker-compose.yml` fornecido não lê nenhuma variável e não precisa de arquivo
`.env`. Tudo é definido em `environment:` do serviço ou editando o arquivo; veja
[Opções avançadas](../install/docker.md#advanced-options) (porta, segredo,
retenção e memória, VictoriaMetrics externo, redefinição de senha, capturas de
tela). A camada de desenvolvimento `docker-compose.dev.yml` lê `DUMBMONIT_VM_PORT`
(padrão `8428`) para a porta do host do VictoriaMetrics embutido.

## Portas {#ports}

| Porta | Contêiner | Propósito |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Interface web, API, instaladores e downloads de agente, ingestão de agente. A única porta a publicar. |
| `8428/tcp` | `dumbmonit` | API HTTP do VictoriaMetrics embutido, no loopback do contêiner. Publicada apenas pela camada de desenvolvimento. |

Saída: SNMP (UDP 161 por padrão) em direção a dispositivos, HTTPS em
direção a Proxmox, PBS, Synology e serviços de notificação, mais o que
quer que seus monitores de serviço visem. O ping ICMP precisa do sysctl
`net.ipv4.ping_group_range` definido no arquivo Compose (sem capacidade).

## Volumes e arquivos {#volumes-and-files}

Um volume, `dumbmonit-data`, montado em `/data`. Seu nome é fixo no
arquivo Compose, qualquer que seja o nome do projeto.

| Caminho (no contêiner) | Volume | Conteúdo |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Monitores, credenciais (criptografadas), regras, estado e histórico de alertas, linhas de base, silêncios, canais, tokens de agente, hash de senha, sessões. |
| `/data/secret.key` | `dumbmonit-data` | O segredo da instância. **Faça backup dele.** Sem ele, credenciais criptografadas não podem ser recuperadas e o servidor recusa iniciar. |
| `/data/backups/` | `dumbmonit-data` | Backups locais programados: `dumbmonit-<timestamp>.db` (uma cópia SQLite online) e o `.key` correspondente. Diário por padrão, sete mantidos. |
| `/data/vm/` | `dumbmonit-data` | Séries temporais do VictoriaMetrics embutido, 12 meses por padrão. Vazio quando `DUMBMONIT_VM_URL` está definido. |
| `/agents/` | imagem | Binários de agente. Monte outro diretório e defina `DUMBMONIT_AGENT_DIR` para entregar os seus próprios. |
| `/etc/ssl/certs/ca-certificates.crt` | imagem | Raízes TLS para HTTPS de saída. Monte seu próprio pacote aqui para confiar em uma autoridade privada. |

## Redefinindo a senha {#resetting-the-password}

```bash
# adicione DUMBMONIT_RESET_PASSWORD: "1" em environment: no docker-compose.yml
docker compose up -d
# abra a interface: /setup pede uma nova senha
# remova a linha de novo e depois:
docker compose up -d
```

A redefinição limpa a senha e toda sessão; monitores, regras e canais
permanecem intactos.

## Agente {#agent}

O agente tem suas próprias variáveis e arquivo de configuração: veja
[Agente Linux, macOS, FreeBSD e Windows](../devices/agent.md#configuration-file).

### Escopo de coleta {#collection-scope}

O que o agente mede, e portanto quantas séries uma máquina produz. Os
padrões mantêm um host com dezenas de contêineres em algumas centenas de
séries; cada chave é uma entrada de `agent.yaml`, substituível por sua
variável de ambiente.

| Chave | Variável | Padrão | Função |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Interfaces de rede deixadas de fora: interfaces de contêiner e VM, o loopback. Uma lista de nomes exatos ou expressões regulares (não ancoradas; qualquer coisa com um metacaractere é uma regex). O valor substitui o padrão; uma lista vazia mantém toda interface. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(vazio)* | Lista de permissão, mesma sintaxe. Quando definido, só essas interfaces são reportadas e `interfaces_ignore` não é consultado. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Pontos de montagem deixados de fora do uso de sistema de arquivos e E/S de disco, mesma sintaxe. Discos removíveis sob `/run/media/` permanecem. Sistemas de arquivos pseudo (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) são sempre pulados, qualquer que seja o caminho. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Uma série `cpu_core_usage_percent` por núcleo, além do uso global. `cpu_count` é reportado de qualquer forma. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Contêineres detalhados por ciclo, os em execução primeiro. Além disso, contêineres são apenas contados (`container_count`, `container_running_count` permanecem exatos) e `container_series_skipped` diz quantos. `0` conta sem detalhar. |

No ambiente, listas são separadas por vírgulas (`^veth, ^br-, docker0`); em
YAML, uma lista ou uma string única separada por vírgulas. Contadores de
E/S de disco (`disk_read_bytes`, `disk_written_bytes`) carregam apenas o
rótulo `device`: uma série por dispositivo de bloco, não importa quantas
vezes seja montado.
