# Referência do formato de pacote {#pack-format-reference}

Um pacote é um único arquivo YAML, `pack.yaml`, na versão de formato 1.
Chaves desconhecidas são erros, não configurações ignoradas: um erro de
digitação como `treshold` é reportado.

```text
my-pack/
  pack.yaml          o próprio pacote: o único arquivo que o servidor precisa
  fixtures/          uma resposta capturada por fonte, nomeada após seu id
    status.json
  expected.prom      o que a extração deve produzir a partir das fixtures
```

## Cabeçalho {#header}

| Chave | Obrigatório | Significado |
|---|---|---|
| `schema` | sim | Sempre `1`. |
| `id` | sim | Letras minúsculas, dígitos e hífens únicos, começando com uma letra, até 40 caracteres: `shelly-plug`. O tipo de monitor se torna `pack.<id>`, o prefixo de métrica `dumbmonit_<id com underscores>_`. |
| `version` | sim | [Versão semântica](https://semver.org) do pacote: `1.2.0`. |
| `requires` | não | Versões do DumbMonit com as quais o pacote funciona, como um requisito semver: `">=0.2"`. Um servidor fora da faixa recusa o pacote. |
| `label` | sim | Nome mostrado na lista de tipos de monitor. |
| `summary` | não | Uma frase: o que o pacote observa. |
| `examples` | não | Produtos a que se aplica, mostrados como dicas. |
| `address_hint` | não | Placeholder do campo de endereço: `192.168.1.50`. |
| `default_port` | não | Porta usada quando o endereço não tem nenhuma. |
| `scheme` | não | `http` (padrão) ou `https`; o monitor pode sobrescrever isso. |
| `setup` | não | O aviso mostrado ao lado do formulário de monitor: `title`, `steps` (uma lista; o que segue uma quebra de linha em um passo é mostrado como um comando copiável), `warning`, `doc_url`. |
| `credentials` | não | Tipos de credencial aceitos, em ordem de preferência: `none`, `api_token`, `username_password`. Padrão: `[none]`. Uma entrada pode ser `{kind, label, help}` para reformular o formulário. |
| `options` | não | Configurações mostradas no formulário de monitor (abaixo). |

## Opções {#options}

Opções são armazenadas no monitor e lidas por `{{option.<key>}}` em
caminhos, cabeçalhos e corpos, e por limites de regra.

```yaml
options:
  - key: switch_id          # minúsculas, dígitos, underscores
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (padrão), number, boolean ou select
    default: 0
    required: false
    placeholder: ""
    choices: []             # os valores de um select
```

Todo pacote que sonda também recebe cinco opções de conexão, que não pode
redeclarar: `scheme`, `port`, `insecure_tls`,
`request_timeout_seconds` (1 a 120, padrão 10) e `allow_private_targets`.

## Fontes {#sources}

Uma fonte é uma requisição por sondagem, sempre enviada ao próprio
endereço do monitor.

```yaml
sources:
  - id: status              # minúsculas, dígitos, underscores
    type: http              # http ou prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (padrão) ou POST
    format: json            # apenas http: json (padrão) ou text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # apenas POST
    auth: auto              # auto (padrão), none, bearer ou basic
```

- `path` precisa começar com uma única `/`. A URL da requisição é o
  endereço do monitor seguido do caminho; depois da substituição
  precisa manter o mesmo esquema, host e porta, ou a requisição é
  recusada. Valores de opção são codificados em percentual em caminhos,
  então um valor não pode adicionar um segmento ou mudar o host.
- `auth: auto` envia uma credencial `api_token` como
  `Authorization: Bearer …` e uma credencial `username_password` como
  autenticação básica. `none` não envia nada implicitamente, então um
  cabeçalho pode carregar a credencial em vez disso:
  `X-Api-Key: "{{credential.token}}"`. `{{credential.token}}`,
  `{{credential.username}}` e `{{credential.password}}` são aceitos
  **apenas em cabeçalhos**: nunca em um caminho ou corpo, que acabam em
  logs.
- Redirecionamentos são seguidos apenas para o mesmo esquema, host e
  porta, três no máximo. `401` e `403` são reportados como erros de
  autenticação, outros status não-2xx como erros de protocolo.
- Uma sondagem faz no máximo 16 requisições e lê no máximo 4 MiB por
  resposta.

### Fontes Prometheus {#prometheus-sources}

```yaml
sources:
  - id: metrics
    type: prometheus
    path: /metrics
    keep: [node_load1, node_memory_MemAvailable_bytes]
    rename:
      node_memory_MemAvailable_bytes: memory_available_bytes
    drop:
      fstype: ^(tmpfs|overlay)$
```

- `keep` lista as famílias importadas como estão, com todos seus
  rótulos. Um nome de família também cobre suas séries de histograma e
  resumo (`_bucket`, `_sum`, `_count`). Contadores, histogramas e somas
  de resumo permanecem contadores; todo o resto é um gauge.
- `rename` dá a uma família mantida seu nome DumbMonit. Um nome que não
  seja letras minúsculas, dígitos e underscores precisa ser renomeado.
- `drop` descarta toda amostra cujo rótulo corresponde à expressão
  regular.
- Rótulos que o DumbMonit define ele mesmo (`target`, `host`, `instance`,
  `job`, `tag_*`…) são mantidos como `exported_<name>`.

## Métricas {#metrics}

Cada métrica tem um nome *sem* prefixo: `power_watts` no pacote
`shelly-plug` é escrito `dumbmonit_shelly_plug_power_watts`. Nomes são
letras minúsculas, dígitos e underscores, e únicos dentro do pacote.

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (padrão) ou counter
    json: $.apower          # exatamente um de json, regex ou prom
    scale: 1                # multiplicador, ex. 0.001 para ms → s
    map: {on: 1, off: 0}    # valor de texto → número
    labels: {}
```

| Extrator | Fonte | Valor | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, formato `json` | Todo nó que o caminho corresponde ([RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath, wildcards e filtros incluídos) faz uma série. | rótulo → JSONPath avaliado no objeto que *contém* o valor: `$.sensors[*].value` com rótulos `sensor: $.name` rotula cada valor com seu `name` irmão. |
| `regex: <expression>` | `http`, formato `text` | Toda correspondência faz uma série; o valor é o grupo nomeado `value`, senão o primeiro grupo. | rótulo → nome de um grupo de captura. |
| `prom: <sample name>` | `prometheus` | Toda amostra com esse nome exato. | rótulo → rótulo da fonte. Vazio: todos os rótulos da fonte são mantidos. |

Valores: números são tomados como estão; `true`/`false` se tornam 1/0;
texto é buscado em `map`, ou analisado como número quando não há `map`.
Um valor que está faltando, `null`, ou fora do `map` não produz nada
nesta sondagem.

## Discover {#discover}

`discover` transforma cada elemento de uma lista em um conjunto de
séries compartilhando rótulos: uma linha por disco, sensor ou fila.

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath: uma linha por correspondência
    max_rows: 100           # padrão 100, no máximo 1.000
    labels:                 # JSONPath avaliado em cada linha
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath avaliado em cada linha
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

Métricas de linha aceitam `name`, `help`, `kind`, `json`, `scale` e
`map`. Linhas além de `max_rows` são ignoradas.

## Regras {#rules}

Regras são instaladas com o pacote, como regras de alerta normais com o
uid `pack:<id>:<name>`. Elas então pertencem a você: uma atualização do
pacote adiciona as regras que ainda não tem, e nunca reescreve uma que
você alterou ou excluiu.

```yaml
rules:
  - name: overheating       # minúsculas, dígitos, underscores
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < ou <=
    threshold: "{{option.max_temperature}}"   # ou um número
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (padrão) ou critical
    unit: °C
```

- `expr` é [MetricsQL](https://docs.victoriametrics.com/metricsql/) e
  precisa usar ao menos uma métrica do pacote, por seu nome completo.
  Qualquer outro nome `dumbmonit_*` que cita também precisa ser produzido
  pelo pacote (`dumbmonit_up` exceto): um pacote não pode alertar na
  série de outro tipo.
- Um `threshold` de `{{option.x}}` instala o valor padrão da opção, e o
  próprio valor da opção de cada monitor se torna seu limite para
  essa regra (uma substituição por monitor, reescrita sempre que o
  monitor é salvo).

## Perfis SNMP {#snmp-profiles}

Um pacote pode trazer perfis SNMP, escritos exatamente como os
integrados em
[`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles).
Eles se juntam ao catálogo SNMP e são aplicados por `sysObjectID` como os
outros; eles não criam um tipo de monitor próprio.

```yaml
snmp_profiles:
  - id: acme-ups            # o id do pacote, ou começando com "<pack id>-"
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # precisa começar com o prefixo do pacote
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

O catálogo SNMP é lido na inicialização: reinicie o DumbMonit depois de
instalar ou atualizar um pacote com perfis SNMP.

## Limites {#limits}

| Limite | Valor |
|---|---|
| Tamanho do pacote | 256 KiB |
| Fontes (requisições por sondagem, redirecionamentos incluídos) | 16 |
| Tamanho de resposta | 4 MiB |
| Séries por sondagem | 5.000 (o resto é descartado e registrado) |
| Métricas declaradas | 256 |
| Regras | 64 |
| Linhas por `discover` | 1.000 |
| Tamanho de valor de rótulo | 200 bytes |

## Fixtures e saída esperada {#fixtures-and-expected-output}

`dumbmonit pack test <dir>` lê `fixtures/<source id>.json`, `.txt` ou
`.prom`, executa a mesma extração que o servidor, e compara o resultado
com `expected.prom`: o formato de texto Prometheus, ordenado, sem
timestamps e sem os rótulos de monitor (`target`, `host`, `tag_*`)
que o servidor adiciona. `--update` escreve `expected.prom` a partir das
fixtures; revise o diff antes de fazer commit.
