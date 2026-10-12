# Immich {#immich}

Se o Immich acompanha o que é enviado: filas de tarefas que bloqueiam ou
ficam pausadas, atualizações, o disco que detém a biblioteca, quantas
fotos e vídeos contém, e — para cada telefone, tablet e desktop com
sessão iniciada na conta — a sua última ligação e última cópia de
segurança.

O DumbMonit lê a API do Immich com uma **chave de API limitada a oito
permissões de leitura**. A chave não pode ver uma foto, um álbum ou uma
pessoa, e não pode alterar nada.

A falha a vigiar: **tarefas que deixam de correr**. Uma fila pausada
durante uma grande importação e esquecida, ou um trabalhador de tarefas
que parou, e as fotos novas não recebem miniatura, nem metadados, nem
rosto, nem pesquisa inteligente, enquanto os envios continuam a ter
sucesso. O Immich só o mostra na sua página Jobs.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_immich_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de versão do Immich conhece uma versão mais recente (ausente quando a verificação está desligada) | `latest_version` |
| `maintenance_mode` | 1 enquanto o Immich está em modo de manutenção | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | o disco que detém a biblioteca | |
| `photos`, `videos`, `usage_bytes`, `users` | tamanho da biblioteca, todos os utilizadores juntos | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | todas as filas juntas | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | por fila: à espera (incluindo adiadas e retidas por uma pausa), a correr, falhadas, pausadas | `queue` |
| `statistics_readable` | 0 quando a chave pertence a uma conta comum e as contagens e filas são ignoradas | |
| `scrape_errors` | chamadas que falharam na última verificação | |

As versões recentes do Immich retiram as tarefas falhadas da fila uma vez
registadas, por isso `queue_failed` geralmente lê 0; uma tarefa falhada
mostra-se no próprio registo do Immich.

As [regras incorporadas](../alerting/rules.md#self-hosted-applications)
que se aplicam: Immich jobs stalled, Immich queue paused, Immich storage
almost full, Immich update available, mais Device unreachable.

## Dispositivos {#devices}

Todo dispositivo com sessão iniciada na conta que detém a chave, com a
sua última ligação e última cópia de segurança, é publicado sob uma
família genérica partilhada por toda integração do DumbMonit que
acompanha dispositivos cliente — `dumbmonit_client_device_*`, prefixada
sem `immich`, documentada na íntegra em
[Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | O quê | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última ligação (a última atividade da sessão) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | Última cópia de segurança; ver o limite abaixo | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo uma vez que um dispositivo passou `device_stale_days` (3 por predefinição) sem se ligar ou fazer cópia de segurança; alimenta a regra [Dispositivo obsoleto](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (`connection` ou `backup`) |

**Limite a conhecer**: a API do Immich não regista qual dispositivo
enviou uma determinada foto, por isso `last_backup_timestamp_seconds` é
o envio mais recente de toda a biblioteca — o mesmo valor para cada
dispositivo, publicado sob um dispositivo chamado `library` em vez de um
real. Apanha todo dispositivo a parar de uma vez (armazenamento cheio, a
conta desligada em todo o lado); não apanha um único dispositivo a ficar
silencioso enquanto outro continua a enviar. Só
`last_seen_timestamp_seconds` — a última vez que a sessão de cada
dispositivo esteve ativa — é genuinamente por dispositivo.

A tabela da página do dispositivo mostra estes valores para cada
dispositivo, com um estado OK ou Stale.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a
página nunca chama o Immich. Diz primeiro o que está mal (filas
bloqueadas, filas pausadas, manutenção), depois atualizações, depois
fotos, vídeos, tamanho da biblioteca, disco usado e tarefas à espera e a
correr, depois as filas com trabalho, pelo nome, depois a tabela de
dispositivos (nome, tipo, SO, última ligação, última cópia de segurança,
estado).

## Criar uma chave de API só de leitura no Immich {#create-a-read-only-api-key-in-immich}

1. Inicie sessão no Immich com uma conta que pode abrir Administration: o Immich só deixa essas contas lerem contagens de fotos e filas de tarefas. Abra Account Settings → API Keys → New API Key, dê-lhe o seguinte nome, e marque estas oito permissões: as primeiras cinco para filas de tarefas, contagens e versão, as últimas três para listar os dispositivos da conta e a sua última ligação e cópia de segurança.

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read session.read asset.read user.read
    ```

2. Copie a chave agora: o Immich mostra-a só uma vez. Lê essas oito páginas e nada mais: nenhuma foto, nenhum álbum, nenhuma definição. Se o seu Immich não oferecer queue.read, marque job.read em vez disso.

3. Uma chave feita por uma conta comum também funciona: versão, atualizações, espaço em disco e dispositivos são lidos, contagens de fotos e filas de tarefas são ignoradas.

4. No DumbMonit, introduza o endereço do Immich, por exemplo «immich.lan» (porta 2283) ou «https://photos.example.com» atrás de um proxy inverso, e cole a chave.

!!! warning
    Uma fila pausada é fácil de esquecer: as fotos novas não recebem miniatura, metadados nem rosto, e o Immich só o diz na sua página Jobs. O DumbMonit avisa quando tarefas esperam numa fila que não está pausada e não corre nada. session.read, asset.read e user.read listam todo dispositivo da conta que detém a chave — os seus telefones, tablets e navegadores — com a última ligação de cada um. A API do Immich não regista qual dispositivo enviou uma determinada foto, por isso «last backup» é o envio mais recente de toda a biblioteca, mostrado igual para cada dispositivo: se só um dispositivo deixar de fazer cópia de segurança enquanto os outros continuam, a sua ligação mostrar-se-á como obsoleta mas o valor de cópia de segurança partilhado continuará a parecer recente.

O que cada permissão abre: `server.about` a versão
(`/api/server/about`), `server.versionCheck` a versão mais recente que o
Immich conhece, `server.storage` o disco, `server.statistics` contagens
de fotos e vídeos, `queue.read` as filas de tarefas (`/api/queues`;
versões mais antigas respondem em `/api/jobs` com `job.read`),
`session.read` os dispositivos da conta (`/api/sessions`), `asset.read`
o envio mais recente da biblioteca (`/api/search/metadata`), `user.read`
o nome de exibição da conta (`/api/users/me`).

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API | Enviada no cabeçalho `x-api-key`. |

Endereço: um nome de anfitrião (`immich.lan`), `anfitrião:porta`, ou um
URL completo com um prefixo de caminho; um `/api` final é aceite. Opções:
protocolo (HTTP por predefinição), porta (2283), verificação de
certificado, tempo limite do pedido (15 s), e obsolescência de
dispositivo (`device_stale_days`, 3 dias por predefinição).

Validado com o Immich v3.2.4 (imagens oficiais, aprendizagem automática
desligada); os endpoints de dispositivo foram verificados contra a tag
`v3.2.4` da própria especificação OpenAPI do Immich
(`immich-app/immich`), que confirmou que `/api/search/metadata` já não
aceita um filtro `deviceId` — a razão pela qual «last backup» é à escala
da biblioteca em vez de por dispositivo.
