# API HTTP {#http-api}

Tudo que a interface web faz passa por `/api/*`, e a mesma API está aberta
aos seus próprios scripts, painéis e assistentes: não há uma metade
privada. É JSON simples sobre HTTP, autenticado por um cookie de sessão (a
interface web) ou por um token de API (tudo mais).

!!! warning "Antes da 1.0"
    O DumbMonit está numa versão anterior à 1.0 e sua API também: rotas e campos ainda podem
    mudar. A versão da API (abaixo) muda sempre que uma rota ou campo é
    removido ou renomeado, para que um cliente saiba.

## Descrição OpenAPI {#openapi-description}

Toda instância descreve sua própria API em um documento
[OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0):

```
GET /api/openapi.json
```

É público — não carrega nenhum segredo, nenhum endereço e nada sobre seus
monitores — e é a referência exaustiva: toda rota que um cliente pode
chamar, seus parâmetros, formas de requisição e resposta, respostas de
erro e o escopo que precisa (a extensão `x-token-scope` em cada operação:
`public`, `read`, `write`, ou `none` para rotas que um token nunca pode
chamar). As tabelas mais abaixo nesta página são um resumo legível da
mesma coisa.

Deixado de fora de propósito: o protocolo de agente (`/api/ingest`,
`/api/agent/commands*`, `/api/agent/relay*`), que só o
[agente](../devices/agent.md) fala; os redirecionamentos de navegador do
login SSO (`/api/auth/oidc/start`, `/api/auth/oidc/callback`); e os
downloads de agente (`/install.sh`, `/install.ps1`, `/download/*`).

Carregue-o em qualquer ferramenta OpenAPI:

```bash
# Navegue, e teste chamadas com seu token, no Swagger UI
docker run --rm -p 8081:8080 \
  -e SWAGGER_JSON_URL=http://monit.lan:8080/api/openapi.json swaggerapi/swagger-ui
# depois abra http://localhost:8081 e cole "dmt_…" em Authorize → bearerAuth

# Gere uma biblioteca de cliente (aqui Python)
docker run --rm -v "$PWD:/local" openapitools/openapi-generator-cli generate \
  -i http://monit.lan:8080/api/openapi.json -g python -o /local/dumbmonit-client

# Ou simplesmente liste o que há
curl -s http://monit.lan:8080/api/openapi.json | jq -r '.paths | keys[]'
```

O Postman e o Insomnia o importam da mesma URL.

## Autenticação {#authentication}

Duas formas de entrada, aceitas em toda rota protegida:

- **Cookie de sessão** — o que a interface web usa. Entre com uma senha
  (ou SSO) e envie o cookie HttpOnly `dumbmonit_session` (SameSite Lax;
  `Secure` quando `DUMBMONIT_COOKIE_SECURE=1`; 30 dias). Toda requisição
  que altera estado (`POST`, `PUT`, `DELETE`) também precisa carregar
  `X-Requested-With: DumbMonit` — a prova anti-CSRF que a interface
  adiciona automaticamente; sem ela a requisição é recusada com `403`.
- **Token de API** — `Authorization: Bearer dmt_…`, para scripts,
  painéis e assistentes. Criado em Configurações → **API e assistentes**
  (os mesmos tokens que o [servidor MCP](../using/assistant.md) usa) e
  mostrado uma vez. Nenhum cookie está envolvido, então nenhum cabeçalho
  `X-Requested-With` é jamais necessário.

```bash
# Token de API: um cabeçalho, nada mais
curl -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/targets
curl -H 'Authorization: Bearer dmt_…' -X POST http://monit.lan:8080/api/targets/4/probe

# Sessão: entre (204 e um cabeçalho Set-Cookie), depois envie o cookie
curl -c cookies.txt -X POST http://monit.lan:8080/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"…"}'
curl -b cookies.txt http://monit.lan:8080/api/targets
curl -b cookies.txt -X POST http://monit.lan:8080/api/targets \
  -H 'X-Requested-With: DumbMonit' -H 'content-type: application/json' \
  -d '{"name":"NAS","address":"nas.lan","kind":"snmp","credential":{"type":"snmp_community","community":"public"}}'
```

Quando uma requisição carrega um token bearer, o cookie é ignorado: um
token revogado é recusado mesmo a partir de um navegador conectado, em
vez de silenciosamente recorrer à sessão.

Tokens de cadastro de agente (`dmon_…`) são uma coisa diferente: só
funcionam nas rotas de agente (`/api/ingest`, `/api/agent/commands/*`,
`/api/agent/relay*`). Eles dizem que uma máquina pode falar, não *qual*
máquina: nessas rotas uma máquina vinculada também precisa apresentar
seu próprio segredo de vínculo em `X-DumbMonit-Agent-Secret` (`dmab_…`),
que o servidor entrega uma vez no cadastro. Veja
[Vínculo](../devices/agent.md#binding-one-machine-one-agent).

O login tem limite de taxa: depois de cinco tentativas falhadas, cada
tentativa adicional é recusada com `429` e um prazo `Retry-After` que
dobra de 30 s até 5 minutos. Nunca tente adivinhar senhas em um loop.

## Escopos {#scopes}

Um token tem um de dois escopos. Não há um terceiro, mais poderoso: o que
um administrador faz na própria conta permanece fora do alcance de todo
token.

| | Token `read` | Token `write` |
|---|---|---|
| Todo `GET` que uma conta *visualizadora* pode fazer (monitores, alertas, métricas, regras, canais sem seus segredos, páginas de status…) | sim | sim |
| `POST`, `PUT`, `DELETE` que um *administrador* pode fazer (monitores, regras, silêncios, canais, páginas de status, pacotes, sondagens, descoberta…) | `403` | sim |
| `/metrics`, `/federate`, `/prometheus/*` ([coleta](metrics.md#scraping-dumbmonit)) | sim | sim |
| Ferramentas MCP de leitura / escrita ([assistentes](../using/assistant.md#what-the-assistant-can-call)) | só leitura | ambas |
| Contas, sessões, dois fatores, SSO, log de auditoria (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| Tokens de API e tokens de cadastro de agente (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Backup e restauração (`/api/backup/*`) | `403` | `403` |

Um token `read` em uma rota de escrita recebe `403` com uma mensagem
nomeando o token e o escopo faltando. Como nenhum token pode listar,
criar ou revogar tokens, um token nunca pode criar outro — muito menos
um mais poderoso.

## Segurança de token {#token-security}

- **Mostrado uma vez, armazenado hasheado.** Um token é `dmt_` seguido de
  128 bits aleatórios. O servidor mantém só seu digest SHA-256 (e os
  primeiros caracteres, `dmt_1a2b3c4d`, para reconhecê-lo em uma lista);
  o token em claro existe só na resposta de sua criação. Tokens
  apresentados são comparados em tempo constante.
- **Expiração.** Escolha 30 dias, 90 dias, 1 ano ou nunca ao criá-lo. Um
  token expirado recebe `401` com `The API token "<name>" expired on …`
  e `WWW-Authenticate: Bearer error="invalid_token"`; permanece na
  lista, marcado expirado, até você o revogar.
- **Redes permitidas.** Opcionalmente restrinja um token a uma lista de
  endereços ou faixas CIDR (`192.168.1.0/24, 10.8.0.5`, 32 entradas no
  máximo). De qualquer outro lugar recebe `403` com
  `The API token "<name>" cannot be used from <address>.` O endereço
  verificado é o do cliente TCP, ou — só quando a conexão vem de um
  proxy listado em `DUMBMONIT_TRUSTED_PROXIES` — o que esse proxy
  reporta em `X-Forwarded-For`. Detrás de um proxy reverso que você não
  declarou, toda requisição parece vir do proxy: declare-o, ou a
  restrição recusará todo mundo.
- **Último uso.** A lista de tokens mostra quando cada token foi usado
  por último e de qual endereço (atualizado no máximo uma vez por
  minuto, ou assim que o endereço muda): um token que ninguém mais usa é
  um para revogar.
- **Vinculado a seu criador.** Um token lembra a conta que o criou. Se
  essa conta é desativada, seus tokens são recusados (`401`); se é
  rebaixada a operador ou visualizador, seus tokens `write` agem como
  tokens `read`; se é excluída, seus tokens são revogados.
- **Revogação** é imediata: a próxima chamada recebe `401`.
- **Registrado.** Toda escrita feita com um token é escrita no log do
  servidor com o nome e id do token, o método, o caminho (sem a string
  de consulta) e o status — nunca o token, nunca o corpo. Criar e
  revogar um token são registrados no log de auditoria (Configurações →
  Segurança).

## Limites de taxa {#rate-limits}

Cada token pode fazer 120 chamadas por minuto, REST e MCP juntos. Além
disso, chamadas são recusadas com `429` e um cabeçalho `Retry-After`
(segundos) até o minuto terminar. Toda resposta a uma requisição
autenticada por token diz onde você está:

| Cabeçalho | Significado |
|---|---|
| `RateLimit-Limit` | Chamadas permitidas por janela (120). |
| `RateLimit-Remaining` | Chamadas restantes na janela atual. |
| `RateLimit-Reset` | Segundos até a janela recomeçar. |

Um painel Grafana com muitos painéis e uma atualização curta pode
alcançar o limite: dê a ele seu próprio token, ou aumente a atualização.

## Requisições entre origens (CORS) {#cross-origin-requests-cors}

Uma página servida de outra origem — seu próprio painel, Swagger UI, um
cliente MCP baseado em navegador — pode chamar a API **com um token
bearer, nunca com uma sessão**:

- uma resposta carrega `Access-Control-Allow-Origin` só quando a
  requisição carregou `Authorization: Bearer dmt_…` (ou para o
  `GET /api/openapi.json` público). `Access-Control-Allow-Credentials`
  nunca é enviado, então um navegador nunca anexa o cookie de sessão a
  uma chamada entre origens;
- um preflight (`OPTIONS`) em `/api/*` só tem sucesso quando os
  cabeçalhos anunciados incluem `authorization`: `204`, métodos
  `GET, POST, PUT, DELETE`, cabeçalhos
  `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`,
  em cache por 10 minutos. `X-Requested-With` não está entre eles, então
  uma sessão de cookie nunca pode fazer uma escrita de outra origem;
- `Retry-After`, `WWW-Authenticate`, os cabeçalhos `RateLimit-*` e
  `X-DumbMonit-Api-Version` são legíveis pelo script que chama.

`DUMBMONIT_API_CORS_ORIGINS` define quais origens são permitidas:

| Valor | Efeito |
|---|---|
| não definido (padrão) | Qualquer origem (`Access-Control-Allow-Origin: *`). Seguro, já que só um token bearer abre qualquer coisa. |
| `https://grafana.example.com,https://dash.lan` | Só essas origens, ecoadas de volta. |
| `off` | Nenhum cabeçalho CORS de forma alguma: só páginas da mesma origem e clientes não-navegador. |

A mesma lista decide qual `Origin` o
[endpoint MCP](../using/assistant.md#security-notes) aceita.

```js
// De uma página em outra origem. Nunca envie um token de escrita em uma página pública.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'chamadas restantes este minuto');
```

## Versionamento {#versioning}

Toda resposta sob `/api` carrega a versão da API:

```
X-DumbMonit-Api-Version: 0.1
```

O mesmo valor é `info.version` no documento OpenAPI, ao lado de
`x-server-version`, a versão do servidor que respondeu. Enquanto a API é
`0.x` ela não é congelada; a versão muda sempre que uma rota ou campo é
removido ou renomeado. Adicionar uma rota ou campo não a muda: ignore
campos que você não conhece.

## Erros {#errors}

Todo erro é JSON: `{"error": "message"}`, escrito para uma pessoa.

| Status | Quando |
|---|---|
| `400` | A requisição está malformada ou um valor é inválido; a mensagem o nomeia. |
| `401` | Nenhuma sessão ou token válido: token faltando, desconhecido, revogado ou expirado (com `WWW-Authenticate: Bearer`), ou um token cujo criador está desativado. |
| `403` | Autenticado, mas não permitido: um visualizador, um operador fora do manuseio de alertas, ou um token `read` em uma rota de escrita, um token em uma rota de conta, token ou backup, um token usado de uma rede de onde não tem permissão, uma escrita por cookie sem `X-Requested-With`, ou qualquer escrita na demo pública (com `"demo": true`). |
| `404` | O id não existe. Um caminho sob `/api` que não corresponde a nenhuma rota responde `404 {"error": "Unknown API route: …"}` em vez do HTML da interface web. |
| `405` | O método não existe nesse caminho (`GET /api/mcp`, por exemplo). |
| `409` | Um conflito com o estado atual (um duplicado, um comando já em execução…). |
| `429` | Limite de taxa, com `Retry-After`. |
| `500` | Uma falha do servidor; os detalhes estão no log do servidor, nunca na resposta. |

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
JSON-RPC em vez disso, com seus próprios códigos de erro.

## Contas, login e tokens {#accounts-sign-in-and-tokens}

Nas tabelas abaixo, *sessão* significa um cookie de sessão **ou** um
token de API; *admin* significa uma sessão de administrador ou um token
`write`; *operador* significa uma sessão de operador ou administrador,
ou um token `write`; *apenas sessão* significa um cookie de sessão,
nunca um token.

**Papéis.** Uma conta `viewer` faz todo `GET`. Um `operator` faz as
mesmas leituras mais as escritas de manuseio de alerta marcadas
*operator* nesta página (e `x-roles: [admin, operator]` no documento
OpenAPI): reconhecer, colocar em soneca (janelas de manutenção), ignorar
uma regra para um monitor, limpar histórico resolvido. Toda outra
escrita é restrita a admin e responde `403` a um operador.

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/auth/status` | público | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` significa uma instância nova: até o primeiro admin existir, toda rota marcada *sessão* responde `401` — só `status`, `setup`, `login` e `health` são alcançáveis. |
| `POST` | `/api/auth/setup` | público | `{"setup_code": "…", "username": "admin", "password": "…"}`. Cria o primeiro administrador em uma instância nova (ao menos 12 caracteres); `username` pode ser omitido e então é `admin`. `setup_code` é o código único impresso nos logs do servidor na inicialização (ou `DUMBMONIT_SETUP_CODE`); maiúsculas, hífens e espaços são ignorados. `204`; não abre uma sessão. `401` para um código errado, `429` depois de muitas tentativas, `409` uma vez que uma conta existe. |
| `POST` | `/api/auth/login` | público | `{"username": "…", "password": "…"}`. `204` com `Set-Cookie`. `401` para uma senha errada, `429` quando limitado por taxa. Quando a conta tem dois fatores ativado, `200` com `{"totp_required": true, "pending": "…"}` em vez disso — um ticket que vive cinco minutos e morre depois de cinco códigos errados: termine com `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | público | `{"pending": "…", "code": "123456"}` — o ticket do primeiro passo e um código (ou um código de recuperação). `204` com o cookie de sessão. |
| `GET` | `/api/auth/oidc/start` | público | Redireciona o navegador para o provedor de identidade. |
| `GET` | `/api/auth/oidc/callback` | público | Retorno do provedor (`code`, `state`): abre a sessão e redireciona para a interface. |
| `GET` | `/api/auth/me` | apenas sessão | A conta atual: `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | apenas sessão | Termina a sessão e limpa o cookie. |
| `POST` | `/api/auth/password` | apenas sessão | `{"current_password": "…", "new_password": "…"}`. Desconecta toda outra sessão. |
| `GET` | `/api/auth/totp` | apenas sessão | Estado de dois fatores da conta atual: `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | apenas sessão | `{"password": "…"}`. Propõe um segredo: `secret` (base32), `otpauth_uri`, `issuer`, `account`. Nada é aplicado até ser verificado. |
| `POST` | `/api/auth/totp/verify` | apenas sessão | `{"code": "123456"}`. Confirma o cadastro e retorna `{"recovery_codes": […]}` — mostrado uma vez. |
| `DELETE` | `/api/auth/totp` | apenas sessão | `{"password": "…"}`. Remove o segundo fator. `204`. |
| `GET` | `/api/auth/audit?limit=200` | admin, apenas sessão | Os eventos de segurança mais recentes: `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | admin, apenas sessão | As configurações SSO e de onde vêm (ambiente ou banco de dados); o segredo do cliente nunca é retornado. |
| `PUT` | `/api/auth/oidc/config` | admin, apenas sessão | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Um `client_secret` ausente ou vazio mantém o armazenado; um `auto_create` ausente significa desligado. |
| `DELETE` | `/api/auth/oidc/config` | admin, apenas sessão | Esquece as configurações SSO armazenadas (variáveis de ambiente, se houver, se aplicam de novo). `204`. |
| `POST` | `/api/auth/oidc/test` | admin, apenas sessão | Busca o documento de descoberta do provedor e reporta o que encontrou. |
| `GET` | `/api/users` | admin, apenas sessão | Toda conta, na forma de `/api/auth/me`. |
| `POST` | `/api/users` | admin, apenas sessão | `{"username", "display_name", "role", "password"}`. `password` pode ser omitido só quando SSO está ativado. `201`. |
| `PUT` | `/api/users/{id}` | admin, apenas sessão | Qualquer de `display_name`, `role`, `disabled`, `password`; um campo omitido mantém seu valor. Não pode rebaixar ou desativar o último administrador. |
| `DELETE` | `/api/users/{id}` | admin, apenas sessão | `204`. Não você mesmo, não o último administrador. |
| `DELETE` | `/api/users/{id}/totp` | admin, apenas sessão | Redefine o segundo fator de outra conta (um colega trancado fora). `204`. |
| `GET` | `/api/tokens` | apenas sessão | Todo token de API: `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (a conta que o criou), `expires_at` (`null`: nunca), `expired`, `allowed_networks` (vazio: de qualquer lugar), `last_used_at`, `last_used_ip`, `revoked_at`. Nunca o segredo. |
| `POST` | `/api/tokens` | admin, apenas sessão | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` é obrigatório (80 caracteres no máximo); `scope` é `read` (padrão) ou `write`; `expires_in_days` vai de 1 a 3650, `null` ou ausente para um token que nunca expira; `allowed_networks` toma até 32 endereços ou faixas CIDR, armazenados em forma de rede (`192.168.1.7/24` se torna `192.168.1.0/24`), vazio ou ausente para qualquer lugar. `201` com os campos do token mais `secret` (mostrado uma vez). `400` nomeia uma rede inválida. |
| `DELETE` | `/api/tokens/{id}` | admin, apenas sessão | Revogar. `204`; `404` quando desconhecido ou já revogado. |

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

Além de `X-DumbMonit-Api-Version` ([versionamento](#versioning)), os
cabeçalhos `RateLimit-*` ([limites de taxa](#rate-limits)) e os
cabeçalhos CORS
([requisições entre origens](#cross-origin-requests-cors)), toda
resposta carrega `X-Content-Type-Options: nosniff`,
`Referrer-Policy: same-origin` e um `Content-Security-Policy`. A
política começa de `default-src 'none'` e abre só o que a interface
realmente usa — tudo servido pelo próprio DumbMonit:

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

Não há origem de terceiros nela, e nunca deveria haver: as fontes, a
biblioteca de gráficos e todo recurso vêm dentro do binário. Os scripts
inline da página (o tema aplicado antes da primeira renderização, o
bootstrap do SvelteKit) são permitidos por um nonce regenerado para cada
resposta, então `'unsafe-inline'` nunca se aplica a scripts. Se aplica a
estilos, e não pode ser removido: o Svelte define atributos `style=` em
elementos, que nem um nonce nem um hash podem cobrir.

Páginas de status públicas são feitas para serem embutidas:
`/s/<slug>` e sua visão compacta `/s/<slug>/embed` — e só essas duas —
recebem a mesma política sem `frame-ancestors`. Tudo mais, as páginas de
confirmação de inscrição e cancelamento sob `/s/` incluídas, também
carrega `X-Frame-Options: DENY`.

Se você colocar o DumbMonit detrás de um proxy reverso, não deixe que ele
adicione um segundo cabeçalho `Content-Security-Policy`: navegadores
aplicam a interseção de todos eles, e um padrão de proxy sem o nonce
deixa uma página vazia.

## Saúde {#health}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/health` | público | Versão do servidor e o estado de suas dependências. Sempre `200`: um componente falhando está no corpo. |
| `GET` | `/api/update` | sessão ou token | Versão em execução, último lançamento, `update_available`; o servidor pergunta ao GitHub no máximo uma vez por dia. `POST /api/update/check` verifica agora, `PUT /api/update/settings` liga ou desliga a verificação. Veja [Atualização](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` é `degraded` quando um componente falha; esse componente então
carrega uma string `error`. `victoria.embedded` é `true` quando o
servidor roda seu próprio VictoriaMetrics, `false` quando
`DUMBMONIT_VM_URL` aponta para um externo.

## Backup e restauração {#backup-and-restore}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/backup` | admin, apenas sessão | O que um pacote conteria (`contents`, uma linha por seção com sua contagem), a versão do formato de pacote, onde o segredo da instância vive (`secret_source`: `file` ou `environment`) e o estado dos backups locais programados (`schedule`). |
| `POST` | `/api/backup` | admin, apenas sessão | `{"passphrase": "…", "include_account_secrets": false}`. Retorna o próprio pacote como um arquivo JSON: um cabeçalho em texto claro (`format`, `version`, `created_at`, `summary`, `kdf`) e um `payload` criptografado com AES-256-GCM sob uma chave derivada da frase secreta pelo Argon2id. Pelo menos 16 caracteres, ou `400`. |
| `POST` | `/api/backup/restore` | admin, apenas sessão | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Simulação a menos que `apply` seja `true`; ambos executam o mesmo código, a simulação dentro de uma transação que é revertida. Responde o relatório: por seção, `created`, `updated`, `skipped` e `notes`, mais `warnings`. `400` para uma frase secreta errada, um arquivo modificado, ou um pacote escrito em uma versão de formato mais nova. |
| `POST` | `/api/backup/local` | admin, apenas sessão | Escreve um backup local estilo programado agora e responde o novo `schedule`. |

Um pacote carrega toda credencial da instância, recriptografada com a
frase secreta, então essas rotas recusam tokens de API com uma
explicação: só um administrador conectado à interface web recebe um.
Veja [Backup e restauração](../install/backup.md).

## Tipos de monitor {#device-types}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/collectors` | Todo tipo de monitor que este servidor pode monitorar: `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) e `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). Tipos trazidos por um [pacote de integração](../packs/index.md) são listados da mesma forma, como `pack.<id>`. |

## Pacotes de integração {#integration-packs}

Sessão de admin ou token de API (escopo `write` para mudanças). Veja
[Pacotes de integração](../packs/index.md).

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/packs` | Pacotes instalados: `id`, `version`, `label`, `summary`, `kind` (`null` para um pacote só com perfis SNMP), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uids), `snmp_profiles`, `targets` (monitores que o usam), `warnings`, e `error` quando o pacote armazenado não passa mais a validação. |
| `GET` | `/api/packs/{id}` | Um pacote. |
| `POST` | `/api/packs` | Instalar ou atualizar. Corpo: o YAML (`application/yaml`, `text/plain`…) ou `{"yaml": "…"}` em JSON. `201` quando criado, `200` quando atualizado ou inalterado, com `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` e `restart_required` (perfis SNMP). `400` lista todo erro de validação. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Registra ou retira o tipo de monitor do pacote; monitores, histórico e regras permanecem. |
| `DELETE` | `/api/packs/{id}` | Desinstala, com suas regras. `409` enquanto monitores o usam. |

## Targets (monitores) {#targets-devices}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets` | Lista todo monitor. |
| `POST` | `/api/targets` | Cria um. `201` com o monitor. A detecção de perfil começa em segundo plano para SNMP. |
| `GET` | `/api/targets/{id}` | Um monitor. |
| `PUT` | `/api/targets/{id}` | Substitui-o. Omitir `credential` mantém a armazenada; omitir `profile_id` mantém o perfil detectado (`""` o limpa). Definir `enabled: false` limpa os alertas do monitor sem notificar. |
| `DELETE` | `/api/targets/{id}` | `204`. Limpa os alertas do monitor sem notificar e exclui suas séries temporais do VictoriaMetrics (melhor esforço). |
| `POST` | `/api/targets/{id}/probe` | Sondar agora: `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — as chaves de série como o coletor as produziu, sem o prefixo `dumbmonit_` que o escritor adiciona no caminho para o VictoriaMetrics. |
| `POST` | `/api/targets/{id}/discover` | Executa a detecção de perfil de novo: `{"profile_id": "host-resources"}` ou `null`. |

Um monitor como retornado:

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

O próprio segredo nunca é retornado: `credential_kind` é um rótulo.
Timestamps do servidor são UTC sem sufixo. `error_kind` classifica
`last_error`: `down` (o monitor não respondeu) ou `config` (do nosso
lado — credenciais, endereço, opção), que é o que a interface mostra
como *Inacessível* ou *Mal configurado*; `null` quando a última sondagem
teve sucesso.

Criando um:

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

Formas de credencial (`type`): `none`; `snmp_community` (`community`);
`snmp_v3` (`username`, `auth: {protocol, passphrase}` opcional com
`md5`, `sha1`, `sha224`, `sha256`, `sha384` ou `sha512`,
`privacy: {protocol, passphrase}` opcional com `des`, `aes128`,
`aes192` ou `aes256`, `context` opcional); `api_token` (`token`, ou as
duas partes `token_id` e `secret`, que o servidor junta em
`user@realm!name=secret` para Proxmox VE e PBS);
`username_password` (`username`, `password`). `interval_secs` tem
padrão 60 e não pode ir abaixo de 10. `name` tem no máximo 200
caracteres e `address` no máximo 253 (`400` além). `parent_id` precisa
nomear um monitor existente
(`400 Parent device N not found.`). Opções de tipo vão em `tags` sob
sua chave.

`via_agent` nomeia o [agente relay](../install/remote-site.md) que
sonda o monitor de sua própria rede em vez do servidor; `null` (o
padrão) significa que o servidor o sonda. Precisa ser um monitor
`agent` (`400` caso contrário), um monitor não pode retransmitir a
si mesmo, e um monitor `agent` não pode ser retransmitido. Em
`PUT`, um `via_agent` *omitido* mantém o valor armazenado, como
`profile_id` e `credential`; envie `null` para voltar à sondagem
direta. `POST /api/targets/{id}/probe` e `/discover` em um monitor
retransmitido passam pelo relay e esperam sua resposta (`409` quando
uma sondagem já está em andamento, `400 Timed out: relay agent …`
quando não responde).

## Descoberta {#discovery}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `POST` | `/api/discovery` | admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Varre uma rede por dispositivos SNMP. `cidr` é obrigatório; `community` tem padrão `public`; `timeout_ms` é limitado a 100–10.000. Redes maiores que 4096 endereços são recusadas. A varredura é um `POST` com um corpo JSON para que a comunidade nunca apareça em uma URL ou linha de log. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Guia de primeira execução {#first-run-guide}

Os três passos que a visão geral mostra em uma instância nova. O estado
é mantido na instância, não no navegador, então pular um passo o pula em
todo lugar.

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/onboarding` | sessão | Onde a instância está. |
| `PUT` | `/api/onboarding` | admin | `{"skipped": true}` afasta todo o guia; `{"dismissed": ["test"]}` afasta passos individuais. Um campo omitido mantém seu valor armazenado; um id de passo diferente de `device`, `channel` ou `test` é um `400`. Retorna o mesmo documento que `GET`. |

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

`has_target`, `has_channel` e `notification_confirmed` são recalculados
a cada leitura: são fatos sobre a instância, não flags que a interface
define. `notification_confirmed` é verdadeiro uma vez que uma mensagem
realmente saiu de algum canal — uma mensagem de teste conta, um canal
apenas criado não.

`complete` é travado e nunca volta a false. O servidor o define quando os
três passos estão resolvidos (feitos, ou dispensados), e também na
primeira leitura de uma instância que já tinha um monitor e um canal
— então atualizar uma instância estabelecida nunca mostra um guia de
primeira execução.

## Métricas {#metrics}

Ambas as rotas retransmitem o VictoriaMetrics e retornam séries na forma
Prometheus:
`[{"metric": {rótulos…}, "values": [[timestamp_seconds, "value"], …]}]`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Consulta MetricsQL instantânea. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Consulta de intervalo. `start` e `end` são **milissegundos** desde a época; `step` é em segundos e é ampliado para que uma série nunca exceda 2.000 pontos. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Coleta (fora de `/api`) {#scraping-outside-api}

Para um Prometheus ou Grafana existente. Ambos levam um token de API
`read` em `Authorization: Bearer dmt_…` (um cookie de sessão também
funciona); um coletor não envia cabeçalho `X-Requested-With` e não
precisa de nenhum — essas rotas só leem. Veja a
[referência de métricas](metrics.md#scraping-dumbmonit) para um
`scrape_config` pronto para copiar e a fonte de dados Grafana.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/metrics` | Saúde da própria instância no formato de exposição de texto Prometheus: agendador, sondagens, escritas, alertas, notificações, agentes, banco de dados, VictoriaMetrics. Algumas dezenas de séries, nunca uma por monitor. |
| `GET` | `/federate` | As medições, no mesmo formato. `match[]` seleciona séries (repetível, 10 no máximo, `{__name__=~"dumbmonit_.*"}` por padrão), `max_lookback` amplia a janela em que o último ponto é procurado (`5m` por padrão). `400` além de 8 MiB, com o que estreitar. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | A metade de leitura da API Prometheus, retransmitida para o armazenamento, então o Grafana pode usar `http://…/prometheus` como uma fonte de dados Prometheus. Só `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` e `status/buildinfo`; qualquer outra coisa é `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` serve as três sem um token — só detrás
de um firewall, já que carregam toda medição de todo monitor.

## Alertas {#further-routes}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts` | Alertas ativos (pendentes, disparando, suprimidos, resolvidos recentemente). Alertas de monitores excluídos ou pausados nunca são listados. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Transições de fase. `since` é RFC 3339, padrão os últimos sete dias; `limit` precisa ser positivo. `dismissed` (padrão `false`) exclui entradas limpas com as duas rotas abaixo; nada é jamais destruído por uma limpeza, só escondido da visão padrão. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Limpa uma entrada `resolved` da visão de histórico padrão. `204`. `404` em um id desconhecido ou uma transição ainda ativa. Operador; auditado. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Limpa toda entrada `resolved` ainda não limpa de uma vez ("Limpar todas resolvidas"). Retorna `{"dismissed": <count>}`. Operador; auditado. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Reconhecer: `{"duration_secs": 14400, "note": "…"}` ou `{"until": "2026-09-22T18:00:00Z"}` (um dos dois; nenhum significa 4 horas, no máximo 30 dias). `{"until": null}` o remove. Retorna o alerta. Operador; auditado. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Remove o reconhecimento. Retorna o alerta. Operador. |

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
`target_id`, `from_phase`, `to_phase`, `severity`, `value`, `notified`,
`reason` (vazio, ou por que nada foi enviado: `learning: would have
fired`, `suppressed: device 2 unreachable`, `maintenance window`,
`acknowledged by admin`, `device removed or disabled`), `at` e
`dismissed` (limpo da visão ativa; a própria linha é mantida). `acked`
é verdadeiro enquanto `acked_until` está no futuro: lembretes e
escalonamentos pausam, a resolução ainda é notificada e limpa o
reconhecimento. Um `target_id` que não existe mais é mostrado como
"(monitor excluído)" pela interface.

### Regras {#alerts}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts/rules` | Toda regra. |
| `POST` | `/api/alerts/rules` | Criar. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Atualizar. Uma regra integrada mantém sua consulta. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. Regras integradas não podem ser excluídas. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Retorna a regra. |

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

Criar uma precisa ao menos de `name` e `query`; `kind` (`threshold`,
`anomaly`, `predict`), `operator`, `threshold`, `for_secs`, `severity`
(`info`, `warning`, `critical`), `selector` (`{"kind":"all"}`,
`{"kind":"ids","ids":[…]}` ou `{"kind":"labels","labels":{…}}`),
`channels` (ids de canal; vazio significa todo canal ativado), `unit`,
`repeat_secs`, `escalate_after_secs` e `enabled` são opcionais.

### Silêncios (janelas de manutenção) {#rules}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts/silences` | Toda janela, com `active_now`, `active_until` e `next_start_at` (RFC 3339, calculado pelo servidor). |
| `POST` | `/api/alerts/silences` | Criar. `201`. Operador. |
| `DELETE` | `/api/alerts/silences/{id}` | `204`. Operador. |

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

Uma programação pontual é
`{"kind": "once", "starts_at": "2026-09-20T22:00:00Z",
"ends_at": "2026-09-21T02:00:00Z"}`. Uma mensal é
`{"kind": "monthly", "days": [1], "nth_weekdays": [{"nth": 1, "weekday": 6}],
"start_minute": 120, "duration_minutes": 120, "timezone": "Europe/Paris"}` —
`nth` é 1 a 5, ou `-1` para o último do mês; um mês sem essa ocorrência é
pulado. Dias são 0 = Segunda … 6 = Domingo (1–31 para um `days` mensal);
minutos são desde a meia-noite local (0–1439). Em uma programação
recorrente, `timezone` (um nome IANA) vence sobre `utc_offset_minutes` e
é o que mantém uma janela em sua hora local através de mudanças de
horário de verão; um nome de fuso desconhecido é recusado.

### Substituições por monitor {#per-device-overrides}

Uma regra pode ser ajustada para um monitor sem tocar na própria
regra. `{"enabled": false}` é o que a ação **Ignorar** da interface (no
alerta, na página do monitor ou na regra) escreve: o monitor é
retirado dessa única regra de vez, reversivelmente, sem desativar a
regra para mais ninguém.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Toda substituição, ou as de um monitor: `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | As substituições de uma regra. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — cada campo opcional; `null` significa "como a regra". Cria ou substitui. Um operador só pode enviar `{"enabled": false}` (ignorar); as substituições de limite do monitor, se houver, são mantidas. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Para um operador, só remove o ignorar: substituições de limite permanecem. |
| `GET` | `/api/targets/{id}/rules` | The rules that watch a device: those whose query reads a metric it wrote in the last 24 h. `{"status": "ok" \| "no_data" \| "unavailable", "rules": [{id, uid, name, description, severity, enabled, ignored, custom_threshold, firing}]}`. |

## Política de notificação {#per-device-overrides}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/notify/policy` | A política global: `batch_window_secs` (0 = envia imediatamente), `max_per_hour` por canal (0 = ilimitado), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = sem detecção de oscilação), `public_url` (links em mensagens; vazio = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` e `escalate_channel` (0 / `null` = sem escalonamento). |
| `PUT` | `/api/notify/policy` | Mesmos campos, todos opcionais: um campo omitido mantém seu valor. `"escalate_channel": null` desliga o escalonamento. Retorna a política. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → quais monitores um filtro de roteamento de canal seleciona: `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Canais de notificação {#notification-policy}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/notify/kinds` | Todo tipo de canal com seus campos `settings` e `secrets` (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) e um `doc_url` como `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Todo canal: `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). Segredos nunca são retornados. |
| `POST` | `/api/notify/channels` | Criar. `201`. |
| `PUT` | `/api/notify/channels/{id}` | Atualizar. Omitir `secrets` mantém os armazenados; `"secrets": {}` os limpa. Em `policy`, omitir `matcher` mantém o filtro de roteamento e `"matcher": null` o limpa. |
| `DELETE` | `/api/notify/channels/{id}` | `204`. |
| `POST` | `/api/notify/channels/{id}/test` | Envia uma mensagem de teste: `{"ok": true, "message": "…"}`. |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

As chaves exatas por tipo vêm de `/api/notify/kinds` e são documentadas
em [Canais de notificação](../notifications.md).

### Dispositivos Web Push {#notification-channels}

Os dispositivos da **conta conectada** (apenas sessão, visualizadores
incluídos; tokens de API recebem `401`). Um canal `webpush` envia para
eles; veja [Notificações push (PWA)](../using/push-notifications.md).

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`: a chave pública VAPID (base64url, o `applicationServerKey` para inscrever) e os dispositivos da conta — `id`, `device` (`Firefox on Android`), `push_service` (host), `fingerprint` (primeiros 16 caracteres hex do SHA-256 do endpoint), `created_at`, `last_success_at`, `last_error`. O endpoint, as chaves do navegador e a chave privada nunca são retornados. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`: `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` com o dispositivo. O endpoint precisa ser HTTPS em um endereço público: endereços privados, loopback e link-local recebem `400`. O mesmo endpoint de novo substitui sua linha. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`; `404` para um dispositivo de outra conta. |
| `POST` | `/api/webpush/test` | `{}` ou `{"id": 3}`. Envia uma notificação de teste: `{"ok", "delivered", "failed", "removed", "message"}`. Inscrições respondidas com `404`/`410` são removidas. |

## Relatórios por e-mail {#email-reports}

Apenas administradores, `GET` incluído (a resposta lista
destinatários). Tokens bearer precisam do escopo `write` para mudanças.
Veja [Relatórios por e-mail](../using/reports.md).

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | admin | Todo relatório: `id`, `name`, `enabled`, `frequency`, `weekday` (0 é segunda), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | admin | Os mesmos campos graváveis; cada um tem padrão semanal, segunda, 8:00, `UTC`. `recipients` mantém no máximo 20 endereços e é obrigatório para ativar o relatório; `channel_id` precisa ser um canal `smtp` ou `null`. `201`; `409` além de 10 relatórios. |
| `GET` | `/api/reports/schedules/{id}` | admin | Um relatório. |
| `PUT` | `/api/reports/schedules/{id}` | admin | Substituição completa. |
| `DELETE` | `/api/reports/schedules/{id}` | admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | admin | Envia por e-mail uma pré-visualização aos destinatários salvos: `{"sent": 2, "failed": 0}`. Não toca a programação. `400` sem um canal de e-mail usável; `409` dentro de 30 segundos da pré-visualização anterior. |

## Páginas de status e incidentes {#email-reports}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/status-pages` | sessão | Toda página com seus itens: `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) e `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` é obrigatório (120 caracteres no máximo); `slug` (`^[a-z0-9-]{2,40}$`) é derivado do título quando omitido; `theme` é `auto` (padrão), `light` ou `dark`; `show_uptime_days` vai de 7 a 90 (90 por padrão). `201`; `409` em um slug já em uso. |
| `GET` | `/api/status-pages/{id}` | sessão | Uma página com seus itens. |
| `PUT` | `/api/status-pages/{id}` | admin | Mesmos campos, mas uma substituição completa em vez de um patch: `title` é obrigatório e todo campo omitido volta ao seu padrão (slug re-derivado do título, descrição vazia, `published: false`, `theme: auto`, 90 dias de disponibilidade). |
| `DELETE` | `/api/status-pages/{id}` | admin | `204`. A URL pública para de responder. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — aparência e inscrições | admin | Também aceitam `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (ids de cenário da lista do editor — cidades, as novas maravilhas e as maravilhas antigas, ex. `venice`, `paris`, `giza`, `tajmahal`, sem duplicatas; `[]` para nenhuma; omitido na criação: `["paris"]`), `simple` (booleano, padrão `false`: a página pública não mostra cenário, mascote ou animação e segue a preferência claro/escuro do visitante), `scene_rotation` (`visit`, `1m`, `10m` ou `1h`), `footer_text` (texto simples, 280 caracteres), `homepage_url` (`http(s)://` ou vazio), `subscribe_channel_id` (um canal `smtp`, ou `null` para desligar a inscrição por e-mail) e `domain` (o nome de host público da página, como `status.example.com`, ou `null`/`""` para nenhum — veja [Domínio personalizado](../using/status-pages.md#custom-domain)). `domain` é convertido para minúsculas e perde um prefixo `https://` ou barra final; uma porta, caminho ou endereço IP é `400`, o endereço do próprio DumbMonit (sua URL pública, ou o host em que a requisição chegou) é `400`, e um nome que outra página usa é `409`. Diferente dos outros campos, esses oito mantêm seu valor armazenado quando omitidos de um `PUT`. A visão da página os retorna com `logo_type` (`image/png`, `image/jpeg`, `image/webp` ou `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | sessão / admin | O logo da página. `PUT` toma `{"data": "<base64 or data: URL>"}`: PNG, JPEG ou WebP reconhecido pelo conteúdo, 256 KiB no máximo, qualquer outra coisa `400`. `204`. `GET` é a pré-visualização do editor (rascunhos incluídos). |
| `GET` | `/api/status-pages/{id}/subscribers` | sessão | Inscritos por e-mail: `id`, `page_id`, `email`, `confirmed_at` (`null` enquanto pendente), `created_at`. Seus tokens nunca saem do servidor. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | admin | Remove um endereço. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — a lista completa ordenada de monitores mostrados na página. |
| `GET` | `/api/incidents` | sessão | Todo incidente com suas atualizações: `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) e `updates`. |
| `POST` | `/api/incidents` | admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` é a primeira atualização. `201`. |
| `PUT` | `/api/incidents/{id}` | admin | Mesmos campos; um campo omitido mantém seu valor. |
| `DELETE` | `/api/incidents/{id}` | admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | sessão | A linha do tempo: `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | admin | `{"status": "monitoring", "body": "…"}`. Adiciona uma atualização e move o incidente para `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | público | O documento JSON a partir do qual uma página publicada é construída (também `badge.svg` e `rss` sob o mesmo caminho). Em `page`, `show_uptime_days` é o máximo configurado e `history_days` a janela realmente mostrada (a idade dos dados da página, de 7 dias até esse máximo) e `simple` espelha o modo simples. Veja [Páginas de status](../using/status-pages.md). No domínio personalizado de uma página (`Host` igual ao seu `domain`), só as rotas `/api/public/status/{slug}/…` desta página respondem; toda outra rota desta referência é `404` ali. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | público | Badges da página: disponibilidade média sobre `?days=` `1`, `7`, `30` (padrão) ou `90` — `400` além do histórico da página — e tempo de resposta médio. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | público | Os mesmos três badges para um serviço; `key` é a `key` do serviço no documento público (seu rótulo em forma de URL). `404` para uma chave desconhecida. |
| `GET` | `/api/public/status/{slug}/logo` | público | O logo da página com seu tipo de imagem, em cache por um dia (o `logo_url` do documento carrega uma versão). `404` sem um. |
| `POST` | `/api/public/status/{slug}/subscribe` | público | `{"email": "…"}`. Envia um link de confirmação pelo canal SMTP da página. Sempre `202` com a mesma mensagem, qualquer que seja o estado do endereço; `400` para um endereço inválido; `404` quando a página não oferece inscrição por e-mail; `409` quando a página não pode aceitar mais inscritos; `429` com `Retry-After` além de cinco requisições por cliente e sessenta no total por 15 minutos. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | público | Confirma uma inscrição (o link do e-mail de confirmação abre `/s/{slug}/confirm`, cujo botão posta aqui). `404` para um token desconhecido ou expirado. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | público | Remove uma inscrição; o corpo é ignorado, então também é o alvo de um clique RFC 8058 de `List-Unsubscribe`. Sempre `200`. |

## Tokens de agente e ingestão {#status-pages-and-incidents}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | apenas sessão | Todo token: `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` para um token de frota), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | admin, apenas sessão | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` com os campos do token mais `secret` (mostrado uma vez), `install_linux` e `install_windows`. `base_url` é a URL que agentes usarão; tem padrão o endereço de escuta. Sem `reusable`, o token é de **uso único**: cadastra uma máquina e nada mais. Com ele, `max_uses` limita os cadastros (`null`: sem limite). `expires_in_days` para *cadastros* depois desses dias; máquinas já cadastradas continuam reportando. `400` em uma contagem abaixo de 1. |
| `DELETE` | `/api/agent/tokens/{id}` | admin, apenas sessão | Revogar. `204`; `404` quando desconhecido ou já revogado. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Recebe um lote de amostras de um agente (corpos de até 16 MB). Não feito para ser chamado manualmente. O cabeçalho carrega o segredo de vínculo da máquina; um agente que não tem nenhum o omite, que é como pede um. A resposta adiciona `agent_secret` (o segredo, retornado exatamente uma vez, quando a máquina é vinculada) e `bound`. `403` quando a máquina está vinculada a outra instalação de agente, ou quando o token não pode mais cadastrar — a mensagem diz qual, e o que fazer. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Sondagens delegadas a um agente relay (`relay: true`). Retido até `wait` segundos (25 no máximo) quando nada está pendente. Cada item é um comando do tipo `probe` cujos `args` carregam o target, sua credencial descriptografada, `timeout_secs` e `discover`. Nunca escrito em disco. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Resultado de uma sondagem delegada: `{"duration_ms", "error", "samples", "profile_id"}` (corpos de até 16 MB). `204`; `404` quando a sondagem expirou ou pertence a outro agente. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Comandos enfileirados para um agente (reinício ou atualização de contêiner), retidos até `wait` segundos quando nada está pendente. Não feito para ser chamado manualmente. `key` por si só não prova nada — é pública — então uma máquina vinculada também precisa apresentar seu segredo de vínculo; `403` caso contrário, que é o que impede uma máquina de pegar os comandos de outra. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Progresso e resultado de um comando, reportado pelo agente. `result` é truncado aos últimos 4 KB, com `[truncated by the server, beginning dropped]` na frente quando isso acontece. |
| `GET` | `/api/relays` | sessão | Todo monitor de agente como um possível relay: `id`, `name`, `site`, `relay` (declarado `relay: true`), `last_seen_at`, `relayed` (monitores alcançados através dele). Relays primeiro, depois por nome. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Contêineres e comandos (monitores de agente) {#agent-tokens-and-ingest}

Ações nos contêineres de uma máquina que roda o agente. Toda rota
responde `404` quando o monitor não é um target `agent`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | A máquina como seu agente a descreveu por último: `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (monitores alcançados através deste agente), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404` até um agente ter reportado. `commands_supported` é `true` só quando o agente declarou que busca comandos (`commands: true`, o padrão dos agentes atuais); um agente mais antigo ou um com `commands: false` dá `false`. `binding` é `bound`, `pending` (o binário pode ser vinculado e será em seu próximo lote) ou `unsupported` (um agente mais antigo que o vínculo: reinstale-o). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Abre uma janela de uma hora durante a qual esta máquina pode se vincular de novo com um token de cadastro válido — o caminho de volta depois de uma reinstalação ter levado o segredo do agente. `{"rebind_until", "minutes"}`. A janela fecha assim que é usada, e o segredo antigo para de funcionar. `404` até um agente ter reportado. |
| `GET` | `/api/targets/{id}/containers` | Todo contêiner do último lote: `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Todo campo é opcional: um campo omitido mantém seu valor armazenado. Retorna a política completa. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Enfileira um reinício. `201` com o comando. `404` quando `name` não está no inventário do agente; `409` quando o mesmo comando já está enfileirado ou em execução, ou quando o agente não pode executar comandos (veja `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Enfileira uma atualização; opcional `{"prune": bool}` (tem padrão a política). Mesmos códigos de status que restart. |
| `GET` | `/api/targets/{id}/commands` | Os últimos 20 comandos, mais recente primeiro: `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (um usuário, ou `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Cancela um comando enfileirado. `204`; `409` uma vez que o agente o pegou ou quando já está fechado; `404` quando não pertence a este monitor. |

`status` vai `queued → running → done | failed`. Dois outros estados
finais existem: `cancelled` (uma pessoa retirou o comando) e `expired`
(o servidor desistiu depois de dez minutos porque nenhum agente veio
buscá-lo — agente parado, muito antigo para o canal de comando, ou
instalado com `commands: false`). O servidor expira comandos antigos a
cada minuto por conta própria; um comando enfileirado nunca bloqueia um
novo por mais que isso.

## Guests Proxmox VE {#containers-and-commands-agent-devices}

As linhas do painel **Guests** de um monitor Proxmox VE, montadas da
última sondagem no armazenamento de métricas (quatro consultas
instantâneas, nada pedido ao hipervisor). `404` quando o monitor não
existe, `400` quando não é um target `proxmox`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Um objeto por VM ou contêiner, ordenado por nó depois VMID: `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (dos núcleos alocados), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` agente guest respondeu, `false` ativado mas silencioso, `null` nenhum), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (seu pool de recursos), `lock` (só enquanto um bloqueio é mantido: `backup`, `migrate`…), `os` e `ip` (vistos de dentro do guest, atualizados por hora). Todo valor desconhecido é `null` — uma VM sem agente guest tem `disk_total_bytes` mas `disk_used_bytes: null`; um guest parado mantém seus tamanhos e perde suas medições. |

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

## Nós e Ceph Proxmox VE {#proxmox-ve-guests}

As seções **Nodes** e **Ceph** da mesma página de monitor, lidas da
mesma última sondagem. Mesmos códigos de status que acima.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — o estado do watchdog HA (`armed`, `standby`…) e se realmente isolaria um nó perdido, depois um objeto por nó, ordenado por nome: `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (a versão `pve-manager` instalada naquele nó), `services_down` (nomes de unidade dos daemons parados ou falhados), `interfaces_offline` (interfaces configuradas para iniciar no boot que não estão ativas), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) e `volume_groups` (`name`, `used_percent`, `size_bytes`). Um nó que parou de responder mantém sua entrada com `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false` quando o cluster não tem Ceph — a interface então não desenha seção alguma), `health` (0 OK, 1 WARN, 2 ERR, 3 desconhecido), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (nomes CephFS), `flags` (flags de OSD atualmente definidas, como `noout`) e `muted_checks` (verificações de saúde silenciadas, que `HEALTH_OK` não menciona mais). |

## Proxmox Backup Server {#proxmox-ve-nodes-and-ceph}

Os painéis de um monitor `pbs`, lidos da última sondagem (nada
pedido ao servidor exceto o log de tarefas). `404` quando o monitor
não existe, `400` quando não é um target `pbs`.

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`: uma entrada por grupo de backup (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) com um array `days` de `{date, state, runs, snapshot}`. `offset` é o deslocamento UTC do visualizador em minutos, para que dias sejam cortados na meia-noite local. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Tarefas com falha: `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`: jobs de sync, verify, prune e coleta de lixo com sua programação e último resultado. |
| `GET` | `/api/targets/{id}/pbs/health` | Datastores (uso, data de esgotamento estimada), discos (SMART, desgaste) e pools ZFS. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | O log de uma tarefa, buscado do servidor: `{"upid", "lines": […]}`; `lines` limita quantas são retornadas. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | Atributos SMART de um disco; `disk` é obrigatório (`400` sem ele). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager: totais do acervo (`estate`) e uma linha por instância federada (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, contagens de nó e guest, memória e armazenamento, `subscription`, `last_collection`, `tasks_failed`), inacessíveis primeiro. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Tarefas que falharam em todo o acervo: `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | O host da console: CPU, memória, sistema de arquivos raiz, tempo de atividade, certificados, atualizações pendentes e assinatura. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway: as quatro filas Postfix na ordem de leitura (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), mais `total_messages` e um `stuck` de todo o gateway. `oldest_age_seconds` é um limite inferior: o qshape reporta faixas de idade. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Totais de hoje (`mail`), a curva de tráfego recente (`recent`), o histograma de score de spam, os vírus pegos hoje, e contagens de quarentena (`quarantine`). Apenas contagens: nenhum conteúdo de mensagem. |
| `GET` | `/api/targets/{id}/pmg/health` | Uma entrada por nó (sistema, serviços, `signatures` com `family`, `age_seconds` e `stale`, `expiring_certificates`, atualizações, assinatura), os membros do `cluster` com seu estado de sincronização, e `stopped_services` entre nós. |

## Synology {#proxmox-backup-server}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Visão geral de um monitor `synology` da última sondagem: `system` (modelo, versão DSM, tempo de atividade, temperatura, CPU, memória), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) e `sampled_at`. `404` quando o monitor não existe, `400` quando não é um Synology. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business: `tasks`, e `devices` — um relatório por monitor protegido com seu ritmo aprendido em `assessment` (`state`, `last_success_s`, `typical_interval_s`, contagens de 30 dias). |

## Heartbeats (monitores push) {#synology}

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `GET` ou `POST` | `/api/push/{token}` | nenhuma (o token) | A chamada que um job faz a cada execução. Opcional `?status=up\|down&msg=…` (compatível com Uptime Kuma). `204` vazio em sucesso, `404` para um token desconhecido, `400` para outro `status`, `429` com `Retry-After` além de sessenta chamadas por minuto por token. Veja [Heartbeat](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | sessão | O monitor de um monitor `push`: `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Criado na primeira leitura. `400` quando o monitor não é um heartbeat. |
| `POST` | `/api/targets/{id}/push/regenerate` | admin | Novo token; a URL anterior responde `404` a partir de então. Mesmo corpo que a leitura. |

## Assistentes (MCP) {#heartbeats-push-monitors}

O servidor [Model Context Protocol](https://modelcontextprotocol.io)
embutido, o que um assistente fala com. Autentica com os tokens de API
acima — mesmos escopos, expiração, redes e limite de taxa — e nunca com
uma sessão.

| Método | Rota | Auth | Propósito |
|---|---|---|---|
| `POST` | `/api/mcp` | token de API | JSON-RPC 2.0 sobre o transporte Streamable HTTP, uma requisição por `POST`, respondida em JSON. Protocolo `2026-07-28` (sem estado: toda requisição carrega sua versão e capacidades de cliente em `_meta`, mais os cabeçalhos `MCP-Protocol-Version`, `Mcp-Method` e `Mcp-Name`; `server/discover`), e `2025-11-25`, `2025-06-18` e `2025-03-26` através do handshake `initialize`. Sem sessão: nenhum `Mcp-Session-Id` é emitido, um enviado é ignorado. Uma notificação recebe `202` e nenhum corpo. |
| `GET`, `DELETE` | `/api/mcp` | público | `405` com `Allow: POST` — não há fluxo server-sent e nenhuma sessão para terminar; o corpo só diz o que este endpoint é. |

`tools` é a única capacidade — sem recursos, sem prompts, sem OAuth. Das
27 ferramentas, um token `read` pode chamar as 15 que só leem
(`get_status`, `list_devices`, `get_device`, `list_alerts`,
`alert_history`, `query_metrics`, `list_silences`, `list_rules`,
`list_device_types`, `list_agents`, `list_containers`,
`list_heartbeats`, `list_status_pages`, `list_channels`, `list_packs`);
as 12 que alteram algo (`silence_device`, `remove_silence`,
`acknowledge_alert`, `probe_device`, `set_device_enabled`,
`set_rule_enabled`, `add_device`, `discover_network`,
`schedule_maintenance`, `restart_container`, `test_channel`,
`post_incident`) precisam de um token `write`. As ferramentas chamam o
mesmo código que a interface web. Versões, cabeçalhos, códigos de erro e
as notas de segurança estão em
[Conectar um assistente](../using/assistant.md#protocol-details).

## Arquivos de agente (fora de `/api`) {#assistants-mcp}

| Método | Rota | Propósito |
|---|---|---|
| `GET` | `/install.sh` | O instalador Linux, macOS e FreeBSD. Público. |
| `GET` | `/install.ps1` | O instalador Windows. Público. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, servidos de `DUMBMONIT_AGENT_DIR`; `{name}.sha256` dá seu checksum no formato `sha256sum`. `404` se o arquivo está ausente. `dumbmonit-agent-macos-aarch64` e `dumbmonit-agent-macos-x86_64` (e seus `.sha256`) respondem `307` para o mesmo arquivo no lançamento mais recente do GitHub. Público. |

Todo outro caminho é servido pela interface web, que pede para você
entrar ela mesma.
