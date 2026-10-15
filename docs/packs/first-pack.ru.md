# Напишите свой первый пак

Примерно за десять минут эта страница с нуля собирает эталонный пак
`shelly-plug`: умная розетка Shelly (Plus, Pro или Gen3), читаемая через её
локальный API, с мощностью, энергией и температурой и оповещением о перегреве.

Вам понадобится Docker, а также либо розетка Shelly в вашей сети, либо
приведённый ниже захваченный ответ.

## 1. Посмотрите, что отвечает устройство

Устройства Shelly второго поколения описывают реле одним вызовом:

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

Сохраните этот ответ как `shelly-plug/fixtures/status.json`: это фикстура, на
которой будет тестироваться пак, так что вы сможете работать без устройства под
рукой.

## 2. Заголовок

Создайте `shelly-plug/pack.yaml`:

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

`id` определяет два имени: тип монитора `pack.shelly-plug` и префикс метрик
`dumbmonit_shelly_plug_`.

## 3. Один источник

Источник — это один запрос, всегда на собственный адрес монитора:

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

`{{option.switch_id}}` заменяется собственной настройкой монитора, которая
показывается в форме монитора как **Switch channel**. Имя файла фикстуры,
`status.json`, совпадает с `id` источника.

## 4. Метрики

Каждая метрика выбирает значение с помощью выражения
[JSONPath](https://www.rfc-editor.org/rfc/rfc9535). Имена пишутся без префикса:

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

`output` — булево значение: оно становится 1 или 0. Суммарная энергия только
растёт, поэтому это `counter`: графики и правила читают её через `rate()`, и
перезапуск розетки не выглядит как падение.

## 5. Проверьте

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` показывает в точности то, что сервер сохранил бы из фикстуры:

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

Прочитайте его один раз. Начиная с этого момента `pack test` без `--update`
завершается ошибкой, если изменение пака меняет извлекаемые данные, и выводит
отличающиеся строки.

## 6. Правило оповещения

Правила поставляются вместе с паком и используют его метрики по полным именам.
Порог может задаваться опцией, так что у каждой розетки может быть свой:

```yaml
options:
  # … switch_id, then:
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

Выполните `pack lint`, чтобы проверить весь файл: правило, ссылающееся на
метрику, которую пак не создаёт, несуществующая опция или путь, ведущий в
другое место, будут показаны с указанием, где они находятся.

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/laupernoe/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. Установите его

С токеном API, имеющим область `write`:

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

Откройте **Monitors → Add monitor**: *Shelly plug (Gen2+)* есть в списке.
Добавьте свою розетку по её адресу; первый опрос произойдёт в течение минуты, а
правило *Shelly plug overheating* появится в **Alerts → Rules**.

Чтобы опубликовать новую версию, увеличьте `version`, выполните `pack test` и
установите пак снова: извлечение данных заменяется, а правило, которое вы,
возможно, настроили, сохраняется.

Полный пак вместе с уведомлением по настройке находится в
[`packs/shelly-plug`](https://github.com/laupernoe/dumbmonit/tree/main/packs/shelly-plug).
Всё, что принимает формат, описано в [справочнике](format.md).
