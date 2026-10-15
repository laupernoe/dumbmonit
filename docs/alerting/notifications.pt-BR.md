# Política de notificação {#notification-policy}

A promessa do DumbMonit é que uma notificação vale a pena ser lida. O motor
de alertas mantém o ruído baixo antes de um alerta poder falar (supressão
por dependência, agrupamento, janelas de manutenção); a política de
notificação o mantém baixo depois, por canal. Nada precisa ser
configurado: os padrões abaixo se aplicam prontos para uso.

## O pipeline, em uma lista {#the-pipeline-in-one-list}

```
avaliar (histerese, substituições por dispositivo)
  → deduplicar → suprimir por dependência → silenciar → agrupar por dispositivo
    → retenção de oscilação → filtro de severidade do canal → filtro de roteamento do canal
      → resolvido ligado/desligado → cooldown → horas de silêncio → janela de lote
        → limite por hora → enviar (+ link profundo)
                                   ↘ ninguém reconheceu → canal de escalonamento
```

| Etapa | Onde é definido | Padrão |
|---|---|---|
| Limite de limpeza (histerese) | Editor de regra → *Limpa abaixo / acima* | nenhum (CPU: 85%, disco: 88% nas regras entregues) |
| Substituições por dispositivo | Editor de regra → *Substituições por dispositivo* | nenhuma |
| Detecção de oscilação | Configurações → Notificações e relatórios → Política de notificação → Mais opções | 4 mudanças em 30 min, retém 30 min |
| Severidade mínima | Canal → Opções de entrega → *Enviar* | tudo |
| Me diga quando limpar | Canal → Opções de entrega | ligado |
| Intervalo mínimo por alerta | Canal → Opções de entrega | nenhum |
| Horas de silêncio | Canal → Opções de entrega | desligado |
| Filtro de roteamento (tags, tipo, regra) | Canal → Opções de entrega → *Apenas alguns alertas* | desligado (o canal ouve tudo) |
| Escalonamento se ninguém reconhecer | Configurações → Notificações e relatórios → Política de notificação | desligado |
| Janela de lote | Configurações → Notificações e relatórios → Política de notificação | 60 s |
| Mensagens por canal por hora | Configurações → Notificações e relatórios → Política de notificação | 20 |
| URL pública (links profundos) | Configurações → Notificações e relatórios → Política de notificação, ou `DUMBMONIT_PUBLIC_URL` | vazia |

## Histerese e substituições por dispositivo {#hysteresis-and-per-device-overrides}

Uma regra que dispara acima de 90% e limpa no momento em que o valor cai
abaixo de 90% vai disparar e limpar a noite toda em uma CPU oscilando
entre 89–91%. Dê a ela um **limite de limpeza**: uma vez disparando, o
alerta só resolve quando o valor vai abaixo dele (para regras "dispara
acima") ou acima dele (para regras "dispara abaixo"). O campo da API é
`clear_threshold` na regra; o servidor recusa um limite de limpeza do
lado errado do limite.

Uma **substituição por dispositivo** muda o limite ou limite de limpeza
de um dispositivo, ou desativa a regra para esse dispositivo, sem
duplicar a regra:

```
PUT /api/alerts/rules/{id}/overrides/{target_id}
{ "threshold": 97, "clear_threshold": 95 }      # ou { "enabled": false }
GET /api/alerts/rules/{id}/overrides
GET /api/alerts/overrides?target_id=7           # toda substituição de um dispositivo
DELETE /api/alerts/rules/{id}/overrides/{target_id}
```

Uma substituição com todo campo vazio é removida. Mensagens citam o
limite realmente aplicado.

## Detecção de oscilação {#flap-detection}

Independente da regra, um alerta cuja impressão digital disparou ou
resolveu 4 vezes em 30 minutos está *oscilando*: um aviso sai
("oscilando: 4 mudanças em 30 min, notificações retidas por 30 min") e
nada mais sobre esse alerta é enviado até a retenção terminar. Uma
resolução depois da retenção ainda é anunciada. Ajuste ou desligue em
Configurações → Notificações e relatórios → Política de notificação → Mais opções
(`flap_events`, `flap_window_secs`, `flap_hold_secs`; `flap_events: 0` o
desativa).

## Opções de entrega por canal {#per-channel-delivery-options}

Cada canal decide o que ouve (`policy` no payload do canal):

- `min_severity` — `info`, `warning` (Aviso e acima) ou `critical`
  (Apenas Alerta). Um canal de celular tipicamente quer `critical`, uma
  sala de chat tudo.
- `notify_resolved` — `false` para ouvir apenas sobre problemas.
- `min_interval_secs` — pelo menos esse tempo entre duas mensagens sobre
  o mesmo alerta (redisparos e lembretes). Resoluções nunca são atrasadas
  por isso, e uma resolução só é enviada a um canal que ouviu o disparo.
- `quiet_hours` — uma janela semanal em seu horário local, mesma forma
  que uma janela de manutenção semanal. Durante horas de silêncio só
  alertas `critical` passam; o resto espera e chega como um resumo
  quando terminam, intitulado "Horas de silêncio terminaram — …". Um
  alerta que dispara e limpa durante horas de silêncio é apenas
  mencionado nesse resumo como "resolvido durante horas de silêncio".
- `matcher` — quais alertas o canal quer, por tag, tipo de dispositivo ou
  regra. Vazio (o padrão, e o que todo canal existente mantém) significa
  tudo.

```
PUT /api/notify/channels/{id}
{ "name": "Phone", "kind": "ntfy", "settings": {…},
  "policy": { "min_severity": "critical", "min_interval_secs": 900,
              "quiet_hours": { "kind": "weekly", "days": [0,1,2,3,4],
                               "start_minute": 1320, "end_minute": 420,
                               "timezone": "Europe/Paris" } } }
```

Omitir `policy` em uma atualização mantém a armazenada;
`"quiet_hours": null` as limpa, e `"matcher": null` limpa o filtro de
roteamento.

## Roteamento por tag {#routing-by-tag}

Dispositivos carregam tags (`site=cellar`, `role=storage`) e um tipo
(`snmp`, `proxmox`, `synology`, `agent`…). O **filtro de roteamento** de
um canal diz quais alertas ele quer:

```json
{ "policy": { "min_severity": "warning",
              "matcher": {
                "include": [ { "field": "tag", "key": "site", "value": "cellar" } ],
                "exclude": [ { "field": "tag", "key": "role", "value": "lab" } ] } } }
```

Três campos, e apenas igualdade exata — nenhuma expressão regular,
nenhum colchete:

| `field` | Corresponde a | Exemplo |
|---|---|---|
| `tag` | Uma tag da própria página do dispositivo | `{"field":"tag","key":"site","value":"cellar"}` |
| `kind` | O coletor que sonda o dispositivo | `{"field":"kind","value":"proxmox"}` |
| `rule` | O `uid` da regra | `{"field":"rule","value":"disk_full"}` |

Como as listas se leem:

- condições no **mesmo** campo (a mesma chave de tag, ou `kind`, ou
  `rule`) são um **OU** — `site=cellar` mais `site=attic` significa
  qualquer um;
- condições em **campos diferentes** são um **E** — `site=cellar` mais
  `role=storage` significa ambos;
- **`exclude` sempre vence**, mesmo sobre um `include` que correspondeu.

Então o exemplo acima se lê, e a interface o diz de volta exatamente
nessas palavras: *este canal recebe avisos e acima de dispositivos
marcados site=cellar, exceto dispositivos marcados role=lab*.

Sob o editor, uma **pré-visualização** lista quais de seus dispositivos
atuais o filtro seleciona. Não é uma aproximação: o servidor executa o
mesmo filtro que o motor de alertas executa, reduzido à parte que depende
apenas do dispositivo. Condições de regra se aplicam a alertas em vez de
dispositivos, então a pré-visualização os nomeia separadamente em vez de
fingir contá-los.

```
POST /api/notify/match-preview
{ "matcher": { "include": [ { "field": "tag", "key": "site", "value": "cellar" } ] } }

→ { "devices": [ { "id": 3, "name": "nas", "kind": "synology",
                   "tags": { "site": "cellar" }, "matched": true } ],
    "matched": 1, "total": 12, "rules": [], "excluded_rules": [] }
```

Um filtro é limitado a doze condições. Além disso, um segundo canal é
mais claro que mais uma condição.

## Escalonamento: se ninguém reconhecer {#escalation-if-nobody-acknowledges}

Um salto extra, e apenas um. Quando a política define
`escalate_after_secs` e `escalate_channel`, um alerta que ninguém
[reconheceu](../using/alerts.md) dentro desse prazo também é enviado —
uma vez — para aquele canal:

```
PUT /api/notify/policy
{ "escalate_after_secs": 900, "escalate_channel": 4 }
```

- O prazo é contado **a partir do momento em que a primeira mensagem
  realmente saiu**, não do disparo do alerta. Se horas de silêncio ou uma
  entrega falhada engoliram a primeira notificação, não há nada para
  escalar ainda e o salto espera.
- **Reconhecer o alerta o interrompe**, exatamente como interrompe
  lembretes; assim como o alerta se resolver.
- A mensagem de escalonamento vai **apenas** para o canal de
  escalonamento — os canais que já ouviram o disparo não são avisados
  duas vezes.
- Ela cruza o próprio piso de severidade, filtro de roteamento e horas de
  silêncio daquele canal: você nomeou esse canal para ser despertado,
  então ele não é retido no único momento em que importa.
- A linha diz isso: `🔴 Critical · Disk full — /data — 95 % … — still
  unacknowledged`.

`"escalate_channel": null` desliga o escalonamento; um prazo de `0` faz
o mesmo. Ambos são necessários para o salto existir, e o servidor recusa
um canal que não existe em vez de escalar para o vazio.

O DumbMonit deliberadamente para aqui: um salto, sem rotação, sem
programação de quem está de plantão nesta semana. Se você precisa disso,
aponte o canal de escalonamento para o PagerDuty ou Opsgenie, que fazem
isso adequadamente.

## Lotes e o limite por hora {#batching-and-the-hourly-cap}

`GET/PUT /api/notify/policy` mantém as configurações globais:

```json
{ "batch_window_secs": 60, "max_per_hour": 20,
  "flap_events": 4, "flap_window_secs": 1800, "flap_hold_secs": 1800,
  "public_url": "https://monit.example.lan",
  "escalate_after_secs": 0, "escalate_channel": null }
```

- **Janela de lote** — alertas que disparam dentro da janela saem como
  *uma* mensagem por canal, agrupados por dispositivo: "3 alertas em 2
  dispositivos" com uma seção por dispositivo; resoluções acompanham ("2
  resolvidos"). `0` envia no próximo ciclo. Um resumo lista no máximo 12
  linhas, depois "…e mais N alertas".
- **Limite por hora** — uma vez que um canal recebeu `max_per_hour`
  mensagens na última hora, mais alertas esperam e chegam como um resumo
  quando uma vaga se libera. `0` remove o limite.
- **Canais de plantão** (PagerDuty, Opsgenie) nunca são agrupados entre
  dispositivos: eles abrem e fecham um incidente por dispositivo, então
  recebem uma mensagem por dispositivo sem esperar.

Falhas de entrega mantêm as linhas na fila; o próximo ciclo tenta de
novo.

## O que uma linha diz {#what-a-line-says}

Cada alerta é uma linha, a mesma no e-mail, ntfy, chat e na variável
`{{message}}` de um webhook personalizado:

```
🔴 Critical · Disk full — /data — 95 % (threshold > 90 %), for 12 min
⚠️ Warning · VM or container stopped — win11-desktop (101), for 59 s
✅ Resolved · Service down — https://example.lan/
```

- O glifo é seguido pela palavra de estado (Critical / Warning / Info,
  Resolved, Flapping), então a severidade é legível sem cor ou emoji.
- Uma regra que observa várias séries nomeia a que disparou: a VM e seu
  id, o ponto de montagem, a interface, o grupo de backup, a URL da
  sondagem. Regras de todo o dispositivo (inacessível) não nomeiam mais
  que o dispositivo.
- O valor e limite são mostrados para regras medidas; uma regra
  tudo-ou-nada em uma métrica 0/1 (na bateria, guest parado) não repete
  "1 (threshold > 0)".

## Links profundos {#deep-links}

Toda mensagem termina com um link para o dispositivo
(`{public_url}/targets/{id}`) quando uma URL pública é conhecida — o
`public_url` da política, ou `DUMBMONIT_PUBLIC_URL`. Webhooks
personalizados também o recebem como a variável `{{link}}` a menos que
seu próprio `base_url` esteja definido.

Deliberadamente não há link "silenciar por 1 h" em mensagens: serviços
de chat buscam todo link que exibem para construir uma pré-visualização,
o que acionaria o silêncio no momento em que a mensagem é postada.
Silencie da página do dispositivo ou da página de Alertas em vez disso.

## O que foi emprestado do Pulse, e o que não foi {#what-was-borrowed-from-pulse-and-what-was-not}

O design segue [Pulse](https://github.com/rcourtman/Pulse) onde se
encaixa em um monitor de homelab: limites de disparo/limpeza, uma camada
de substituição por recurso, um cooldown de notificação, uma janela de
agrupamento, um limite de taxa por hora, horas de silêncio, detecção de
oscilação com um cooldown, "notificar ao resolver", e ferramentas de
plantão mantidas fora de resumos. Não adotado: o período de observação de
24 horas do Pulse antes de notificar (a regra de linha de base do
DumbMonit já aprende silenciosamente; regras de limite devem falar desde
o primeiro dia) e soneca/reconhecimento a partir da mensagem (veja
acima). Roteamento baseado em tag por destino *foi* adotado, como o
filtro de roteamento acima — escolher canais na regra se mostrou o lugar
errado para "tudo no porão vai para esta sala".
