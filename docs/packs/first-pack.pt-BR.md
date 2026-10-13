# Escreva seu primeiro pacote {#write-your-first-pack}

Em cerca de dez minutos, esta página constrói o pacote de referência
`shelly-plug` do zero: uma tomada inteligente Shelly (Plus, Pro ou Gen3)
lida através de sua API local, com potência, energia e temperatura, e um
alerta quando ela sobreaquece.

Você precisa do Docker, e de uma tomada Shelly em sua rede ou da resposta
capturada abaixo.

## 1. Veja o que o dispositivo responde {#1-look-at-what-the-device-answers}

Dispositivos de segunda geração da Shelly descrevem um relé com uma
chamada:

```bash
curl http://192.168.1.50/rpc/Switch.GetStatus?id=0
```

```json
{
  "id": 0,
  "source": "HTTP_in",
  "output": true,
  "apower": 8.9,
  "voltage": 237.5,
  "current": 0.069,
  "aenergy": { "total": 6532.2, "by_minute": [45.199, 47.141, 88.397] },
  "temperature": { "tC": 41.3, "tF": 106.3 }
}
```

Salve esta resposta como `shelly-plug/fixtures/status.json`: é a fixture
contra a qual o pacote será testado, então você pode trabalhar sem o
dispositivo à mão.

## 2. O cabeçalho {#2-the-header}

Crie `shelly-plug/pack.yaml`:

```yaml
schema: 1
id: shelly-plug
version: 1.0.0
label: Shelly plug (Gen2+)
summary: Power, energy, voltage and temperature of a Shelly Plus/Pro/Gen3 plug or relay.
examples: [Shelly Plus Plug S, Shelly Pro 1PM]
address_hint: 192.168.1.50
default_port: 80
credentials: [none]
```

O `id` decide dois nomes: o tipo de dispositivo `pack.shelly-plug`, e o
prefixo de métrica `dumbmonit_shelly_plug_`.

## 3. Uma fonte {#3-one-source}

Uma fonte é uma requisição, sempre para o próprio endereço do
dispositivo:

```yaml
options:
  - key: switch_id
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a multi-channel relay.
    input: number
    default: 0

sources:
  - id: status
    type: http
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
```

`{{option.switch_id}}` é substituído pela própria configuração do
dispositivo, mostrada no formulário do dispositivo como **Switch
channel**. O nome do arquivo da fixture, `status.json`, corresponde ao
`id` da fonte.

## 4. As métricas {#4-the-metrics}

Cada métrica escolhe um valor com uma expressão
[JSONPath](https://www.rfc-editor.org/rfc/rfc9535). Nomes são escritos
sem o prefixo:

```yaml
metrics:
  - name: output_on
    help: 1 when the relay is closed (power flows), 0 when it is open.
    source: status
    json: $.output
  - name: power_watts
    source: status
    json: $.apower
  - name: voltage_volts
    source: status
    json: $.voltage
  - name: current_amperes
    source: status
    json: $.current
  - name: energy_watthours_total
    source: status
    kind: counter
    json: $.aenergy.total
  - name: temperature_celsius
    source: status
    json: $.temperature.tC
```

`output` é um booleano: ele se torna 1 ou 0. O total de energia só
cresce, então é um `counter`: gráficos e regras o leem com `rate()`, e um
reinício da tomada não parece uma queda.

## 5. Verifique-o {#5-check-it}

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` mostra exatamente o que o servidor armazenaria a partir
da fixture:

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

Leia-o uma vez. A partir de agora, `pack test` sem `--update` falha se
uma mudança no pacote alterar o que ele extrai, e imprime as linhas que
diferem.

## 6. Uma regra de alerta {#6-an-alert-rule}

Regras vêm com o pacote e usam suas métricas pelo nome completo. O limite
pode vir de uma opção, então cada tomada pode ter o seu próprio:

```yaml
options:
  # … switch_id, depois:
  - key: max_temperature
    label: Temperature alert (°C)
    input: number
    default: 70

rules:
  - name: overheating
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"
    threshold: "{{option.max_temperature}}"
    for: 5m
    severity: warning
    unit: °C
```

Execute `pack lint` para verificar todo o arquivo: uma regra citando uma
métrica que o pacote não produz, uma opção que não existe ou um caminho
que apontar para outro lugar são reportados com onde estão.

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/laupernoe/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. Instale-o {#7-install-it}

Com um token de API que tenha o escopo `write`:

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

Abra **Dispositivos → Adicionar dispositivo**: *Shelly plug (Gen2+)* está
na lista. Adicione sua tomada com seu endereço; a primeira sondagem vem
em menos de um minuto, e a regra *Shelly plug overheating* está em
**Alertas → Regras**.

Para publicar uma versão nova, aumente `version`, execute `pack test`, e
instale de novo: a extração é substituída, a regra que você possa ter
ajustado é mantida.

O pacote completo, com seu aviso de configuração, está em
[`packs/shelly-plug`](https://github.com/laupernoe/dumbmonit/tree/main/packs/shelly-plug).
Tudo que o formato aceita está na [referência](format.md).
