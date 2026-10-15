# Política de notificação {#notification-policy}

A promessa do DumbMonit é que uma notificação vale a pena ler. O motor
de alertagem mantém o ruído baixo antes de um alerta poder falar
(supressão por dependência, agrupamento, janelas de manutenção); a
política de notificação mantém-o baixo depois, por canal. Nada precisa
de configuração: as predefinições abaixo aplicam-se de fábrica.

## O pipeline, numa lista {#the-pipeline-in-one-list}

```
evaluate (hysteresis, per-device overrides)
  → deduplicate → suppress by dependency → silence → group by device
    → flap hold → channel severity filter → channel routing filter
      → resolved on/off → cooldown → quiet hours → batch window
        → hourly cap → send (+ deep link)
                                   ↘ nobody acknowledged → escalation channel
```

| Etapa | Onde é definida | Predefinição |
|---|---|---|
| Limiar de limpeza (histerese) | Editor de regra → *Clear below / above* | nenhum (CPU: 85%, disco: 88% nas regras incluídas) |
| Substituições por dispositivo | Editor de regra → *Per-device overrides* | nenhuma |
| Deteção de oscilação | Settings → Notifications & reports → Notification policy → More options | 4 alterações em 30 min, retenção de 30 min |
| Severidade mínima | Canal → Delivery options → *Send* | tudo |
| Avisar-me quando se resolve | Canal → Delivery options | ativado |
| Intervalo mínimo por alerta | Canal → Delivery options | nenhum |
| Horas de silêncio | Canal → Delivery options | desligado |
| Filtro de encaminhamento (etiquetas, tipo, regra) | Canal → Delivery options → *Only some alerts* | desligado (o canal ouve tudo) |
| Escalonamento se ninguém confirmar | Settings → Notifications & reports → Notification policy | desligado |
| Janela de lote | Settings → Notifications & reports → Notification policy | 60 s |
| Mensagens por canal por hora | Settings → Notifications & reports → Notification policy | 20 |
| URL público (ligações diretas) | Settings → Notifications & reports → Notification policy, ou `DUMBMONIT_PUBLIC_URL` | vazio |

## Histerese e substituições por dispositivo {#hysteresis-and-per-device-overrides}

Uma regra que dispara acima de 90% e se limpa no momento em que o
valor desce abaixo de 90% vai disparar e limpar-se toda a noite numa
CPU a oscilar entre 89–91%. Dê-lhe um **limiar de limpeza**: uma vez
ativo, o alerta só se resolve quando o valor desce abaixo dele (para
regras «fires above») ou sobe acima dele (para regras «fires below»).
O campo da API é `clear_threshold` na regra; o servidor recusa um
limiar de limpeza do lado errado do limiar.

Uma **substituição por dispositivo** altera o limiar ou limiar de
limpeza de um dispositivo, ou desativa a regra para esse dispositivo,
sem duplicar a regra:

```
PUT /api/alerts/rules/{id}/overrides/{target_id}
{ "threshold": 97, "clear_threshold": 95 }      # ou { "enabled": false }
GET /api/alerts/rules/{id}/overrides
GET /api/alerts/overrides?target_id=7           # toda substituição de um dispositivo
DELETE /api/alerts/rules/{id}/overrides/{target_id}
```

Uma substituição com todos os campos vazios é removida. As mensagens
citam o limiar realmente aplicado.

## Deteção de oscilação {#flap-detection}

Independentemente da regra, um alerta cuja impressão digital disparou
ou se resolveu 4 vezes dentro de 30 minutos está *a oscilar*: um
aviso sai («flapping: 4 changes in 30 min, notifications held for 30
min») e nada mais sobre esse alerta é enviado até a retenção
terminar. Uma resolução depois da retenção continua a ser anunciada.
Ajuste-a ou desligue-a em Settings → Notifications & reports →
Notification policy → More options (`flap_events`, `flap_window_secs`,
`flap_hold_secs`; `flap_events: 0` desativa-a).

## Opções de entrega por canal {#per-channel-delivery-options}

Cada canal decide o que ouve (`policy` na carga útil do canal):

- `min_severity` — `info`, `warning` (Advisory e acima) ou `critical`
  (só Warning). Um canal de telefone tipicamente quer `critical`, uma
  sala de chat tudo.
- `notify_resolved` — `false` para só ouvir problemas.
- `min_interval_secs` — pelo menos este tempo entre duas mensagens
  sobre o mesmo alerta (re-disparos e lembretes). As resoluções nunca
  são atrasadas por isto, e uma resolução só é enviada a um canal que
  ouviu o disparo.
- `quiet_hours` — uma janela semanal na sua hora local, a mesma forma
  de uma janela de manutenção semanal. Durante as horas de silêncio só
  os alertas `critical` passam; o resto espera e chega como um único
  resumo quando terminam, titulado «Quiet hours over — …». Um alerta
  que dispara e se limpa durante as horas de silêncio só é mencionado
  nesse resumo como «resolved during quiet hours».
- `matcher` — quais alertas o canal quer, por etiqueta, tipo de
  dispositivo ou regra. Vazio (a predefinição, e o que todo canal
  existente mantém) significa tudo.

```
PUT /api/notify/channels/{id}
{ "name": "Phone", "kind": "ntfy", "settings": {…},
  "policy": { "min_severity": "critical", "min_interval_secs": 900,
              "quiet_hours": { "kind": "weekly", "days": [0,1,2,3,4],
                               "start_minute": 1320, "end_minute": 420,
                               "timezone": "Europe/Paris" } } }
```

Omitir `policy` numa atualização mantém a guardada; `"quiet_hours":
null` limpa-as, e `"matcher": null` limpa o filtro de encaminhamento.

## Encaminhamento por etiqueta {#routing-by-tag}

Os dispositivos transportam etiquetas (`site=cellar`, `role=storage`)
e um tipo (`snmp`, `proxmox`, `synology`, `agent`…). O **filtro de
encaminhamento** de um canal diz quais alertas quer:

```json
{ "policy": { "min_severity": "warning",
              "matcher": {
                "include": [ { "field": "tag", "key": "site", "value": "cellar" } ],
                "exclude": [ { "field": "tag", "key": "role", "value": "lab" } ] } } }
```

Três campos, e só igualdade exata — sem expressões regulares, sem
parênteses:

| `field` | Corresponde a | Exemplo |
|---|---|---|
| `tag` | Uma etiqueta da própria página do dispositivo | `{"field":"tag","key":"site","value":"cellar"}` |
| `kind` | O coletor que sonda o dispositivo | `{"field":"kind","value":"proxmox"}` |
| `rule` | O `uid` da regra | `{"field":"rule","value":"disk_full"}` |

Como as listas se lêem:

- condições no **mesmo** campo (a mesma chave de etiqueta, ou `kind`,
  ou `rule`) são um **OU** — `site=cellar` mais `site=attic` significa
  qualquer um;
- condições em campos **diferentes** são um **E** — `site=cellar`
  mais `role=storage` significa ambos;
- **`exclude` ganha sempre**, mesmo sobre um `include` que
  correspondeu.

Por isso o exemplo acima lê-se, e a interface diz-o exatamente nestas
palavras: *este canal recebe avisos e acima de dispositivos
etiquetados site=cellar, exceto dispositivos etiquetados role=lab*.

Sob o editor, uma **pré-visualização** lista quais dos seus
dispositivos atuais o filtro seleciona. Não é uma aproximação: o
servidor executa o próprio filtro que o motor de alertagem executa,
reduzido à parte que depende só do dispositivo. As condições de regra
aplicam-se a alertas em vez de dispositivos, por isso a
pré-visualização nomeia-as separadamente em vez de fingir contá-las.

```
POST /api/notify/match-preview
{ "matcher": { "include": [ { "field": "tag", "key": "site", "value": "cellar" } ] } }

→ { "devices": [ { "id": 3, "name": "nas", "kind": "synology",
                   "tags": { "site": "cellar" }, "matched": true } ],
    "matched": 1, "total": 12, "rules": [], "excluded_rules": [] }
```

Um filtro é limitado a doze condições. Além disso, um segundo canal é
mais claro do que mais uma condição.

## Escalonamento: se ninguém confirmar {#escalation-if-nobody-acknowledges}

Um salto extra, e só um. Quando a política define
`escalate_after_secs` e `escalate_channel`, um alerta que ninguém
[confirmou](../using/alerts.md) dentro desse prazo também é enviado —
uma vez — a esse canal:

```
PUT /api/notify/policy
{ "escalate_after_secs": 900, "escalate_channel": 4 }
```

- O prazo conta **a partir do momento em que a primeira mensagem
  realmente saiu**, não do disparo do alerta. Se as horas de silêncio
  ou uma entrega falhada engoliram a primeira notificação, não há
  ainda nada para escalonar e o salto espera.
- **Confirmar o alerta para-o**, exatamente como para os lembretes; o
  mesmo acontece quando o alerta se limpa.
- A mensagem de escalonamento vai **só** para o canal de
  escalonamento — os canais que já ouviram o disparo não são
  avisados duas vezes.
- Atravessa o próprio piso de severidade, filtro de encaminhamento e
  horas de silêncio desse canal: nomeou este canal para ser acordado,
  por isso não é retido no único momento em que importa.
- A linha diz isso: `🔴 Critical · Disk full — /data — 95 % … — still
  unacknowledged`.

`"escalate_channel": null` desliga o escalonamento; um prazo de `0`
faz o mesmo. Ambos são necessários para o salto existir, e o servidor
recusa um canal que não existe em vez de escalonar para o vazio.

O DumbMonit para deliberadamente aqui: um salto, sem rotação, sem
calendário de quem está de serviço esta semana. Se precisar disso,
aponte o canal de escalonamento para o PagerDuty ou o Opsgenie, que o
fazem corretamente.

## Agrupamento em lote e o limite horário {#batching-and-the-hourly-cap}

`GET/PUT /api/notify/policy` contém as definições globais:

```json
{ "batch_window_secs": 60, "max_per_hour": 20,
  "flap_events": 4, "flap_window_secs": 1800, "flap_hold_secs": 1800,
  "public_url": "https://monit.example.lan",
  "escalate_after_secs": 0, "escalate_channel": null }
```

- **Batch window** — alertas que disparam dentro da janela saem como
  *uma* mensagem por canal, agrupados por dispositivo: «3 alerts on 2
  devices» com uma secção por dispositivo; as resoluções vão junto
  («2 resolved»). `0` envia no ciclo seguinte. Um resumo lista no
  máximo 12 linhas, depois «…and N more alerts».
- **Hourly cap** — uma vez um canal tenha recebido `max_per_hour`
  mensagens na última hora, os alertas seguintes esperam e chegam
  como um único resumo quando uma vaga liberta. `0` remove o limite.
- **Canais de prevenção** (PagerDuty, Opsgenie) nunca são agrupados em
  lote entre dispositivos: abrem e fecham um incidente por
  dispositivo, por isso recebem uma mensagem por dispositivo sem
  esperar.

As falhas de entrega mantêm as linhas em fila; o ciclo seguinte
repete-as.

## O que uma linha diz {#what-a-line-says}

Cada alerta é uma linha, a mesma em email, ntfy, chat e a variável
`{{message}}` de um webhook personalizado:

```
🔴 Critical · Disk full — /data — 95 % (threshold > 90 %), for 12 min
⚠️ Warning · VM or container stopped — win11-desktop (101), for 59 s
✅ Resolved · Service down — https://example.lan/
```

- O glifo é seguido pela palavra de estado (Critical / Warning / Info,
  Resolved, Flapping), para que a severidade seja legível sem cor ou
  emoji.
- Uma regra que monitoriza várias séries nomeia a que disparou: a VM e
  o seu id, o ponto de montagem, a interface, o grupo de cópia de
  segurança, o URL da sonda. As regras à escala do dispositivo
  (inacessível) não nomeiam mais do que o dispositivo.
- O valor e o limiar são mostrados para regras medidas; uma regra
  tudo-ou-nada numa métrica 0/1 (com bateria, convidado parado) não
  repete «1 (threshold > 0)».

## Ligações diretas {#deep-links}

Toda mensagem termina com uma ligação ao dispositivo
(`{public_url}/targets/{id}`) quando um URL público é conhecido — o
`public_url` da política, ou `DUMBMONIT_PUBLIC_URL`. Os webhooks
personalizados também a recebem como a variável `{{link}}` a menos que
o seu próprio `base_url` esteja definido.

Não há, de propósito, nenhuma ligação «silence for 1 h» nas mensagens:
os serviços de chat obtêm toda ligação que mostram para construir uma
pré-visualização, o que acionaria o silêncio no momento em que a
mensagem é publicada. Silencie a partir da página do dispositivo ou da
página Alerts em vez disso.

## O que foi emprestado do Pulse, e o que não foi {#what-was-borrowed-from-pulse-and-what-was-not}

O design segue o [Pulse](https://github.com/rcourtman/Pulse) onde se
adequa a um monitor de homelab: limiares de disparo/limpeza, uma
camada de substituição por recurso, um tempo de espera de notificação,
uma janela de agrupamento, um limite de taxa horário, horas de
silêncio, deteção de oscilação com um tempo de espera, «notify on
resolve», e ferramentas de prevenção mantidas fora dos resumos. Não
adotado: o período de observação de 24 horas do Pulse antes de
notificar (a regra de linha de base do DumbMonit já aprende
silenciosamente; as regras de limiar devem falar desde o primeiro
dia) e adiar/confirmar a partir da mensagem (ver acima). O
encaminhamento baseado em etiquetas por destino *foi* adotado, como o
filtro de encaminhamento acima — colocar canais na regra revelou-se o
lugar errado para «tudo na adega vai para esta sala».
