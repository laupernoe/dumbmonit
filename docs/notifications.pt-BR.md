# Notificações DumbMonit {#dumbmonit-notifications}

Este guia explica como receber alertas do DumbMonit no serviço de sua escolha.
Não requer habilidades de programação: cada seção diz onde encontrar o endereço ou
token para copiar, e o que digitar no formulário.

---

## Conteúdo {#contents}

- [Como funciona](#how-it-works)
- [Criando e testando um canal](#creating-and-testing-a-channel)
- [Configurações ou segredos: a diferença](#settings-or-secrets-the-difference)
- [Certificados auto-assinados](#self-signed-certificates)
- [O canal personalizado](#the-custom-channel)
  - [Variáveis disponíveis](#available-variables)
  - [Exemplo 1 — uma API JSON interna](#example-1--an-internal-json-api)
  - [Exemplo 2 — um serviço que espera um formulário](#example-2--a-service-that-expects-a-form)
  - [Exemplo 3 — SMS simples através de uma operadora](#example-3--a-plain-text-sms-through-a-carrier)
  - [Exemplo 4 — uma solicitação GET sem corpo](#example-4--a-get-request-without-a-body)
  - [Erros comuns de modelo](#common-template-mistakes)
- [Canais suportados](#supported-channels)
- [Solução de problemas](#troubleshooting)

---

## Como funciona {#how-it-works}

Um **canal** é um destino: um canal Discord, um número de telefone, uma caixa de correio. Você
cria quantos quiser, e suas regras de alerta escolhem quais notificar.

Três princípios aplicam-se a cada canal:

1. **Os alertas são agrupados.** Uma mensagem por dispositivo e por ciclo, mesmo que cinco discos
   se encham ao mesmo tempo. Sem isso, você silenciaria notificações em uma
   semana.
2. **Um canal quebrado nunca bloqueia outro.** A falha é registrada na
   coluna "último erro" do canal, e a entrega continua para os outros.
3. **Um serviço que não responde é abandonado após 15 segundos.** Uma notificação
   que chega um minuto depois não é mais útil, e o loop de monitoramento não deve
   esperar atrás de um servidor inacessível.

---

## Criando e testando um canal {#creating-and-testing-a-channel}

1. Abra **Alertas → Notificações → Adicionar canal** (`/alerts#notifications`).
2. Dê um nome significativo ("Discord — família", "SMS — de plantão"). Este é o
   nome que você escolherá depois em suas regras de alerta.
3. Escolha o tipo de canal, depois preencha os campos descritos abaixo.
4. Salve, depois clique em **Enviar teste**.

A mensagem de teste segue **exatamente o mesmo caminho** de um alerta real: mesma solicitação,
mesmo token, mesma formatação. Se chegar, o canal funciona. Se falhar, a
mensagem de erro nomeará a configuração a corrigir.

> **Um token nunca é mostrado novamente.** Uma vez salvo, ele é criptografado no banco de dados e
> a interface apenas diz "configurado". Para editar um canal sem redigitar seu
> token, simplesmente deixe o campo vazio.

---

## Configurações ou segredos: a diferença {#settings-or-secrets-the-difference}

Cada canal tem dois grupos de campos.

| | **Configurações** | **Segredos** |
|---|---|---|
| Conteúdo | endereço do servidor, nome do canal, tópico | tokens, senhas, URLs de webhook |
| Armazenamento | texto simples | criptografado |
| Relido pela interface | sim | nunca |

A interface **recusa** salvar um campo sensível nas configurações, e o diz
a você. Não é uma formalidade: as configurações são retornadas como estão pela API, então um token
armazenado lá seria publicado toda vez que a página abrisse.

As tabelas na seção [Canais suportados](#supported-channels) dizem, para cada
canal, o que vai aonde.

---

## Certificados auto-assinados {#self-signed-certificates}

Por padrão, o DumbMonit recusa qualquer servidor cuja certificação não possa verificar. Para um
serviço auto-hospedado servido com um certificado auto-assinado ou de uma autoridade privada, os canais para Matrix, Mattermost, Rocket.Chat, Zulip, ntfy, Gotify,
Bark, Apprise, Home Assistant, Signal e o webhook personalizado têm uma configuração **Aceitar um
certificado não verificável** (`insecure_tls`, um booleano, desativado por padrão).

Ativá-lo mantém a conexão criptografada mas para de verificar a identidade do
servidor, então use-o apenas para um servidor que você controla, em uma rede que você confia. Quando um
certificado é o motivo de uma falha, o erro mostrado para o canal o nomeia
(por exemplo, "invalid peer certificate: UnknownIssuer").

---

## O canal personalizado {#the-custom-channel}

O canal **`webhook`** é a válvula de escape do DumbMonit. Você descreve toda a solicitação HTTP — método, endereço, cabeçalhos, tipo de conteúdo e corpo —, o que permite conectar
quase qualquer serviço, incluindo uma API interna que o DumbMonit nunca conhecerá.

**Sem nenhuma configuração particular**, ele já envia uma carga JSON
estruturada que um script pode usar:

```json
{
  "source": "dumbmonit",
  "target": "nas",
  "rule": "Disco cheio",
  "severity": "warning",
  "status": "firing",
  "title": "nas — Disco cheio",
  "text": "⚠️ Aviso · Disco cheio — /data — 95 % (limite > 90 %)",
  "value": 95.0,
  "threshold": 90.0,
  "unit": "%",
  "fingerprint": "nas/disk-full",
  "count": 1,
  "at": "2026-09-01T14:12:05+00:00",
  "link": ""
}
```

### Configurações de canal personalizado {#custom-channel-settings}

| Configuração | Obrigatória | Padrão | Papel |
|---|---|---|---|
| `url` | sim (aqui ou como um segredo) | — | Endereço para chamar. Pode conter variáveis. |
| `method` | não | `POST` | `POST`, `PUT` ou `GET`. |
| `content_type` | não | `json` | `json`, `form` ou `text`. Também determina como as variáveis são escapadas. |
| `body_template` | não | — | Modelo de corpo. Quando ausente, a carga acima é enviada. |
| `base_url` | não | — | Endereço público do seu DumbMonit, para que `{{link}}` dê um link clicável. |
| `username` | não | — | Nome de usuário para autenticação HTTP "Básica". Requer o segredo `password`. |

| Segredo | Papel |
|---|---|
| `url` | Use em vez da configuração `url` se o endereço já contiver um token. |
| `token` | Enviado como um cabeçalho `Authorization: Bearer …`, a menos que seu modelo use `{{token}}`. |
| `password` | Senha para autenticação "Básica". |
| `headers` | Cabeçalhos extras, como um objeto JSON `{"X-Api-Key": "value"}`. Armazenado criptografado como o token e nunca mostrado novamente, então chaves de API e cabeçalhos `Authorization` pertencem aqui. |

`headers` é um segredo. Canais salvos por 0.1.0-alpha.6 ou anterior, que os mantiveram nas
configurações, foram movidos para os segredos criptografados no primeiro início,
e a API não os retorna mais. Como todo segredo, eles são mantidos quando você edita o
canal sem digitá-los novamente; enviar `secrets` substitui todo o conjunto.

O DumbMonit segue um redirecionamento apenas para o mesmo host (ou de `http` para `https` naquele
host): os cabeçalhos e token de um canal nunca vão a lugar nenhum além do endereço que você inseriu.

### Variáveis disponíveis {#available-variables}

Uma variável é escrita entre **chaves duplas**: `{{title}}`. Espaços ao redor do nome
são tolerados (`{{ title }}`). Chaves únicas em um corpo JSON ficam literais, então você pode
escrever seu JSON normalmente.

| Variável | O que contém | Exemplo |
|---|---|---|
| `{{title}}` | Título completo da mensagem | `nas — Disco cheio` |
| `{{message}}` | Corpo de texto simples, uma linha por alerta | `⚠️ Aviso · Disco cheio — /data — 95 % (limite > 90 %)` |
| `{{message_markdown}}` | O mesmo corpo em Markdown | `**nas — Disco cheio**\n• ⚠️ …` |
| `{{rule}}` | Nome da regra de alerta | `Disco cheio` |
| `{{target}}` | Nome do dispositivo monitorado | `nas` |
| `{{target_id}}` | Identificador numérico do dispositivo | `42` |
| `{{severity}}` | Severidade técnica | `info`, `warning` ou `critical` |
| `{{severity_fr}}` | Severidade como uma palavra legível | `informação`, `aviso`, `crítico` |
| `{{status}}` | Status técnico | `firing` ou `resolved` |
| `{{status_fr}}` | Status como uma palavra legível | `em andamento` ou `resolvido` |
| `{{value}}` | Valor medido, unidade incluída | `95 %` |
| `{{value_raw}}` | Valor simples, usável como um **número** JSON | `95` |
| `{{threshold}}` | Limite cruzado, unidade incluída | `90 %` |
| `{{threshold_raw}}` | Limite simples, usável como um **número** JSON | `90` |
| `{{unit}}` | Unidade do valor | `%` |
| `{{operator}}` | Comparação de limite | `>` |
| `{{count}}` | Número de alertas agrupados nesta mensagem | `3` |
| `{{fingerprint}}` | Identificador estável do alerta | `nas/disk-full` |
| `{{timestamp}}` | Carimbo de data/hora ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Carimbo de data/hora em segundos desde 1970 | `1788358325` |
| `{{date}}` | Carimbo de data/hora legível, em UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Prioridade de 1 (baixa) a 5 (alta) | `4` |
| `{{color}}` | Cor de destaque, sem hash | `D98A00` |
| `{{color_hex}}` | Cor de destaque, com hash | `#D98A00` |
| `{{emoji}}` | Pictograma de status | `🔴`, `⚠️`, `ℹ️` ou `✅` |
| `{{link}}` | Link para o dispositivo em DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Sempre `dumbmonit` | `dumbmonit` |
| `{{token}}` | O segredo `token` do canal | *(seu token)* |

Dois comentários úteis:

- **Algumas variáveis podem estar vazias.** Um alerta de disponibilidade ("host inacessível") não tem
  nem um valor nem um limite: `{{value}}` e `{{value_raw}}` então renderizam uma
  string vazia, nunca `null` ou `NaN`. Se você as colocar como um número JSON, lide com
  esse caso — ou use `"{{value}}"` entre aspas.
- **`{{link}}` fica vazio** até que você preencha `base_url`. Um link errado desperdiçaria
  mais tempo que um alerta sem um.

**Você nunca precisa escapar nada sozinho.** Dependendo do escolhido
`content_type`, o DumbMonit automaticamente protege valores substituídos: aspas e quebras de linha escapadas em JSON, codificação percentual em um formulário, sem transformação em texto simples.
Uma regra nomeada `"System" disco cheio` não quebrará seu modelo JSON.

### Exemplo 1 — uma API JSON interna {#example-1--an-internal-json-api}

Uma aplicação caseira expõe `POST /incidents` e espera um token em um
cabeçalho personalizado.

**Configurações**

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
  "token": "seu-token-de-aplicação",
  "headers": "{\"X-Application\": \"dumbmonit\"}"
}
```

O token é enviado automaticamente como um cabeçalho `Authorization: Bearer …`. Aqui está o que
a aplicação recebe:

```json
{
  "title": "nas — Disco cheio",
  "device": "nas",
  "severity": "warning",
  "state": "firing",
  "value": 95,
  "threshold": 90,
  "detail": "⚠️ Aviso · Disco cheio — /data — 95 % (limite > 90 %)",
  "link": "https://dumbmonit.home/targets/42",
  "seen_at": "2026-09-01T14:12:05+00:00"
}
```

### Exemplo 2 — um serviço que espera um formulário {#example-2--a-service-that-expects-a-form}

Muitas transportadoras de SMS e APIs mais antigas esperam `application/x-www-form-urlencoded` em vez
de JSON, com o token no corpo.

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
{ "token": "chave-da-gateway" }
```

Como o modelo já usa `{{token}}`, o DumbMonit **não adiciona** um
cabeçalho `Authorization`: alguns serviços rejeitam uma solicitação que leva duas
autenticações. Os valores são codificados automaticamente, espaços se tornando `+` e caracteres acentuados
seu equivalente codificado em percentual.

### Exemplo 3 — SMS simples através de uma operadora {#example-3--a-plain-text-sms-through-a-carrier}

Alguns serviços, como a API de SMS de uma operadora ou uma automação caseira, querem uma única
linha de texto.

**Configurações**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} em {{target}}: {{rule}} ({{value}}, limite {{operator}} {{threshold}}) — {{date}}"
}
```

A mensagem enviada, sem nenhuma escapa:

```
⚠️ aviso em nas: Disco cheio (95 %, limite > 90 %) — 01/09/2026 14:12 UTC
```

### Exemplo 4 — uma solicitação GET sem corpo {#example-4--a-get-request-without-a-body}

Quando o serviço apenas aceita parâmetros de URL, coloque as variáveis em `url` e escolha
o método `GET`. Uma solicitação `GET` não tem corpo: `body_template` é então rejeitado, então
ninguém acredita que faz algo.

**Configurações**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Segredos**

```json
{ "token": "chave-api" }
```

Apenas os **valores substituídos** são codificados; o resto do endereço é copiado
como está. A solicitação realmente enviada se parece com:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disco%20cheio%20%E2%80%94%2095%20%25
```

### Erros comuns de modelo {#common-template-mistakes}

Um modelo defeituoso é **rejeitado quando salvo**, enquanto você está olhando para o formulário, não
na noite do incidente.

| O que você escreve | O que acontece |
|---|---|
| `{{severite}}` | Rejeitado: variável desconhecida "severite", seguida pela lista de nomes válidos. O nome correto é `severity`. |
| `{"t": "{{title"}` | Rejeitado: "chaves não fechadas". O `}}` está faltando. |
| `{{}}` ou `{{  }}` | Rejeitado: uma variável sem nome não significa nada. |
| `{"a": {"b": 1}}` | **Aceito.** Chaves únicas são literais: escreva seu JSON normalmente. |
| `{{token}}` sem um segredo `token` | Rejeitado: o modelo pede um token que o canal não tem. |
| `method: GET` com `body_template` | Rejeitado, com a alternativa: coloque as variáveis em `url`. |
| `method: DELETE` | Rejeitado: apenas `POST`, `PUT` e `GET` são aceitos. |

---

## Canais suportados {#supported-channels}

### Discord {#discord}

No Discord: **Configurações do canal → Integrações → Webhooks → Novo webhook**, depois
**Copiar URL do webhook**. Este URL contém o token: vai nos segredos.

- **Segredos** — `webhook_url`

### Slack {#slack}

Em <https://api.slack.com/apps>, crie um aplicativo, ative **Webhooks recebidos**, depois
**Adicionar novo webhook ao espaço de trabalho** e escolha o canal. Copie o
URL `https://hooks.slack.com/services/…`.

- **Segredos** — `webhook_url`

### Telegram {#telegram}

Você precisa de duas coisas: o **token do bot** (segredo) e o **chat ID** (`chat_id`)
onde ele deve escrever. O primeiro demora um minuto para obter; o segundo depende de onde
você quer receber os alertas.

#### 1. Criar o bot e obter seu token {#1-create-the-bot-and-get-its-token}

1. No Telegram, procure **@BotFather** e clique em *Iniciar*.
2. Envie `/newbot`, depois responda as perguntas (um nome, depois um nome de usuário que deve terminar
   com `bot`).
3. O BotFather termina com uma mensagem assim:

   ```
   Feito! Parabéns pelo seu novo bot. Você o encontrará em t.me/my_homelab_bot.
   Use este token para acessar a API HTTP:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   A linha `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` é o **token do bot**. Não
   compartilhe: ele dá controle total do bot.

Nos exemplos abaixo, substitua `<TOKEN>` por este token. Note o prefixo `bot`
colado na frente dele em URLs: `https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Encontre o `chat_id` {#2-find-the-chat_id}

**Chat privado** (o bot escreve para você diretamente)

1. Abra seu bot (`t.me/my_homelab_bot`) e clique em *Iniciar*, ou envie qualquer mensagem.
   Sem isto, ele nunca será capaz de escrever para você.
2. Abra em um navegador: `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Você obtém um documento JSON; procure por `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   O `chat_id` é o valor de `chat.id`: aqui `21xxxxx38` (um número positivo).

   Se `result` estiver vazio (`"result":[]{}`), envie ao bot outra mensagem e recarregue
   a página.

**Canal**

1. Adicione o bot como administrador do canal (Gerenciar canal →
   Administradores → Adicionar).
2. Poste uma mensagem no canal.
3. Abra `https://api.telegram.org/bot<TOKEN>/getUpdates`: a resposta contém um
   bloco `channel_post` com `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   O `chat_id` é aquele número negativo, `-100` incluído: `-1001xxxxxx062`.

**Grupo**

O caminho mais fácil passa pelo aplicativo desktop:

1. Adicione o bot ao grupo.
2. Envie uma mensagem no grupo, clique com o botão direito → *Copiar link da mensagem*. Você obtém
   `https://t.me/c/194xxxx987/13` (ou `https://t.me/c/194xxxx987/11/13` se o
   grupo tiver tópicos).
3. O número certo após `/c/` é o identificador interno do grupo: `194xxxx987`.
   O `chat_id` a inserir é aquele número **prefixado com `-100`**: `-100194xxxx987`.

Sem o aplicativo desktop, o método `getUpdates` acima funciona também: `message.chat.id`
é diretamente o número certo (já negativo). Se o bot não ver as mensagens do grupo, peça ao BotFather por `/setprivacy` → *Desabilitar*, depois remova o bot do
grupo e adicione novamente.

**Tópico de grupo** (grupos em modo "Tópicos")

O `chat_id` é do grupo. Além disso, preencha **Tópico do grupo**
(`message_thread_id`) com o número do meio do link copiado: em
`https://t.me/c/194xxxx987/11/13`, o tópico é `11`. Sem essa configuração, a
mensagem cai no thread *Geral*.

#### 3. Verifique antes de salvar {#3-check-before-saving}

Abra no navegador:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(adicione `&message_thread_id=11` para um tópico). Se `test123` chegar, o token e o
`chat_id` estão corretos: insira-os no DumbMonit e clique em *Teste*.

- **Configurações** — `chat_id` (obrigatória), `message_thread_id` (tópico, opcional),
  `api_base` (padrão `https://api.telegram.org`, mude-o apenas para passar por um
  retransmissor)
- **Segredos** — `bot_token`

> **"Telegram não consegue encontrar este chat"** — para um chat privado, o bot nunca recebeu uma mensagem
> de você: envie-lhe `/start`. Para um grupo ou canal, o prefixo `-100` provavelmente
> está faltando, ou o bot não foi adicionado a ele.
>
> **"thread não encontrada"** — o `message_thread_id` não corresponde a nenhum tópico do
> grupo, ou o grupo não está em modo "Tópicos".

### Microsoft Teams {#microsoft-teams}

**Aviso: o método mudou.** Os antigos "Conectores do Office 365" foram aposentados em
maio de 2026; uma URL `outlook.office.com/webhook/…` não funciona mais.

O procedimento atual:

1. No canal do Teams, **⋯ → Fluxos de trabalho**.
2. Escolha o modelo **"Postar em um canal quando uma solicitação de webhook é recebida"**.
3. Confirme os passos, depois **copie a URL POST HTTPS** mostrada no final.

- **Segredos** — `webhook_url`

O DumbMonit envia um *Cartão Adaptável* embrulhado, o único formato que esses fluxos exibem
corretamente. Se você receber um erro mencionando Power Automate, a URL provavelmente vem
de um conector antigo.

### Matrix {#matrix}

1. Crie uma conta dedicada ao DumbMonit no seu servidor e **convide-a para o
   quarto**. Uma conta que não é membro não pode escrever.
2. Obtenha seu token de acesso. No Element: **Configurações → Ajuda e Sobre → Avançado → Token de acesso**.
3. Obtenha o identificador **interno** do quarto — não seu nome de exibição. No Element:
   **Configurações da sala → Avançado → ID interno da sala**. Começa com `!` (por exemplo
   `!aBcDeF:matrix.org`).

- **Configurações** — `server_url` (por exemplo `https://matrix.org`), `room_id`
- **Segredos** — `token`

### Mattermost {#mattermost}

**Integrações → Webhooks de entrada → Adicionar**. Escolha o canal padrão e copie a URL.

- **Configurações** — `channel` (opcional, para escrever em outro lugar que não o canal
  padrão), `username` (opcional, padrão `DumbMonit`)
- **Segredos** — `webhook_url`

### Rocket.Chat {#rocketchat}

**Administração → Integrações → Nova integração → Webhook de entrada**. Escolha a sala,
ative a integração e copie a *URL do webhook*.

- **Configurações** — `channel` (opcional), `alias` (opcional, padrão `DumbMonit`)
- **Segredos** — `webhook_url`

### Google Chat {#google-chat}

No espaço do Google Chat: **nome do espaço → Aplicativos e integrações → Webhooks →
Adicionar webhook**. Copie a URL, que já contém a chave e o token.

- **Segredos** — `webhook_url`

### Zulip {#zulip}

1. Crie um **bot**: **Configurações pessoais → Bots → Adicionar um novo bot**, tipo
   "Genérico". Anote seu endereço de e-mail e sua chave de API.
2. Inscreva o bot no stream de destino, caso contrário o envio será recusado.

- **Configurações** — `server_url` (por exemplo `https://sua-org.zulipchat.com`),
  `email` (o endereço do bot), `stream` (o nome do stream), `topic` (opcional,
  padrão `DumbMonit`)
- **Segredos** — `api_key`

### ntfy {#ntfy}

O canal mais simples de todos: escolha um nome de tópico difícil de adivinhar e
inscreva-se nele pelo aplicativo ntfy.

- **Configurações** — `topic`, `server_url` (opcional, padrão `https://ntfy.sh`)
- **Segredos** — `token` (somente se o seu servidor exigir autenticação)

> Na instância pública, **qualquer pessoa que saiba o nome de um tópico pode lê-lo**.
> Escolha um nome longo e aleatório, ou hospede seu próprio servidor.

### Gotify {#gotify}

Na interface do Gotify: **Apps → Create Application**. Copie o token exibido.

- **Configurações** — `server_url` (por exemplo `https://gotify.home`)
- **Segredos** — `token`

### Pushover {#pushover}

1. Em <https://pushover.net>, sua **User Key** aparece logo na página inicial.
2. Depois crie um aplicativo: **Create an Application/API Token**. Você recebe um
   **token de aplicativo**.

- **Configurações** — `priority` (opcional, de -2 a 2), `sound` (opcional), `retry`
  e `expire` (somente para `priority: 2`)
- **Segredos** — `token` (token do aplicativo), `user_key` (chave do usuário)

Sem a configuração `priority`, o DumbMonit escolhe conforme a gravidade: `-1` para um
aviso ou uma resolução, `0` para um alerta de atenção, `1` para um alerta crítico. Um
aviso, portanto, não faz o telefone tocar.

> A prioridade `2` ("emergência") repete a notificação até que seja confirmada. O
> DumbMonit então adiciona `retry` e `expire` automaticamente; sem eles, o Pushover
> rejeitaria a mensagem.

### Pushbullet {#pushbullet}

Em <https://www.pushbullet.com/#settings/account>, clique em **Create Access Token**.

- **Configurações** — `device_iden` (opcional; sem ele, todos os seus dispositivos são
  notificados)
- **Segredos** — `token`

### Bark (iOS) {#bark-ios}

Abra o aplicativo Bark no seu iPhone: ele mostra uma URL contendo sua **chave de
dispositivo**, a longa sequência de caracteres após o nome de domínio.

- **Configurações** — `server_url` (opcional, padrão `https://api.day.app`), `group`
  (opcional), `level` (opcional), `sound` (opcional)
- **Segredos** — `token` (a chave do dispositivo)

Sem a configuração `level`, o DumbMonit escolhe: `passive` para um aviso, `active`
para um alerta de atenção, `timeSensitive` para um alerta crítico. O nível `critical`,
que ignora o "Não Perturbe", nunca é escolhido automaticamente — defina-o
explicitamente se o quiser.

### Web Push {#web-push}

Notificações nativas nos seus celulares e computadores, direto do seu próprio
DumbMonit, sem aplicativo para instalar e sem conta de terceiros: o navegador
(instalado como aplicativo ou não) recebe os alertas mesmo com todas as abas fechadas.

1. Em cada celular ou computador, abra o DumbMonit e vá em **Configurações →
   Notificações push → Ativar neste dispositivo**, depois **Enviar um teste**. No
   iPhone e no iPad, adicione primeiro o DumbMonit à Tela de Início. O passo a passo
   está em [Notificações push (PWA)](using/push-notifications.md).
2. Crie aqui um canal **Web Push**. Ele não tem segredo: seus destinatários são os
   dispositivos inscritos no passo 1.

- **Configurações** — `users` (opcional; uma conta por linha, vazio: todos os
  dispositivos inscritos de todas as contas), `urgency` (`high` por padrão, `normal`
  ou `low`), `ttl` (segundos que o serviço de push guarda a mensagem para um
  dispositivo desligado, padrão `86400`), `contact` (endereço `mailto:` ou `https:`
  opcional fornecido aos serviços de push; vazio: `DUMBMONIT_PUBLIC_URL` quando for
  HTTPS, caso contrário a página do projeto)
- **Segredos** — nenhum. O par de chaves VAPID do servidor é gerado na primeira
  inicialização e armazenado criptografado; a chave privada nunca sai do servidor.

O título da notificação é a gravidade e o dispositivo (`Crítico · nas01`), o corpo um
resumo curto; um toque abre a página **Alertas**. O DumbMonit precisa de HTTPS de saída
para os serviços de push (`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`); as mensagens são
criptografadas de ponta a ponta, esses serviços não podem lê-las. Uma inscrição que o
serviço de push informa como inexistente (HTTP 404 ou 410, por exemplo após a remoção
do aplicativo) é excluída automaticamente.

### Apprise {#apprise}

O Apprise é um **gateway**: com uma única configuração, dá acesso a dezenas de serviços
que o DumbMonit não implementa por conta própria. Implante o contêiner `caronc/apprise`
e escolha um dos dois modos.

**Modo recomendado — configuração armazenada no Apprise.** Crie uma configuração na
interface do Apprise, anote sua chave e não coloque mais nada aqui.

- **Configurações** — `server_url`, `config_key`

**Modo direto — o DumbMonit fornece os destinos.** As URLs do Apprise costumam conter
uma senha (`mailto://user:password@…`): por isso vão para os segredos.

- **Configurações** — `server_url`
- **Segredos** — `urls` (uma ou mais URLs do Apprise, separadas por vírgulas)

Configurações comuns opcionais: `tag` (para atingir apenas parte dos destinos de uma
configuração) e `format` (`text` por padrão, ou `markdown`).

> Se o teste responder "Apprise has no configuration under this config_key", a chave
> não corresponde a nada. O DumbMonit informa isso em vez de fingir que a mensagem foi
> enviada.

### Home Assistant {#home-assistant}

1. No Home Assistant: **seu perfil → Segurança → Tokens de acesso de longa duração →
   Criar token**. Copie-o, ele não será mostrado novamente.
2. Escolha o serviço a chamar. Por padrão, o DumbMonit cria uma notificação persistente
   no painel.

- **Configurações** — `server_url` (por exemplo `http://homeassistant.local:8123`),
  `service` (opcional, padrão `persistent_notification.create`), `target`
  (opcional), `data` (opcional, objeto mesclado à chamada)
- **Segredos** — `token`

Alguns serviços úteis:

| `service` | Efeito |
|---|---|
| `persistent_notification.create` | Faixa na interface do Home Assistant |
| `notify.notify` | Todos os destinos de notificação configurados |
| `notify.mobile_app_<device>` | O aplicativo móvel de um dispositivo específico |
| `light.turn_on` | Acende uma luz — use `data` para a entidade e a cor |

A configuração `data` é mesclada **por último**: ela pode, portanto, substituir o título
ou a mensagem que o DumbMonit preparou.

### E-mail (SMTP) {#email-smtp}

O único canal que não depende de nenhum serviço de terceiros.

- **Configurações** — `host`, `from`, `to` (lista de endereços), `security` (`starttls`
  por padrão, ou `tls`, ou `none`), `port` (derivada de `security`: 587, 465 ou
  25), `username` (opcional)
- **Segredos** — `password`

> Com o Gmail, a senha da conta não funciona: crie uma **senha de app** nas
> configurações de segurança da sua conta Google.

### Signal {#signal}

Passa por uma instância do
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api) que você
hospeda e na qual registra primeiro o seu número.

- **Configurações** — `server_url` (por exemplo `http://signal.home:8080`), `number`
  (o número remetente registrado), `recipients` (lista de números ou identificadores
  de grupo), `username` (opcional)
- **Segredos** — `password` (opcional, se a sua instância for protegida)

Todos os números são escritos em **formato internacional**, `+33612345678`.

### SMS via Twilio {#sms-via-twilio}

No console do Twilio, a página inicial mostra seu **Account SID** e seu **Auth Token**.
Depois compre um número capaz de enviar SMS.

- **Configurações** — `account_sid`, `from` (seu número Twilio) **ou**
  `messaging_service_sid`, `to` (lista de destinatários)
- **Segredos** — `token` (o *Auth Token*)

O corpo do SMS é truncado em 300 caracteres: além disso, o Twilio o divide em
segmentos cobrados separadamente.

> Em uma conta de teste, o Twilio só envia para números que você verificou.

### PagerDuty {#pagerduty}

No PagerDuty: **Services → seu serviço → Integrations → Add integration** e escolha
**Events API v2**. Copie a *Integration Key*.

- **Configurações** — `region` (`eu` se a sua conta estiver hospedada na Europa),
  `source` (opcional, padrão `dumbmonit`)
- **Segredos** — `token` (a *Integration Key*)

Um alerta abre um incidente; sua resolução **o fecha automaticamente**. A
correspondência é feita pela impressão digital do alerta, prefixada com `dumbmonit-`,
de modo que um lembrete nunca abre um segundo incidente.

> Uma chave de outro tipo ("Events API v1", "REST API") recebe uma recusa seca.
> Verifique se a integração é realmente do tipo *Events API v2*.

### Opsgenie {#opsgenie}

**Teams → sua equipe → Integrations → Add integration → API**. Copie a chave de API.

- **Configurações** — `region` (`eu` para uma conta europeia), `responders` (lista de
  nomes de equipes, opcional), `tags` (lista, opcional)
- **Segredos** — `api_key`

Como no PagerDuty, uma resolução fecha o alerta, identificado por seu alias.

> A Atlassian definiu o **fim do serviço do Opsgenie em 5 de abril de 2027**. Este
> canal permanece para as instalações existentes; para uma nova, prefira o PagerDuty
> ou um canal personalizado apontando para o Jira Service Management.

### Webhook personalizado {#custom-webhook}

Veja [O canal personalizado](#the-custom-channel) acima.

---

## Solução de problemas {#troubleshooting}

**O teste falha imediatamente, com "missing setting …".**
Um campo obrigatório está vazio. A mensagem nomeia a configuração exata.

**O teste falha com "no answer within 15 s".**
O servidor de destino não é alcançável a partir do DumbMonit. Se for um serviço da sua
rede local, verifique se o contêiner do DumbMonit consegue alcançá-lo — isso costuma
ser um problema de rede do Docker, não de configuração do canal.

**O teste falha com um erro de certificado.**
Seu servidor usa um certificado auto-assinado ou de uma autoridade privada. Veja
[Certificados auto-assinados](#self-signed-certificates).

**O teste é bem-sucedido, mas não recebo nada.**
O serviço aceitou a mensagem e a enviou para outro lugar: canal errado, tópico ntfy
errado, dispositivo Pushbullet errado. Verifique a configuração de destino.

**A mensagem de erro contém `***`.**
É intencional: seu token foi removido da mensagem antes de ser registrado. Nenhum
segredo aparece nos logs, na coluna "último erro" ou em uma resposta da API.

**Onde posso ver o último erro de um canal?**
Na lista de canais: cada linha mostra a data da última entrega bem-sucedida e, se
houver, o erro da última falha.
