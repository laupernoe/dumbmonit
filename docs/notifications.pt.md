# Notificações do DumbMonit {#dumbmonit-notifications}

Este guia explica como receber alertas do DumbMonit no serviço da sua
escolha. Não exige nenhuma competência de programação: cada secção
diz onde encontrar o endereço ou token a copiar, e o que escrever no
formulário.

---

## Conteúdo {#contents}

- [Como funciona](#how-it-works)
- [Criar e testar um canal](#creating-and-testing-a-channel)
- [Definições ou segredos: a diferença](#settings-or-secrets-the-difference)
- [O canal personalizado](#the-custom-channel)
  - [Variáveis disponíveis](#available-variables)
  - [Exemplo 1 — uma API JSON interna](#example-1--an-internal-json-api)
  - [Exemplo 2 — um serviço que espera um formulário](#example-2--a-service-that-expects-a-form)
  - [Exemplo 3 — um SMS em texto simples através de uma operadora](#example-3--a-plain-text-sms-through-a-carrier)
  - [Exemplo 4 — um pedido GET sem corpo](#example-4--a-get-request-without-a-body)
  - [Erros comuns de modelo](#common-template-mistakes)
- [Canais suportados](#supported-channels)
- [Resolução de problemas](#troubleshooting)

---

## Como funciona {#how-it-works}

Um **canal** é um destino: um canal Discord, um número de telefone,
uma caixa de correio. Cria quantos quiser, e as suas regras de
alerta escolhem quais notificar.

Três princípios aplicam-se a todo canal:

1. **Os alertas são agrupados.** Uma mensagem por dispositivo e por
   ciclo, mesmo que cinco discos se enchem ao mesmo tempo. Sem isto,
   silenciaria as notificações numa semana.
2. **Um canal avariado nunca bloqueia outro.** A falha é registada
   na coluna «last error» do canal, e a entrega continua para os
   outros.
3. **Um serviço que não responde é abandonado depois de 15
   segundos.** Uma notificação que chega um minuto atrasada já não
   é útil, e o ciclo de monitorização não deve esperar atrás de um
   servidor inacessível.

---

## Criar e testar um canal {#creating-and-testing-a-channel}

1. Abra **Alerts → Notifications → Add channel** (`/alerts#notifications`).
2. Dê-lhe um nome significativo («Discord — family», «SMS —
   on-call»). É o nome que escolherá depois nas suas regras de
   alerta.
3. Escolha o tipo de canal, depois preencha os campos descritos
   abaixo.
4. Guarde, depois clique em **Send test**.

A mensagem de teste segue **exatamente o mesmo caminho** que um
alerta real: mesmo pedido, mesmo token, mesma formatação. Se chegar,
o canal funciona. Se falhar, a mensagem de erro nomeia a definição a
corrigir.

> **Um token nunca é mostrado de novo.** Uma vez guardado, é
> encriptado na base de dados e a interface só diz «configured».
> Para editar um canal sem reescrever o seu token, deixe simplesmente
> o campo vazio.

---

## Definições ou segredos: a diferença {#settings-or-secrets-the-difference}

Todo canal tem dois grupos de campos.

| | **Settings** | **Secrets** |
|---|---|---|
| Conteúdo | endereço de servidor, nome de canal, tópico | tokens, senhas, URLs de webhook |
| Armazenamento | texto simples | encriptado |
| Lido de volta pela interface | sim | nunca |

A interface **recusa** guardar um campo sensível entre as
definições, e diz-lhe isso. Não é uma formalidade: as definições são
devolvidas como estão pela API, por isso um token guardado lá seria
publicado cada vez que a página fosse aberta.

As tabelas na secção [Canais suportados](#supported-channels) dizem,
para cada canal, o que vai onde.

---

## O canal personalizado {#the-custom-channel}

O canal **`webhook`** é a válvula de escape do DumbMonit. Descreve
todo o pedido HTTP — método, endereço, cabeçalhos, tipo de conteúdo e
corpo —, o que lhe permite ligar-se a quase qualquer serviço,
incluindo uma API interna que o DumbMonit nunca conhecerá.

**Sem nenhuma configuração particular**, já envia uma carga útil
JSON estruturada que um script pode usar:

```json
{
  "source": "dumbmonit",
  "target": "nas",
  "rule": "Disk full",
  "severity": "warning",
  "status": "firing",
  "title": "nas — Disk full",
  "text": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "value": 95.0,
  "threshold": 90.0,
  "unit": "%",
  "fingerprint": "nas/disk-full",
  "count": 1,
  "at": "2026-09-01T14:12:05+00:00",
  "link": ""
}
```

### Definições do canal personalizado {#custom-channel-settings}

| Definição | Obrigatória | Predefinição | Função |
|---|---|---|---|
| `url` | sim (aqui ou como segredo) | — | Endereço a chamar. Pode conter variáveis. |
| `method` | não | `POST` | `POST`, `PUT` ou `GET`. |
| `content_type` | não | `json` | `json`, `form` ou `text`. Também determina como as variáveis são escapadas. |
| `body_template` | não | — | Modelo do corpo. Quando ausente, a carga útil acima é enviada. |
| `headers` | não | — | Cabeçalhos extra, como um objeto `{"X-Key": "value"}`. |
| `base_url` | não | — | Endereço público do seu DumbMonit, para que `{{link}}` dê uma ligação clicável. |
| `username` | não | — | Nome de utilizador para autenticação HTTP «Basic». Exige o segredo `password`. |

| Segredo | Função |
|---|---|
| `url` | Use-o em vez da definição `url` se o endereço já contiver um token. |
| `token` | Enviado como um cabeçalho `Authorization: Bearer …`, a menos que o seu modelo use `{{token}}`. |
| `password` | Senha para autenticação «Basic». |

### Variáveis disponíveis {#available-variables}

Uma variável é escrita entre **chavetas duplas**: `{{title}}`.
Espaços em volta do nome são tolerados (`{{ title }}`). Chavetas
simples num corpo JSON permanecem literais, por isso pode escrever o
seu JSON normalmente.

| Variável | O que contém | Exemplo |
|---|---|---|
| `{{title}}` | Título completo da mensagem | `nas — Disk full` |
| `{{message}}` | Corpo em texto simples, uma linha por alerta | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | O mesmo corpo em Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Nome da regra de alerta | `Disk full` |
| `{{target}}` | Nome do dispositivo monitorizado | `nas` |
| `{{target_id}}` | Identificador numérico do dispositivo | `42` |
| `{{severity}}` | Severidade técnica | `info`, `warning` ou `critical` |
| `{{severity_fr}}` | Severidade como uma palavra legível (francês) | `information`, `avertissement`, `critique` |
| `{{status}}` | Estado técnico | `firing` ou `resolved` |
| `{{status_fr}}` | Estado como uma palavra legível (francês) | `en cours` ou `résolu` |
| `{{value}}` | Valor medido, unidade incluída | `95 %` |
| `{{value_raw}}` | Valor nu, usável como um **número** JSON | `95` |
| `{{threshold}}` | Limiar atravessado, unidade incluída | `90 %` |
| `{{threshold_raw}}` | Limiar nu, usável como um **número** JSON | `90` |
| `{{unit}}` | Unidade do valor | `%` |
| `{{operator}}` | Comparação do limiar | `>` |
| `{{count}}` | Número de alertas agrupados nesta mensagem | `3` |
| `{{fingerprint}}` | Identificador estável do alerta | `nas/disk-full` |
| `{{timestamp}}` | Data-hora ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Data-hora em segundos desde 1970 | `1788358325` |
| `{{date}}` | Data-hora legível, em UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Prioridade de 1 (baixa) a 5 (alta) | `4` |
| `{{color}}` | Cor de destaque, sem cardinal | `D98A00` |
| `{{color_hex}}` | Cor de destaque, com cardinal | `#D98A00` |
| `{{emoji}}` | Pictograma de estado | `🔴`, `⚠️`, `ℹ️` ou `✅` |
| `{{link}}` | Ligação ao dispositivo no DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Sempre `dumbmonit` | `dumbmonit` |
| `{{token}}` | O segredo `token` do canal | *(o seu token)* |

Duas observações úteis:

- **Algumas variáveis podem estar vazias.** Um alerta de
  disponibilidade («host unreachable») não tem nem valor nem limiar:
  `{{value}}` e `{{value_raw}}` então renderizam uma cadeia vazia,
  nunca `null` ou `NaN`. Se os colocar como um número JSON, trate
  esse caso — ou use `"{{value}}"` entre aspas.
- **`{{link}}` fica vazio** até preencher `base_url`. Uma ligação
  errada perderia mais tempo do que um alerta sem nenhuma.

**Nunca precisa de escapar nada você mesmo.** Dependendo do
`content_type` escolhido, o DumbMonit protege automaticamente os
valores substituídos: aspas e quebras de linha escapadas em JSON,
percent-encoding num formulário, nenhuma transformação em texto
simples. Uma regra chamada `"System" disk full` não quebrará o seu
modelo JSON.

### Exemplo 1 — uma API JSON interna {#example-1--an-internal-json-api}

Uma aplicação caseira expõe `POST /incidents` e espera um token num
cabeçalho personalizado.

**Settings**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "headers": {
    "X-Application": "dumbmonit"
  },
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Secrets**

```json
{ "token": "your-application-token" }
```

O token é enviado automaticamente como um cabeçalho `Authorization:
Bearer …`. Isto é o que a aplicação recebe:

```json
{
  "title": "nas — Disk full",
  "device": "nas",
  "severity": "warning",
  "state": "firing",
  "value": 95,
  "threshold": 90,
  "detail": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "link": "https://dumbmonit.home/targets/42",
  "seen_at": "2026-09-01T14:12:05+00:00"
}
```

### Exemplo 2 — um serviço que espera um formulário {#example-2--a-service-that-expects-a-form}

Muitos gateways de SMS e APIs mais antigas esperam
`application/x-www-form-urlencoded` em vez de JSON, com o token no
corpo.

**Settings**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Secrets**

```json
{ "token": "gateway-key" }
```

Como o modelo já usa `{{token}}`, o DumbMonit **não adiciona** um
cabeçalho `Authorization`: alguns serviços rejeitam um pedido que
transporte duas autenticações. Os valores são codificados
automaticamente, os espaços tornando-se `+` e os caracteres
acentuados o seu equivalente percent-encoded.

### Exemplo 3 — um SMS em texto simples através de uma operadora {#example-3--a-plain-text-sms-through-a-carrier}

Alguns serviços, como a API de SMS de uma operadora ou uma automação
caseira, querem uma única linha de texto.

**Settings**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

A mensagem enviada, sem nenhum escape:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Exemplo 4 — um pedido GET sem corpo {#example-4--a-get-request-without-a-body}

Quando o serviço só aceita parâmetros de URL, coloque as variáveis em
`url` e escolha o método `GET`. Um pedido `GET` não tem corpo:
`body_template` é então recusado, para que ninguém pense que faz
alguma coisa.

**Settings**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Secrets**

```json
{ "token": "api-key" }
```

Só os **valores substituídos** são codificados; o resto do endereço
é copiado como está. O pedido realmente enviado parece-se com:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Erros comuns de modelo {#common-template-mistakes}

Um modelo defeituoso é **recusado ao guardar**, enquanto está a
olhar para o formulário, não na noite em que o incidente acontece.

| O que escreve | O que acontece |
|---|---|
| `{{severite}}` | Recusado: variável desconhecida «severite», seguida pela lista de nomes válidos. O nome correto é `severity`. |
| `{"t": "{{title"}` | Recusado: «unclosed braces». O `}}` está em falta. |
| `{{}}` ou `{{  }}` | Recusado: uma variável sem nome não significa nada. |
| `{"a": {"b": 1}}` | **Aceite.** Chavetas simples são literais: escreva o seu JSON normalmente. |
| `{{token}}` sem um segredo `token` | Recusado: o modelo pede um token que o canal não tem. |
| `method: GET` com `body_template` | Recusado, com a alternativa: coloque as variáveis em `url`. |
| `method: DELETE` | Recusado: só `POST`, `PUT` e `GET` são aceites. |

---

## Canais suportados {#supported-channels}

### Discord {#discord}

No Discord: **Channel settings → Integrations → Webhooks → New
webhook**, depois **Copy webhook URL**. Este URL contém o token: vai
para os segredos.

- **Secrets** — `webhook_url`

### Slack {#slack}

Em <https://api.slack.com/apps>, crie uma app, ative **Incoming
Webhooks**, depois **Add New Webhook to Workspace** e escolha o
canal. Copie o URL `https://hooks.slack.com/services/…`.

- **Secrets** — `webhook_url`

### Telegram {#telegram}

Precisa de duas coisas: o **token do bot** (segredo) e o **ID de
chat** (`chat_id`) onde tem de escrever. A primeira demora um minuto
a obter; a segunda depende de onde quer receber os alertas.

#### 1. Criar o bot e obter o seu token {#1-create-the-bot-and-get-its-token}

1. No Telegram, procure por **@BotFather** e clique em *Start*.
2. Envie `/newbot`, depois responda às perguntas (um nome, depois
   um nome de utilizador que tem de terminar com `bot`).
3. O BotFather termina com uma mensagem como esta:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   A linha `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` é o **token do
   bot**. Não o partilhe: dá controlo total do bot.

Nos exemplos abaixo, substitua `<TOKEN>` por este token. Note o
prefixo `bot` colado à frente dele nos URLs:
`https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Encontrar o `chat_id` {#2-find-the-chat_id}

**Chat privado** (o bot escreve-lhe diretamente)

1. Abra o seu bot (`t.me/my_homelab_bot`) e clique em *Start*, ou
   envie-lhe qualquer mensagem. Sem isto, nunca conseguirá
   escrever-lhe.
2. Abra num navegador: `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Obtém um documento JSON; procure por `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   O `chat_id` é o valor de `chat.id`: aqui `21xxxxx38` (um número
   positivo).

   Se `result` estiver vazio (`"result":[]`), envie ao bot outra
   mensagem e recarregue a página.

**Canal**

1. Adicione o bot como administrador do canal (Manage channel →
   Administrators → Add).
2. Publique uma mensagem no canal.
3. Abra `https://api.telegram.org/bot<TOKEN>/getUpdates`: a resposta
   contém um bloco `channel_post` com
   `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   O `chat_id` é esse número negativo, `-100` incluído:
   `-1001xxxxxx062`.

**Grupo**

A forma mais fácil passa pela app de desktop:

1. Adicione o bot ao grupo.
2. Envie uma mensagem no grupo, clique com o botão direito nela →
   *Copy message link*. Obtém `https://t.me/c/194xxxx987/13` (ou
   `https://t.me/c/194xxxx987/11/13` se o grupo tiver tópicos).
3. O número logo depois de `/c/` é o identificador interno do grupo:
   `194xxxx987`. O `chat_id` a introduzir é esse número **prefixado
   com `-100`**: `-100194xxxx987`.

Sem a app de desktop, o método `getUpdates` acima também funciona:
`message.chat.id` já é diretamente o número certo (já negativo). Se
o bot não vir as mensagens do grupo, peça ao BotFather `/setprivacy`
→ *Disable*, depois remova o bot do grupo e adicione-o de novo.

**Tópico de grupo** (grupos no modo «Topics»)

O `chat_id` é o do grupo. Além disso, preencha **Group topic**
(`message_thread_id`) com o número do meio da ligação copiada: em
`https://t.me/c/194xxxx987/11/13`, o tópico é `11`. Sem esta
definição, a mensagem aterra no fio *General*.

#### 3. Verificar antes de guardar {#3-check-before-saving}

Abra no navegador:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(adicione `&message_thread_id=11` para um tópico). Se `test123`
chegar, o token e o `chat_id` estão certos: introduza-os no DumbMonit
e clique em *Test*.

- **Settings** — `chat_id` (obrigatório), `message_thread_id`
  (tópico, opcional), `api_base` (predefinição
  `https://api.telegram.org`, altere-o só para passar por um relay)
- **Secrets** — `bot_token`

> **«Telegram cannot find this chat»** — para um chat privado, o bot
> nunca recebeu uma mensagem sua: envie-lhe `/start`. Para um grupo
> ou canal, o prefixo `-100` está provavelmente em falta, ou o bot
> não foi adicionado a ele.
>
> **«thread not found»** — o `message_thread_id` não corresponde a
> nenhum tópico do grupo, ou o grupo não está no modo «Topics».

### Microsoft Teams {#microsoft-teams}

**Aviso: o método mudou.** Os antigos «Office 365 connectors» foram
retirados em maio de 2026; um URL `outlook.office.com/webhook/…` já
não funciona.

O procedimento atual:

1. No canal Teams, **⋯ → Workflows**.
2. Escolha o modelo **«Post to a channel when a webhook request is
   received»**.
3. Confirme os passos, depois **copie o URL POST HTTPS** mostrado no
   fim.

- **Secrets** — `webhook_url`

O DumbMonit envia um *Adaptive Card* encapsulado, o único formato
que estes fluxos mostram corretamente. Se obtiver um erro
mencionando o Power Automate, o URL provavelmente vem de um
conector antigo.

### Matrix {#matrix}

1. Crie uma conta dedicada ao DumbMonit no seu homeserver, e
   **convide-a para a sala**. Uma conta que não é membro não pode
   escrever.
2. Obtenha o seu token de acesso. No Element: **Settings → Help &
   About → Advanced → Access token**.
3. Obtenha o identificador **interno** da sala — não o seu nome de
   exibição. No Element: **Room settings → Advanced → Internal room
   ID**. Começa com `!` (por exemplo `!aBcDeF:matrix.org`).

- **Settings** — `server_url` (por exemplo `https://matrix.org`), `room_id`
- **Secrets** — `token`

### Mattermost {#mattermost}

**Integrations → Incoming Webhooks → Add**. Escolha o canal
predefinido, depois copie o URL.

- **Settings** — `channel` (opcional, para escrever noutro lugar
  além do canal predefinido), `username` (opcional, predefinição
  `DumbMonit`)
- **Secrets** — `webhook_url`

### Rocket.Chat {#rocketchat}

**Administration → Integrations → New integration → Incoming
webhook**. Escolha a sala, ative a integração, depois copie o
*Webhook URL*.

- **Settings** — `channel` (opcional), `alias` (opcional,
  predefinição `DumbMonit`)
- **Secrets** — `webhook_url`

### Google Chat {#google-chat}

No espaço Google Chat: **nome do espaço → Apps & integrations →
Webhooks → Add webhook**. Copie o URL, que já contém a chave e o
token.

- **Secrets** — `webhook_url`

### Zulip {#zulip}

1. Crie um **bot**: **Personal settings → Bots → Add a new bot**,
   tipo «Generic». Anote o seu endereço de email e a sua chave de
   API.
2. Subscreva o bot ao stream destino, caso contrário o envio será
   recusado.

- **Settings** — `server_url` (por exemplo
  `https://your-org.zulipchat.com`), `email` (o endereço do bot),
  `stream` (o nome do stream), `topic` (opcional, predefinição
  `DumbMonit`)
- **Secrets** — `api_key`

### ntfy {#ntfy}

O canal mais simples de todos: escolha um nome de tópico difícil de
adivinhar, depois subscreva-o a partir da app ntfy.

- **Settings** — `topic`, `server_url` (opcional, predefinição
  `https://ntfy.sh`)
- **Secrets** — `token` (só se o seu servidor exigir autenticação)

> Na instância pública, **qualquer um que conheça o nome de um
> tópico pode lê-lo**. Escolha um nome longo e aleatório, ou aloje
> o seu próprio servidor.

### Gotify {#gotify}

Na interface Gotify: **Apps → Create Application**. Copie o token
mostrado.

- **Settings** — `server_url` (por exemplo `https://gotify.home`)
- **Secrets** — `token`

### Pushover {#pushover}

1. Em <https://pushover.net>, a sua **User Key** é mostrada logo na
   página inicial.
2. Depois crie uma aplicação: **Create an Application/API Token**.
   Obtém um **token de aplicação**.

- **Settings** — `priority` (opcional, de -2 a 2), `sound`
  (opcional), `retry` e `expire` (só para `priority: 2`)
- **Secrets** — `token` (token de aplicação), `user_key` (chave de
  utilizador)

Sem uma definição `priority`, o DumbMonit escolhe conforme a
severidade: `-1` para um aviso ou uma resolução, `0` para um warning,
`1` para um alerta crítico. Um aviso portanto não faz o telefone
tocar.

> A prioridade `2` («emergency») repete a notificação até ser
> confirmada. O DumbMonit então adiciona `retry` e `expire`
> automaticamente, sem os quais o Pushover recusaria a mensagem.

### Pushbullet {#pushbullet}

Em <https://www.pushbullet.com/#settings/account>, clique em
**Create Access Token**.

- **Settings** — `device_iden` (opcional; sem ele, todos os seus
  dispositivos são notificados)
- **Secrets** — `token`

### Bark (iOS) {#bark-ios}

Abra a app Bark no seu iPhone: mostra um URL contendo a sua **chave
de dispositivo**, a cadeia longa de caracteres depois do nome de
domínio.

- **Settings** — `server_url` (opcional, predefinição
  `https://api.day.app`), `group` (opcional), `level` (opcional),
  `sound` (opcional)
- **Secrets** — `token` (a chave do dispositivo)

Sem uma definição `level`, o DumbMonit escolhe: `passive` para um
aviso, `active` para um warning, `timeSensitive` para um alerta
crítico. O nível `critical`, que atravessa o «Do Not Disturb», nunca
é escolhido automaticamente — defina-o explicitamente se o quiser.

### Web Push {#web-push}

Notificações nativas nos seus telefones e computadores, diretamente
do seu próprio DumbMonit, sem nenhuma app para instalar e nenhuma
conta de terceiros: o navegador (instalado como app ou não) recebe
os alertas mesmo com todos os separadores fechados.

1. Em cada telefone ou computador, abra o DumbMonit e vá a
   **Settings → Push notifications → Enable on this device**, depois
   **Send a test**. No iPhone e iPad, adicione primeiro o DumbMonit
   ao ecrã principal. O guia passo a passo está em
   [Notificações push (PWA)](using/push-notifications.md).
2. Crie um canal **Web Push** aqui. Não tem segredo: os seus
   destinatários são os dispositivos subscritos no passo 1.

- **Settings** — `users` (opcional; uma conta por linha, vazio: todo
  dispositivo subscrito de toda conta), `urgency` (`high` por
  predefinição, `normal` ou `low`), `ttl` (segundos que o serviço
  push mantém a mensagem para um dispositivo desligado, predefinição
  `86400`), `contact` (endereço `mailto:` ou `https:` opcional dado
  aos serviços push; vazio: `DUMBMONIT_PUBLIC_URL` quando é HTTPS,
  caso contrário a página do projeto)
- **Secrets** — nenhum. O par de chaves VAPID do servidor é gerado no
  primeiro arranque e guardado encriptado; a chave privada nunca
  sai do servidor.

O título da notificação é a severidade e o dispositivo («Critical ·
nas01»), o corpo um resumo curto; um toque abre a página **Alerts**.
O DumbMonit precisa de HTTPS de saída para os serviços push
(`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`); as
mensagens são encriptadas de ponta a ponta, esses serviços não as
podem ler. Uma subscrição que o serviço push reporta como
desaparecida (HTTP 404 ou 410, por exemplo depois de a app ser
removida) é eliminada automaticamente.

### Apprise {#apprise}

O Apprise é uma **gateway**: com uma única configuração dá acesso a
dezenas de serviços que o DumbMonit não implementa ele próprio. Faça
deploy do contentor `caronc/apprise`, depois escolha um dos dois
modos.

**Modo recomendado — configuração guardada no Apprise.** Crie uma
configuração na interface do Apprise, anote a sua chave, e não ponha
mais nada aqui.

- **Settings** — `server_url`, `config_key`

**Modo direto — o DumbMonit fornece os destinos.** Os URLs do
Apprise frequentemente contêm uma senha (`mailto://user:password@…`):
vão portanto para os segredos.

- **Settings** — `server_url`
- **Secrets** — `urls` (um ou mais URLs Apprise, separados por
  vírgulas)

Definições comuns opcionais: `tag` (para visar só parte dos destinos
de uma configuração) e `format` (`text` por predefinição, ou
`markdown`).

> Se o teste responder «Apprise has no configuration under this
> config_key», a chave não corresponde a nada. O DumbMonit reporta-o
> em vez de fingir que a mensagem foi enviada.

### Home Assistant {#home-assistant}

1. No Home Assistant: **o seu perfil → Security → Long-lived access
   tokens → Create token**. Copie-o, não será mostrado de novo.
2. Escolha o serviço a chamar. Por predefinição, o DumbMonit cria
   uma notificação persistente no painel.

- **Settings** — `server_url` (por exemplo
  `http://homeassistant.local:8123`), `service` (opcional,
  predefinição `persistent_notification.create`), `target`
  (opcional), `data` (opcional, objeto fundido na chamada)
- **Secrets** — `token`

Alguns serviços úteis:

| `service` | Efeito |
|---|---|
| `persistent_notification.create` | Faixa na interface do Home Assistant |
| `notify.notify` | Todos os destinos de notificação configurados |
| `notify.mobile_app_<device>` | A app móvel de um dispositivo específico |
| `light.turn_on` | Liga uma luz — use `data` para a entidade e a cor |

A definição `data` é fundida **por último**: pode portanto
substituir o título ou a mensagem que o DumbMonit preparou.

### Email (SMTP) {#email-smtp}

O único canal que não depende de nenhum serviço de terceiros.

- **Settings** — `host`, `from`, `to` (lista de endereços),
  `security` (`starttls` por predefinição, ou `tls`, ou `none`),
  `port` (derivada de `security`: 587, 465 ou 25), `username`
  (opcional)
- **Secrets** — `password`

> Com o Gmail, uma senha de conta não funciona: crie uma **senha de
> app** nas definições de segurança da sua conta Google.

### Signal {#signal}

Passa por uma instância
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api)
que aloja e onde primeiro regista o seu número.

- **Settings** — `server_url` (por exemplo
  `http://signal.home:8080`), `number` (o número remetente
  registado), `recipients` (lista de números ou identificadores de
  grupo), `username` (opcional)
- **Secrets** — `password` (opcional, se a sua instância está
  protegida)

Todos os números são escritos em **formato internacional**,
`+351912345678`.

### SMS via Twilio {#sms-via-twilio}

Na consola Twilio, a página inicial mostra o seu **Account SID** e o
seu **Auth Token**. Depois compre um número que possa enviar SMS.

- **Settings** — `account_sid`, `from` (o seu número Twilio) **ou**
  `messaging_service_sid`, `to` (lista de destinatários)
- **Secrets** — `token` (o *Auth Token*)

O corpo do SMS é truncado a 300 caracteres: além disso, o Twilio
divide-o em segmentos cobrados separadamente.

> Numa conta de teste, o Twilio só envia para números que verificou.

### PagerDuty {#pagerduty}

No PagerDuty: **Services → o seu serviço → Integrations → Add
integration**, e escolha **Events API v2**. Copie a *Integration
Key*.

- **Settings** — `region` (`eu` se a sua conta está hospedada na
  Europa), `source` (opcional, predefinição `dumbmonit`)
- **Secrets** — `token` (a *Integration Key*)

Um alerta abre um incidente; a sua resolução **fecha-o
automaticamente**. A correspondência é feita na impressão digital do
alerta, prefixada com `dumbmonit-`, por isso um lembrete nunca abre
um segundo incidente.

> Uma chave de outro tipo («Events API v1», «REST API») recebe uma
> recusa seca. Verifique que a integração é de facto do tipo *Events
> API v2*.

### Opsgenie {#opsgenie}

**Teams → a sua equipa → Integrations → Add integration → API**.
Copie a chave de API.

- **Settings** — `region` (`eu` para uma conta europeia),
  `responders` (lista de nomes de equipa, opcional), `tags` (lista,
  opcional)
- **Secrets** — `api_key`

Como com o PagerDuty, uma resolução fecha o alerta, identificado pelo
seu alias.

> A Atlassian definiu o **fim do serviço do Opsgenie para 5 de abril
> de 2027**. Este canal fica para instalações existentes; para uma
> nova, prefira o PagerDuty ou um canal personalizado que aponte para
> o Jira Service Management.

### Webhook personalizado {#custom-webhook}

Ver [O canal personalizado](#the-custom-channel) acima.

---

## Resolução de problemas {#troubleshooting}

**O teste falha de imediato, com «missing setting …».**
Um campo obrigatório está vazio. A mensagem nomeia a definição exata.

**O teste falha com «no answer within 15 s».**
O servidor de destino está inacessível a partir do DumbMonit. Se é
um serviço na sua rede local, verifique que o contentor DumbMonit o
consegue alcançar — isto é frequentemente um problema de rede
Docker, não um problema de configuração de canal.

**O teste tem sucesso, mas não recebo nada.**
O serviço aceitou a mensagem e enviou-a para outro lugar: canal
errado, tópico ntfy errado, dispositivo Pushbullet errado. Verifique
a definição de destino.

**A mensagem de erro contém `***`.**
Isto é intencional: o seu token foi removido da mensagem antes de
ser registada. Nenhum segredo aparece alguma vez nos registos, na
coluna «last error», ou numa resposta de API.

**Onde posso ver o último erro de um canal?**
Na lista de canais: cada linha mostra a data da última entrega
bem-sucedida e, se existir, o erro da última falha.
