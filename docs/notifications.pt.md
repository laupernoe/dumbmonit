# Notificações do DumbMonit {#dumbmonit-notifications}

Este guia explica como receber os alertas do DumbMonit no serviço da sua escolha.
Não exige conhecimentos de programação: cada secção diz onde encontrar o endereço
ou token a copiar, e o que escrever no formulário.

---

## Índice {#contents}

- [Como funciona](#how-it-works)
- [Criar e testar um canal](#creating-and-testing-a-channel)
- [Definições ou segredos: a diferença](#settings-or-secrets-the-difference)
- [Certificados autoassinados](#self-signed-certificates)
- [O canal personalizado](#the-custom-channel)
  - [Variáveis disponíveis](#available-variables)
  - [Exemplo 1 — uma API JSON interna](#example-1--an-internal-json-api)
  - [Exemplo 2 — um serviço que espera um formulário](#example-2--a-service-that-expects-a-form)
  - [Exemplo 3 — um SMS em texto simples através de um operador](#example-3--a-plain-text-sms-through-a-carrier)
  - [Exemplo 4 — um pedido GET sem corpo](#example-4--a-get-request-without-a-body)
  - [Erros comuns de modelos](#common-template-mistakes)
- [Canais suportados](#supported-channels)
- [Resolução de problemas](#troubleshooting)

---

## Como funciona {#how-it-works}

Um **canal** é um destino: um canal do Discord, um número de telefone, uma caixa
de correio. Cria quantos quiser, e as suas regras de alerta escolhem quais
notificar.

Três princípios aplicam-se a todos os canais:

1. **Os alertas são agrupados.** Uma mensagem por dispositivo e por ciclo, mesmo
   que cinco discos encham ao mesmo tempo. Sem isto, silenciaria as notificações
   numa semana.
2. **Um canal avariado nunca bloqueia outro.** A falha fica registada na coluna
   «último erro» do canal, e a entrega continua para os outros.
3. **Um serviço que não responde é abandonado ao fim de 15 segundos.** Uma
   notificação que chega um minuto atrasada já não serve, e o ciclo de
   monitorização não deve esperar por um servidor inacessível.

---

## Criar e testar um canal {#creating-and-testing-a-channel}

1. Abra **Settings → Notifications & reports → Add channel** (`/settings/notifications`).
2. Dê-lhe um nome com significado («Discord — família», «SMS — piquete»). É o
   nome que escolherá mais tarde nas suas regras de alerta.
3. Escolha o tipo de canal, depois preencha os campos descritos abaixo.
4. Guarde, depois clique em **Enviar teste**.

A mensagem de teste segue **exatamente o mesmo caminho** que um alerta real: o
mesmo pedido, o mesmo token, a mesma formatação. Se chegar, o canal funciona. Se
falhar, a mensagem de erro nomeia a definição a corrigir.

> **Um token nunca é mostrado de novo.** Depois de guardado, é cifrado na base de
> dados e a interface diz apenas «configurado». Para editar um canal sem voltar
> a escrever o seu token, deixe simplesmente o campo vazio.

---

## Definições ou segredos: a diferença {#settings-or-secrets-the-difference}

Todo o canal tem dois grupos de campos.

| | **Definições** | **Segredos** |
|---|---|---|
| Conteúdo | endereço do servidor, nome do canal, tópico | tokens, palavras-passe, URLs de webhooks |
| Armazenamento | texto simples | cifrado |
| Relido pela interface | sim | nunca |

A interface **recusa** guardar um campo sensível entre as definições, e di-lo.
Não é uma formalidade: as definições são devolvidas tal como estão pela API, pelo
que um token guardado aí seria publicado de cada vez que a página fosse aberta.

As tabelas da secção [Canais suportados](#supported-channels) dizem, para cada
canal, o que vai onde.

---

## Certificados autoassinados {#self-signed-certificates}

Por predefinição o DumbMonit recusa qualquer servidor cujo certificado não consiga
verificar. Para um serviço auto-alojado servido com um certificado autoassinado ou
de uma autoridade privada, os canais Matrix, Mattermost, Rocket.Chat, Zulip, ntfy,
Gotify, Bark, Apprise, Home Assistant, Signal e o webhook personalizado têm uma
definição **Aceitar um certificado não verificável** (`insecure_tls`, um booleano,
desativado por predefinição).

Ativá-la mantém a ligação cifrada mas deixa de verificar a identidade do
servidor, pelo que só a deve usar para um servidor que controla, numa rede em que
confia. Quando um certificado é o motivo de uma falha, o erro mostrado para o
canal nomeia-o (por exemplo «invalid peer certificate: UnknownIssuer»).

---

## O canal personalizado {#the-custom-channel}

O canal **`webhook`** é a válvula de escape do DumbMonit. Descreve todo o pedido
HTTP — método, endereço, cabeçalhos, tipo de conteúdo e corpo —, o que permite
ligar quase qualquer serviço, incluindo uma API interna que o DumbMonit nunca
conhecerá.

**Sem qualquer configuração particular**, já envia uma carga JSON estruturada que
um script pode usar:

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
| `content_type` | não | `json` | `json`, `form` ou `text`. Determina também a forma como as variáveis são escapadas. |
| `body_template` | não | — | Modelo do corpo. Quando ausente, é enviada a carga acima. |
| `base_url` | não | — | Endereço público do seu DumbMonit, para que `{{link}}` dê uma ligação clicável. |
| `username` | não | — | Nome de utilizador para autenticação HTTP «Basic». Requer o segredo `password`. |

| Segredo | Função |
|---|---|
| `url` | Use-o em vez da definição `url` se o endereço já contém um token. |
| `token` | Enviado como cabeçalho `Authorization: Bearer …`, a menos que o seu modelo use `{{token}}`. |
| `password` | Palavra-passe para autenticação «Basic». |
| `headers` | Cabeçalhos extra, como objeto JSON `{"X-Api-Key": "value"}`. Guardados cifrados como o token e nunca mais mostrados, pelo que as chaves de API e os cabeçalhos `Authorization` pertencem aqui. |

`headers` é um segredo. Os canais guardados pela 0.1.0-alpha.6 ou anterior, que os
mantinham nas definições, têm-nos movidos para os segredos cifrados no primeiro
arranque, e a API deixa de os devolver. Como todo o segredo, são mantidos quando
edita o canal sem os escrever de novo; enviar `secrets` substitui o conjunto
inteiro.

O DumbMonit segue um redirecionamento apenas para o mesmo anfitrião (ou de `http`
para `https` nesse anfitrião): os cabeçalhos e o token de um canal nunca vão para
lado nenhum além do endereço que introduziu.

### Variáveis disponíveis {#available-variables}

Uma variável escreve-se entre **chavetas duplas**: `{{title}}`. Os espaços à volta
do nome são tolerados (`{{ title }}`). As chavetas simples num corpo JSON ficam
literais, pelo que pode escrever o seu JSON normalmente.

| Variável | O que contém | Exemplo |
|---|---|---|
| `{{title}}` | Título completo da mensagem | `nas — Disk full` |
| `{{message}}` | Corpo em texto simples, uma linha por alerta | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | O mesmo corpo em Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Nome da regra de alerta | `Disk full` |
| `{{target}}` | Nome do dispositivo monitorizado | `nas` |
| `{{target_id}}` | Identificador numérico do dispositivo | `42` |
| `{{severity}}` | Gravidade técnica | `info`, `warning` ou `critical` |
| `{{severity_fr}}` | Gravidade como palavra legível | `information`, `avertissement`, `critique` |
| `{{status}}` | Estado técnico | `firing` ou `resolved` |
| `{{status_fr}}` | Estado como palavra legível | `en cours` ou `résolu` |
| `{{value}}` | Valor medido, unidade incluída | `95 %` |
| `{{value_raw}}` | Valor simples, utilizável como **número** JSON | `95` |
| `{{threshold}}` | Limiar ultrapassado, unidade incluída | `90 %` |
| `{{threshold_raw}}` | Limiar simples, utilizável como **número** JSON | `90` |
| `{{unit}}` | Unidade do valor | `%` |
| `{{operator}}` | Comparação com o limiar | `>` |
| `{{count}}` | Número de alertas agrupados nesta mensagem | `3` |
| `{{fingerprint}}` | Identificador estável do alerta | `nas/disk-full` |
| `{{timestamp}}` | Carimbo temporal ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Carimbo temporal em segundos desde 1970 | `1788358325` |
| `{{date}}` | Carimbo temporal legível, em UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Prioridade de 1 (baixa) a 5 (alta) | `4` |
| `{{color}}` | Cor de destaque, sem cardinal | `D98A00` |
| `{{color_hex}}` | Cor de destaque, com cardinal | `#D98A00` |
| `{{emoji}}` | Pictograma do estado | `🔴`, `⚠️`, `ℹ️` ou `✅` |
| `{{link}}` | Ligação para o dispositivo no DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Sempre `dumbmonit` | `dumbmonit` |
| `{{token}}` | O segredo `token` do canal | *(o seu token)* |

Duas observações úteis:

- **Algumas variáveis podem estar vazias.** Um alerta de disponibilidade
  («anfitrião inacessível») não tem valor nem limiar: `{{value}}` e
  `{{value_raw}}` produzem então uma cadeia vazia, nunca `null` nem `NaN`. Se as
  colocar como número JSON, trate esse caso — ou use `"{{value}}"` entre aspas.
- **`{{link}}` fica vazio** até preencher `base_url`. Uma ligação errada
  desperdiçaria mais tempo do que um alerta sem ela.

**Nunca precisa de escapar nada você próprio.** Consoante o `content_type`
escolhido, o DumbMonit protege automaticamente os valores substituídos: aspas e
quebras de linha escapadas em JSON, codificação por percentagem num formulário,
nenhuma transformação em texto simples. Uma regra chamada `"System" disk full`
não quebrará o seu modelo JSON.

### Exemplo 1 — uma API JSON interna {#example-1--an-internal-json-api}

Uma aplicação caseira expõe `POST /incidents` e espera um token num cabeçalho
personalizado.

**Definições**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Segredos**

```json
{
  "token": "your-application-token",
  "headers": "{\"X-Application\": \"dumbmonit\"}"
}
```

O token é enviado automaticamente como cabeçalho `Authorization: Bearer …`. Eis o
que a aplicação recebe:

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

Muitos gateways de SMS e APIs mais antigas esperam `application/x-www-form-urlencoded`
em vez de JSON, com o token no corpo.

**Definições**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Segredos**

```json
{ "token": "gateway-key" }
```

Como o modelo já usa `{{token}}`, o DumbMonit **não acrescenta** um cabeçalho
`Authorization`: alguns serviços rejeitam um pedido com duas autenticações. Os
valores são codificados automaticamente, os espaços tornando-se `+` e os
caracteres acentuados o seu equivalente codificado por percentagem.

### Exemplo 3 — um SMS em texto simples através de um operador {#example-3--a-plain-text-sms-through-a-carrier}

Alguns serviços, como a API de SMS de um operador ou uma automação caseira,
querem uma única linha de texto.

**Definições**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

A mensagem enviada, sem qualquer escape:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Exemplo 4 — um pedido GET sem corpo {#example-4--a-get-request-without-a-body}

Quando o serviço só aceita parâmetros de URL, ponha as variáveis em `url` e escolha
o método `GET`. Um pedido `GET` não tem corpo: `body_template` é então rejeitado,
para que ninguém acredite que faz alguma coisa.

**Definições**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Segredos**

```json
{ "token": "api-key" }
```

Apenas os **valores substituídos** são codificados; o resto do endereço é copiado
tal como está. O pedido realmente enviado é semelhante a:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Erros comuns de modelos {#common-template-mistakes}

Um modelo defeituoso é **rejeitado ao guardar**, enquanto olha para o formulário,
não na noite em que o incidente acontece.

| O que escreve | O que acontece |
|---|---|
| `{{severite}}` | Rejeitado: variável desconhecida «severite», seguida da lista de nomes válidos. O nome correto é `severity`. |
| `{"t": "{{title"}` | Rejeitado: «unclosed braces». Falta o `}}`. |
| `{{}}` ou `{{  }}` | Rejeitado: uma variável sem nome não significa nada. |
| `{"a": {"b": 1}}` | **Aceite.** As chavetas simples são literais: escreva o seu JSON normalmente. |
| `{{token}}` sem segredo `token` | Rejeitado: o modelo pede um token que o canal não tem. |
| `method: GET` com `body_template` | Rejeitado, com a alternativa: ponha as variáveis em `url`. |
| `method: DELETE` | Rejeitado: só `POST`, `PUT` e `GET` são aceites. |

---

## Canais suportados {#supported-channels}

### Discord {#discord}

No Discord: **Definições do canal → Integrações → Webhooks → Novo webhook**, depois
**Copiar URL do webhook**. Este URL contém o token: vai para os segredos.

- **Segredos** — `webhook_url`

### Slack {#slack}

Em <https://api.slack.com/apps>, crie uma app, ative os **Incoming Webhooks**,
depois **Add New Webhook to Workspace** e escolha o canal. Copie o URL
`https://hooks.slack.com/services/…`.

- **Segredos** — `webhook_url`

### Telegram {#telegram}

Precisa de duas coisas: o **token do bot** (segredo) e o **ID do chat**
(`chat_id`) onde ele deve escrever. O primeiro demora um minuto a obter; o
segundo depende de onde quer receber os alertas.

#### 1. Criar o bot e obter o seu token {#1-create-the-bot-and-get-its-token}

1. No Telegram, procure **@BotFather** e clique em *Start*.
2. Envie `/newbot`, depois responda às perguntas (um nome, depois um nome de
   utilizador que tem de terminar em `bot`).
3. O BotFather termina com uma mensagem como esta:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   A linha `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` é o **token do bot**. Não o
   partilhe: dá controlo total do bot.

Nos exemplos abaixo, substitua `<TOKEN>` por este token. Repare no prefixo `bot`
colado à frente dele nos URLs: `https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Encontrar o `chat_id` {#2-find-the-chat_id}

**Chat privado** (o bot escreve-lhe diretamente)

1. Abra o seu bot (`t.me/my_homelab_bot`) e clique em *Start*, ou envie-lhe uma
   mensagem qualquer. Sem isto, ele nunca poderá escrever-lhe.
2. Abra num navegador: `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Obtém um documento JSON; procure `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   O `chat_id` é o valor de `chat.id`: aqui `21xxxxx38` (um número positivo).

   Se `result` estiver vazio (`"result":[]`), envie outra mensagem ao bot e
   recarregue a página.

**Canal**

1. Adicione o bot como administrador do canal (Gerir canal → Administradores →
   Adicionar).
2. Publique uma mensagem no canal.
3. Abra `https://api.telegram.org/bot<TOKEN>/getUpdates`: a resposta contém um
   bloco `channel_post` com `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   O `chat_id` é esse número negativo, `-100` incluído: `-1001xxxxxx062`.

**Grupo**

A forma mais fácil passa pela app de ambiente de trabalho:

1. Adicione o bot ao grupo.
2. Envie uma mensagem no grupo, clique com o botão direito → *Copy message link*.
   Obtém `https://t.me/c/194xxxx987/13` (ou `https://t.me/c/194xxxx987/11/13` se
   o grupo tem tópicos).
3. O número logo a seguir a `/c/` é o identificador interno do grupo:
   `194xxxx987`. O `chat_id` a introduzir é esse número **prefixado com `-100`**:
   `-100194xxxx987`.

Sem a app de ambiente de trabalho, o método `getUpdates` acima também serve:
`message.chat.id` é diretamente o número certo (já negativo). Se o bot não vê as
mensagens do grupo, peça ao BotFather `/setprivacy` → *Disable*, depois remova o
bot do grupo e adicione-o de novo.

**Tópico de grupo** (grupos em modo «Topics»)

O `chat_id` é o do grupo. Além disso, preencha **Tópico do grupo**
(`message_thread_id`) com o número do meio da ligação copiada: em
`https://t.me/c/194xxxx987/11/13`, o tópico é `11`. Sem esta definição, a mensagem
cai na conversa *General*.

#### 3. Verificar antes de guardar {#3-check-before-saving}

Abra no navegador:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(acrescente `&message_thread_id=11` para um tópico). Se `test123` chegar, o token e
o `chat_id` estão certos: introduza-os no DumbMonit e clique em *Testar*.

- **Definições** — `chat_id` (obrigatória), `message_thread_id` (tópico, opcional),
  `api_base` (predefinição `https://api.telegram.org`, altere-a apenas para passar
  por um relay)
- **Segredos** — `bot_token`

> **«Telegram cannot find this chat»** — para um chat privado, o bot nunca recebeu
> uma mensagem sua: envie-lhe `/start`. Para um grupo ou um canal, provavelmente
> falta o prefixo `-100`, ou o bot não foi adicionado.
>
> **«thread not found»** — o `message_thread_id` não corresponde a nenhum tópico do
> grupo, ou o grupo não está em modo «Topics».

### Microsoft Teams {#microsoft-teams}

**Atenção: o método mudou.** Os antigos «conectores do Office 365» foram
descontinuados em maio de 2026; um URL `outlook.office.com/webhook/…` já não
funciona.

O procedimento atual:

1. No canal do Teams, **⋯ → Workflows**.
2. Escolha o modelo **«Post to a channel when a webhook request is received»**.
3. Confirme os passos, depois **copie o URL HTTPS POST** mostrado no fim.

- **Segredos** — `webhook_url`

O DumbMonit envia um *Adaptive Card* embrulhado, o único formato que estes fluxos
apresentam corretamente. Se obtiver um erro que menciona o Power Automate, o URL
provavelmente vem de um conector antigo.

### Matrix {#matrix}

1. Crie uma conta dedicada ao DumbMonit no seu homeserver, e **convide-a para a
   sala**. Uma conta que não é membro não pode escrever.
2. Obtenha o seu token de acesso. No Element: **Settings → Help & About → Advanced
   → Access token**.
3. Obtenha o identificador **interno** da sala — não o seu nome de apresentação. No
   Element: **Room settings → Advanced → Internal room ID**. Começa por `!` (por
   exemplo `!aBcDeF:matrix.org`).

- **Definições** — `server_url` (por exemplo `https://matrix.org`), `room_id`
- **Segredos** — `token`

### Mattermost {#mattermost}

**Integrations → Incoming Webhooks → Add**. Escolha o canal predefinido, depois
copie o URL.

- **Definições** — `channel` (opcional, para escrever noutro sítio que não o canal
  predefinido), `username` (opcional, predefinição `DumbMonit`)
- **Segredos** — `webhook_url`

### Rocket.Chat {#rocketchat}

**Administration → Integrations → New integration → Incoming webhook**. Escolha a
sala, ative a integração, depois copie o *Webhook URL*.

- **Definições** — `channel` (opcional), `alias` (opcional, predefinição
  `DumbMonit`)
- **Segredos** — `webhook_url`

### Google Chat {#google-chat}

No espaço do Google Chat: **nome do espaço → Apps & integrations → Webhooks → Add
webhook**. Copie o URL, que já contém a chave e o token.

- **Segredos** — `webhook_url`

### Zulip {#zulip}

1. Crie um **bot**: **Personal settings → Bots → Add a new bot**, tipo «Generic».
   Anote o seu endereço de email e a sua chave de API.
2. Subscreva o bot no stream de destino, caso contrário o envio será recusado.

- **Definições** — `server_url` (por exemplo `https://your-org.zulipchat.com`),
  `email` (o endereço do bot), `stream` (o nome do stream), `topic` (opcional,
  predefinição `DumbMonit`)
- **Segredos** — `api_key`

### ntfy {#ntfy}

O canal mais simples de todos: escolha um nome de tópico difícil de adivinhar,
depois subscreva-o na app ntfy.

- **Definições** — `topic`, `server_url` (opcional, predefinição `https://ntfy.sh`)
- **Segredos** — `token` (apenas se o seu servidor exigir autenticação)

> Na instância pública, **qualquer pessoa que saiba o nome de um tópico pode lê-lo**.
> Escolha um nome longo e aleatório, ou aloje o seu próprio servidor.

### Gotify {#gotify}

Na interface do Gotify: **Apps → Create Application**. Copie o token mostrado.

- **Definições** — `server_url` (por exemplo `https://gotify.home`)
- **Segredos** — `token`

### Pushover {#pushover}

1. Em <https://pushover.net>, a sua **User Key** é mostrada logo na página inicial.
2. Depois crie uma aplicação: **Create an Application/API Token**. Obtém um
   **token de aplicação**.

- **Definições** — `priority` (opcional, de -2 a 2), `sound` (opcional), `retry` e
  `expire` (apenas para `priority: 2`)
- **Segredos** — `token` (token de aplicação), `user_key` (chave de utilizador)

Sem uma definição `priority`, o DumbMonit escolhe consoante a gravidade: `-1` para
um aviso consultivo ou uma resolução, `0` para um aviso, `1` para um alerta
crítico. Um alerta consultivo não faz portanto o telemóvel tocar.

> A prioridade `2` («emergency») repete a notificação até ser reconhecida. O
> DumbMonit acrescenta então `retry` e `expire` automaticamente, sem os quais o
> Pushover rejeitaria a mensagem.

### Pushbullet {#pushbullet}

Em <https://www.pushbullet.com/#settings/account>, clique em **Create Access
Token**.

- **Definições** — `device_iden` (opcional; sem ele, todos os seus dispositivos são
  notificados)
- **Segredos** — `token`

### Bark (iOS) {#bark-ios}

Abra a app Bark no seu iPhone: mostra um URL que contém a sua **device key**, a
longa cadeia de caracteres depois do nome de domínio.

- **Definições** — `server_url` (opcional, predefinição `https://api.day.app`),
  `group` (opcional), `level` (opcional), `sound` (opcional)
- **Segredos** — `token` (a device key)

Sem uma definição `level`, o DumbMonit escolhe: `passive` para um aviso
consultivo, `active` para um aviso, `timeSensitive` para um alerta crítico. O
nível `critical`, que ultrapassa o «Não incomodar», nunca é escolhido
automaticamente — defina-o explicitamente se o quiser.

### Web Push {#web-push}

Notificações nativas nos seus telemóveis e computadores, diretamente do seu próprio
DumbMonit, sem app a instalar nem conta de terceiros: o navegador (instalado como
app ou não) recebe os alertas mesmo com todos os separadores fechados.

1. Em cada telemóvel ou computador, abra o DumbMonit e vá a **Definições →
   Notificações push → Ativar neste dispositivo**, depois **Enviar um teste**. No
   iPhone e no iPad, adicione primeiro o DumbMonit ao Ecrã Principal. O guia
   passo a passo está em [Notificações push (PWA)](using/push-notifications.md).
2. Crie aqui um canal **Web Push**. Não tem segredo: os seus destinatários são os
   dispositivos subscritos no passo 1.

- **Definições** — `users` (opcional; uma conta por linha, vazio: todos os
  dispositivos subscritos de todas as contas), `urgency` (`high` por predefinição,
  `normal` ou `low`), `ttl` (segundos durante os quais o serviço push guarda a
  mensagem para um dispositivo desligado, predefinição `86400`), `contact`
  (endereço `mailto:` ou `https:` opcional dado aos serviços push; vazio:
  `DUMBMONIT_PUBLIC_URL` quando é HTTPS, caso contrário a página do projeto)
- **Segredos** — nenhum. O par de chaves VAPID do servidor é gerado no primeiro
  arranque e guardado cifrado; a chave privada nunca sai do servidor.

O título da notificação é a gravidade e o dispositivo (`Critical · nas01`), o corpo
um breve resumo; um toque abre a página de **Alertas**. O DumbMonit precisa de
HTTPS de saída para os serviços push (`fcm.googleapis.com`,
`*.notify.windows.com`, `updates.push.services.mozilla.com`, `*.push.apple.com`);
as mensagens são cifradas ponta a ponta, esses serviços não as podem ler. Uma
subscrição que o serviço push reporta como desaparecida (HTTP 404 ou 410, por
exemplo depois de a app ser removida) é apagada automaticamente.

### Apprise {#apprise}

O Apprise é um **gateway**: com uma única configuração dá acesso a dezenas de
serviços que o DumbMonit não implementa por si. Implemente o contentor
`caronc/apprise`, depois escolha um dos dois modos.

**Modo recomendado — configuração guardada no Apprise.** Crie uma configuração na
interface do Apprise, anote a sua chave, e não ponha mais nada aqui.

- **Definições** — `server_url`, `config_key`

**Modo direto — o DumbMonit fornece os destinos.** Os URLs do Apprise contêm muitas
vezes uma palavra-passe (`mailto://user:password@…`): vão portanto para os
segredos.

- **Definições** — `server_url`
- **Segredos** — `urls` (um ou mais URLs do Apprise, separados por vírgulas)

Definições comuns opcionais: `tag` (para visar apenas parte dos destinos de uma
configuração) e `format` (`text` por predefinição, ou `markdown`).

> Se o teste responder «Apprise has no configuration under this config_key», a chave não
> corresponde a nada. O DumbMonit reporta-o em vez de fingir que a mensagem foi enviada.

### Home Assistant {#home-assistant}

1. No Home Assistant: **o seu perfil → Security → Long-lived access tokens → Create
   token**. Copie-o, não será mostrado de novo.
2. Escolha o serviço a chamar. Por predefinição, o DumbMonit cria uma notificação
   persistente no dashboard.

- **Definições** — `server_url` (por exemplo `http://homeassistant.local:8123`),
  `service` (opcional, predefinição `persistent_notification.create`), `target`
  (opcional), `data` (opcional, objeto fundido na chamada)
- **Segredos** — `token`

Alguns serviços úteis:

| `service` | Efeito |
|---|---|
| `persistent_notification.create` | Faixa na interface do Home Assistant |
| `notify.notify` | Todos os destinos de notificação configurados |
| `notify.mobile_app_<device>` | A app móvel de um dispositivo específico |
| `light.turn_on` | Acende uma luz — use `data` para a entidade e a cor |

A definição `data` é fundida **em último lugar**: pode portanto sobrepor-se ao
título ou à mensagem que o DumbMonit preparou.

### Email (SMTP) {#email-smtp}

O único canal que não depende de nenhum serviço de terceiros.

- **Definições** — `host`, `from`, `to` (lista de endereços), `security`
  (`starttls` por predefinição, ou `tls`, ou `none`), `port` (derivada de
  `security`: 587, 465 ou 25), `username` (opcional)
- **Segredos** — `password`

> Com o Gmail, a palavra-passe da conta não funciona: crie uma **palavra-passe de
> aplicação** nas definições de segurança da sua conta Google.

### Signal {#signal}

Passa por uma instância de
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api) que você
aloja e onde primeiro regista o seu número.

- **Definições** — `server_url` (por exemplo `http://signal.home:8080`), `number`
  (o número remetente registado), `recipients` (lista de números ou
  identificadores de grupos), `username` (opcional)
- **Segredos** — `password` (opcional, se a sua instância está protegida)

Todos os números escrevem-se em **formato internacional**, `+33612345678`.

### SMS via Twilio {#sms-via-twilio}

Na consola do Twilio, a página inicial mostra o seu **Account SID** e o seu **Auth
Token**. Depois compre um número que possa enviar SMS.

- **Definições** — `account_sid`, `from` (o seu número Twilio) **ou**
  `messaging_service_sid`, `to` (lista de destinatários)
- **Segredos** — `token` (o *Auth Token*)

O corpo do SMS é truncado a 300 caracteres: além disso, o Twilio divide-o em
segmentos faturados separadamente.

> Numa conta de teste, o Twilio só envia para números que tenha verificado.

### PagerDuty {#pagerduty}

No PagerDuty: **Services → o seu serviço → Integrations → Add integration**, e
escolha **Events API v2**. Copie a *Integration Key*.

- **Definições** — `region` (`eu` se a sua conta está alojada na Europa), `source`
  (opcional, predefinição `dumbmonit`)
- **Segredos** — `token` (a *Integration Key*)

Um alerta abre um incidente; a sua resolução **fecha-o automaticamente**. A
correspondência faz-se pela impressão digital do alerta, prefixada com
`dumbmonit-`, pelo que um lembrete nunca abre um segundo incidente.

> Uma chave de outro tipo («Events API v1», «REST API») recebe uma recusa seca.
> Verifique que a integração é de facto do tipo *Events API v2*.

### Opsgenie {#opsgenie}

**Teams → a sua equipa → Integrations → Add integration → API**. Copie a chave de
API.

- **Definições** — `region` (`eu` para uma conta europeia), `responders` (lista de
  nomes de equipas, opcional), `tags` (lista, opcional)
- **Segredos** — `api_key`

Tal como no PagerDuty, uma resolução fecha o alerta, identificado pelo seu alias.

> A Atlassian fixou o **fim de serviço do Opsgenie a 5 de abril de 2027**. Este
> canal mantém-se para instalações existentes; para uma nova, prefira o PagerDuty
> ou um canal personalizado a apontar para o Jira Service Management.

### Webhook personalizado {#custom-webhook}

Veja [O canal personalizado](#the-custom-channel) acima.

---

## Resolução de problemas {#troubleshooting}

**O teste falha imediatamente, com «missing setting …».**
Um campo obrigatório está vazio. A mensagem nomeia a definição exata.

**O teste falha com «no answer within 15 s».**
O servidor de destino é inacessível a partir do DumbMonit. Se é um serviço na sua
rede local, verifique que o contentor do DumbMonit o consegue alcançar — é
frequentemente um problema de rede do Docker, não de configuração do canal.

**O teste falha com um erro de certificado.**
O seu servidor usa um certificado autoassinado ou de uma CA privada. Veja
[Certificados autoassinados](#self-signed-certificates).

**O teste tem êxito, mas não recebo nada.**
O serviço aceitou a mensagem e enviou-a para outro lado: canal errado, tópico ntfy
errado, dispositivo Pushbullet errado. Verifique a definição de destino.

**A mensagem de erro contém `***`.**
É intencional: o seu token foi removido da mensagem antes de ser registada.
Nenhum segredo aparece alguma vez nos registos, na coluna «último erro» ou numa
resposta da API.

**Onde posso ver o último erro de um canal?**
Na lista de canais: cada linha mostra a data da última entrega bem-sucedida e, se
existir, o erro da última falha.
