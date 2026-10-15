# Écrire votre premier pack

En environ dix minutes, cette page construit le pack de référence
`shelly-plug` depuis zéro : une prise connectée Shelly (Plus, Pro ou Gen3)
lue via son API locale, avec puissance, énergie et température, et une
alerte en cas de surchauffe.

Il vous faut Docker, et soit une prise Shelly sur votre réseau soit la
réponse capturée ci-dessous.

## 1. Regarder ce que répond l'appareil

Les appareils de seconde génération de Shelly décrivent un relais avec un
appel :

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

Enregistrez cette réponse comme `shelly-plug/fixtures/status.json` : c'est
la fixture contre laquelle le pack sera testé, vous pouvez donc travailler
sans l'appareil sous la main.

## 2. L'en-tête

Créez `shelly-plug/pack.yaml` :

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

L'`id` décide deux noms : le type de surveillance `pack.shelly-plug`, et le
préfixe de métrique `dumbmonit_shelly_plug_`.

## 3. Une source

Une source est une requête, toujours vers la propre adresse de la surveillance :

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

`{{option.switch_id}}` est remplacé par le propre réglage de la surveillance,
montré dans le formulaire de surveillance comme **Switch channel**. Le nom de
fichier de la fixture, `status.json`, correspond à l'`id` de la source.

## 4. Les métriques

Chaque métrique prend une valeur avec une expression
[JSONPath](https://www.rfc-editor.org/rfc/rfc9535). Les noms s'écrivent
sans le préfixe :

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

`output` est un booléen : il devient 1 ou 0. Le total d'énergie ne fait
que croître, c'est donc un `counter` : graphiques et règles le lisent avec
`rate()`, et un redémarrage de la prise ne ressemble pas à une chute.

## 5. Vérifier

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` montre exactement ce que le serveur stockerait depuis la
fixture :

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

Lisez-le une fois. À partir de là, `pack test` sans `--update` échoue si
un changement du pack change ce qu'il extrait, et affiche les lignes qui
diffèrent.

## 6. Une règle d'alerte

Les règles viennent avec le pack et utilisent ses métriques par leur nom
complet. Le seuil peut venir d'une option, pour que chaque prise ait le
sien :

```yaml
options:
  # … switch_id, puis :
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

Lancez `pack lint` pour vérifier tout le fichier : une règle citant une
métrique que le pack ne produit pas, une option qui n'existe pas ou un
chemin qui pointe ailleurs sont rapportés avec leur emplacement.

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/laupernoe/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. L'installer

Avec un jeton d'API qui a la portée `write` :

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

Ouvrez **Surveillances → Ajouter une surveillance** : *Shelly plug (Gen2+)* est
dans la liste. Ajoutez votre prise avec son adresse ; le premier sondage
arrive dans la minute, et la règle *Shelly plug overheating* est dans
**Alertes → Règles**.

Pour publier une nouvelle version, augmentez `version`, lancez
`pack test`, et installez-le à nouveau : l'extraction est remplacée, la
règle que vous avez pu ajuster est conservée.

Le pack complet, avec son avis de configuration, est dans
[`packs/shelly-plug`](https://github.com/laupernoe/dumbmonit/tree/main/packs/shelly-plug).
Tout ce que le format accepte est dans la [référence](format.md).
