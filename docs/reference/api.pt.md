# API HTTP {#http-api}

Tudo o que a interface web faz passa por `/api/*`, e a mesma API está
aberta aos seus próprios scripts, painéis e assistentes: não há
nenhuma metade privada. É JSON simples sobre HTTP, autenticado por um
cookie de sessão (a interface web) ou por um token de API (tudo o
resto).

!!! warning "Alpha"
    O DumbMonit é uma alpha e a sua API também: as rotas e campos
    ainda podem mudar. A versão da API (abaixo) muda sempre que uma
    rota ou campo é removido ou renomeado, para que um cliente possa
    distinguir.

## Descrição OpenAPI {#openapi-description}

Toda instância descreve a sua própria API num documento
[OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0):

```
GET /api/openapi.json
```

É público — não transporta nenhum segredo, nenhum endereço e nada
sobre os seus dispositivos — e é a referência exaustiva: toda rota
que um cliente pode chamar, os seus parâmetros, formas de pedido e
resposta, respostas de erro e o âmbito que precisa (a extensão
`x-token-scope` em cada operação: `public`, `read`, `write`, ou `none`
para rotas que um token nunca pode chamar). As tabelas mais abaixo
nesta página são um resumo legível da mesma coisa.

Deixados de fora de propósito: o protocolo de agente (`/api/ingest`,
`/api/agent/commands*`, `/api/agent/relay*`), que só o
[agente](../devices/agent.md) fala; os redirecionamentos de navegador
do início de sessão SSO (`/api/auth/oidc/start`,
`/api/auth/oidc/callback`); e as descargas de agente (`/install.sh`,
`/install.ps1`, `/download/*`).

Carregue-o em qualquer ferramenta OpenAPI:

```bash
# Navegue nele, e experimente chamadas com o seu token, no Swagger UI
docker run --rm -p 8081:8080 \
  -e SWAGGER_JSON_URL=http://monit.lan:8080/api/openapi.json swaggerapi/swagger-ui
# depois abra http://localhost:8081 e cole "dmt_…" em Authorize → bearerAuth

# Gere uma biblioteca de cliente (aqui Python)
docker run --rm -v "$PWD:/local" openapitools/openapi-generator-cli generate \
  -i http://monit.lan:8080/api/openapi.json -g python -o /local/dumbmonit-client

# Ou simplesmente liste o que existe
curl -s http://monit.lan:8080/api/openapi.json | jq -r '.paths | keys[]'
```

O Postman e o Insomnia importam-no do mesmo URL.

## Autenticação {#authentication}

Duas formas de entrar, aceites em toda rota protegida:

- **Cookie de sessão** — o que a interface web usa. Inicie sessão com
  uma senha (ou SSO) e envie o cookie `dumbmonit_session` HttpOnly
  (SameSite Lax; `Secure` quando `DUMBMONIT_COOKIE_SECURE=1`; 30
  dias). Todo pedido que altera estado (`POST`, `PUT`, `DELETE`)
  também tem de transportar `X-Requested-With: DumbMonit` — a prova
  anti-CSRF que a interface adiciona automaticamente; sem ela o
  pedido é recusado com `403`.
- **Token de API** — `Authorization: Bearer dmt_…`, para scripts,
  painéis e assistentes. Criado em Settings → **API & assistants**
  (os mesmos tokens que o [servidor MCP](../using/assistant.md) usa)
  e mostrado uma vez. Nenhum cookie está envolvido, por isso nenhum
  cabeçalho `X-Requested-With` é alguma vez necessário.

```bash
# Token de API: um cabeçalho, nada mais
curl -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/targets
curl -H 'Authorization: Bearer dmt_…' -X POST http://monit.lan:8080/api/targets/4/probe

# Sessão: iniciar sessão (204 e um cabeçalho Set-Cookie), depois enviar o cookie
curl -c cookies.txt -X POST http://monit.lan:8080/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"…"}'
curl -b cookies.txt http://monit.lan:8080/api/targets
curl -b cookies.txt -X POST http://monit.lan:8080/api/targets \
  -H 'X-Requested-With: DumbMonit' -H 'content-type: application/json' \
  -d '{"name":"NAS","address":"nas.lan","kind":"snmp","credential":{"type":"snmp_community","community":"public"}}'
```

Quando um pedido transporta um token bearer, o cookie é ignorado: um
token revogado é recusado mesmo de um navegador com sessão iniciada,
em vez de recuar silenciosamente para a sessão.

Os tokens de registo de agente (`dmon_…`) são uma coisa diferente: só
funcionam nas rotas de agente (`/api/ingest`,
`/api/agent/commands/*`, `/api/agent/relay*`). Dizem que uma máquina
pode falar, não *qual* máquina: nessas rotas uma máquina associada
também tem de apresentar o seu próprio segredo de associação em
`X-DumbMonit-Agent-Secret` (`dmab_…`), que o servidor entrega uma vez
no registo. Ver
[Associação](../devices/agent.md#binding-one-machine-one-agent).

O início de sessão é limitado por taxa: depois de cinco tentativas
falhadas, cada tentativa seguinte é recusada com `429` e um prazo
`Retry-After` que duplica de 30 s até 5 minutos. Nunca adivinhe
senhas num loop.

## Âmbitos {#scopes}

Um token tem um de dois âmbitos. Não há um terceiro, mais poderoso: o
que um administrador faz na sua própria conta fica fora do alcance de
todo token.

| | token `read` | token `write` |
|---|---|---|
| Todo `GET` que uma conta *viewer* pode fazer (dispositivos, alertas, métricas, regras, canais sem os seus segredos, páginas de estado…) | sim | sim |
| `POST`, `PUT`, `DELETE` que um *administrador* pode fazer (dispositivos, regras, silêncios, canais, páginas de estado, pacotes, verificações, descoberta…) | `403` | sim |
| `/metrics`, `/federate`, `/prometheus/*` ([consulta](metrics.md#scraping-dumbmonit)) | sim | sim |
| Ferramentas MCP de leitura / escrita ([assistentes](../using/assistant.md#what-the-assistant-can-call)) | só leitura | ambas |
| Contas, sessões, dois fatores, SSO, registo de auditoria (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| Tokens de API e tokens de registo de agente (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Cópia de segurança e restauro (`/api/backup/*`) | `403` | `403` |

Um token `read` numa rota de escrita recebe `403` com uma mensagem
que nomeia o token e o âmbito em falta. Como nenhum token pode listar,
criar ou revogar tokens, um token nunca pode criar outro — muito menos
um mais poderoso.

## Segurança de token {#token-security}

- **Mostrado uma vez, guardado com hash.** Um token é `dmt_` seguido
  de 128 bits aleatórios. O servidor só guarda o seu digest SHA-256
  (e os primeiros caracteres, `dmt_1a2b3c4d`, para o reconhecer numa
  lista); o token em claro só existe na resposta à sua criação. Os
  tokens apresentados são comparados em tempo constante.
- **Expiração.** Escolha 30 dias, 90 dias, 1 ano ou nunca ao criá-lo.
  Um token expirado recebe `401` com
  `The API token "<name>" expired on …` e
  `WWW-Authenticate: Bearer error="invalid_token"`; fica na lista,
  marcado como expirado, até o revogar.
- **Redes permitidas.** Opcionalmente restrinja um token a uma lista
  de endereços ou intervalos CIDR (`192.168.1.0/24, 10.8.0.5`, 32
  entradas no máximo). De qualquer outro lugar recebe `403` com
  `The API token "<name>" cannot be used from <address>.` O endereço
  verificado é o do cliente TCP, ou — só quando a ligação vem de um
  proxy listado em `DUMBMONIT_TRUSTED_PROXIES` — o que esse proxy
  reporta em `X-Forwarded-For`. Atrás de um proxy inverso que não
  declarou, todo pedido parece vir do proxy: declare-o, ou a
  restrição recusará todos.
- **Última utilização.** A lista de tokens mostra quando cada token
  foi usado por último e a partir de que endereço (atualizado no
  máximo uma vez por minuto, ou assim que o endereço mudar): um token
  que mais ninguém usa é um para revogar.
- **Ligado ao seu criador.** Um token lembra a conta que o criou. Se
  essa conta é desativada, os seus tokens são recusados (`401`); se é
  despromovida a operator ou viewer, os seus tokens `write` atuam
  como tokens `read`; se é eliminada, os seus tokens são revogados.
- **A revogação** é imediata: a chamada seguinte recebe `401`.
- **Registado.** Toda escrita feita com um token é escrita no
  registo do servidor com o nome e id do token, o método, o caminho
  (sem a cadeia de consulta) e o estado — nunca o token, nunca o
  corpo. Criar e revogar um token são registados no registo de
  auditoria (Settings → Security).

## Limites de taxa {#rate-limits}

Cada token pode fazer 120 chamadas por minuto, REST e MCP juntos.
Além disso, as chamadas são recusadas com `429` e um cabeçalho
`Retry-After` (segundos) até o minuto terminar. Toda resposta a um
pedido autenticado por token diz onde está:

| Cabeçalho | Significado |
|---|---|
| `RateLimit-Limit` | Chamadas permitidas por janela (120). |
| `RateLimit-Remaining` | Chamadas restantes na janela atual. |
| `RateLimit-Reset` | Segundos até a janela recomeçar. |

Um painel Grafana com muitos painéis e uma atualização curta pode
atingir o limite: dê-lhe o seu próprio token, ou alargue a
atualização.

## Pedidos de origem cruzada (CORS) {#cross-origin-requests-cors}

Uma página servida de outra origem — o seu próprio painel, Swagger
UI, um cliente MCP baseado em navegador — pode chamar a API **com um
token bearer, nunca com uma sessão**:

- uma resposta transporta `Access-Control-Allow-Origin` só quando o
  pedido transportou `Authorization: Bearer dmt_…` (ou para o
  público `GET /api/openapi.json`). `Access-Control-Allow-Credentials`
  nunca é enviado, por isso um navegador nunca anexa o cookie de
  sessão a uma chamada de origem cruzada;
- um preflight (`OPTIONS`) em `/api/*` só tem sucesso quando os
  cabeçalhos anunciados incluem `authorization`: `204`, métodos
  `GET, POST, PUT, DELETE`, cabeçalhos
  `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`,
  em cache 10 minutos. `X-Requested-With` não está entre eles, por
  isso uma sessão de cookie nunca pode fazer uma escrita de outra
  origem;
- `Retry-After`, `WWW-Authenticate`, os cabeçalhos `RateLimit-*` e
  `X-DumbMonit-Api-Version` são legíveis pelo script chamador.

`DUMBMONIT_API_CORS_ORIGINS` define quais origens são permitidas:

| Valor | Efeito |
|---|---|
| não definido (predefinição) | Qualquer origem (`Access-Control-Allow-Origin: *`). Seguro, já que só um token bearer abre alguma coisa. |
| `https://grafana.example.com,https://dash.lan` | Só estas origens, repetidas de volta. |
| `off` | Nenhum cabeçalho CORS de todo: só páginas da mesma origem e clientes não-navegador. |

A mesma lista decide qual `Origin` o
[endpoint MCP](../using/assistant.md#security-notes) aceita.

```js
// De uma página noutra origem. Nunca envie um token de escrita numa página pública.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'calls left this minute');
```

## Versionamento {#versioning}

Toda resposta sob `/api` transporta a versão da API:

```
X-DumbMonit-Api-Version: 0.1
```

O mesmo valor é `info.version` no documento OpenAPI, junto a
`x-server-version`, a versão do servidor que respondeu. Enquanto a
API é `0.x` não está congelada; a versão muda sempre que uma rota ou
campo é removido ou renomeado. Adicionar uma rota ou campo não a
altera: ignore campos que não conhece.

## Erros {#errors}

Todo erro é JSON: `{"error": "message"}`, escrito para uma pessoa.

| Estado | Quando |
|---|---|
| `400` | O pedido está malformado ou um valor é inválido; a mensagem nomeia-o. |
| `401` | Nenhuma sessão ou token válido: token em falta, desconhecido, revogado ou expirado (com `WWW-Authenticate: Bearer`), ou um token cujo criador está desativado. |
| `403` | Autenticado, mas não permitido: um viewer, um operator fora da gestão de alertas, ou um token `read` numa rota de escrita, um token numa rota de conta, token ou cópia de segurança, um token usado de uma rede de onde não é permitido, uma escrita por cookie sem `X-Requested-With`, ou qualquer escrita na demo pública (com `"demo": true`). |
| `404` | O id não existe. Um caminho sob `/api` que não corresponde a nenhuma rota responde `404 {"error": "Unknown API route: …"}` em vez do HTML da interface web. |
| `405` | O método não existe nesse caminho (`GET /api/mcp`, por exemplo). |
| `409` | Um conflito com o estado atual (um duplicado, um comando já em execução…). |
| `429` | Limitado por taxa, com `Retry-After`. |
| `500` | Uma falha do servidor; os detalhes estão no registo do servidor, nunca na resposta. |

```bash
$ curl -i -H 'Authorization: Bearer dmt_revoked…' http://monit.lan:8080/api/targets
HTTP/1.1 401 Unauthorized
www-authenticate: Bearer
{"error":"A valid API token is required."}

$ curl -i -X DELETE -H 'Authorization: Bearer dmt_…read…' http://monit.lan:8080/api/targets/4
HTTP/1.1 403 Forbidden
{"error":"This action needs a token with the \"write\" scope; the token \"Grafana\" is \"read\" only. Create a write token in Settings → API & assistants."}

$ curl -i -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/alerts   # a 121ª chamada este minuto
HTTP/1.1 429 Too Many Requests
retry-after: 17
ratelimit-limit: 120
ratelimit-remaining: 0
{"error":"Rate limit reached (120 calls per minute per token). Retry in 17 s."}
```

O [endpoint MCP](../using/assistant.md#protocol-details) responde em
JSON-RPC em vez disso, com os seus próprios códigos de erro.

## Contas, início de sessão e tokens {#accounts-sign-in-and-tokens}

Nas tabelas abaixo, *session* significa um cookie de sessão **ou**
um token de API; *admin* significa uma sessão de administrador ou um
token `write`; *operator* significa uma sessão de operator ou
administrador, ou um token `write`; *session only* significa um
cookie de sessão, nunca um token.

**Funções.** Uma conta `viewer` faz todo `GET`. Uma conta `operator`
faz as mesmas leituras mais as escritas de gestão de alertas marcadas
*operator* nesta página (e `x-roles: [admin, operator]` no documento
OpenAPI): confirmar, adiar (janelas de manutenção), ignorar uma regra
para um dispositivo, limpar o histórico resolvido. Toda outra escrita
é só de administrador e responde `403` a um operator.

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/auth/status` | público | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` significa uma instância nova: até o primeiro administrador existir, toda rota marcada *session* responde `401` — só `status`, `setup`, `login` e `health` são alcançáveis. |
| `POST` | `/api/auth/setup` | público | `{"setup_code": "…", "username": "admin", "password": "…"}`. Cria o primeiro administrador numa instância nova (pelo menos 12 caracteres); `username` pode ser omitido e é então `admin`. `setup_code` é o código único impresso nos registos do servidor no arranque (ou `DUMBMONIT_SETUP_CODE`); caixa, hífenes e espaços são ignorados. `204`; não abre uma sessão. `401` para um código errado, `429` depois de demasiadas, `409` uma vez existente uma conta. |
| `POST` | `/api/auth/login` | público | `{"username": "…", "password": "…"}`. `204` com `Set-Cookie`. `401` numa senha errada, `429` quando limitado por taxa. Quando a conta tem dois fatores ativos, `200` com `{"totp_required": true, "pending": "…"}` em vez disso — um bilhete que vive cinco minutos e morre depois de cinco códigos errados: termine com `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | público | `{"pending": "…", "code": "123456"}` — o bilhete do primeiro passo e um código (ou um código de recuperação). `204` com o cookie de sessão. |
| `GET` | `/api/auth/oidc/start` | público | Redireciona o navegador para o fornecedor de identidade. |
| `GET` | `/api/auth/oidc/callback` | público | Retorno do fornecedor (`code`, `state`): abre a sessão e redireciona para a interface. |
| `GET` | `/api/auth/me` | só sessão | A conta atual: `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | só sessão | Termina a sessão e limpa o cookie. |
| `POST` | `/api/auth/password` | só sessão | `{"current_password": "…", "new_password": "…"}`. Termina a sessão de toda outra sessão. |
| `GET` | `/api/auth/totp` | só sessão | Estado de dois fatores da conta atual: `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | só sessão | `{"password": "…"}`. Propõe um segredo: `secret` (base32), `otpauth_uri`, `issuer`, `account`. Nada é imposto até ser verificado. |
| `POST` | `/api/auth/totp/verify` | só sessão | `{"code": "123456"}`. Confirma o registo e devolve `{"recovery_codes": […]}` — mostrado uma vez. |
| `DELETE` | `/api/auth/totp` | só sessão | `{"password": "…"}`. Remove o segundo fator. `204`. |
| `GET` | `/api/auth/audit?limit=200` | admin, só sessão | Os eventos de segurança mais recentes: `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | admin, só sessão | As definições SSO e de onde vêm (ambiente ou base de dados); o segredo do cliente nunca é devolvido. |
| `PUT` | `/api/auth/oidc/config` | admin, só sessão | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Um `client_secret` ausente ou vazio mantém o guardado; um `auto_create` ausente significa desligado. |
| `DELETE` | `/api/auth/oidc/config` | admin, só sessão | Esquece as definições SSO guardadas (as variáveis de ambiente, se existirem, aplicam-se de novo). `204`. |
| `POST` | `/api/auth/oidc/test` | admin, só sessão | Obtém o documento de descoberta do fornecedor e reporta o que encontrou. |
| `GET` | `/api/users` | admin, só sessão | Toda conta, na forma de `/api/auth/me`. |
| `POST` | `/api/users` | admin, só sessão | `{"username", "display_name", "role", "password"}`. `password` só pode ser omitida quando o SSO está ativo. `201`. |
| `PUT` | `/api/users/{id}` | admin, só sessão | Qualquer de `display_name`, `role`, `disabled`, `password`; um campo omitido mantém o seu valor. Não pode despromover ou desativar o último administrador. |
| `DELETE` | `/api/users/{id}` | admin, só sessão | `204`. Não você mesmo, não o último administrador. |
| `DELETE` | `/api/users/{id}/totp` | admin, só sessão | Repõe os dois fatores de outra conta (um colega bloqueado). `204`. |
| `GET` | `/api/tokens` | só sessão | Todo token de API: `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (a conta que o criou), `expires_at` (`null`: nunca), `expired`, `allowed_networks` (vazio: de qualquer lugar), `last_used_at`, `last_used_ip`, `revoked_at`. Nunca o segredo. |
| `POST` | `/api/tokens` | admin, só sessão | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` é obrigatório (no máximo 80 caracteres); `scope` é `read` (predefinição) ou `write`; `expires_in_days` vai de 1 a 3650, `null` ou ausente para um token que nunca expira; `allowed_networks` toma até 32 endereços ou intervalos CIDR, guardados em forma de rede (`192.168.1.7/24` torna-se `192.168.1.0/24`), vazio ou ausente para de qualquer lugar. `201` com os campos do token mais `secret` (mostrado uma vez). `400` nomeia uma rede inválida. |
| `DELETE` | `/api/tokens/{id}` | admin, só sessão | Revogar. `204`; `404` quando desconhecido ou já revogado. |

Um token como listado:

```json
{
  "id": 3, "name": "Grafana", "prefix": "dmt_1a2b3c4d", "scope": "read",
  "created_at": "2026-10-01T09:12:00Z", "created_by": "admin",
  "expires_at": "2026-12-30T09:12:00Z", "expired": false,
  "allowed_networks": ["192.168.1.0/24"],
  "last_used_at": "2026-10-01T09:30:00Z", "last_used_ip": "192.168.1.20",
  "revoked_at": null
}
```

## Cabeçalhos de resposta {#response-headers}

Além de `X-DumbMonit-Api-Version` ([versionamento](#versioning)), dos
cabeçalhos `RateLimit-*` ([limites de taxa](#rate-limits)) e dos
cabeçalhos CORS ([pedidos de origem cruzada](#cross-origin-requests-cors)),
toda resposta transporta `X-Content-Type-Options: nosniff`,
`Referrer-Policy: same-origin` e um `Content-Security-Policy`. A
política começa a partir de `default-src 'none'` e abre só o que a
interface realmente usa — tudo servido pelo próprio DumbMonit:

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

Não há nenhuma origem de terceiros nela, e nunca deveria haver: as
fontes, a biblioteca de gráficos e todo recurso vêm dentro do binário.
Os scripts em linha da página (o tema aplicado antes do primeiro
desenho, o bootstrap do SvelteKit) são permitidos por um nonce
regenerado para cada resposta, por isso `'unsafe-inline'` nunca se
aplica a scripts. Aplica-se a estilos, e não pode ser removido: o
Svelte define atributos `style=` em elementos, que nem um nonce nem
um hash conseguem cobrir.

As páginas de estado públicas são feitas para serem incorporadas:
`/s/<slug>` e a sua vista compacta `/s/<slug>/embed` — e só essas duas
— recebem a mesma política sem `frame-ancestors`. Tudo o resto,
incluindo as páginas de confirmação de subscrição e cancelamento sob
`/s/`, também transporta `X-Frame-Options: DENY`.

Se colocar o DumbMonit atrás de um proxy inverso, não o deixe
adicionar um segundo cabeçalho `Content-Security-Policy`: os
navegadores impõem a intersecção de todos eles, e uma predefinição de
proxy sem o nonce deixa uma página em branco.

## Saúde {#health}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/health` | público | Versão do servidor e o estado das suas dependências. Sempre `200`: um componente a falhar está no corpo. |
| `GET` | `/api/update` | sessão ou token | Versão em execução, última versão, `update_available`; o servidor pergunta ao GitHub no máximo uma vez por dia. `POST /api/update/check` verifica agora, `PUT /api/update/settings` liga ou desliga a verificação. Ver [Atualizar](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` é `degraded` quando um componente falha; esse componente
então transporta uma cadeia `error`. `victoria.embedded` é `true`
quando o servidor executa a sua própria VictoriaMetrics, `false`
quando `DUMBMONIT_VM_URL` aponta para uma externa.

## Cópia de segurança e restauro {#backup-and-restore}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/backup` | admin, só sessão | O que um pacote conteria (`contents`, uma linha por secção com a sua contagem), a versão de formato do pacote, onde vive o segredo da instância (`secret_source`: `file` ou `environment`) e o estado das cópias de segurança locais agendadas (`schedule`). |
| `POST` | `/api/backup` | admin, só sessão | `{"passphrase": "…", "include_account_secrets": false}`. Devolve o próprio pacote como um ficheiro JSON: um cabeçalho em texto simples (`format`, `version`, `created_at`, `summary`, `kdf`) e um `payload` encriptado com AES-256-GCM sob uma chave derivada da frase-passe pelo Argon2id. Pelo menos 16 caracteres, ou `400`. |
| `POST` | `/api/backup/restore` | admin, só sessão | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Ensaio a menos que `apply` seja `true`; ambos executam o mesmo código, o ensaio dentro de uma transação que é revertida. Responde o relatório: por secção, `created`, `updated`, `skipped` e `notes`, mais `warnings`. `400` numa frase-passe errada, um ficheiro modificado, ou um pacote escrito numa versão de formato mais recente. |
| `POST` | `/api/backup/local` | admin, só sessão | Escreve agora mesmo uma cópia de segurança local estilo agendada e responde o novo `schedule`. |

Um pacote transporta toda credencial da instância, reencriptada com a
frase-passe, por isso estas rotas recusam tokens de API com uma
explicação: só um administrador com sessão iniciada na interface web
recebe uma. Ver [Cópia de segurança e restauro](../install/backup.md).

## Tipos de dispositivo {#device-types}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/collectors` | Todo tipo de dispositivo que este servidor pode monitorizar: `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) e `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). Os tipos trazidos por um [pacote de integração](../packs/index.md) são listados da mesma forma, como `pack.<id>`. |

## Pacotes de integração {#integration-packs}

Sessão de administrador ou token de API (âmbito `write` para
alterações). Ver [Pacotes de integração](../packs/index.md).

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/packs` | Pacotes instalados: `id`, `version`, `label`, `summary`, `kind` (`null` para um pacote só com perfis SNMP), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uids), `snmp_profiles`, `targets` (dispositivos a usá-lo), `warnings`, e `error` quando o pacote guardado já não passa a validação. |
| `GET` | `/api/packs/{id}` | Um pacote. |
| `POST` | `/api/packs` | Instalar ou atualizar. Corpo: o YAML (`application/yaml`, `text/plain`…) ou `{"yaml": "…"}` em JSON. `201` quando criado, `200` quando atualizado ou inalterado, com `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` e `restart_required` (perfis SNMP). `400` lista todo erro de validação. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Registar ou retirar o tipo de dispositivo do pacote; dispositivos, histórico e regras permanecem. |
| `DELETE` | `/api/packs/{id}` | Desinstalar, com as suas regras. `409` enquanto dispositivos o usam. |

## Destinos (dispositivos) {#targets-devices}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets` | Lista todo dispositivo. |
| `POST` | `/api/targets` | Criar um. `201` com o dispositivo. A deteção de perfil inicia em segundo plano para SNMP. |
| `GET` | `/api/targets/{id}` | Um dispositivo. |
| `PUT` | `/api/targets/{id}` | Substitui-o. Omitir `credential` mantém a guardada; omitir `profile_id` mantém o perfil detetado (`""` limpa-o). Definir `enabled: false` limpa os alertas do dispositivo sem notificar. |
| `DELETE` | `/api/targets/{id}` | `204`. Limpa os alertas do dispositivo sem notificar e elimina as suas séries temporais da VictoriaMetrics (melhor esforço). |
| `POST` | `/api/targets/{id}/probe` | Verificar agora: `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — as chaves de série como o coletor as produziu, sem o prefixo `dumbmonit_` que o escritor adiciona no caminho para a VictoriaMetrics. |
| `POST` | `/api/targets/{id}/discover` | Executar de novo a deteção de perfil: `{"profile_id": "host-resources"}` ou `null`. |

Um dispositivo como devolvido:

```json
{
  "id": 4,
  "name": "Core switch",
  "address": "192.168.1.2",
  "kind": "snmp",
  "profile_id": "host-resources",
  "parent_id": null,
  "via_agent": null,
  "interval_secs": 60,
  "enabled": true,
  "tags": {"role": "network"},
  "credential_kind": "SNMP community",
  "last_probe_at": "2026-09-15 13:20:05",
  "last_error": null,
  "error_kind": null
}
```

O próprio segredo nunca é devolvido: `credential_kind` é um rótulo.
As data-horas do servidor são UTC sem sufixo. `error_kind` classifica
`last_error`: `down` (o dispositivo não respondeu) ou `config` (do
nosso lado — credenciais, endereço, opção), que é o que a interface
mostra como *Unreachable* ou *Misconfigured*; `null` quando a última
verificação teve sucesso.

Criar um:

```json
{
  "name": "Core switch",
  "address": "192.168.1.10",
  "kind": "snmp",
  "interval_secs": 60,
  "parent_id": null,
  "tags": {},
  "credential": {"type": "snmp_community", "community": "public"}
}
```

Formas de credencial (`type`): `none`; `snmp_community`
(`community`); `snmp_v3` (`username`, `auth: {protocol, passphrase}`
opcional com `md5`, `sha1`, `sha224`, `sha256`, `sha384` ou `sha512`,
`privacy: {protocol, passphrase}` opcional com `des`, `aes128`,
`aes192` ou `aes256`, `context` opcional); `api_token` (`token`, ou as
duas metades `token_id` e `secret`, que o servidor junta em
`user@realm!name=secret` para o Proxmox VE e o PBS);
`username_password` (`username`, `password`).
`interval_secs` tem predefinição 60 e não pode ir abaixo de 10.
`name` tem no máximo 200 caracteres e `address` no máximo 253 (`400`
além disso). `parent_id` tem de nomear um dispositivo existente
(`400 Parent device N not found.`). As opções de tipo vão em `tags`
sob a sua chave.

`via_agent` nomeia o [agente relay](../install/remote-site.md) que
sonda o dispositivo a partir da sua própria rede em vez do servidor;
`null` (a predefinição) significa que o servidor o sonda. Tem de ser
um dispositivo `agent` (`400` caso contrário), um dispositivo não
pode relayar-se a si mesmo, e um dispositivo `agent` não pode ser
relayed. Num `PUT`, um `via_agent` *omitido* mantém o valor guardado,
como `profile_id` e `credential`; envie `null` para voltar à
sondagem direta. `POST /api/targets/{id}/probe` e `/discover` num
dispositivo relayed passam pelo relay e esperam pela sua resposta
(`409` quando uma verificação já está em curso,
`400 Timed out: relay agent …` quando não responde).

## Descoberta {#discovery}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `POST` | `/api/discovery` | admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Analisa uma rede por dispositivos SNMP. `cidr` é obrigatório; `community` tem predefinição `public`; `timeout_ms` é limitado a 100–10000. Redes maiores do que 4096 endereços são recusadas. A análise é um `POST` com um corpo JSON para que a comunidade nunca apareça num URL ou numa linha de registo. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Guia de primeira utilização {#first-run-guide}

Os três passos que a visão geral mostra numa instância nova. O estado
é guardado na instância, não no navegador, por isso dispensar um
passo dispensa-o em todo lado.

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/onboarding` | sessão | Onde a instância está. |
| `PUT` | `/api/onboarding` | admin | `{"skipped": true}` arreda todo o guia; `{"dismissed": ["test"]}` arreda passos individuais. Um campo omitido mantém o seu valor guardado; um id de passo que não seja `device`, `channel` ou `test` é um `400`. Devolve o mesmo documento que `GET`. |

```json
{
  "skipped": false,
  "complete": false,
  "completed_at": null,
  "dismissed": [],
  "has_target": true,
  "has_channel": true,
  "notification_confirmed": false
}
```

`has_target`, `has_channel` e `notification_confirmed` são
recalculados em cada leitura: são factos sobre a instância, não flags
que a interface define. `notification_confirmed` é verdadeiro uma vez
que uma mensagem realmente saiu de algum canal — uma mensagem de
teste conta, um canal que só foi criado não.

`complete` fica preso e nunca volta a falso. O servidor define-o
quando os três passos estão resolvidos (feitos, ou dispensados), e
também na primeiríssima leitura de uma instância que já tinha um
dispositivo e um canal — por isso atualizar uma instância
estabelecida nunca lhe mostra um guia de primeira utilização.

## Métricas {#metrics}

Ambas as rotas reencaminham a VictoriaMetrics e devolvem séries na
forma Prometheus:
`[{"metric": {labels…}, "values": [[timestamp_seconds, "value"], …]}]`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Consulta MetricsQL instantânea. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Consulta de intervalo. `start` e `end` são **milissegundos** desde a época; `step` está em segundos e é alargado para que uma série nunca exceda 2000 pontos. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Consultar (fora de `/api`) {#scraping-outside-api}

Para um Prometheus ou Grafana existente. Ambos tomam um token de API
`read` em `Authorization: Bearer dmt_…` (um cookie de sessão também
funciona); um consultor não envia nenhum cabeçalho
`X-Requested-With` e não precisa de nenhum — estas rotas só lêem. Ver
a [referência de métricas](metrics.md#scraping-dumbmonit) para um
`scrape_config` pronto a copiar e a fonte de dados Grafana.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/metrics` | Saúde da própria instância no formato de exposição de texto Prometheus: escalonador, verificações, escritas, alertagem, notificações, agentes, base de dados, VictoriaMetrics. Algumas dezenas de séries, nunca uma por dispositivo. |
| `GET` | `/federate` | As medições, no mesmo formato. `match[]` seleciona séries (repetível, no máximo 10, `{__name__=~"dumbmonit_.*"}` por predefinição), `max_lookback` alarga a janela em que o último ponto é procurado (`5m` por predefinição). `400` além de 8 MiB, com o que restringir. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | A metade de leitura da API Prometheus, reencaminhada para o armazenamento, para que o Grafana possa usar `http://…/prometheus` como uma fonte de dados Prometheus. Só `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` e `status/buildinfo`; qualquer outra coisa é `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` serve as três sem token — só atrás de
uma firewall, já que transportam toda medição de todo dispositivo.

## Alertas {#alerts}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts` | Alertas ativos (pending, firing, suppressed, recentemente resolvidos). Os alertas de dispositivos eliminados ou pausados nunca são listados. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Transições de fase. `since` é RFC 3339, predefinição os últimos sete dias; `limit` tem de ser positivo. `dismissed` (predefinição `false`) exclui entradas limpas com as duas rotas abaixo; nada é alguma vez destruído por uma limpeza, só escondido da vista predefinida. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Limpa uma entrada `resolved` da vista de histórico predefinida. `204`. `404` num id desconhecido ou uma transição ainda ativa. Operator; auditado. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Limpa toda entrada `resolved` ainda não limpa de uma vez («Clear all resolved»). Devolve `{"dismissed": <count>}`. Operator; auditado. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Confirmar: `{"duration_secs": 14400, "note": "…"}` ou `{"until": "2026-09-22T18:00:00Z"}` (um dos dois; nenhum significa 4 horas, no máximo 30 dias). `{"until": null}` levanta-a. Devolve o alerta. Operator; auditado. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Levantar a confirmação. Devolve o alerta. Operator. |

Um alerta ativo:

```json
{
  "fingerprint": "…",
  "rule_uid": "disk_almost_full",
  "rule_name": "Disk almost full",
  "severity": "warning",
  "target_id": 4,
  "series_key": "…",
  "labels": {"host": "Core switch", "mountpoint": "/", "target": "4"},
  "phase": "firing",
  "effective_phase": "suppressed",
  "suppressed": true,
  "suppressed_by": 2,
  "silenced": false,
  "learning": false,
  "acked": true,
  "acked_until": "2026-09-15T17:20:00Z",
  "acked_by": "admin",
  "ack_note": "replacing the disk",
  "value": 95.96,
  "score": null,
  "condition_since": "2026-09-15T12:00:00Z",
  "firing_since": "2026-09-15T12:15:00Z",
  "last_eval_at": "2026-09-15T13:20:00Z",
  "last_notified_at": null,
  "notify_count": 0
}
```

Uma entrada de histórico tem `id`, `fingerprint`, `rule_uid`,
`target_id`, `from_phase`, `to_phase`, `severity`, `value`,
`notified`, `reason` (vazio, ou porque nada foi enviado:
`learning: would have fired`, `suppressed: device 2 unreachable`,
`maintenance window`, `acknowledged by admin`, `device removed or
disabled`), `at` e `dismissed` (limpo da vista ativa; a própria linha
é mantida). `acked` é verdadeiro enquanto `acked_until` está no
futuro: lembretes e escalonamentos pausam, a resolução continua a ser
notificada e limpa a confirmação. Um `target_id` que já não existe é
mostrado como «(deleted device)» pela interface.

### Regras {#rules}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts/rules` | Toda regra. |
| `POST` | `/api/alerts/rules` | Criar. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Atualizar. Uma regra incorporada mantém a sua consulta. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. As regras incorporadas não podem ser eliminadas. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Devolve a regra. |

Uma regra:

```json
{
  "id": 2, "uid": "cpu_high", "name": "High CPU",
  "description": "CPU load sustained above 90%.",
  "kind": "threshold",
  "query": "avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)",
  "operator": ">", "threshold": 90, "for_secs": 600,
  "severity": "warning",
  "selector": {"kind": "all"},
  "channels": [],
  "params": {"k": 3.5, "alpha": 0.05, "mad_floor_abs": 1e-6, "mad_floor_rel": 0.01, "min_samples": 3},
  "unit": "%", "repeat_secs": 21600, "escalate_after_secs": 3600,
  "enabled": true, "builtin": true
}
```

Criar uma precisa pelo menos de `name` e `query`; `kind`
(`threshold`, `anomaly`, `predict`), `operator`, `threshold`,
`for_secs`, `severity` (`info`, `warning`, `critical`), `selector`
(`{"kind":"all"}`, `{"kind":"ids","ids":[…]}` ou
`{"kind":"labels","labels":{…}}`), `channels` (ids de canal, vazio
significa todo canal ativado), `unit`, `repeat_secs`,
`escalate_after_secs` e `enabled` são opcionais.

### Silêncios (janelas de manutenção) {#silences-maintenance-windows}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts/silences` | Toda janela, com `active_now`, `active_until` e `next_start_at` (RFC 3339, calculado pelo servidor). |
| `POST` | `/api/alerts/silences` | Criar. `201`. Operator. |
| `DELETE` | `/api/alerts/silences/{id}` | `204`. Operator. |

```json
{
  "name": "Sunday backups",
  "comment": "NAS is busy",
  "target_id": 4,
  "matchers": {},
  "schedule": {"kind": "weekly", "days": [6], "start_minute": 120, "end_minute": 240, "timezone": "Europe/Paris"},
  "enabled": true
}
```

Um calendário pontual é
`{"kind": "once", "starts_at": "2026-09-20T22:00:00Z",
"ends_at": "2026-09-21T02:00:00Z"}`. Um mensal é
`{"kind": "monthly", "days": [1], "nth_weekdays": [{"nth": 1, "weekday": 6}],
"start_minute": 120, "duration_minutes": 120, "timezone": "Europe/Paris"}` —
`nth` é 1 a 5, ou `-1` para o último do mês; um mês sem essa
ocorrência é saltado. Os dias são 0 = segunda-feira … 6 = domingo (1–31
para um `days` mensal); os minutos são desde a meia-noite local
(0–1439). Num calendário recorrente, `timezone` (um nome IANA) ganha
sobre `utc_offset_minutes` e é o que mantém uma janela à sua hora
local através das mudanças de hora de verão; um nome de fuso
desconhecido é recusado.

### Substituições por dispositivo {#per-device-overrides}

Uma regra pode ser ajustada para um dispositivo sem tocar na regra em
si. `{"enabled": false}` é o que a ação **Ignore** da interface (no
alerta, na página do dispositivo ou na regra) escreve: o dispositivo
é retirado dessa única regra para sempre, reversivelmente, sem
desativar a regra para mais ninguém.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Toda substituição, ou as de um dispositivo: `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | As substituições de uma regra. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — cada campo opcional; `null` significa «como a regra». Cria ou substitui. Um operator só pode enviar `{"enabled": false}` (ignorar); as substituições de limiar do dispositivo, se existirem, são mantidas. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Para um operator, só levanta o ignorar: as substituições de limiar permanecem. |

## Política de notificação {#notification-policy}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/notify/policy` | A política global: `batch_window_secs` (0 = enviar de imediato), `max_per_hour` por canal (0 = ilimitado), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = nenhuma deteção de oscilação), `public_url` (ligações em mensagens; vazio = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` e `escalate_channel` (0 / `null` = nenhum escalonamento). |
| `PUT` | `/api/notify/policy` | Os mesmos campos, cada um opcional: um campo omitido mantém o seu valor. `"escalate_channel": null` desliga o escalonamento. Devolve a política. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → quais dispositivos um filtro de encaminhamento de canal seleciona: `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Canais de notificação {#notification-channels}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/notify/kinds` | Todo tipo de canal com os seus campos `settings` e `secrets` (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) e um `doc_url` como `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Todo canal: `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). Os segredos nunca são devolvidos. |
| `POST` | `/api/notify/channels` | Criar. `201`. |
| `PUT` | `/api/notify/channels/{id}` | Atualizar. Omitir `secrets` mantém os guardados; `"secrets": {}` limpa-os. Em `policy`, omitir `matcher` mantém o filtro de encaminhamento e `"matcher": null` limpa-o. |
| `DELETE` | `/api/notify/channels/{id}` | `204`. |
| `POST` | `/api/notify/channels/{id}/test` | Enviar uma mensagem de teste: `{"ok": true, "message": "…"}`. |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

As chaves exatas por tipo vêm de `/api/notify/kinds` e estão
documentadas em [Canais de notificação](../notifications.md).

### Dispositivos Web Push {#web-push-devices}

Os dispositivos da **conta com sessão iniciada** (só sessão, leitores
incluídos; os tokens de API recebem `401`). Um canal `webpush` envia
para eles; ver
[Notificações push (PWA)](../using/push-notifications.md).

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`: a chave pública VAPID (base64url, o `applicationServerKey` com que subscrever) e os dispositivos da conta — `id`, `device` (`Firefox on Android`), `push_service` (anfitrião), `fingerprint` (primeiros 16 caracteres hex do SHA-256 do endpoint), `created_at`, `last_success_at`, `last_error`. O endpoint, as chaves do navegador e a chave privada nunca são devolvidos. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`: `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` com o dispositivo. O endpoint tem de ser HTTPS num endereço público: endereços privados, loopback e link-local recebem `400`. O mesmo endpoint de novo substitui a sua linha. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`; `404` para um dispositivo de outra conta. |
| `POST` | `/api/webpush/test` | `{}` ou `{"id": 3}`. Envia uma notificação de teste: `{"ok", "delivered", "failed", "removed", "message"}`. As subscrições respondidas com `404`/`410` são removidas. |

## Relatórios por email {#email-reports}

Só administradores, `GET` incluído (a resposta lista destinatários).
Os tokens bearer precisam do âmbito `write` para alterações. Ver
[Relatórios por email](../using/reports.md).

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | admin | Todo relatório: `id`, `name`, `enabled`, `frequency`, `weekday` (0 é segunda-feira), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | admin | Os mesmos campos escrevíveis; cada um por predefinição semanal, segunda-feira, 8:00, `UTC`. `recipients` detém no máximo 20 endereços e é obrigatório para ativar o relatório; `channel_id` tem de ser um canal `smtp` ou `null`. `201`; `409` além de 10 relatórios. |
| `GET` | `/api/reports/schedules/{id}` | admin | Um relatório. |
| `PUT` | `/api/reports/schedules/{id}` | admin | Substituição completa. |
| `DELETE` | `/api/reports/schedules/{id}` | admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | admin | Envia por email uma pré-visualização aos destinatários guardados: `{"sent": 2, "failed": 0}`. Não toca no calendário. `400` sem um canal de email usável; `409` dentro de 30 segundos da pré-visualização anterior. |

## Páginas de estado e incidentes {#status-pages-and-incidents}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/status-pages` | sessão | Toda página com os seus itens: `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) e `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` é obrigatório (no máximo 120 caracteres); `slug` (`^[a-z0-9-]{2,40}$`) é derivado do título quando omitido; `theme` é `auto` (predefinição), `light` ou `dark`; `show_uptime_days` vai de 7 a 90 (90 por predefinição). `201`; `409` num slug já ocupado. |
| `GET` | `/api/status-pages/{id}` | sessão | Uma página com os seus itens. |
| `PUT` | `/api/status-pages/{id}` | admin | Os mesmos campos, mas uma substituição completa em vez de um patch: `title` é obrigatório e todo campo omitido volta à sua predefinição (slug re-derivado do título, descrição vazia, `published: false`, `theme: auto`, 90 dias de disponibilidade). |
| `DELETE` | `/api/status-pages/{id}` | admin | `204`. O URL público deixa de responder. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — aspeto e subscrições | admin | Também aceitam `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (ids de cenário da lista do editor — cidades, as novas maravilhas e as maravilhas antigas, por exemplo `venice`, `paris`, `giza`, `tajmahal`, sem duplicados; `[]` para nenhum; omitido na criação: `["paris"]`), `simple` (booleano, predefinição `false`: a página pública não mostra nenhum cenário, mascote ou animação e segue a preferência clara/escura do visitante), `scene_rotation` (`visit`, `1m`, `10m` ou `1h`), `footer_text` (texto simples, 280 caracteres), `homepage_url` (`http(s)://` ou vazio), `subscribe_channel_id` (um canal `smtp`, ou `null` para desligar a subscrição por email) e `domain` (o nome de anfitrião público da página, como `status.example.com`, ou `null`/`""` para nenhum — ver [Domínio personalizado](../using/status-pages.md#custom-domain)). `domain` fica em minúsculas e perde um prefixo `https://` ou barra final; uma porta, caminho ou endereço IP é `400`, o endereço do próprio DumbMonit (o seu URL público, ou o anfitrião em que o pedido chegou) é `400`, e um nome que outra página usa é `409`. Ao contrário dos outros campos, estes oito mantêm o seu valor guardado quando omitidos de um `PUT`. A vista da página devolve-os com `logo_type` (`image/png`, `image/jpeg`, `image/webp` ou `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | sessão / admin | O logo da página. `PUT` toma `{"data": "<base64 ou data: URL>"}`: PNG, JPEG ou WebP reconhecido pelo conteúdo, no máximo 256 KiB, qualquer outra coisa `400`. `204`. `GET` é a pré-visualização do editor (rascunhos incluídos). |
| `GET` | `/api/status-pages/{id}/subscribers` | sessão | Subscritores por email: `id`, `page_id`, `email`, `confirmed_at` (`null` enquanto pendente), `created_at`. Os seus tokens nunca saem do servidor. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | admin | Remove um endereço. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — a lista completa e ordenada de dispositivos mostrados na página. |
| `GET` | `/api/incidents` | sessão | Todo incidente com as suas atualizações: `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) e `updates`. |
| `POST` | `/api/incidents` | admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` é a primeira atualização. `201`. |
| `PUT` | `/api/incidents/{id}` | admin | Os mesmos campos; um campo omitido mantém o seu valor. |
| `DELETE` | `/api/incidents/{id}` | admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | sessão | A linha do tempo: `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | admin | `{"status": "monitoring", "body": "…"}`. Adiciona uma atualização e move o incidente para `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | público | O documento JSON a partir do qual uma página publicada é construída (também `badge.svg` e `rss` sob o mesmo caminho). Em `page`, `show_uptime_days` é o máximo configurado e `history_days` a janela realmente mostrada (a idade dos dados da página, de 7 dias até esse máximo) e `simple` reflete o modo simples. Ver [Páginas de estado](../using/status-pages.md). No domínio personalizado de uma página (`Host` igual ao seu `domain`), só as rotas `/api/public/status/{slug}/…` desta página respondem; toda outra rota desta referência é `404` aí. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | público | Emblemas de página: disponibilidade média sobre `?days=` `1`, `7`, `30` (predefinição) ou `90` — `400` além do histórico da página — e tempo de resposta médio. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | público | Os mesmos três emblemas para um serviço; `key` é o `key` do serviço no documento público (o seu rótulo em forma de URL). `404` para uma chave desconhecida. |
| `GET` | `/api/public/status/{slug}/logo` | público | O logo da página com o seu tipo de imagem, em cache por um dia (o `logo_url` do documento transporta uma versão). `404` sem um. |
| `POST` | `/api/public/status/{slug}/subscribe` | público | `{"email": "…"}`. Envia por email uma ligação de confirmação através do canal SMTP da página. Sempre `202` com a mesma mensagem, seja qual for o estado do endereço; `400` para um endereço inválido; `404` quando a página não oferece subscrição por email; `409` quando a página não pode receber mais subscritores; `429` com `Retry-After` além de cinco pedidos por cliente e sessenta no total por 15 minutos. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | público | Confirma uma subscrição (a ligação do email de confirmação abre `/s/{slug}/confirm`, cujo botão publica aqui). `404` para um token desconhecido ou expirado. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | público | Remove uma subscrição; o corpo é ignorado, por isso também é o destino de um clique RFC 8058 de `List-Unsubscribe`. Sempre `200`. |

## Tokens de agente e ingestão {#agent-tokens-and-ingest}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | só sessão | Todo token: `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` para um token de frota), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | admin, só sessão | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` com os campos do token mais `secret` (mostrado uma vez), `install_linux` e `install_windows`. `base_url` é o URL que os agentes usarão; tem predefinição o endereço de escuta. Sem `reusable`, o token é **de uso único**: regista uma máquina e não mais. Com ele, `max_uses` limita os registos (`null`: sem limite). `expires_in_days` para *registos* depois desses dias; as máquinas já registadas continuam a reportar. `400` numa contagem abaixo de 1. |
| `DELETE` | `/api/agent/tokens/{id}` | admin, só sessão | Revogar. `204`; `404` quando desconhecido ou já revogado. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Recebe um lote de amostras de um agente (corpos até 16 MB). Não destinado a ser chamado manualmente. O cabeçalho transporta o segredo de associação da máquina; um agente que não tem nenhum omite-o, o que é como pede um. A resposta adiciona `agent_secret` (o segredo, devolvido exatamente uma vez, quando a máquina é associada) e `bound`. `403` quando a máquina está associada a outra instalação de agente, ou quando o token já não pode registar — a mensagem diz qual, e o que fazer. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Verificações delegadas a um agente relay (`relay: true`). Retido até `wait` segundos (25 no máximo) quando nada está pendente. Cada item é um comando do tipo `probe` cujos `args` transportam o destino, a sua credencial desencriptada, `timeout_secs` e `discover`. Nunca escrito em disco. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Resultado de uma verificação delegada: `{"duration_ms", "error", "samples", "profile_id"}` (corpos até 16 MB). `204`; `404` quando a verificação expirou ou pertence a outro agente. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Comandos enfilerados para um agente (reinício ou atualização de contentor), retidos até `wait` segundos quando nada está pendente. Não destinado a ser chamado manualmente. `key` por si só não prova nada — é pública — por isso uma máquina associada também tem de apresentar o seu segredo de associação; `403` caso contrário, o que é o que impede uma máquina de tomar os comandos de outra. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Progresso e resultado de um comando, reportado pelo agente. `result` é truncado aos últimos 4 KB, com `[truncated by the server, beginning dropped]` na frente quando isso acontece. |
| `GET` | `/api/relays` | sessão | Todo dispositivo de agente como um possível relay: `id`, `name`, `site`, `relay` (declarado `relay: true`), `last_seen_at`, `relayed` (dispositivos alcançados através dele). Relays primeiro, depois por nome. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Contentores e comandos (dispositivos de agente) {#containers-and-commands-agent-devices}

Ações nos contentores de uma máquina que executa o agente. Toda rota
responde `404` quando o dispositivo não é um destino `agent`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | A máquina como o seu agente a descreveu por último: `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (dispositivos alcançados através deste agente), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404` até um agente ter reportado. `commands_supported` só é `true` quando o agente declarou que obtém comandos (`commands: true`, a predefinição dos agentes atuais); um agente mais antigo ou um com `commands: false` dá `false`. `binding` é `bound`, `pending` (o binário pode ser associado e será no seu próximo lote) ou `unsupported` (um agente mais antigo do que a associação: reinstale-o). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Abre uma janela de uma hora durante a qual esta máquina pode associar-se de novo com um token de registo válido — o caminho de volta depois de uma reinstalação ter levado o segredo do agente com ela. `{"rebind_until", "minutes"}`. A janela fecha-se assim que é usada, e o antigo segredo deixa de funcionar. `404` até um agente ter reportado. |
| `GET` | `/api/targets/{id}/containers` | Todo contentor do último lote: `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Todo campo é opcional: um campo omitido mantém o seu valor guardado. Devolve a política completa. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Enfilerar um reinício. `201` com o comando. `404` quando `name` não está no inventário do agente; `409` quando o mesmo comando já está enfilerado ou em execução, ou quando o agente não pode executar comandos (ver `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Enfilerar uma atualização; `{"prune": bool}` opcional (predefinição a política). Os mesmos códigos de estado que restart. |
| `GET` | `/api/targets/{id}/commands` | Os últimos 20 comandos, mais recentes primeiro: `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (um utilizador, ou `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Cancelar um comando enfilerado. `204`; `409` uma vez o agente o ter recolhido ou quando já está fechado; `404` quando não pertence a este dispositivo. |

`status` vai `queued → running → done | failed`. Existem dois outros
estados finais: `cancelled` (uma pessoa retirou o comando) e
`expired` (o servidor desistiu depois de dez minutos porque nenhum
agente veio obtê-lo — agente parado, demasiado antigo para o canal de
comando, ou instalado com `commands: false`). O servidor expira
comandos obsoletos a cada minuto por si próprio; um comando enfilerado
nunca bloqueia um novo por mais tempo do que isso.

## Convidados do Proxmox VE {#proxmox-ve-guests}

As linhas do painel **Guests** de um dispositivo Proxmox VE,
montadas a partir da última verificação no armazenamento de
métricas (quatro consultas instantâneas, nada pedido ao hipervisor).
`404` quando o dispositivo não existe, `400` quando não é um destino
`proxmox`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Um objeto por VM ou contentor, ordenado por nó depois VMID: `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (dos núcleos atribuídos), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` o agente convidado respondeu, `false` ativado mas silencioso, `null` nenhum), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (o seu pool de recursos), `lock` (só enquanto um bloqueio é detido: `backup`, `migrate`…), `os` e `ip` (vistos de dentro do convidado, atualizados a cada hora). Todo valor desconhecido é `null` — uma VM sem agente convidado tem `disk_total_bytes` mas `disk_used_bytes: null`; um convidado parado mantém os seus tamanhos e perde as suas medições. |

```json
[
  {
    "vmid": 202, "name": "nextcloud", "node": "pve2", "kind": "lxc", "status": "running",
    "cpu_percent": 1.0, "cpu_count": 4,
    "memory_used_bytes": 1879048192, "memory_total_bytes": 4294967296, "memory_percent": 43.75, "balloon_bytes": null,
    "disk_used_bytes": 61203283968, "disk_total_bytes": 107374182400, "disk_percent": 57.0, "agent": null,
    "network_in_bps": 1024.0, "network_out_bps": 512.0, "disk_read_bps": 0.0, "disk_write_bps": 2048.0,
    "uptime_seconds": 3196800, "last_backup_age_seconds": 25200, "ha_state": "started",
    "pool": "production", "lock": null,
    "os": "Debian GNU/Linux 12 (bookworm)", "ip": "192.168.10.60"
  }
]
```

## Nós e Ceph do Proxmox VE {#proxmox-ve-nodes-and-ceph}

As secções **Nodes** e **Ceph** da mesma página de dispositivo, lidas
da mesma última verificação. Mesmos códigos de estado que acima.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — o estado do watchdog HA (`armed`, `standby`…) e se realmente isolaria um nó perdido, depois um objeto por nó, ordenado por nome: `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (a versão `pve-manager` instalada nesse nó), `services_down` (nomes de unidade dos daemons parados ou falhados), `interfaces_offline` (interfaces definidas para iniciar no arranque que não estão ativas), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) e `volume_groups` (`name`, `used_percent`, `size_bytes`). Um nó que deixou de responder mantém a sua entrada com `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false` quando o cluster não tem Ceph — a interface então não desenha nenhuma secção de todo), `health` (0 OK, 1 WARN, 2 ERR, 3 desconhecido), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (nomes CephFS), `flags` (flags de OSD atualmente definidas, como `noout`) e `muted_checks` (verificações de saúde silenciadas, que `HEALTH_OK` já não menciona). |

## Proxmox Backup Server {#proxmox-backup-server}

Os painéis de um dispositivo `pbs`, lidos da última verificação (nada
pedido ao servidor exceto o registo de tarefas). `404` quando o
dispositivo não existe, `400` quando não é um destino `pbs`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`: uma entrada por grupo de cópia de segurança (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) com um array `days` de `{date, state, runs, snapshot}`. `offset` é o desvio UTC do visitante em minutos, para que os dias sejam cortados à meia-noite local. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Tarefas falhadas: `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`: tarefas de sync, verify, prune e garbage-collection com o seu calendário e último resultado. |
| `GET` | `/api/targets/{id}/pbs/health` | Datastores (utilização, data estimada de enchimento), discos (SMART, desgaste) e pools ZFS. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | O registo de uma tarefa, obtido do servidor: `{"upid", "lines": […]}`; `lines` limita quantas são devolvidas. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | Atributos SMART de um disco; `disk` é obrigatório (`400` sem ele). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager: totais do património (`estate`) e uma linha por instância federada (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, contagens de nó e convidado, memória e armazenamento, `subscription`, `last_collection`, `tasks_failed`), inacessíveis primeiro. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Tarefas que falharam em todo o património: `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | O anfitrião da consola: CPU, memória, sistema de ficheiros raiz, tempo de atividade, certificados, atualizações pendentes e subscrição. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway: as quatro filas Postfix pela ordem de leitura (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), mais `total_messages` e um `stuck` à escala do gateway. `oldest_age_seconds` é um limite inferior: o qshape reporta intervalos de idade. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Os totais de hoje (`mail`), a curva de tráfego recente (`recent`), o histograma de pontuação de spam, os vírus apanhados hoje, e as contagens de quarentena (`quarantine`). Só contagens: nenhum conteúdo de mensagem. |
| `GET` | `/api/targets/{id}/pmg/health` | Uma entrada por nó (sistema, serviços, `signatures` com `family`, `age_seconds` e `stale`, `expiring_certificates`, atualizações, subscrição), os membros do `cluster` com o seu estado de sincronização, e `stopped_services` entre nós. |

## Synology {#synology}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Vista geral de um dispositivo `synology` da última verificação: `system` (modelo, versão DSM, tempo de atividade, temperatura, CPU, memória), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) e `sampled_at`. `404` quando o dispositivo não existe, `400` quando não é um Synology. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business: `tasks`, e `devices` — um relatório por dispositivo protegido com o seu ritmo aprendido em `assessment` (`state`, `last_success_s`, `typical_interval_s`, contagens de 30 dias). |

## Heartbeats (monitores push) {#heartbeats-push-monitors}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` ou `POST` | `/api/push/{token}` | nenhuma (o token) | A chamada que uma tarefa faz cada vez que corre. Opcional `?status=up\|down&msg=…` (compatível com Uptime Kuma). `204` vazio no sucesso, `404` para um token desconhecido, `400` para outro `status`, `429` com `Retry-After` além de sessenta chamadas por minuto por token. Ver [Heartbeat](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | sessão | O monitor de um dispositivo `push`: `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Criado na primeira leitura. `400` quando o dispositivo não é um heartbeat. |
| `POST` | `/api/targets/{id}/push/regenerate` | admin | Novo token; o URL anterior responde `404` a partir de então. Mesmo corpo que a leitura. |

## Assistentes (MCP) {#assistants-mcp}

O servidor [Model Context Protocol](https://modelcontextprotocol.io)
incorporado, aquele com que um assistente fala. Autentica com os
tokens de API acima — mesmos âmbitos, expiração, redes e limite de
taxa — e nunca com uma sessão.

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `POST` | `/api/mcp` | token de API | JSON-RPC 2.0 sobre o transporte Streamable HTTP, um pedido por `POST`, respondido em JSON. Protocolo `2026-07-28` (sem estado: todo pedido transporta a sua versão e capacidades de cliente em `_meta`, mais os cabeçalhos `MCP-Protocol-Version`, `Mcp-Method` e `Mcp-Name`; `server/discover`), e `2025-11-25`, `2025-06-18` e `2025-03-26` através do handshake `initialize`. Nenhuma sessão: nenhum `Mcp-Session-Id` é emitido, um enviado é ignorado. Uma notificação recebe `202` e nenhum corpo. |
| `GET`, `DELETE` | `/api/mcp` | público | `405` com `Allow: POST` — não há nenhum fluxo server-sent e nenhuma sessão para terminar; o corpo só diz o que este endpoint é. |

`tools` é a única capacidade — sem recursos, sem prompts, sem OAuth.
Das 27 ferramentas, um token `read` pode chamar as 15 que só lêem
(`get_status`, `list_devices`, `get_device`, `list_alerts`,
`alert_history`, `query_metrics`, `list_silences`, `list_rules`,
`list_device_types`, `list_agents`, `list_containers`,
`list_heartbeats`, `list_status_pages`, `list_channels`,
`list_packs`); as 12 que alteram algo (`silence_device`,
`remove_silence`, `acknowledge_alert`, `probe_device`,
`set_device_enabled`, `set_rule_enabled`, `add_device`,
`discover_network`, `schedule_maintenance`, `restart_container`,
`test_channel`, `post_incident`) precisam de um token `write`. As
ferramentas chamam o mesmo código que a interface web. Versões,
cabeçalhos, códigos de erro e as notas de segurança estão em
[Ligar um assistente](../using/assistant.md#protocol-details).

## Ficheiros de agente (fora de `/api`) {#agent-files-outside-api}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/install.sh` | O instalador Linux, macOS e FreeBSD. Público. |
| `GET` | `/install.ps1` | O instalador Windows. Público. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, servidos de `DUMBMONIT_AGENT_DIR`; `{name}.sha256` dá o seu checksum em formato `sha256sum`. `404` se o ficheiro está ausente. `dumbmonit-agent-macos-aarch64` e `dumbmonit-agent-macos-x86_64` (e os seus `.sha256`) respondem `307` para o mesmo ficheiro na última versão GitHub. Público. |

Todo outro caminho é servido pela interface web, que lhe pede para
iniciar sessão ela própria.
