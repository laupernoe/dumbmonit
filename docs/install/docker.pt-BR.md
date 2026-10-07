# Instalar com Docker {#install-with-docker}

O DumbMonit roda como um único contêiner: o servidor `dumbmonit` (coleta, API,
alertas, interface web) inicia seu próprio VictoriaMetrics para armazenar as séries
temporais; o binário acompanha a imagem. A configuração e o estado ficam em um banco
SQLite embutido. Um volume, `/data`, guarda o banco, o segredo da instância e as
séries temporais.

!!! note "Imagem alfa"
    `ghcr.io/laupernoe/dumbmonit:latest` é a última build alfa com tag (amd64;
    as imagens arm64 estão pausadas por enquanto); `:edge` acompanha o último commit de `main`. Para rodar a partir do código-fonte,
    `docker compose up -d --build` compila a mesma imagem localmente (cerca de dez
    minutos na primeira vez; só o Docker é necessário).

## Pré-requisitos {#prerequisites}

- Docker Engine com o plugin Compose (`docker compose version` funciona).
- Uma máquina que alcance os dispositivos que você quer monitorar. Ela não precisa
  ser alcançável por eles, exceto no caso dos [agentes](../devices/agent.md), que enviam
  suas medições ao servidor por HTTP.
- A porta `8080` livre no host, ou outra porta de sua escolha (veja abaixo).

## O arquivo Compose {#the-compose-file}

Este é o `docker-compose.yml` do repositório; salve-o em um diretório
só dele:

```yaml
# Um contêiner: o servidor DumbMonit executa seu próprio VictoriaMetrics (embutido na
# imagem) e mantém tudo em um único volume. Para usar um VictoriaMetrics externo,
# defina DUMBMONIT_VM_URL e o embutido não será iniciado.
name: dumbmonit

services:
  dumbmonit:
    # `docker compose up -d` baixa a imagem publicada (última versão; use
    # `:edge` para o último commit de main). Para compilar a partir deste checkout,
    # execute `docker compose up -d --build`: o resultado recebe o mesmo nome
    # e passa a ser usado a partir daí.
    image: ghcr.io/laupernoe/dumbmonit:latest
    build: .
    ports:
      # A porta do host é configurável: 8080 é uma porta muito disputada em uma
      # máquina de homelab. `DUMBMONIT_PORT=8099 docker compose up -d` a muda.
      - "${DUMBMONIT_PORT:-8080}:8080"
    volumes:
      # Banco SQLite, segredo da instância e as séries temporais (/data/vm).
      #
      # O servidor roda como usuário 65532 (não root). Um volume nomeado criado pelo
      # Docker herda esse dono da imagem: nada a fazer. Um bind mount
      # (`./data:/data`) ou um volume criado antes dessa mudança precisa ser
      # transferido uma vez:
      #   docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
      # ou, para manter os arquivos como estão, execute o contêiner como o dono deles
      # com `user: "1000:1000"` (qualquer uid serve: a imagem não tem /etc/passwd).
      - dumbmonit-data:/data
    environment:
      # Descomente para definir o segredo você mesmo em vez de deixar o DumbMonit
      # gerá-lo em /data/secret.key. Ele criptografa as credenciais dos dispositivos: perdê-lo
      # significa digitar todas elas de novo.
      # DUMBMONIT_SECRET: replace-me-with-32-random-characters
      # Senha perdida: `DUMBMONIT_RESET_PASSWORD=1 docker compose up -d` apaga
      # a senha na inicialização e a interface pede uma nova; depois inicie de novo
      # sem a variável.
      DUMBMONIT_RESET_PASSWORD: ${DUMBMONIT_RESET_PASSWORD:-}
      # VictoriaMetrics embutido: retenção (meses, ou p. ex. 30d / 2y) e o
      # orçamento de memória de seus caches. Um homelab com algumas dezenas de dispositivos cabe em
      # 256 MB; aumente quando monitorar centenas de hosts.
      DUMBMONIT_VM_RETENTION: ${DUMBMONIT_VM_RETENTION:-12}
      DUMBMONIT_VM_MEMORY: ${DUMBMONIT_VM_MEMORY:-256MB}
      # VictoriaMetrics externo no lugar do embutido:
      # DUMBMONIT_VM_URL: http://victoriametrics:8428
    # Privilégio mínimo: nenhuma capability, sem escalada de privilégios, e o
    # sistema de arquivos da imagem somente leitura — /data (volume) e /tmp (tmpfs) são os
    # únicos locais graváveis. Os monitores "ping" ICMP também não precisam de capability: o
    # sysctl abaixo permite que o servidor (sem privilégios) abra sockets ICMP echo dentro
    # do namespace de rede do próprio contêiner. Remova-o se nunca usar ping.
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    read_only: true
    tmpfs:
      - /tmp
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
    # O servidor para o VictoriaMetrics depois de si mesmo: dê tempo para isso.
    stop_grace_period: 30s
    restart: unless-stopped

volumes:
  dumbmonit-data:
    # Nome fixo, independente do nome do projeto compose: o volume é o que
    # você faz backup e o que uma atualização precisa encontrar de novo.
    name: dumbmonit-data
```

Em seguida, inicie:

```bash
docker compose up -d
```

!!! tip "Compilando a imagem você mesmo"
    `docker compose up -d` baixa a imagem publicada e ignora `build: .`.
    A partir de um clone do repositório, `docker compose up -d --build` compila a
    mesma imagem localmente (cerca de dez minutos a frio; não é necessário Rust nem Node
    no host). Use isso para rodar a partir do código-fonte ou de um
    branch.

## VictoriaMetrics embutido {#embedded-victoriametrics}

A imagem contém o binário do VictoriaMetrics (`/victoria-metrics-prod`, de
`victoriametrics/victoria-metrics:v1.152.0`). Quando `DUMBMONIT_VM_URL` não está
definida, o servidor o inicia como processo filho escutando em `127.0.0.1:8428`,
armazena suas séries em `/data/vm`, encaminha suas linhas de log para o próprio log,
o reinicia com backoff se ele morrer e o encerra no desligamento. Nada é
publicado: a porta fica dentro do contêiner.

Duas variáveis merecem ser conhecidas: `DUMBMONIT_VM_RETENTION` (`12` meses por
padrão; `30d` ou `2y` também funcionam) e `DUMBMONIT_VM_MEMORY` (`256MB`, o
orçamento de seus caches; aumente para centenas de hosts). O restante está na
[referência de configuração](../reference/configuration.md#embedded-victoriametrics).

Para usar um VictoriaMetrics que você já executa, defina `DUMBMONIT_VM_URL` com o
endereço dele (`http://host:8428`): o embutido então não é iniciado e
`/data/vm` fica vazio.

## Primeira inicialização {#first-start}

Abra `http://<seu-host>:8080`. Uma instância nova mostra a tela `/setup`,
onde você cria a primeira conta de administrador (senha de pelo menos 12
caracteres).

A tela pede primeiro o **código de configuração**. Enquanto não existir administrador,
o servidor imprime um código de uso único nos logs na inicialização:

```sh
docker compose logs dumbmonit | grep -A1 "setup code"
```

```
  First-run setup code: K7XQ4-M9PRT
```

Ele comprova que quem cria a conta de administrador controla o servidor, de modo que outra
pessoa na rede não possa reivindicar uma instância nova primeiro. O código fica
apenas na memória, muda a cada reinício até que exista um administrador, e as
tentativas erradas têm limite de frequência. Para uma implantação automatizada, defina
`DUMBMONIT_SETUP_CODE` para escolhê-lo você mesmo.

![A tela de login](../assets/screenshots/login-light.png){ loading=lazy }

Depois adicione seu primeiro dispositivo: veja [Adicione seu primeiro dispositivo](first-device.md).

## Variáveis de ambiente {#environment-variables}

Tudo passa por variáveis de ambiente; nenhuma é obrigatória.

| Variável | Padrão | Função |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Endereço de escuta dentro do contêiner. |
| `DUMBMONIT_DATA_DIR` | `/data` | Banco SQLite (`dumbmonit.db`), segredo da instância (`secret.key`) e os dados do VictoriaMetrics embutido (`vm/`). |
| `DUMBMONIT_VM_URL` | *(não definida)* | URL de um VictoriaMetrics externo. Quando definida, o embutido não é iniciado. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retenção do VictoriaMetrics embutido: meses, ou `30d`, `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Orçamento de memória dos caches do VictoriaMetrics embutido. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Endereço de escuta do VictoriaMetrics embutido, dentro do contêiner. |
| `DUMBMONIT_SECRET` | *(gerado)* | Segredo da instância que criptografa credenciais de dispositivos e tokens. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Sondagens simultâneas, todos os coletores somados. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Duração máxima de uma sondagem. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Período de gravação para o VictoriaMetrics. |
| `DUMBMONIT_LOG` | `info` | Filtro de log (sintaxe do `tracing`, p. ex. `debug`, `dumbmonit=trace`). |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Binários dos agentes servidos em `/download/…`. |
| `DUMBMONIT_RESET_PASSWORD` | *(vazia)* | Defina como `1` para apagar a senha e todas as sessões na inicialização. |
| `DUMBMONIT_SETUP_CODE` | *(gerado)* | Código de configuração pedido por `/setup` enquanto não existir administrador. Se não definido, um código aleatório é impresso nos logs a cada inicialização. |
| `DUMBMONIT_COOKIE_SECURE` | *(desligado)* | Defina como `1` atrás de um proxy reverso TLS para marcar o cookie de sessão como `Secure`. |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Período de avaliação dos alertas (nunca abaixo de 10). |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | 90 dias | Retenção do histórico de alertas. Veja a [referência de configuração](../reference/configuration.md) para uma ressalva sobre sua unidade. |
| `DUMBMONIT_BACKUP_ENABLED` | ligado | Backups locais agendados do banco em `/data/backups/`. `DUMBMONIT_BACKUP_DIR`, `_INTERVAL_HOURS` (24) e `_KEEP` (7) os ajustam; veja [Backup e restauração](backup.md). |

A lista completa, com detalhes, está na [referência de configuração](../reference/configuration.md).
Os nomes `EZYMONIT_*` anteriores à renomeação ainda são lidos como alternativa; veja
[Atualização](#upgrading).

## Faça backup da chave secreta {#back-up-the-secret-key}

Comunidades SNMP, senhas e tokens de API são criptografados com AES-256-GCM usando uma
chave derivada do segredo da instância. Na primeira inicialização, o DumbMonit gera esse
segredo em `/data/secret.key` (dentro do volume `dumbmonit-data`).

!!! danger "Faça backup de `secret.key` junto com o banco de dados"
    Sem ela, as credenciais dos dispositivos são irrecuperáveis: um banco restaurado
    sozinho resulta em uma instância que não consegue falar com nada. O servidor detecta uma
    chave ausente ou alterada na inicialização e se recusa a continuar com uma mensagem
    explícita, em vez de falhar silenciosamente a cada sondagem. Os backups agendados
    a copiam para você — veja [Backup e restauração](backup.md).

Se preferir ser dono do segredo, defina `DUMBMONIT_SECRET` no arquivo Compose
(32 caracteres aleatórios ou mais). Guarde-o no seu gerenciador de senhas.

## Atualização {#upgrading}

```bash
docker compose pull
docker compose up -d
```

As migrações do banco rodam na inicialização. Os dados do VictoriaMetrics não são tocados.

!!! tip "Faça um backup antes"
    **Settings → Backup → Back up now** grava em poucos segundos uma cópia consistente do banco
    e de `secret.key` em `/data/backups/`, e é a partir dela que você
    restaura se a atualização der errado. Veja
    [Backup e restauração](backup.md).

### A partir da 0.1.0-alpha.1 {#from-010-alpha1}

Desde a 0.1.0-alpha.2, o contêiner roda como usuário 65532 em vez de root. Um
volume criado pela alpha.1 ainda pertence ao root, e o servidor se recusa a
iniciar (`/data is not writable by the server`). Transfira o volume uma vez:

```bash
docker compose down
docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
docker compose up -d
```

Ou mantenha os arquivos como estão e execute o contêiner como o dono deles, com
`user: "0:0"` (ou o uid de um bind mount) no serviço em
`docker-compose.yml`.

### A partir do EzyMonit e da configuração com dois contêineres {#from-ezymonit-and-from-the-two-container-setup}

O DumbMonit se chamava EzyMonit até setembro de 2026, e rodava como dois contêineres,
o servidor e um serviço `victoriametrics` separado. A atualização exige mover uma vez os dados
do projeto antigo para o novo volume. O projeto antigo se chamava
`ezymonit` (seus volumes `ezymonit_ezymonit-data` e `ezymonit_vm-data`;
`docker volume ls` confirma). A partir do diretório antigo:

```bash
docker compose down
```

Depois, com o novo `docker-compose.yml`:

```bash
docker volume create dumbmonit-data
docker run --rm -v ezymonit_ezymonit-data:/from -v dumbmonit-data:/to alpine cp -a /from/. /to/
docker run --rm -v ezymonit_vm-data:/from -v dumbmonit-data:/to alpine sh -c 'mkdir -p /to/vm && cp -a /from/. /to/vm/'
docker compose up -d
```

O diretório de dados do VictoriaMetrics antigo tem a estrutura que o embutido usa:
nada a converter. A alternativa é manter o VictoriaMetrics externo e
apontar `DUMBMONIT_VM_URL` para ele.

O que o servidor resolve sozinho na primeira inicialização:

- As variáveis `EZYMONIT_*` ainda são lidas como alternativa, com um aviso na inicialização
  por variável (`EZYMONIT_X is deprecated, use DUMBMONIT_X`).
- `/data/ezymonit.db` é renomeado para `dumbmonit.db`.
- Os tokens de agente `ezym_…` continuam funcionando; os novos são `dmon_…`. Os agentes continuam
  enviando dados; execute novamente o comando de instalação em cada máquina quando for conveniente, ele
  [migra o serviço antigo no lugar](../devices/agent.md#upgrade).
- O cookie de sessão mudou de nome: todos entram de novo uma vez. A preferência de
  tema do navegador é mantida.

O que ele não resolve: o prefixo das métricas mudou de `ezymonit_` para
`dumbmonit_` sem compatibilidade. As séries antigas permanecem no VictoriaMetrics com
o nome antigo e expiram conforme a retenção; os gráficos e as regras integradas recomeçam
a partir da atualização. Regras de alerta personalizadas que citam métricas `ezymonit_…` precisam ser
editadas.

## Backup e restauração {#backup-and-restore}

Um volume nomeado, `dumbmonit-data`, guarda tudo:

| Caminho | Conteúdo |
|---|---|
| `dumbmonit.db` | Dispositivos, regras, canais, estado dos alertas, sessões. |
| `secret.key` | O segredo da instância. |
| `backups/` | Os backups locais agendados: uma cópia do banco e do segredo, diária por padrão, sete mantidas. |
| `vm/` | Séries temporais do VictoriaMetrics (12 meses de retenção por padrão). |

O DumbMonit faz backup do próprio banco em um agendamento, e exporta toda a
configuração — dispositivos, credenciais, regras, canais — como um único arquivo
criptografado com uma senha que você escolhe, que pode ser restaurado em uma instância nova.
Ambos estão em **[Backup e restauração](backup.md)**, junto com o que fazer antes de
uma atualização e a única regra sobre `secret.key` que vale a pena ler duas vezes.

Para arquivar o próprio volume, gráficos incluídos, pare a stack e compacte-o com tar:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Para restaurar, crie o volume, extraia o arquivo nele da mesma forma e depois
`docker compose up -d`.

Se o espaço for importante, `--exclude=./vm` mantém o arquivo pequeno: o banco e
o segredo são a configuração, `vm/` são apenas os gráficos.

## Mudando a porta {#changing-the-port}

Defina `DUMBMONIT_PORT` ao iniciar:

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Ou coloque `DUMBMONIT_PORT=8099` em um arquivo `.env` ao lado do arquivo Compose.

## Proxy reverso {#reverse-proxy}

A interface e a API são servidas em uma única porta por HTTP simples. Nenhum WebSocket é usado:
a interface consulta a API periodicamente, então qualquer proxy reverso funciona sem configuração
especial.

=== "Caddy"

    ```
    monit.example.com {
        reverse_proxy 127.0.0.1:8080
    }
    ```

=== "nginx"

    ```nginx
    server {
        listen 443 ssl;
        server_name monit.example.com;
        # ssl_certificate / ssl_certificate_key …

        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            # Os agentes enviam lotes de recuperação após uma queda:
            client_max_body_size 16m;
        }
    }
    ```

Quando o proxy termina o TLS, defina `DUMBMONIT_COOKIE_SECURE: "1"` para que o cookie
de sessão seja enviado apenas por HTTPS. Não o defina em uma implantação HTTP simples: o
navegador nunca devolveria o cookie e o login seria impossível.

!!! note "Agentes atrás de um proxy"
    O comando de instalação exibido ao criar um token de agente usa a URL que o seu
    navegador usou para acessar a interface. Se for a URL do proxy, os agentes também a
    usarão: garanta que `/install.sh`, `/install.ps1`, `/download/…` e `/api/ingest`
    passem pelo proxy.

## Roda como usuário não root {#runs-as-a-non-root-user}

O contêiner executa o servidor como usuário `65532:65532` (numérico: a imagem `scratch`
não tem `/etc/passwd`), com todas as capabilities removidas, `no-new-privileges`
e um sistema de arquivos raiz somente leitura; `/data` (o volume) e `/tmp` (um tmpfs)
são os únicos caminhos graváveis. Tudo isso está no arquivo Compose acima.

Um volume nomeado criado pelo Docker na primeira inicialização herda o dono de `/data`
da imagem: nada a fazer. Dois casos exigem um comando:

- **Um volume criado por uma versão anterior** (o servidor rodava como root,
  então os arquivos pertencem ao root). Transfira-os uma vez, com a stack parada:

  ```bash
  docker compose stop
  docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
  docker compose up -d
  ```

  Caso contrário, o sintoma é um contêiner que sai imediatamente com
  `creating directory /data … Permission denied` (ou `unable to open database
  file`).

- **Um bind mount** (`./data:/data`) mantém o dono do diretório do host.
  Use `chown -R 65532:65532 ./data`, ou execute o contêiner como o dono do
  diretório: `user: "1000:1000"` no serviço, qualquer uid serve.

## Ping ICMP sem capability {#icmp-ping-without-a-capability}

O [monitor de ping](../devices/services.md#ping) envia ecos ICMP. Em vez de
um socket raw — que exige `NET_RAW`, e uma capability concedida a um usuário
não root do contêiner não é utilizável de qualquer forma — ele usa um socket ICMP echo, que o Linux
permite aos grupos listados em `net.ipv4.ping_group_range`. O arquivo Compose
define esse sysctl dentro do namespace de rede do próprio contêiner:

```yaml
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Nada muda no host. O Docker 20.10 e posteriores definem essa faixa em todo
contêiner por conta própria; as linhas explícitas garantem que ela valha em engines mais antigos e
no Podman. Sem ela, a verificação informa um erro de configuração (exibido no
dispositivo, sem notificação), nunca um falso "host fora do ar". Com `docker run`, passe
`--sysctl net.ipv4.ping_group_range="0 2147483647"`.

## Testando sem hardware {#testing-without-hardware}

Adicione um [dispositivo de demonstração](../devices/demo.md): ele produz medições falsas
sem nada a preparar, de modo que gráficos, regras e notificações podem ser testados
imediatamente. Para dados SNMP reais, a máquina que executa o Docker muitas vezes basta —
instale o `snmpd` nela (`apt install snmpd`, permita a comunidade `public` na bridge
do Docker em `/etc/snmp/snmpd.conf`) e adicione um dispositivo SNMP com o
endereço do host nessa bridge (`172.17.0.1` por padrão): o perfil é detectado
automaticamente e interfaces, memória e processos aparecem.

Desenvolvedores podem adicionar o overlay `docker-compose.dev.yml` para publicar o
VictoriaMetrics embutido em `:8428` do host, para consultas MetricsQL diretas:

```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```
