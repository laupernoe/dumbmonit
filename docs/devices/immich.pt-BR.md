# Immich {#immich}

Se o Immich acompanha o que é enviado: filas de jobs que travam ou ficam
pausadas, atualizações, o disco que contém a biblioteca, quantas fotos e
vídeos ela contém, e — para cada celular, tablet e desktop conectado à
conta — sua última conexão e último backup.

O DumbMonit lê a API do Immich com uma **chave de API limitada a oito
permissões de leitura**. A chave não pode ver uma foto, um álbum ou uma
pessoa, e não pode alterar nada.

A falha que vale a pena observar: **jobs que não rodam mais**. Uma fila
pausada durante uma grande importação e esquecida, ou um worker de job que
parou, e fotos novas não recebem miniatura, nem metadados, nem
reconhecimento facial e nem busca inteligente, enquanto os envios continuam
tendo sucesso. O Immich só mostra isso em sua página Jobs.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_immich_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de versão do Immich conhece uma versão mais nova (ausente quando a verificação está desligada) | `latest_version` |
| `maintenance_mode` | 1 enquanto o Immich está em modo de manutenção | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | o disco que contém a biblioteca | |
| `photos`, `videos`, `usage_bytes`, `users` | tamanho da biblioteca, todos os usuários juntos | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | todas as filas juntas | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | por fila: esperando (incluindo atrasados e retidos por uma pausa), em execução, com falha, pausada | `queue` |
| `statistics_readable` | 0 quando a chave pertence a uma conta comum e as contagens e filas são ignoradas | |
| `scrape_errors` | chamadas que falharam na última sondagem | |

Versões recentes do Immich removem jobs com falha da fila uma vez que são
registrados, então `queue_failed` geralmente marca 0; um job que falhou
aparece no próprio log do Immich.

As [regras integradas](../alerting/rules.md#self-hosted-applications) que se
aplicam: Immich jobs stalled, Immich queue paused, Immich storage almost
full, Immich update available, além de Monitor unreachable.

## Dispositivos {#devices}

Todo dispositivo conectado à conta que possui a chave, com sua última
conexão e último backup, é publicado sob uma família genérica compartilhada
por toda integração do DumbMonit que rastreia dispositivos clientes —
`dumbmonit_client_device_*`, com prefixo sem `immich`, documentada
completamente em [Dispositivos clientes](../alerting/rules.md#client-devices):

| Métrica | O que é | Rótulos |
|---|---|---|
| `last_seen_timestamp_seconds` | Última conexão (a última atividade da sessão) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | Último backup; veja a limitação abaixo | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo uma vez que um dispositivo passou `device_stale_days` (3 por padrão) sem se conectar ou fazer backup; aciona a regra [Dispositivo cliente desatualizado](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (`connection` ou `backup`) |

**Limitação que vale a pena conhecer**: a API do Immich não registra qual
dispositivo enviou uma dada foto, então `last_backup_timestamp_seconds` é o
envio mais recente de toda a biblioteca — o mesmo valor para todo
dispositivo, publicado sob um dispositivo chamado `library` em vez de um
real. Ele detecta todo dispositivo tendo parado de uma vez (armazenamento
cheio, a conta desconectada em todo lugar); ele não detectará um único
dispositivo ficando silencioso enquanto outro continua enviando. Apenas
`last_seen_timestamp_seconds` — a última vez que a sessão de cada
dispositivo esteve ativa — é genuinamente por dispositivo.

A tabela da página do monitor mostra esses números para cada
dispositivo, com um status de OK ou Desatualizado.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca chama o Immich. Ele diz primeiro o que está errado (filas travadas,
filas pausadas, manutenção), depois atualizações, depois fotos, vídeos,
tamanho da biblioteca, disco usado e jobs esperando e em execução, depois as
filas que têm trabalho, pelo nome, depois a tabela de dispositivos (nome,
tipo, SO, última conexão, último backup, status).

## Criar uma chave de API somente leitura no Immich {#create-a-read-only-api-key-in-immich}

1. Entre no Immich com uma conta que possa abrir Administration: o Immich
   só permite que essas contas leiam contagens de fotos e filas de jobs.
   Abra Account Settings → API Keys → New API Key, nomeie-a como abaixo, e
   marque estas oito permissões: as primeiras cinco para filas de jobs,
   contagens e versão, as últimas três para listar os dispositivos da conta
   e sua última conexão e backup.

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read session.read asset.read user.read
    ```

2. Copie a chave agora: o Immich a mostra apenas uma vez. Ela lê essas oito
   páginas e nada mais: nenhuma foto, nenhum álbum, nenhuma configuração.
   Se seu Immich não oferece queue.read, marque job.read em vez disso.

3. Uma chave feita por uma conta comum também funciona: versão,
   atualizações, espaço em disco e dispositivos são lidos, contagens de
   fotos e filas de jobs são ignoradas.

4. No DumbMonit, informe o endereço do Immich, por exemplo "immich.lan"
   (porta 2283) ou "https://photos.example.com" detrás de um proxy
   reverso, e cole a chave.

!!! warning
    Uma fila pausada é fácil de esquecer: fotos novas não recebem
    miniatura, metadados ou reconhecimento facial, e o Immich só avisa isso
    em sua página Jobs. O DumbMonit avisa quando jobs esperam em uma fila
    que não está pausada e não executa nada. session.read, asset.read e
    user.read listam todo dispositivo da conta que possui a chave — seus
    celulares, tablets e navegadores — com a última conexão de cada um. A
    API do Immich não registra qual dispositivo enviou uma dada foto, então
    "último backup" é o envio mais recente de toda a biblioteca, mostrado
    igual para todo dispositivo: se apenas um dispositivo parar de fazer
    backup enquanto os outros continuam, sua conexão aparecerá como
    desatualizada mas o número de backup compartilhado ainda parecerá
    recente.

O que cada permissão abre: `server.about` a versão
(`/api/server/about`), `server.versionCheck` a versão mais recente que o
Immich conhece, `server.storage` o disco, `server.statistics` contagens de
fotos e vídeos, `queue.read` as filas de jobs (`/api/queues`; versões mais
antigas respondem em `/api/jobs` com `job.read`), `session.read` os
dispositivos da conta (`/api/sessions`), `asset.read` o envio mais recente
da biblioteca (`/api/search/metadata`), `user.read` o nome de exibição da
conta (`/api/users/me`).

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API | Enviada no cabeçalho `x-api-key`. |

Endereço: um nome de host (`immich.lan`), `host:porta`, ou uma URL completa
com um prefixo de caminho; um `/api` final é aceito. Opções: protocolo
(HTTP por padrão), porta (2283), verificação de certificado, tempo limite
da requisição (15 s), e desatualização de dispositivo (`device_stale_days`,
3 dias por padrão).

Validado contra o Immich v3.2.4 (imagens oficiais, aprendizado de máquina
desligado); os endpoints de dispositivo foram verificados contra a tag
`v3.2.4` da própria especificação OpenAPI do Immich (`immich-app/immich`),
que confirmou que `/api/search/metadata` não aceita mais um filtro
`deviceId` — o motivo pelo qual "último backup" é de toda a biblioteca em
vez de por dispositivo.
