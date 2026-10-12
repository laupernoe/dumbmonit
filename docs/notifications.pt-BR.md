# Notificações do DumbMonit

Este guia explica como receber alertas do DumbMonit no serviço de sua
escolha. Não exige habilidades de programação: cada seção diz onde
encontrar o endereço ou token para copiar, e o que digitar no
formulário.

---

## Conteúdo

- [Como funciona](#how-it-works)
- [Criando e testando um canal](#creating-and-testing-a-channel)
- [Configurações ou segredos: a diferença](#settings-or-secrets-the-difference)
- [O canal personalizado](#the-custom-channel)
  - [Variáveis disponíveis](#available-variables)
  - [Exemplo 1 — uma API JSON interna](#example-1--an-internal-json-api)
  - [Exemplo 2 — um serviço que espera um formulário](#example-2--a-service-that-expects-a-form)
  - [Exemplo 3 — um SMS em texto simples via uma operadora](#example-3--a-plain-text-sms-through-a-carrier)
  - [Exemplo 4 — uma requisição GET sem corpo](#example-4--a-get-request-without-a-body)
  - [Erros comuns de template](#common-template-mistakes)
- [Canais suportados](#supported-channels)
- [Solução de problemas](#troubleshooting)

---

## Como funciona

Um **canal** é um destino: um canal do Discord, um número de telefone,
uma caixa de e-mail. Você cria quantos quiser, e suas regras de alerta
escolhem quais notificar.

Três princípios se aplicam a todo canal:

1. **Alertas são agrupados.** Uma mensagem por dispositivo e por ciclo,
   mesmo que cinco discos se enchem ao mesmo tempo. Sem isso, você
   silenciaria notificações dentro de uma semana.
2. **Um canal quebrado nunca bloqueia outro.** A falha é registrada na
   coluna "último erro" do canal, e a entrega continua para os outros.
3. **Um serviço que não responde é abandonado depois de 15 segundos.**
   Uma notificação que chega um minuto atrasada não é mais útil, e o
   loop de monitoramento não deve esperar detrás de um servidor
   inacessível.

---

## Criando e testando um canal

1. Abra **Alertas → Notificações → Adicionar canal**
   (`/alerts#notifications`).
2. Dê a ele um nome significativo ("Discord — família", "SMS —
   plantão"). Este é o nome que você escolherá depois em suas regras de
   alerta.
3. Escolha o tipo de canal, depois preencha os campos descritos abaixo.
4. Salve, depois clique em **Enviar teste**.

A mensagem de teste segue **exatamente o mesmo caminho** que um alerta
real: mesma requisição, mesmo token, mesma formatação. Se chegar, o
canal funciona. Se falhar, a mensagem de erro nomeia a configuração a
corrigir.

> **Um token nunca é mostrado de novo.** Uma vez salvo, é criptografado
> no banco de dados e a interface só diz "configurado". Para editar um
> canal sem digitar o token de novo, simplesmente deixe o campo vazio.

---

## Configurações ou segredos: a diferença

Todo canal tem dois grupos de campos.

| | **Configurações** | **Segredos** |
|---|---|---|
| Conteúdo | endereço do servidor, nome do canal, tópico | tokens, senhas, URLs de webhook |
| Armazenamento | texto simples | criptografado |
| Lido de volta pela interface | sim | nunca |

A interface **recusa** salvar um campo sensível entre as configurações,
e avisa isso. Isso não é uma formalidade: configurações são retornadas
como estão pela API, então um token armazenado lá seria publicado toda
vez que a página é aberta.

As tabelas na seção [Canais suportados](#supported-channels) dizem,
para cada canal, o que vai onde.

---

## O canal personalizado

O canal **`webhook`** é a válvula de escape do DumbMonit. Você descreve
toda a requisição HTTP — método, endereço, cabeçalhos, tipo de conteúdo
e corpo —, o que permite conectar quase qualquer serviço, incluindo uma
API interna que o DumbMonit nunca vai conhecer.

**Sem nenhuma configuração particular**, ele já envia um payload JSON
estruturado que um script pode usar:

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

### Configurações do canal personalizado

| Configuração | Obrigatória | Padrão | Função |
|---|---|---|---|
| `url` | sim (aqui ou como um segredo) | — | Endereço a chamar. Pode conter variáveis. |
| `method` | não | `POST` | `POST`, `PUT` ou `GET`. |
| `content_type` | não | `json` | `json`, `form` ou `text`. Também determina como variáveis são escapadas. |
| `body_template` | não | — | Template do corpo. Quando ausente, o payload acima é enviado. |
| `headers` | não | — | Cabeçalhos extras, como um objeto `{"X-Key": "value"}`. |
| `base_url` | não | — | Endereço público do seu DumbMonit, para que `{{link}}` dê um link clicável. |
| `username` | não | — | Nome de usuário para autenticação HTTP "Basic". Requer o segredo `password`. |

| Segredo | Função |
|---|---|
| `url` | Use-o em vez da configuração `url` se o endereço já contém um token. |
| `token` | Enviado como um cabeçalho `Authorization: Bearer …`, a menos que seu template use `{{token}}`. |
| `password` | Senha para autenticação "Basic". |

### Variáveis disponíveis

Uma variável é escrita entre **chaves duplas**: `{{title}}`. Espaços em
torno do nome são tolerados (`{{ title }}`). Chaves simples em um corpo
JSON permanecem literais, então você pode escrever seu JSON
normalmente.

| Variável | O que contém | Exemplo |
|---|---|---|
| `{{title}}` | Título completo da mensagem | `nas — Disk full` |
| `{{message}}` | Corpo em texto simples, uma linha por alerta | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | O mesmo corpo em Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Nome da regra de alerta | `Disk full` |
| `{{target}}` | Nome do dispositivo monitorado | `nas` |
| `{{target_id}}` | Identificador numérico do dispositivo | `42` |
| `{{severity}}` | Severidade técnica | `info`, `warning` ou `critical` |
| `{{severity_fr}}` | Severidade como palavra legível | `information`, `avertissement`, `critique` |
| `{{status}}` | Status técnico | `firing` ou `resolved` |
| `{{status_fr}}` | Status como palavra legível | `en cours` ou `résolu` |
| `{{value}}` | Valor medido, unidade incluída | `95 %` |
| `{{value_raw}}` | Valor bruto, usável como **número** JSON | `95` |
| `{{threshold}}` | Limite cruzado, unidade incluída | `90 %` |
| `{{threshold_raw}}` | Limite bruto, usável como **número** JSON | `90` |
| `{{unit}}` | Unidade do valor | `%` |
| `{{operator}}` | Comparação de limite | `>` |
| `{{count}}` | Número de alertas agrupados nesta mensagem | `3` |
| `{{fingerprint}}` | Identificador estável do alerta | `nas/disk-full` |
| `{{timestamp}}` | Timestamp ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Timestamp em segundos desde 1970 | `1788358325` |
| `{{date}}` | Timestamp legível, em UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Prioridade de 1 (baixa) a 5 (alta) | `4` |
| `{{color}}` | Cor de destaque, sem o cerquilha | `D98A00` |
| `{{color_hex}}` | Cor de destaque, com o cerquilha | `#D98A00` |
| `{{emoji}}` | Pictograma de status | `🔴`, `⚠️`, `ℹ️` ou `✅` |
| `{{link}}` | Link para o dispositivo no DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Sempre `dumbmonit` | `dumbmonit` |
| `{{token}}` | O segredo `token` do canal | *(seu token)* |

Duas observações úteis:

- **Algumas variáveis podem ser vazias.** Um alerta de disponibilidade
  ("host inacessível") não tem nem valor nem limite: `{{value}}` e
  `{{value_raw}}` então renderizam uma string vazia, nunca `null` ou
  `NaN`. Se você as coloca como um número JSON, trate esse caso — ou
  use `"{{value}}"` entre aspas.
- **`{{link}}` permanece vazio** até você preencher `base_url`. Um link
  errado desperdiçaria mais tempo que um alerta sem um.

**Você nunca precisa escapar nada você mesmo.** Dependendo do
`content_type` escolhido, o DumbMonit automaticamente protege valores
substituídos: aspas e quebras de linha escapadas em JSON, codificação
percentual em um formulário, nenhuma transformação em texto simples.
Uma regra chamada `"System" disk full` não vai quebrar seu template
JSON.

### Exemplo 1 — uma API JSON interna

Uma aplicação caseira expõe `POST /incidents` e espera um token em um
cabeçalho personalizado.

**Configurações**

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

**Segredos**

```json
{ "token": "your-application-token" }
```

O token é enviado automaticamente como um cabeçalho
`Authorization: Bearer …`. Aqui está o que a aplicação recebe:

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

### Exemplo 2 — um serviço que espera um formulário

Muitos gateways SMS e APIs mais antigas esperam
`application/x-www-form-urlencoded` em vez de JSON, com o token no
corpo.

**Configurações**

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

Já que o template já usa `{{token}}`, o DumbMonit **não adiciona** um
cabeçalho `Authorization`: alguns serviços rejeitam uma requisição que
carrega duas autenticações. Valores são codificados automaticamente,
espaços se tornando `+` e caracteres acentuados seu equivalente
codificado em percentual.

### Exemplo 3 — um SMS em texto simples via uma operadora

Alguns serviços, como a API de SMS de uma operadora ou uma automação
caseira, querem uma única linha de texto.

**Configurações**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

A mensagem enviada, sem escapar nada:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Exemplo 4 — uma requisição GET sem corpo

Quando o serviço só aceita parâmetros de URL, coloque as variáveis em
`url` e escolha o método `GET`. Uma requisição `GET` não tem corpo:
`body_template` então é recusado, para que ninguém pense que faz algo.

**Configurações**

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

Só os **valores substituídos** são codificados; o resto do endereço é
copiado como está. A requisição realmente enviada se parece com:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Erros comuns de template

Um template defeituoso é **recusado ao salvar**, enquanto você está
olhando o formulário, não na noite em que o incidente acontece.

| O que você escreve | O que acontece |
|---|---|
| `{{severite}}` | Recusado: variável desconhecida "severite", seguida da lista de nomes válidos. O nome correto é `severity`. |
| `{"t": "{{title"}` | Recusado: "chaves não fechadas". O `}}` está faltando. |
| `{{}}` ou `{{  }}` | Recusado: uma variável sem nome não significa nada. |
| `{"a": {"b": 1}}` | **Aceito.** Chaves simples são literais: escreva seu JSON normalmente. |
| `{{token}}` sem um segredo `token` | Recusado: o template pede um token que o canal não tem. |
| `method: GET` com `body_template` | Recusado, com a alternativa: coloque as variáveis em `url`. |
| `method: DELETE` | Recusado: só `POST`, `PUT` e `GET` são aceitos. |

---

## Canais suportados

### Discord

No Discord: **Configurações do canal → Integrações → Webhooks → Novo
webhook**, depois **Copiar URL do webhook**. Essa URL contém o token:
ela vai nos segredos.

- **Segredos** — `webhook_url`

### Slack

Em <https://api.slack.com/apps>, crie um app, ative **Incoming
Webhooks**, depois **Add New Webhook to Workspace** e escolha o canal.
Copie a URL `https://hooks.slack.com/services/…`.

- **Segredos** — `webhook_url`

### Telegram

Você precisa de duas coisas: o **token do bot** (segredo) e o **chat
ID** (`chat_id`) onde ele precisa escrever. O primeiro leva um minuto
para obter; o segundo depende de onde você quer receber os alertas.

#### 1. Criar o bot e obter seu token

1. No Telegram, procure por **@BotFather** e clique em *Start*.
2. Envie `/newbot`, depois responda às perguntas (um nome, depois um
   nome de usuário que precisa terminar com `bot`).
3. O BotFather termina com uma mensagem como esta:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   A linha `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` é o **token do bot**.
   Não compartilhe: ele dá controle total do bot.

Nos exemplos abaixo, substitua `<TOKEN>` por este token. Note o prefixo
`bot` colado na frente dele em URLs:
`https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Encontrar o `chat_id`

**Chat privado** (o bot escreve direto para você)

1. Abra seu bot (`t.me/my_homelab_bot`) e clique em *Start*, ou envie
   qualquer mensagem a ele. Sem isso, ele nunca conseguirá escrever
   para você.
2. Abra em um navegador:
   `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Você recebe um documento JSON; procure por `"chat"`:

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
2. Poste uma mensagem no canal.
3. Abra `https://api.telegram.org/bot<TOKEN>/getUpdates`: a resposta
   contém um bloco `channel_post` com
   `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   O `chat_id` é esse número negativo, `-100` incluído:
   `-1001xxxxxx062`.

**Grupo**

A forma mais fácil passa pelo app desktop:

1. Adicione o bot ao grupo.
2. Envie uma mensagem no grupo, clique com o botão direito nela →
   *Copy message link*. Você recebe `https://t.me/c/194xxxx987/13` (ou
   `https://t.me/c/194xxxx987/11/13` se o grupo tem tópicos).
3. O número logo depois de `/c/` é o identificador interno do grupo:
   `194xxxx987`. O `chat_id` a informar é esse número **prefixado com
   `-100`**: `-100194xxxx987`.

Sem o app desktop, o método `getUpdates` acima também funciona:
`message.chat.id` é diretamente o número certo (já negativo). Se o bot
não vê as mensagens do grupo, peça ao BotFather `/setprivacy` →
*Disable*, depois remova o bot do grupo e adicione-o de novo.

**Tópico de grupo** (grupos em modo "Topics")

O `chat_id` é o do grupo. Além disso, preencha **Tópico de grupo**
(`message_thread_id`) com o número do meio do link copiado: em
`https://t.me/c/194xxxx987/11/13`, o tópico é `11`. Sem essa
configuração, a mensagem cai no tópico *General*.

#### 3. Verificar antes de salvar

Abra no navegador:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(adicione `&message_thread_id=11` para um tópico). Se `test123`
chegar, o token e o `chat_id` estão certos: informe-os no DumbMonit e
clique em *Test*.

- **Configurações** — `chat_id` (obrigatório), `message_thread_id`
  (tópico, opcional), `api_base` (padrão
  `https://api.telegram.org`, mude apenas para passar por um relay)
- **Segredos** — `bot_token`

> **"Telegram cannot find this chat"** — para um chat privado, o bot
> nunca recebeu uma mensagem sua: envie-lhe `/start`. Para um grupo ou
> canal, o prefixo `-100` provavelmente está faltando, ou o bot não foi
> adicionado a ele.
>
> **"thread not found"** — o `message_thread_id` não corresponde a
> nenhum tópico do grupo, ou o grupo não está em modo "Topics".

### Microsoft Teams

**Aviso: o método mudou.** Os antigos "Office 365 connectors" foram
descontinuados em maio de 2026; uma URL `outlook.office.com/webhook/…`
não funciona mais.

O procedimento atual:

1. No canal do Teams, **⋯ → Workflows**.
2. Escolha o template **"Post to a channel when a webhook request is
   received"**.
3. Confirme os passos, depois **copie a URL HTTPS POST** mostrada no
   final.

- **Segredos** — `webhook_url`

O DumbMonit envia um *Adaptive Card* empacotado, o único formato que
esses fluxos exibem corretamente. Se você recebe um erro mencionando
Power Automate, a URL provavelmente vem de um conector antigo.

### Matrix

1. Crie uma conta dedicada ao DumbMonit no seu homeserver, e **a
   convide para a sala**. Uma conta que não é membro não pode
   escrever.
2. Obtenha seu token de acesso. No Element: **Settings → Help & About
   → Advanced → Access token**.
3. Obtenha o identificador **interno** da sala — não seu nome de
   exibição. No Element: **Room settings → Advanced → Internal room
   ID**. Começa com `!` (por exemplo `!aBcDeF:matrix.org`).

- **Configurações** — `server_url` (por exemplo `https://matrix.org`),
  `room_id`
- **Segredos** — `token`

### Mattermost

**Integrations → Incoming Webhooks → Add**. Escolha o canal padrão,
depois copie a URL.

- **Configurações** — `channel` (opcional, para escrever em outro
  lugar além do canal padrão), `username` (opcional, padrão
  `DumbMonit`)
- **Segredos** — `webhook_url`

### Rocket.Chat

**Administration → Integrations → New integration → Incoming
webhook**. Escolha a sala, ative a integração, depois copie a *Webhook
URL*.

- **Configurações** — `channel` (opcional), `alias` (opcional, padrão
  `DumbMonit`)
- **Segredos** — `webhook_url`

### Google Chat

No espaço do Google Chat: **nome do espaço → Apps & integrations →
Webhooks → Add webhook**. Copie a URL, que já contém a chave e o
token.

- **Segredos** — `webhook_url`

### Zulip

1. Crie um **bot**: **Personal settings → Bots → Add a new bot**,
   tipo "Generic". Anote seu endereço de e-mail e sua chave de API.
2. Inscreva o bot no stream alvo, senão o envio será recusado.

- **Configurações** — `server_url` (por exemplo
  `https://your-org.zulipchat.com`), `email` (o endereço do bot),
  `stream` (o nome do stream), `topic` (opcional, padrão `DumbMonit`)
- **Segredos** — `api_key`

### ntfy

O canal mais simples de todos: escolha um nome de tópico difícil de
adivinhar, depois se inscreva nele a partir do app ntfy.

- **Configurações** — `topic`, `server_url` (opcional, padrão
  `https://ntfy.sh`)
- **Segredos** — `token` (apenas se seu servidor exigir autenticação)

> Na instância pública, **qualquer um que conheça o nome de um tópico
> pode lê-lo**. Escolha um nome longo e aleatório, ou hospede seu
> próprio servidor.

### Gotify

Na interface do Gotify: **Apps → Create Application**. Copie o token
mostrado.

- **Configurações** — `server_url` (por exemplo `https://gotify.home`)
- **Segredos** — `token`

### Pushover

1. Em <https://pushover.net>, sua **User Key** é mostrada direto na
   página inicial.
2. Depois crie uma aplicação: **Create an Application/API Token**.
   Você recebe um **token de aplicação**.

- **Configurações** — `priority` (opcional, de -2 a 2), `sound`
  (opcional), `retry` e `expire` (apenas para `priority: 2`)
- **Segredos** — `token` (token de aplicação), `user_key` (chave de
  usuário)

Sem uma configuração `priority`, o DumbMonit escolhe de acordo com a
severidade: `-1` para um aviso ou resolução, `0` para um alerta, `1`
para um alerta crítico. Um aviso portanto não faz o celular tocar.

> Prioridade `2` ("emergency") repete a notificação até ser
> reconhecida. O DumbMonit então adiciona `retry` e `expire`
> automaticamente, sem os quais o Pushover rejeitaria a mensagem.

### Pushbullet

Em <https://www.pushbullet.com/#settings/account>, clique em **Create
Access Token**.

- **Configurações** — `device_iden` (opcional; sem ele, todos os seus
  dispositivos são notificados)
- **Segredos** — `token`

### Bark (iOS)

Abra o app Bark no seu iPhone: ele mostra uma URL contendo sua **chave
de dispositivo**, a string longa de caracteres depois do nome de
domínio.

- **Configurações** — `server_url` (opcional, padrão
  `https://api.day.app`), `group` (opcional), `level` (opcional),
  `sound` (opcional)
- **Segredos** — `token` (a chave de dispositivo)

Sem uma configuração `level`, o DumbMonit escolhe: `passive` para um
aviso, `active` para um alerta, `timeSensitive` para um alerta
crítico. O nível `critical`, que atravessa "Não Perturbe", nunca é
escolhido automaticamente — defina-o explicitamente se quiser.

### Web Push

Notificações nativas em seus celulares e computadores, direto do seu
próprio DumbMonit, sem app para instalar e sem conta de terceiros: o
navegador (instalado como app ou não) recebe os alertas mesmo com toda
aba fechada.

1. Em cada celular ou computador, abra o DumbMonit e vá para
   **Configurações → Notificações push → Ativar neste dispositivo**,
   depois **Enviar um teste**. No iPhone e iPad, primeiro adicione o
   DumbMonit à Tela de Início. O guia passo a passo está em
   [Notificações push (PWA)](using/push-notifications.md).
2. Crie um canal **Web Push** aqui. Não tem segredo: seus
   destinatários são os dispositivos inscritos no passo 1.

- **Configurações** — `users` (opcional; uma conta por linha, vazio:
  todo dispositivo inscrito de toda conta), `urgency` (`high` por
  padrão, `normal` ou `low`), `ttl` (segundos que o serviço de push
  mantém a mensagem para um dispositivo desligado, padrão `86400`),
  `contact` (endereço `mailto:` ou `https:` opcional dado aos serviços
  de push; vazio: `DUMBMONIT_PUBLIC_URL` quando é HTTPS, senão a
  página do projeto)
- **Segredos** — nenhum. O par de chaves VAPID do servidor é gerado na
  primeira inicialização e armazenado criptografado; a chave privada
  nunca deixa o servidor.

O título da notificação é a severidade e o dispositivo (`Critical ·
nas01`), o corpo um resumo curto; um toque abre a página **Alertas**. O
DumbMonit precisa de HTTPS de saída para os serviços de push
(`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`); mensagens são
criptografadas de ponta a ponta, esses serviços não podem lê-las. Uma
inscrição que o serviço de push reporta como desaparecida (HTTP 404 ou
410, por exemplo depois que o app foi removido) é excluída
automaticamente.

### Apprise

O Apprise é um **gateway**: com uma única configuração ele dá acesso a
dezenas de serviços que o DumbMonit não implementa ele mesmo. Implante
o contêiner `caronc/apprise`, depois escolha um dos dois modos.

**Modo recomendado — configuração armazenada no Apprise.** Crie uma
configuração na interface do Apprise, anote sua chave, e não coloque
mais nada aqui.

- **Configurações** — `server_url`, `config_key`

**Modo direto — o DumbMonit fornece os destinos.** URLs do Apprise
frequentemente contêm uma senha (`mailto://user:password@…`): elas
portanto vão nos segredos.

- **Configurações** — `server_url`
- **Segredos** — `urls` (uma ou mais URLs do Apprise, separadas por
  vírgulas)

Configurações comuns opcionais: `tag` (para alvejar só parte dos
destinos de uma configuração) e `format` (`text` por padrão, ou
`markdown`).

> Se o teste responder "Apprise has no configuration under this
> config_key", a chave não corresponde a nada. O DumbMonit o reporta em
> vez de fingir que a mensagem foi enviada.

### Home Assistant

1. No Home Assistant: **seu perfil → Security → Long-lived access
   tokens → Create token**. Copie-o, não será mostrado de novo.
2. Escolha o serviço a chamar. Por padrão, o DumbMonit cria uma
   notificação persistente no painel.

- **Configurações** — `server_url` (por exemplo
  `http://homeassistant.local:8123`), `service` (opcional, padrão
  `persistent_notification.create`), `target` (opcional), `data`
  (opcional, objeto mesclado na chamada)
- **Segredos** — `token`

Alguns serviços úteis:

| `service` | Efeito |
|---|---|
| `persistent_notification.create` | Banner na interface do Home Assistant |
| `notify.notify` | Todos os destinos de notificação configurados |
| `notify.mobile_app_<device>` | O aplicativo móvel de um dispositivo específico |
| `light.turn_on` | Liga uma luz — use `data` para a entidade e a cor |

A configuração `data` é mesclada **por último**: pode portanto
sobrescrever o título ou a mensagem que o DumbMonit preparou.

### E-mail (SMTP)

O único canal que não depende de nenhum serviço de terceiros.

- **Configurações** — `host`, `from`, `to` (lista de endereços),
  `security` (`starttls` por padrão, ou `tls`, ou `none`), `port`
  (derivada de `security`: 587, 465 ou 25), `username` (opcional)
- **Segredos** — `password`

> Com o Gmail, uma senha de conta não funciona: crie uma **senha de
> app** nas configurações de segurança da sua conta Google.

### Signal

Passa por uma instância
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api)
que você hospeda e onde você primeiro registra seu número.

- **Configurações** — `server_url` (por exemplo
  `http://signal.home:8080`), `number` (o número remetente
  registrado), `recipients` (lista de números ou identificadores de
  grupo), `username` (opcional)
- **Segredos** — `password` (opcional, se sua instância é protegida)

Todos os números são escritos em **formato internacional**,
`+33612345678`.

### SMS via Twilio

No painel do Twilio, a página inicial mostra seu **Account SID** e seu
**Auth Token**. Depois compre um número que possa enviar SMS.

- **Configurações** — `account_sid`, `from` (seu número Twilio) **ou**
  `messaging_service_sid`, `to` (lista de destinatários)
- **Segredos** — `token` (o *Auth Token*)

O corpo do SMS é truncado em 300 caracteres: além disso, o Twilio o
divide em segmentos cobrados separadamente.

> Em uma conta trial, o Twilio só envia para números que você
> verificou.

### PagerDuty

No PagerDuty: **Services → seu serviço → Integrations → Add
integration**, e escolha **Events API v2**. Copie a *Integration Key*.

- **Configurações** — `region` (`eu` se sua conta está hospedada na
  Europa), `source` (opcional, padrão `dumbmonit`)
- **Segredos** — `token` (a *Integration Key*)

Um alerta abre um incidente; sua resolução **o fecha
automaticamente**. A correspondência é feita na impressão digital do
alerta, prefixada com `dumbmonit-`, então um lembrete nunca abre um
segundo incidente.

> Uma chave de outro tipo ("Events API v1", "REST API") recebe uma
> recusa seca. Verifique que a integração é realmente do tipo *Events
> API v2*.

### Opsgenie

**Teams → seu time → Integrations → Add integration → API**. Copie a
chave de API.

- **Configurações** — `region` (`eu` para uma conta europeia),
  `responders` (lista de nomes de time, opcional), `tags` (lista,
  opcional)
- **Segredos** — `api_key`

Como com o PagerDuty, uma resolução fecha o alerta, identificado por
seu alias.

> A Atlassian definiu o **fim do serviço do Opsgenie para 5 de abril de
> 2027**. Este canal permanece para instalações existentes; para uma
> nova, prefira o PagerDuty ou um canal personalizado apontando para o
> Jira Service Management.

### Webhook personalizado

Veja [O canal personalizado](#the-custom-channel) acima.

---

## Solução de problemas

**O teste falha imediatamente, com "missing setting …".**
Um campo obrigatório está vazio. A mensagem nomeia a configuração
exata.

**O teste falha com "no answer within 15 s".**
O servidor alvo é inacessível a partir do DumbMonit. Se é um serviço
na sua rede local, verifique que o contêiner DumbMonit pode alcançá-lo
— isso frequentemente é um problema de rede Docker, não um problema de
configuração de canal.

**O teste tem sucesso, mas não recebo nada.**
O serviço aceitou a mensagem e a enviou para outro lugar: canal
errado, tópico ntfy errado, dispositivo Pushbullet errado. Verifique a
configuração de destino.

**A mensagem de erro contém `***`.**
Isso é intencional: seu token foi removido da mensagem antes de ser
registrado. Nenhum segredo jamais aparece nos logs, na coluna "último
erro", ou em uma resposta de API.

**Onde posso ver o último erro de um canal?**
Na lista de canais: cada linha mostra a data da última entrega bem
sucedida e, se houver, o erro da última falha.
