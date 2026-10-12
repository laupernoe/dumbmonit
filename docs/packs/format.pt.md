# Referência do formato de pacote {#pack-format-reference}

Um pacote é um único ficheiro YAML, `pack.yaml`, na versão de formato
1. Chaves desconhecidas são erros, não definições ignoradas: um erro
de escrita como `treshold` é reportado.

```text
my-pack/
  pack.yaml          o próprio pacote: o único ficheiro que o servidor precisa
  fixtures/          uma resposta capturada por fonte, nomeada pelo seu id
    status.json
  expected.prom      o que a extração tem de produzir a partir dos fixtures
```

## Cabeçalho {#header}

| Chave | Obrigatória | Significado |
|---|---|---|
| `schema` | sim | Sempre `1`. |
| `id` | sim | Letras minúsculas, dígitos e hífenes simples, começando com uma letra, até 40 caracteres: `shelly-plug`. O tipo de dispositivo torna-se `pack.<id>`, o prefixo de métrica `dumbmonit_<id com underscores>_`. |
| `version` | sim | [Versão semântica](https://semver.org) do pacote: `1.2.0`. |
| `requires` | não | Versões do DumbMonit com que o pacote funciona, como um requisito semver: `">=0.2"`. Um servidor fora do intervalo recusa o pacote. |
| `label` | sim | Nome mostrado na lista de tipos de dispositivo. |
| `summary` | não | Uma frase: o que o pacote monitoriza. |
| `examples` | não | Produtos a que se aplica, mostrados como sugestões. |
| `address_hint` | não | Marcador de posição do campo de endereço: `192.168.1.50`. |
| `default_port` | não | Porta usada quando o endereço não tem nenhuma. |
| `scheme` | não | `http` (predefinição) ou `https`; o dispositivo pode substituí-lo. |
| `setup` | não | O aviso mostrado junto ao formulário de dispositivo: `title`, `steps` (uma lista; o que segue uma quebra de linha num passo é mostrado como um comando copiável), `warning`, `doc_url`. |
| `credentials` | não | Tipos de credencial aceites, por ordem de preferência: `none`, `api_token`, `username_password`. Predefinição: `[none]`. Uma entrada pode ser `{kind, label, help}` para reformular o formulário. |
| `options` | não | Definições mostradas no formulário de dispositivo (abaixo). |

## Opções {#options}

As opções são guardadas no dispositivo e lidas por `{{option.<key>}}`
em caminhos, cabeçalhos e corpos, e por limiares de regra.

```yaml
options:
  - key: switch_id          # lowercase, digits, underscores
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (default), number, boolean or select
    default: 0
    required: false
    placeholder: ""
    choices: []             # the values of a select
```

Todo pacote que sonda também recebe cinco opções de ligação, que não
pode redeclarar: `scheme`, `port`, `insecure_tls`,
`request_timeout_seconds` (1 a 120, predefinição 10) e
`allow_private_targets`.

## Fontes {#sources}

Uma fonte é um pedido por sondagem, sempre enviado ao próprio endereço
do dispositivo.

```yaml
sources:
  - id: status              # lowercase, digits, underscores
    type: http              # http or prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (default) or POST
    format: json            # http only: json (default) or text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # POST only
    auth: auto              # auto (default), none, bearer or basic
```

- `path` tem de começar com uma única `/`. O URL do pedido é o
  endereço do dispositivo seguido do caminho; depois da substituição
  tem de manter o mesmo esquema, anfitrião e porta, ou o pedido é
  recusado. Os valores de opção são percent-encoded nos caminhos, por
  isso um valor não pode adicionar um segmento ou alterar o
  anfitrião.
- `auth: auto` envia uma credencial `api_token` como
  `Authorization: Bearer …` e uma credencial `username_password` como
  autenticação básica. `none` não envia nada implicitamente, por isso
  um cabeçalho pode transportar a credencial em vez disso:
  `X-Api-Key: "{{credential.token}}"`. `{{credential.token}}`,
  `{{credential.username}}` e `{{credential.password}}` são aceites
  **só em cabeçalhos**: nunca num caminho ou num corpo, que acabam em
  registos.
- Os redirecionamentos só são seguidos para o mesmo esquema,
  anfitrião e porta, no máximo três. `401` e `403` são reportados
  como erros de autenticação, outros estados não-2xx como erros de
  protocolo.
- Uma sondagem faz no máximo 16 pedidos e lê no máximo 4 MiB por
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

- `keep` lista as famílias importadas como estão, com todas as suas
  etiquetas. Um nome de família também cobre as suas séries de
  histograma e resumo (`_bucket`, `_sum`, `_count`). Contadores,
  histogramas e somas de resumo continuam contadores; tudo o resto é
  um gauge.
- `rename` dá a uma família mantida o seu nome DumbMonit. Um nome que
  não seja letras minúsculas, dígitos e underscores tem de ser
  renomeado.
- `drop` descarta toda amostra cuja etiqueta corresponda à expressão
  regular.
- As etiquetas que o DumbMonit define ele próprio (`target`, `host`,
  `instance`, `job`, `tag_*`…) são mantidas como `exported_<name>`.

## Métricas {#metrics}

Cada métrica tem um nome *sem* prefixo: `power_watts` no pacote
`shelly-plug` é escrito `dumbmonit_shelly_plug_power_watts`. Os nomes
são letras minúsculas, dígitos e underscores, e únicos dentro do
pacote.

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (default) or counter
    json: $.apower          # exactly one of json, regex or prom
    scale: 1                # multiplier, e.g. 0.001 for ms → s
    map: {on: 1, off: 0}    # text value → number
    labels: {}
```

| Extrator | Fonte | Valor | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, formato `json` | Todo nó que o caminho corresponde ([RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath, wildcards e filtros incluídos) faz uma série. | etiqueta → JSONPath avaliado no objeto que *contém* o valor: `$.sensors[*].value` com `sensor: $.name` etiqueta cada valor com o seu `name` irmão. |
| `regex: <expression>` | `http`, formato `text` | Toda correspondência faz uma série; o valor é o grupo chamado `value`, senão o primeiro grupo. | etiqueta → nome de um grupo de captura. |
| `prom: <sample name>` | `prometheus` | Toda amostra com esse nome exato. | etiqueta → etiqueta de fonte. Vazio: todas as etiquetas de fonte são mantidas. |

Valores: números são tomados como estão; `true`/`false` tornam-se
1/0; o texto é procurado em `map`, ou analisado como um número quando
não há `map`. Um valor em falta, `null`, ou fora do `map` não produz
nada para essa sondagem.

## Discover {#discover}

`discover` transforma cada elemento de uma lista num conjunto de
séries que partilham etiquetas: uma linha por disco, sensor ou fila.

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath: one row per match
    max_rows: 100           # default 100, at most 1,000
    labels:                 # JSONPath evaluated on each row
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath evaluated on each row
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

As métricas de linha aceitam `name`, `help`, `kind`, `json`, `scale`
e `map`. Linhas além de `max_rows` são ignoradas.

## Regras {#rules}

As regras são instaladas com o pacote, como regras de alerta comuns
com o uid `pack:<id>:<name>`. Depois pertencem a si: uma atualização
do pacote adiciona as regras que ainda não tem, e nunca reescreve uma
que alterou ou eliminou.

```yaml
rules:
  - name: overheating       # lowercase, digits, underscores
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < or <=
    threshold: "{{option.max_temperature}}"   # or a number
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (default) or critical
    unit: °C
```

- `expr` é [MetricsQL](https://docs.victoriametrics.com/metricsql/) e
  tem de usar pelo menos uma métrica do pacote, pelo seu nome
  completo. Qualquer outro nome `dumbmonit_*` que cite também tem de
  ser produzido pelo pacote (`dumbmonit_up` excetuado): um pacote não
  pode alertar sobre a série de outro tipo.
- Um `threshold` de `{{option.x}}` instala o valor predefinido da
  opção, e o próprio valor dessa opção de cada dispositivo torna-se o
  seu limiar para esta regra (uma substituição por dispositivo,
  reescrita sempre que o dispositivo é guardado).

## Perfis SNMP {#snmp-profiles}

Um pacote pode trazer perfis SNMP, escritos exatamente como os
incorporados em
[`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles).
Juntam-se ao catálogo SNMP e são aplicados por `sysObjectID` como os
outros; não criam um tipo de dispositivo próprio.

```yaml
snmp_profiles:
  - id: acme-ups            # the pack id, or starting with "<pack id>-"
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # must start with the pack prefix
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

O catálogo SNMP é lido no arranque: reinicie o DumbMonit depois de
instalar ou atualizar um pacote com perfis SNMP.

## Limites {#limits}

| Limite | Valor |
|---|---|
| Tamanho do pacote | 256 KiB |
| Fontes (pedidos por sondagem, redirecionamentos incluídos) | 16 |
| Tamanho da resposta | 4 MiB |
| Séries por sondagem | 5000 (o resto é descartado e registado) |
| Métricas declaradas | 256 |
| Regras | 64 |
| Linhas por `discover` | 1000 |
| Comprimento do valor de etiqueta | 200 bytes |

## Fixtures e saída esperada {#fixtures-and-expected-output}

`dumbmonit pack test <dir>` lê `fixtures/<source id>.json`, `.txt` ou
`.prom`, executa a mesma extração que o servidor, e compara o
resultado com `expected.prom`: o formato de texto Prometheus,
ordenado, sem data-horas e sem as etiquetas de dispositivo (`target`,
`host`, `tag_*`) que o servidor adiciona. `--update` escreve
`expected.prom` a partir dos fixtures; reveja o diff antes de o
submeter.
