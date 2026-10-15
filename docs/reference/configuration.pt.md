# Configuração {#configuration}

Tudo passa por variáveis de ambiente; nenhuma é obrigatória. Não há ficheiro de
configuração a montar.

## Servidor {#server}

| Variável | Predefinição | Função |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Endereço de escuta da API e da interface web, dentro do contentor. |
| `PUID` / `PGID` | `65532` | Utilizador e grupo com que o servidor corre quando o contentor arranca como root (o predefinido da imagem): a diretoria de dados é-lhes entregue no arranque e depois o root é abandonado. O `0` é recusado. `DUMBMONIT_PUID` / `DUMBMONIT_PGID` têm prioridade. Ignorados quando o contentor é iniciado com `user:` / `--user`. Veja [Permissões](../install/docker.md#permissions). |
| `DUMBMONIT_DATA_DIR` | `/data` | Diretoria persistente: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (VictoriaMetrics incorporado) e `webchange/` (capturas de ecrã das alterações de sites). |
| `DUMBMONIT_VM_URL` | *(não definida)* | URL base de um VictoriaMetrics externo (`http://host:8428`). Quando definida, o incorporado não é iniciado; veja [abaixo](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(gerado)* | Segredo da instância. Se vazio, é lido de `<data dir>/secret.key`, gerado no primeiro arranque. Cifra as credenciais dos dispositivos, os segredos dos canais e os tokens dos agentes com AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Máximo de sondas simultâneas, todos os coletores combinados. |
| `DUMBMONIT_WEBSITE_MAX_CONCURRENT` | `10` | Máximo de verificações simultâneas de [saúde de sites](../devices/website.md#pacing-hundreds-of-sites), 1 a 256, dentro do limite acima. |
| `DUMBMONIT_WEBSITE_MAX_PER_HOST` | `1` | Máximo de verificações de sites simultâneas para um domínio (`www.example.com` e `shop.example.com` contam como um), 1 a 16. |
| `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` | `2` | Segundos mínimos entre duas verificações de sites para um domínio, 0 a 3600. A opção `host_spacing_seconds` de um site sobrepõe-se-lhe depois da verificação desse site. |
| `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` | `360` | Período predefinido, em minutos, da comparação dos metadados do site (título, descrição, canónico, endereço final, `robots.txt`), 0 a 10080; 0 compara em cada verificação. O estado ativo/inativo continua a ser verificado em cada intervalo de sondagem. A opção `change_check_minutes` de um site sobrepõe-se-lhe. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Tempo limite rígido de uma sonda. Os monitores de serviços têm a sua própria opção `timeout_seconds`, mais curta. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Período das escritas em lote para o VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Número de amostras em buffer que dispara uma escrita antes de o período terminar. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Threads do runtime assíncrono. O servidor passa o tempo à espera da rede: quatro cobrem um homelab, aumente apenas para centenas de dispositivos. |
| `DUMBMONIT_DB_POOL` | `4` | Máximo de ligações SQLite abertas. Cada uma custa uma thread e uma cache de páginas de 2 MB; as ligações inativas fecham após cinco minutos. |
| `DUMBMONIT_LOG` | `info` | Filtro de registo, sintaxe `tracing`: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Diretoria dos binários do agente servidos em `/download/`. Vazia ou em falta: o comando de instalação falha com `404`, o servidor continua a correr. |
| `DUMBMONIT_RESET_PASSWORD` | *(desativada)* | `1`, `true`, `yes` ou `on`: apaga a palavra-passe e todas as sessões no arranque. A interface mostra então `/setup` de novo. Remova-a depois. |
| `DUMBMONIT_COOKIE_SECURE` | *(desativada)* | `1` para definir sempre o atributo `Secure` no cookie de sessão. Apenas atrás de HTTPS: em HTTP simples o navegador nunca devolveria o cookie. Sem ela, o atributo continua a ser acrescentado sempre que o navegador está numa página `https://` (o seu `Origin` o diz, ou um proxy de confiança envia `X-Forwarded-Proto: https`), pelo que o HTTP simples na LAN continua a funcionar. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(vazia)* | Proxies inversos cujos `X-Forwarded-For`, `X-Forwarded-Proto` e `X-Forwarded-Host` são tidos por verdadeiros, como endereços ou intervalos CIDR separados por vírgulas (`10.0.0.5, 172.16.0.0/12`). O limitador de tentativas de início de sessão e o registo de segurança veem então o endereço real do cliente em vez do do proxy; sem ela, todos os visitantes atrás do proxy partilham um só balde. As ligações das páginas de estado (feed RSS, emails de subscrição) e o endereço de retorno do início de sessão único também só usam o anfitrião reencaminhado vindo destes proxies. Os cabeçalhos são ignorados de qualquer outro endereço, pelo que um cliente não pode escolher o seu balde nem o endereço de uma ligação. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(desativada)* | `true` para criar uma conta para uma identidade desconhecida no seu primeiro início de sessão único. Desativada, só as contas existentes (já associadas, ou correspondidas por um email verificado pelo fornecedor) podem iniciar sessão. Como as outras variáveis `DUMBMONIT_OIDC_*`, lida apenas enquanto não houver definições de início de sessão único guardadas na interface. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(desativada)* | `1` para aceitar um emissor OpenID Connect em `http://` simples. Desativada, o ecrã de definições recusa tudo exceto `https://`: um emissor em texto simples entrega o código de autorização e o client secret a quem escutar na rede. Apenas para um fornecedor de teste no loopback. |
| `DUMBMONIT_SETUP_CODE` | *(gerado)* | Código que `/setup` pede enquanto não existe administrador. Não definido, um aleatório é impresso no registo do servidor em cada arranque; defina-o para uma implementação automatizada. Veja [Instalar com Docker](../install/docker.md). |
| `DUMBMONIT_PUBLIC_URL` | *(não definida)* | Endereço com que os navegadores alcançam o DumbMonit (`https://monit.example.org`). Usado para ligações em notificações e emails de páginas de estado, o URI de redirecionamento do início de sessão único e o contacto dado aos serviços push (apenas HTTPS). Um valor guardado na interface (política de notificação, definições de SSO) prevalece. |
| `DUMBMONIT_API_CORS_ORIGINS` | *(qualquer origem)* | Origens autorizadas a chamar a API entre origens com um token bearer, e a alcançar o endpoint MCP: uma lista separada por vírgulas, ou `off` para nenhuma. Veja [API HTTP](api.md#cross-origin-requests-cors). |
| `DUMBMONIT_OIDC_ISSUER` | *(não definida)* | URL do emissor do fornecedor OpenID Connect; a descoberta é lida em `{issuer}/.well-known/openid-configuration`. Tem de ser `https://` a menos que `DUMBMONIT_OIDC_ALLOW_HTTP` esteja definida. |
| `DUMBMONIT_OIDC_CLIENT_ID` | *(não definida)* | Client id registado no fornecedor. |
| `DUMBMONIT_OIDC_CLIENT_SECRET` | *(não definida)* | Client secret. |
| `DUMBMONIT_OIDC_PROVIDER_NAME` | `SSO` | Rótulo do botão «Continuar com …». |
| `DUMBMONIT_OIDC_SCOPES` | `openid profile email` | Scopes pedidos, separados por espaços. |
| `DUMBMONIT_OIDC_GROUPS_CLAIM` | `groups` | Claim do token que lista os grupos do utilizador. |
| `DUMBMONIT_OIDC_ADMIN_GROUPS` | *(vazia)* | Grupos separados por vírgulas cujos membros passam a administradores. |
| `DUMBMONIT_OIDC_OPERATOR_GROUPS` | *(vazia)* | Grupos separados por vírgulas cujos outros membros passam a operadores. Veja [Início de sessão único](../using/settings.md#single-sign-on). |
| `DUMBMONIT_METRICS_PUBLIC` | *(desativada)* | `1` para servir `/metrics`, `/federate` e `/prometheus/api/v1/…` sem token de API. Quem alcançar a porta lê então todas as medições de todos os dispositivos e o estado da instância; defina-a apenas onde outra coisa já restrinja o acesso à porta. Veja [Métricas](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Período de avaliação de alertas. Valores abaixo de 10 são elevados a 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Retenção do histórico de alertas, em dias. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Cópias de segurança locais agendadas da base de dados. `0`, `false`, `no` ou `off` para deixar de as escrever. Veja [Cópia de segurança e restauro](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` pára a consulta diária à última versão no GitHub (instâncias isoladas da rede). Veja [Atualizar](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Onde as cópias de segurança agendadas são escritas. Aponte-a para um segundo volume para sobreviver à perda do primeiro. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Horas entre duas cópias de segurança, 1 a 8760. A primeira acontece um intervalo após o arranque, não no arranque. |
| `DUMBMONIT_DEMO` | *(desativada)* | `1` para uma demonstração pública, só de leitura, com um parque fictício fixo, início de sessão `demo` / `demo`. **Recria a base de dados em cada arranque**: nunca num volume de dados real. Veja [Executar uma instância de demonstração](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quantas cópias de segurança são mantidas, 1 a 365. As mais antigas são removidas com a sua `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(não definida)* | Chromium sem interface usado para as capturas de ecrã das [alterações de sites](../devices/webchange.md#screenshots), como `http://host:9222` (a sua porta DevTools). Acrescente um serviço `chromedp/headless-shell` ao seu ficheiro Compose, como mostrado em [Capturas de ecrã](../devices/webchange.md#screenshots). Não definida: apenas comparação de texto. |

A retenção das linhas de base (60 dias) e o período de aprendizagem de 14 dias
não são configuráveis.

### Antigos nomes `EZYMONIT_*` {#former-ezymonit_-names}

O DumbMonit chamava-se EzyMonit até setembro de 2026. Cada variável
`DUMBMONIT_*` é também lida sob o seu antigo nome `EZYMONIT_*`, como alternativa,
com um aviso no arranque por cada variável ainda definida dessa forma
(`EZYMONIT_X is deprecated, use DUMBMONIT_X`). O antigo ficheiro de base de
dados `/data/ezymonit.db` é renomeado para `dumbmonit.db` no arranque, e os
tokens de agente `ezym_…` continuam válidos (os novos são `dmon_…`). O cookie de
sessão é agora `dumbmonit_session`: o primeiro arranque após a mudança de nome
termina a sessão de todos uma vez.

### VictoriaMetrics incorporado {#embedded-victoriametrics}

A imagem traz o binário do VictoriaMetrics. A menos que `DUMBMONIT_VM_URL` esteja
definida, o servidor inicia-o como processo filho no arranque, reencaminha as
suas linhas de registo para o registo do servidor, reinicia-o com um backoff se
morrer, e pára-o no encerramento (`SIGTERM`, 20 s de tolerância, depois kill; o
`stop_grace_period` do ficheiro Compose deixa espaço para isso).

| Variável | Predefinição | Função |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Caminho do binário do VictoriaMetrics a iniciar. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Endereço de escuta, dentro do contentor. Loopback por predefinição: só o servidor lhe fala. A sobreposição de desenvolvimento define `0.0.0.0:8428` e publica a porta. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retenção, na sintaxe de `-retentionPeriod`: um número de meses, ou uma duração como `30d` ou `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Orçamento para as caches do VictoriaMetrics (`-memory.allowedBytes`). Sem orçamento o VM dimensiona-as para 60 % da RAM do anfitrião e a sua memória residente cresce durante dias. Algumas dezenas de dispositivos cabem em 256 MB; aumente para centenas de anfitriões. |

Os seus dados vivem em `<data dir>/vm`, no mesmo volume que a base de dados.
Três flags são fixadas pelo servidor e não são configuráveis:
`-search.maxUniqueTimeseries=100000` (uma consulta descontrolada não pode esgotar
o orçamento), `-search.maxConcurrentRequests=4` (uma interface mais o motor de
alertas) e `-dedup.minScrapeInterval=10s` (pontos duplicados a menos de dez
segundos são colapsados; nada se perde na cadência de 30 s dos coletores).

Com `DUMBMONIT_VM_URL` definida, nada disto se aplica: o servidor usa a instância
que indicar e o incorporado nunca é iniciado. `GET /api/health` diz em que caso
está (`victoria.embedded`).

## Definições ao nível do Compose {#compose-level-variables}

O `docker-compose.yml` fornecido não lê nenhuma variável e não precisa de
ficheiro `.env`. Tudo se define em `environment:` do serviço ou editando o
ficheiro; veja [Opções avançadas](../install/docker.md#advanced-options) (porta,
segredo, retenção e memória, VictoriaMetrics externo, reposição da
palavra-passe, capturas de ecrã). A sobreposição de desenvolvimento
`docker-compose.dev.yml` lê `DUMBMONIT_VM_PORT` (predefinição `8428`) para a
porta do anfitrião do VictoriaMetrics incorporado.

## Portas {#ports}

| Porta | Contentor | Finalidade |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Interface web, API, instaladores e downloads do agente, ingestão do agente. A única porta a publicar. |
| `8428/tcp` | `dumbmonit` | API HTTP do VictoriaMetrics incorporado, no loopback do contentor. Publicada apenas pela sobreposição de desenvolvimento. |

De saída: SNMP (UDP 161 por predefinição) para os dispositivos, HTTPS para o
Proxmox, PBS, Synology e serviços de notificação, mais o que os seus monitores
de serviços visarem. O ping ICMP precisa do sysctl `net.ipv4.ping_group_range`
definido no ficheiro Compose (sem capability).

## Volumes e ficheiros {#volumes-and-files}

Um volume, `dumbmonit-data`, montado em `/data`. O seu nome é fixo no ficheiro
Compose, seja qual for o nome do projeto.

| Caminho (no contentor) | Volume | Conteúdo |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Dispositivos, credenciais (cifradas), regras, estado e histórico de alertas, linhas de base, silêncios, canais, tokens de agentes, hash da palavra-passe, sessões. |
| `/data/secret.key` | `dumbmonit-data` | O segredo da instância. **Faça cópia de segurança.** Sem ele, as credenciais cifradas são irrecuperáveis e o servidor recusa-se a arrancar. |
| `/data/backups/` | `dumbmonit-data` | Cópias de segurança locais agendadas: `dumbmonit-<timestamp>.db` (uma cópia SQLite em linha) e a `.key` correspondente. Diárias por predefinição, sete mantidas. |
| `/data/vm/` | `dumbmonit-data` | Séries temporais do VictoriaMetrics incorporado, 12 meses por predefinição. Vazia quando `DUMBMONIT_VM_URL` está definida. |
| `/agents/` | imagem | Binários do agente. Monte outra diretoria e defina `DUMBMONIT_AGENT_DIR` para fornecer os seus. |
| `/etc/ssl/certs/ca-certificates.crt` | imagem | Raízes TLS para HTTPS de saída. Monte aqui o seu próprio pacote para confiar numa autoridade privada. |

## Repor a palavra-passe {#resetting-the-password}

```bash
# acrescente DUMBMONIT_RESET_PASSWORD: "1" em environment: no docker-compose.yml
docker compose up -d
# abra a UI: /setup pede uma nova palavra-passe
# remova a linha de novo e depois:
docker compose up -d
```

A reposição apaga a palavra-passe e todas as sessões; dispositivos, regras e
canais ficam intocados.

## Agente {#agent}

O agente tem as suas próprias variáveis e ficheiro de configuração: veja
[Agente Linux, macOS, FreeBSD e Windows](../devices/agent.md#configuration-file).

### Âmbito da recolha {#collection-scope}

O que o agente mede, e portanto quantas séries uma máquina produz. As
predefinições mantêm um anfitrião com dezenas de contentores em algumas centenas
de séries; cada chave é uma entrada de `agent.yaml`, substituível pela sua
variável de ambiente.

| Chave | Variável | Predefinição | Função |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Interfaces de rede deixadas de fora: interfaces de contentores e VM, o loopback. Uma lista de nomes exatos ou expressões regulares (sem âncora; qualquer coisa com um metacaractere é uma regex). O valor substitui a predefinição; uma lista vazia mantém todas as interfaces. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(vazia)* | Lista de permissão, mesma sintaxe. Quando definida, só estas interfaces são reportadas e `interfaces_ignore` não é consultada. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Pontos de montagem deixados fora do uso do sistema de ficheiros e da E/S de disco, mesma sintaxe. As unidades amovíveis em `/run/media/` ficam. Os sistemas de ficheiros pseudo (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) são sempre ignorados, seja qual for o caminho. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Uma série `cpu_core_usage_percent` por núcleo, além do uso global. `cpu_count` é reportado de qualquer forma. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Contentores detalhados por ciclo, os que estão em execução primeiro. Além disso, os contentores são apenas contados (`container_count`, `container_running_count` continuam exatos) e `container_series_skipped` diz quantos. `0` conta sem detalhar. |

No ambiente, as listas são separadas por vírgulas (`^veth, ^br-, docker0`); em
YAML, ou uma lista ou uma cadeia separada por vírgulas. Os contadores de E/S de
disco (`disk_read_bytes`, `disk_written_bytes`) trazem apenas a etiqueta
`device`: uma série por dispositivo de blocos, quantas vezes quer que esteja
montado.
