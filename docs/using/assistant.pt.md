# Ligar um assistente {#connect-an-assistant}

O DumbMonit tem um servidor [MCP](https://modelcontextprotocol.io)
incorporado: o Model Context Protocol, a forma como assistentes como
o Claude, o ChatGPT, o Cursor ou o agente do VS Code chamam
ferramentas externas. Ligue um, depois pergunte-lhe coisas como:

- «Está tudo bem?»
- «Silencia o NAS por duas horas, estou a trocar um disco.»
- «O que aconteceu ontem à noite?»
- «Quão cheio está o servidor de cópias de segurança, e está a
  piorar?»
- «Publica uma atualização na página de estado: o servidor de email
  voltou.»

O assistente vê o que a interface vê — monitores, alertas,
histórico, métricas, agentes, páginas de estado — e usa as mesmas
palavras: *reporting*, *unreachable*, *waiting*, *advisory*,
*warning*, *suppressed by parent*. Corre dentro do servidor: nada
para instalar junto ao DumbMonit, e as ferramentas chamam o mesmo
código que a interface web.

## 1. Criar um token {#1-create-a-token}

Settings → **API & assistants** (`/settings#assistant`). Nomeie o
token com o assistente ou a máquina em que corre («Claude on my
laptop»), depois escolha:

| Definição | Escolhas |
| --- | --- |
| **Scope** | **Read**: olhar — estado, monitores, alertas, histórico, métricas, regras, agentes, páginas de estado. Nunca pode alterar nada. **Read and write**: também agir — silenciar, confirmar, verificar, adicionar um monitor, reiniciar um contentor, publicar um incidente… (as [ferramentas de escrita](#what-the-assistant-can-call)). |
| **Expires** | 30 dias, 90 dias (predefinição), 1 ano ou nunca. Um token expirado deixa de funcionar; crie um novo. |
| **Allowed networks** | Opcional. Endereços ou intervalos CIDR de onde o token pode ser usado (`192.168.1.0/24, 10.8.0.5`). Vazio: de qualquer lugar. |

O token parece-se com `dmt_` seguido de 32 caracteres hexadecimais. É
mostrado **uma vez**, logo depois da criação; o DumbMonit só guarda
um hash. Perdeu-o, revogue-o e crie outro.

Prefira um token de leitura a menos que realmente queira que o
assistente aja.

O mesmo token também abre a [API REST](../reference/api.md#authentication)
a scripts e painéis — `curl -H "Authorization: Bearer dmt_…"
https://monit.example.lan/api/targets` — onde `read` vê o que um
leitor vê e `write` faz o que um administrador faz, contas, tokens e
cópias de segurança excetuados.

## 2. Colar o trecho {#2-paste-the-snippet}

A página de definições preenche-os com o endereço do seu servidor e o
novo token. Abaixo, `https://monit.example.lan` representa o seu
servidor.

### Claude Code {#claude-code}

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

Adicione `--scope user` para o ter em todo projeto, não só no atual.

### Claude Desktop {#claude-desktop}

O Claude Desktop inicia servidores locais a partir do seu ficheiro de
configuração; o [`mcp-remote`](https://www.npmjs.com/package/mcp-remote)
faz a ponte para o endpoint HTTP do DumbMonit (precisa do
[Node.js](https://nodejs.org)). O DumbMonit autentica com um
cabeçalho bearer em vez de OAuth, por isso adicione-o através deste
ficheiro em vez do ecrã Connectors. Settings → Developer → **Edit
Config**, depois adicione a `claude_desktop_config.json` e reinicie o
Claude Desktop:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote", "https://monit.example.lan/api/mcp",
        "--header", "Authorization:${DUMBMONIT_AUTH}"
      ],
      "env": { "DUMBMONIT_AUTH": "Bearer dmt_…" }
    }
  }
}
```

O cabeçalho passa por uma variável de ambiente, sem espaço depois dos
dois pontos, porque algumas plataformas dividem argumentos em
espaços. Para um servidor alcançado via HTTP simples na sua rede,
adicione `"--allow-http"` a `args` — a página de definições fá-lo por
si quando ela própria é servida via HTTP.

### VS Code {#vs-code}

`.vscode/mcp.json` num espaço de trabalho (ou o comando **MCP: Add
Server** → HTTP):

```json
{
  "servers": {
    "dumbmonit": {
      "type": "http",
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

### Cursor e outros clientes {#cursor-and-other-clients}

`.cursor/mcp.json`, e a mesma forma para a maioria dos clientes que
suportam servidores Streamable HTTP com cabeçalhos personalizados:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

Um cliente que só inicia servidores locais (stdio) pode usar a ponte
`mcp-remote` mostrada para o Claude Desktop.

### ChatGPT {#chatgpt}

Settings → Connectors → **Create** (modo desenvolvedor). URL do
servidor MCP: `https://monit.example.lan/api/mcp`, cabeçalho de
autorização: `Bearer dmt_…`.

O ChatGPT liga-se a partir dos servidores da OpenAI, não do seu
navegador: o DumbMonit tem de ser **acessível a partir da internet
via HTTPS**. Ver as [notas de segurança](#security-notes) antes de
fazer isso.

### Qualquer outro cliente {#any-other-client}

O que um cliente precisa de saber: o endpoint é
`https://<server>/api/mcp`, o transporte é Streamable HTTP, e todo
pedido transporta `Authorization: Bearer dmt_…`. Uma chamada em
bruto, para verificar o token e o caminho de rede a partir de um
terminal:

```bash
curl -s https://monit.example.lan/api/mcp \
  -H 'Authorization: Bearer dmt_…' \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

## O que o assistente pode chamar {#what-the-assistant-can-call}

Vinte e sete ferramentas. Cada uma tem um título, uma descrição, um
esquema de entrada, um esquema de saída e anotações (`readOnlyHint`,
`destructiveHint`, `idempotentHint`), para que um cliente possa
distinguir as que só olham e perguntar-lhe antes de executar as
outras.

| Ferramenta | Âmbito | Faz |
| --- | --- | --- |
| `get_status` | read | O boletim, a resposta a «está tudo bem?»: contagens de monitor, alertas ativos e a formar-se, uma frase («2 advisories, 1 unreachable.»). |
| `list_devices` | read | Monitores e serviços com estado, pai e etiquetas; filtrar por subcadeia ou estado. |
| `get_device` | read | Um monitor: configuração, estado, alertas ativos, resumo de CPU / memória / disco mais cheio de 24 horas (disponibilidade e latência para uma verificação de serviço). |
| `list_alerts` | read | Alertas ativos com palavras de severidade, desde quando, valor, supressão, confirmação. |
| `alert_history` | read | Transições de alerta, as mais recentes primeiro (`since` ou `hours`, monitor opcional, limite). |
| `query_metrics` | read | Uma consulta de intervalo MetricsQL, no máximo 60 pontos por série. |
| `list_silences` | read | Janelas de manutenção e se estão ativas agora. |
| `list_rules` | read | Regras de alerta com tipo, severidade, limiar, ativada. |
| `list_device_types` | read | Os tipos de monitor que este servidor pode monitorizar, com as opções e credenciais que cada um espera — o que `add_device` precisa. |
| `list_agents` | read | Máquinas a executar o agente: SO, versão do agente, modo relay, site, monitores relayed, último contacto, associação. |
| `list_containers` | read | Os contentores Docker de uma máquina a executar o agente: estado, saúde, reinícios, atualização disponível, último comando. |
| `list_heartbeats` | read | Monitores heartbeat: intervalo esperado, última chamada, veredicto (à espera, a tempo, em falta, reportado em baixo). O URL secreto nunca é mostrado. |
| `list_status_pages` | read | Páginas de estado, publicadas ou não, com o seu caminho público (`/s/<slug>`), os monitores que mostram e os seus incidentes abertos. |
| `list_channels` | read | Canais de notificação: tipo, ativado, último erro, última mensagem enviada. Nunca os seus segredos. |
| `list_packs` | read | Pacotes de integração instalados: versão, tipo de monitor, ativado, monitores a usá-lo, erros. |
| `silence_device` | write | Uma janela de manutenção pontual num monitor, a começar agora (1 hora por predefinição, no máximo uma semana). |
| `remove_silence` | write | Remove uma janela de manutenção. |
| `schedule_maintenance` | write | Uma janela de manutenção pontual numa data dada, num monitor ou em toda a instância, no máximo uma semana. |
| `acknowledge_alert` | write | Confirma um alerta por impressão digital durante um número de horas (4 por predefinição): os lembretes pausam, a resolução continua a ser notificada. `hours: 0` levanta-a. |
| `probe_device` | write | Verifica um monitor imediatamente e reporta o que foi medido. |
| `set_device_enabled` | write | Ativa ou desativa a monitorização de um monitor. |
| `set_rule_enabled` | write | Ativa ou desativa uma regra de alerta. |
| `add_device` | write | Adiciona um monitor: nome, tipo, endereço, intervalo, pai, opções, credenciais, agente relay. |
| `discover_network` | write | Analisa uma rede (CIDR, no máximo 4096 endereços) por dispositivos SNMP e sugere um perfil para cada um. |
| `restart_container` | write | Pede ao agente de uma máquina para reiniciar um dos seus contentores. |
| `test_channel` | write | Envia uma mensagem de teste através de um canal de notificação. |
| `post_incident` | write | Abre um incidente numa página de estado, ou publica uma atualização num existente. |

Um token de leitura a chamar uma ferramenta de escrita recebe uma
recusa clara como resultado da ferramenta (`isError: true`); o
assistente explica-a e nada muda. O mesmo se aplica a qualquer coisa
que a ferramenta não possa fazer — um monitor desconhecido, um
nome ambíguo, uma janela mais longa do que uma semana: a mensagem diz
o que corrigir.

## Detalhes do protocolo {#protocol-details}

`POST /api/mcp` implementa o MCP sobre o transporte **Streamable
HTTP**: JSON-RPC 2.0, um pedido por `POST`, respondido com JSON
simples (sem server-sent events). É sem estado: nenhum
`Mcp-Session-Id` é emitido, um que um cliente envia é ignorado, e
`GET` ou `DELETE` no endpoint respondem `405`. Só a capacidade
`tools` é oferecida — sem recursos, prompts, sampling ou OAuth.

Duas gerações do protocolo são servidas no mesmo endpoint:

| Versões | Como um cliente o fala |
| --- | --- |
| `2026-07-28` | Sem estado. Todo pedido transporta `io.modelcontextprotocol/protocolVersion` e `io.modelcontextprotocol/clientCapabilities` em `params._meta`, e os cabeçalhos `MCP-Protocol-Version`, `Mcp-Method` e, para `tools/call`, `Mcp-Name`, que tem de corresponder ao corpo. Métodos: `server/discover`, `tools/list`, `tools/call`. Os resultados transportam `resultType: "complete"` e a identidade do servidor em `_meta` (`io.modelcontextprotocol/serverInfo`); `tools/list` também transporta `ttlMs` e `cacheScope`. |
| `2025-11-25`, `2025-06-18`, `2025-03-26` | O handshake `initialize` escolhe a versão (uma desconhecida recebe a mais recente destas três); `notifications/initialized` é confirmado com `202`; depois `ping`, `tools/list`, `tools/call`. |

Erros, além do `401`, `403` e `429` das
[verificações de token](#security-notes), que vêm antes de qualquer
JSON-RPC:

| HTTP | Código JSON-RPC | Quando |
| --- | --- | --- |
| `400` | `-32020` (incompatibilidade de cabeçalho) | Um pedido `2026-07-28` sem `MCP-Protocol-Version`, `Mcp-Method` ou `Mcp-Name`, ou com valores que diferem do corpo. |
| `400` | `-32022` (versão de protocolo não suportada) | Uma versão que este servidor não fala; `error.data` lista as `supported` e a `requested`. |
| `400` | `-32602` (parâmetros inválidos) | Um pedido `2026-07-28` sem os campos `_meta` obrigatórios. |
| `404` | `-32601` (método não encontrado) | Um pedido `2026-07-28` para um método que este servidor não implementa. |
| `200` | `-32602` | Uma ferramenta desconhecida, ou `arguments` que não é um objeto. |
| | `-32700`, `-32600` | JSON ilegível; não é um pedido JSON-RPC 2.0 (lotes não são suportados). |
| `202` | — | Uma notificação: aceite, sem corpo. |

Uma ferramenta que corre mas falha — âmbito em falta, monitor
desconhecido — não é um erro de protocolo: responde normalmente com
`isError: true` e uma frase que o assistente pode transmitir.

## Notas de segurança {#security-notes}

- **Um token é uma senha.** Quem o detém lê tudo o que a interface
  mostra — e, com um token de escrita, altera tudo o que um
  administrador pode através da API REST (monitores, regras,
  canais), contas, tokens e cópias de segurança excetuados.
  Mantenha-o fora de conversas partilhadas, capturas de ecrã e
  repositórios. Revogue um token de que não tem certeza; crie outro
  num minuto.
- **Prefira read.** A maioria das perguntas («está tudo bem?») não
  precisa de nenhum âmbito de escrita. Crie um token de escrita só
  para um assistente que realmente quer que aja, e nomeie-o para o
  reconhecer na lista. A maioria dos clientes pergunta antes de
  executar uma ferramenta que não está marcada só de leitura; deixe
  essa confirmação ativa.
- **Dê-lhe uma vida e uma rede.** Um token que expira em 90 dias e só
  funciona a partir da sua LAN ou VPN (`192.168.1.0/24`) vale pouco
  para quem o encontrar. Atrás de um proxy inverso, declare o proxy
  em `DUMBMONIT_TRUSTED_PROXIES`, caso contrário toda chamada parece
  vir do proxy e uma restrição de rede recusa-as todas.
- **Ligado ao seu criador.** Um token deixa de funcionar quando a
  conta que o criou é desativada, perde o seu poder de escrita quando
  essa conta já não é administradora, e é revogado quando essa conta
  é eliminada.
- **Nada secreto volta.** As ferramentas nunca devolvem credenciais
  de monitor (só o seu tipo), segredos de canal, URLs de
  heartbeat ou tokens. `add_device` recebe credenciais: viajam
  através da conversa do seu assistente, e portanto através do seu
  fornecedor. Para um monitor com credenciais sensíveis, prefira
  a interface web.
- **Navegadores e `Origin`.** Um pedido que transporta um cabeçalho
  `Origin` (um cliente baseado em navegador) é recusado com `403` a
  menos que essa origem seja a do próprio servidor ou seja permitida
  por `DUMBMONIT_API_CORS_ORIGINS` (por predefinição qualquer origem,
  já que o token é obrigatório de qualquer forma). Liste as suas
  origens aí para a restringir; ver
  [pedidos de origem cruzada](../reference/api.md#cross-origin-requests-cors).
- **Limite de taxa.** Cada token é limitado a 120 chamadas por
  minuto, REST e MCP juntos: um assistente preso num loop recebe
  `429` com `Retry-After`, não toda a atenção do seu servidor.
- **HTTPS antes da internet.** Os conectores do ChatGPT precisam de
  um URL público. Coloque o DumbMonit atrás de um proxy inverso com
  TLS (Caddy, Traefik, nginx…) e defina
  `DUMBMONIT_COOKIE_SECURE=1`. Nunca exponha a porta HTTP simples. O
  Claude Desktop, Claude Code, VS Code e Cursor correm na sua máquina
  e podem alcançar um endereço LAN diretamente, por isso não precisam
  de nenhuma exposição.
- **Vigie «last used».** A lista de tokens mostra quando cada token
  foi usado por último e a partir de que endereço. Um token usado
  numa hora em que não estava a falar com o seu assistente, ou a
  partir de um endereço que não conhece, merece uma revogação.
- **Registos.** Toda chamada de ferramenta é registada ao nível
  `info` com o nome do token e a ferramenta, nunca os argumentos;
  toda escrita através da API REST com o nome do token, o método, o
  caminho e o estado, nunca o corpo. Criar e revogar tokens está no
  registo de auditoria (Settings → Security).

## Resolução de problemas {#troubleshooting}

| Sintoma | Causa e correção |
| --- | --- |
| `401` / «A valid API token is required.» | O cabeçalho está em falta ou mal escrito (`Authorization: Bearer dmt_…`, um espaço depois de `Bearer`), ou o token foi revogado. Crie um novo. |
| `401` / «The API token "…" expired on …» | Crie um novo token; escolha uma vida mais longa se foi demasiado curta. |
| `403` / «cannot be used from …» | O token está restrito a outras redes. O endereço na mensagem é o que o servidor vê: atrás de um proxy inverso, declare o proxy em `DUMBMONIT_TRUSTED_PROXIES`. |
| `403` em toda chamada de um cliente baseado em navegador | A sua `Origin` não é permitida: adicione-a a `DUMBMONIT_API_CORS_ORIGINS`. |
| Uma ferramenta responde «needs a token with the "write" scope» | O token é só de leitura, ou o seu criador já não é administrador. Crie um token de escrita se quiser que o assistente aja. |
| `429` | 120 chamadas num minuto: o assistente está provavelmente em loop. Espere por `Retry-After`, depois faça uma pergunta mais restrita. |
| «Unsupported protocol version» | O cliente fala uma versão que o DumbMonit não fala; atualize o cliente. |
| O Claude Desktop mostra o servidor como falhado | O Node.js tem de estar instalado para o `npx`; via HTTP simples, `--allow-http` tem de estar em `args`. |
| O ChatGPT não consegue ligar-se | Precisa de um URL HTTPS público; um endereço LAN ou HTTP simples não funciona a partir dos servidores da OpenAI. |
