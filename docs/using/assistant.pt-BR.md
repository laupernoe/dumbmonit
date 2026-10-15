# Conectar um assistente {#connect-an-assistant}

O DumbMonit tem um servidor [MCP](https://modelcontextprotocol.io)
embutido: o Model Context Protocol, a forma como assistentes como
Claude, ChatGPT, Cursor ou o agente do VS Code chamam ferramentas
externas. Conecte um, depois pergunte coisas como:

- "Tudo está bem?"
- "Silencie o NAS por duas horas, estou trocando um disco."
- "O que aconteceu ontem à noite?"
- "Quão cheio está o servidor de backup, e está piorando?"
- "Poste uma atualização na página de status: o servidor de e-mail
  voltou."

O assistente vê o que a interface vê — monitores, alertas, histórico,
métricas, agentes, páginas de status — e usa as mesmas palavras:
*reportando*, *inacessível*, *esperando*, *aviso*, *alerta*, *suprimido
pelo pai*. Ele roda dentro do servidor: nada para instalar ao lado do
DumbMonit, e as ferramentas chamam o mesmo código que a interface web.

## 1. Criar um token {#1-create-a-token}

Configurações → **API e assistentes** (`/settings#assistant`). Nomeie o
token após o assistente ou a máquina em que roda ("Claude no meu
laptop"), depois escolha:

| Configuração | Escolhas |
| --- | --- |
| **Escopo** | **Leitura**: olhar — status, monitores, alertas, histórico, métricas, regras, agentes, páginas de status. Nunca pode alterar nada. **Leitura e escrita**: também agir — silenciar, reconhecer, sondar, adicionar um monitor, reiniciar um contêiner, postar um incidente… (as [ferramentas de escrita](#what-the-assistant-can-call)). |
| **Expira** | 30 dias, 90 dias (o padrão), 1 ano ou nunca. Um token expirado para de funcionar; crie um novo. |
| **Redes permitidas** | Opcional. Endereços ou faixas CIDR de onde o token pode ser usado (`192.168.1.0/24, 10.8.0.5`). Vazio: de qualquer lugar. |

O token parece com `dmt_` seguido de 32 caracteres hexadecimais. É
mostrado **uma vez**, imediatamente após a criação; o DumbMonit só
mantém um hash. Perdeu-o, revogue-o e crie outro.

Prefira um token de leitura a menos que você realmente queira que o
assistente aja.

O mesmo token também abre a [API REST](../reference/api.md#authentication)
para scripts e painéis — `curl -H "Authorization: Bearer dmt_…"
https://monit.example.lan/api/targets` — onde `read` vê o que um
visualizador vê e `write` faz o que um administrador faz, contas, tokens
e backups excetuados.

## 2. Cole o trecho {#2-paste-the-snippet}

A página de configurações os preenche com o endereço do seu servidor e o
novo token. Abaixo, `https://monit.example.lan` representa seu
servidor.

### Claude Code {#claude-code}

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

Adicione `--scope user` para tê-lo em todo projeto, não apenas o atual.

### Claude Desktop {#claude-desktop}

O Claude Desktop inicia servidores locais a partir de seu arquivo de
configuração; o [`mcp-remote`](https://www.npmjs.com/package/mcp-remote)
o conecta ao endpoint HTTP do DumbMonit (precisa do
[Node.js](https://nodejs.org)). O DumbMonit autentica com um cabeçalho
bearer em vez de OAuth, então adicione-o através deste arquivo em vez da
tela de Conectores. Settings → Developer → **Edit Config**, depois
adicione a `claude_desktop_config.json` e reinicie o Claude Desktop:

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
dois pontos, porque algumas plataformas separam argumentos por espaços.
Para um servidor alcançado via HTTP simples na sua rede, adicione
`"--allow-http"` em `args` — a página de configurações faz isso por
você quando ela mesma é servida via HTTP.

### VS Code {#vs-code}

`.vscode/mcp.json` em um workspace (ou o comando **MCP: Add Server** →
HTTP):

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

Um cliente que só inicia servidores locais (stdio) pode usar o bridge
`mcp-remote` mostrado para o Claude Desktop.

### ChatGPT {#chatgpt}

Settings → Connectors → **Create** (modo desenvolvedor). URL do
servidor MCP: `https://monit.example.lan/api/mcp`, cabeçalho de
autorização: `Bearer dmt_…`.

O ChatGPT se conecta a partir dos servidores da OpenAI, não do seu
navegador: o DumbMonit precisa estar **acessível pela internet via
HTTPS**. Veja as [notas de segurança](#security-notes) antes de fazer
isso.

### Qualquer outro cliente {#any-other-client}

O que um cliente precisa saber: o endpoint é
`https://<server>/api/mcp`, o transporte é Streamable HTTP, e toda
requisição carrega `Authorization: Bearer dmt_…`. Uma chamada bruta,
para verificar o token e o caminho de rede a partir de um terminal:

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
`destructiveHint`, `idempotentHint`), então um cliente pode dizer quais
só olham e pedir sua confirmação antes de executar as outras.

| Ferramenta | Escopo | Faz |
| --- | --- | --- |
| `get_status` | read | O boletim, a resposta a "tudo está bem?": contagens de monitor, alertas disparando e acumulando, uma frase ("2 avisos, 1 inacessível."). |
| `list_devices` | read | Monitores e serviços com estado, pai e tags; filtra por substring ou estado. |
| `get_device` | read | Um monitor: configuração, estado, alertas ativos, resumo de 24 horas de CPU / memória / disco mais cheio (disponibilidade e latência para uma verificação de serviço). |
| `list_alerts` | read | Alertas ativos com palavras de severidade, desde quando, valor, supressão, reconhecimento. |
| `alert_history` | read | Transições de alerta, mais recente primeiro (`since` ou `hours`, monitor opcional, limite). |
| `query_metrics` | read | Uma consulta de intervalo MetricsQL, no máximo 60 pontos por série. |
| `list_silences` | read | Janelas de manutenção e se estão ativas agora. |
| `list_rules` | read | Regras de alerta com tipo, severidade, limite, ativado. |
| `list_device_types` | read | Os tipos de monitor que este servidor pode monitorar, com as opções e credenciais que cada um espera — o que `add_device` precisa. |
| `list_agents` | read | Máquinas rodando o agente: SO, versão do agente, modo relay, site, monitores retransmitidos, último contato, vinculação. |
| `list_containers` | read | Os contêineres Docker de uma máquina rodando o agente: estado, saúde, reinícios, atualização disponível, último comando. |
| `list_heartbeats` | read | Monitores heartbeat: intervalo esperado, última chamada, veredito (esperando, no horário, perdido, reportado fora do ar). A URL secreta nunca é mostrada. |
| `list_status_pages` | read | Páginas de status, publicadas ou não, com seu caminho público (`/s/<slug>`), os monitores que mostram e seus incidentes abertos. |
| `list_channels` | read | Canais de notificação: tipo, ativado, último erro, última mensagem enviada. Nunca seus segredos. |
| `list_packs` | read | Pacotes de integração instalados: versão, tipo de monitor, ativado, monitores que o usam, erros. |
| `silence_device` | write | Uma janela de manutenção pontual em um monitor, começando agora (padrão 1 hora, no máximo uma semana). |
| `remove_silence` | write | Remove uma janela de manutenção. |
| `schedule_maintenance` | write | Uma janela de manutenção pontual em uma data específica, em um monitor ou toda a instância, no máximo uma semana. |
| `acknowledge_alert` | write | Reconhece um alerta por impressão digital por um número de horas (padrão 4): lembretes pausam, a resolução ainda é notificada. `hours: 0` o remove. |
| `probe_device` | write | Sonda um monitor imediatamente e reporta o que foi medido. |
| `set_device_enabled` | write | Ativa ou desativa o monitoramento de um monitor. |
| `set_rule_enabled` | write | Ativa ou desativa uma regra de alerta. |
| `add_device` | write | Adiciona um monitor: nome, tipo, endereço, intervalo, pai, opções, credenciais, agente relay. |
| `discover_network` | write | Varre uma rede (CIDR, no máximo 4096 endereços) por dispositivos SNMP e sugere um perfil para cada um. |
| `restart_container` | write | Pede ao agente de uma máquina para reiniciar um de seus contêineres. |
| `test_channel` | write | Envia uma mensagem de teste através de um canal de notificação. |
| `post_incident` | write | Abre um incidente em uma página de status, ou posta uma atualização em um existente. |

Um token de leitura chamando uma ferramenta de escrita recebe uma
recusa clara como resultado da ferramenta (`isError: true`); o
assistente a explica e nada muda. O mesmo vale para qualquer coisa que
a ferramenta não pode fazer — um monitor desconhecido, um nome
ambíguo, uma janela mais longa que uma semana: a mensagem diz o que
corrigir.

## Detalhes do protocolo {#protocol-details}

`POST /api/mcp` implementa MCP sobre o transporte **Streamable HTTP**:
JSON-RPC 2.0, uma requisição por `POST`, respondida com JSON simples
(sem server-sent events). É sem estado: nenhum `Mcp-Session-Id` é
emitido, um que um cliente enviar é ignorado, e `GET` ou `DELETE` no
endpoint respondem `405`. Só a capacidade `tools` é oferecida — sem
recursos, prompts, amostragem ou OAuth.

Duas gerações do protocolo são servidas no mesmo endpoint:

| Versões | Como um cliente fala |
| --- | --- |
| `2026-07-28` | Sem estado. Toda requisição carrega `io.modelcontextprotocol/protocolVersion` e `io.modelcontextprotocol/clientCapabilities` em `params._meta`, e os cabeçalhos `MCP-Protocol-Version`, `Mcp-Method` e, para `tools/call`, `Mcp-Name`, que precisam corresponder ao corpo. Métodos: `server/discover`, `tools/list`, `tools/call`. Resultados carregam `resultType: "complete"` e a identidade do servidor em `_meta` (`io.modelcontextprotocol/serverInfo`); `tools/list` também carrega `ttlMs` e `cacheScope`. |
| `2025-11-25`, `2025-06-18`, `2025-03-26` | O handshake `initialize` escolhe a versão (uma desconhecida recebe a mais nova dessas três); `notifications/initialized` é confirmado com `202`; depois `ping`, `tools/list`, `tools/call`. |

Erros, além do `401`, `403` e `429` das
[verificações de token](#security-notes), que vêm antes de qualquer
JSON-RPC:

| HTTP | Código JSON-RPC | Quando |
| --- | --- | --- |
| `400` | `-32020` (incompatibilidade de cabeçalho) | Uma requisição `2026-07-28` sem `MCP-Protocol-Version`, `Mcp-Method` ou `Mcp-Name`, ou com valores que diferem do corpo. |
| `400` | `-32022` (versão de protocolo não suportada) | Uma versão que este servidor não fala; `error.data` lista as `supported` e a `requested`. |
| `400` | `-32602` (parâmetros inválidos) | Uma requisição `2026-07-28` faltando os campos `_meta` obrigatórios. |
| `404` | `-32601` (método não encontrado) | Uma requisição `2026-07-28` para um método que este servidor não implementa. |
| `200` | `-32602` | Uma ferramenta desconhecida, ou `arguments` que não é um objeto. |
| | `-32700`, `-32600` | JSON ilegível; não é uma requisição JSON-RPC 2.0 (lotes não são suportados). |
| `202` | — | Uma notificação: aceita, sem corpo. |

Uma ferramenta que roda mas falha — escopo faltando, monitor
desconhecido — não é um erro de protocolo: responde normalmente com
`isError: true` e uma frase que o assistente pode retransmitir.

## Notas de segurança {#security-notes}

- **Um token é uma senha.** Quem o possui lê tudo que a interface
  mostra — e, com um token de escrita, altera qualquer coisa que um
  administrador pode através da API REST (monitores, regras,
  canais), contas, tokens e backups excetuados. Mantenha-o fora de
  chats compartilhados, capturas de tela e repositórios. Revogue um
  token de que você não tem certeza; crie outro em um minuto.
- **Prefira leitura.** A maioria das perguntas ("tudo está bem?") não
  precisa de escopo de escrita. Crie um token de escrita apenas para um
  assistente que você realmente quer que aja, e nomeie-o para
  reconhecê-lo na lista. A maioria dos clientes pede confirmação antes
  de executar uma ferramenta que não é marcada como somente leitura;
  deixe essa confirmação ativada.
- **Dê a ele uma validade e uma rede.** Um token que expira em 90 dias
  e só funciona da sua LAN ou VPN (`192.168.1.0/24`) vale pouco para
  quem o encontrar. Detrás de um proxy reverso, declare o proxy em
  `DUMBMONIT_TRUSTED_PROXIES`, senão toda chamada parece vir do proxy
  e uma restrição de rede as recusa todas.
- **Vinculado a seu criador.** Um token para de funcionar quando a
  conta que o criou é desativada, perde seu poder de escrita quando
  essa conta não é mais administradora, e é revogado quando essa conta
  é excluída.
- **Nada secreto volta.** As ferramentas nunca retornam credenciais de
  monitor (apenas seu tipo), segredos de canal, URLs de heartbeat
  ou tokens. `add_device` recebe credenciais: elas viajam pela
  conversa do seu assistente, e portanto por seu provedor. Para um
  monitor com credenciais sensíveis, prefira a interface web.
- **Navegadores e `Origin`.** Uma requisição que carrega um cabeçalho
  `Origin` (um cliente baseado em navegador) é recusada com `403` a
  menos que essa origem seja a do próprio servidor ou seja permitida
  por `DUMBMONIT_API_CORS_ORIGINS` (por padrão qualquer origem, já que
  o token é exigido de qualquer forma). Liste suas origens ali para
  estreitar; veja
  [requisições entre origens](../reference/api.md#cross-origin-requests-cors).
- **Limite de taxa.** Cada token é limitado a 120 chamadas por minuto,
  REST e MCP juntos: um assistente travado em loop recebe `429` com
  `Retry-After`, não toda a atenção do seu servidor.
- **HTTPS antes da internet.** Os conectores do ChatGPT precisam de uma
  URL pública. Coloque o DumbMonit detrás de um proxy reverso com TLS
  (Caddy, Traefik, nginx…) e defina `DUMBMONIT_COOKIE_SECURE=1`. Nunca
  exponha a porta HTTP simples. Claude Desktop, Claude Code, VS Code e
  Cursor rodam na sua máquina e podem alcançar um endereço LAN
  diretamente, então não precisam de exposição alguma.
- **Observe "último uso".** A lista de tokens mostra quando cada token
  foi usado por último e de qual endereço. Um token usado em um
  momento em que você não estava falando com seu assistente, ou de um
  endereço que você não conhece, merece uma revogação.
- **Logs.** Toda chamada de ferramenta é registrada em nível `info`
  com o nome do token e a ferramenta, nunca os argumentos; toda
  escrita através da API REST com o nome do token, o método, o
  caminho e o status, nunca o corpo. Criar e revogar tokens está no
  log de auditoria (Configurações → Segurança).

## Solução de problemas {#troubleshooting}

| Sintoma | Causa e correção |
| --- | --- |
| `401` / "A valid API token is required." | O cabeçalho está faltando ou mal digitado (`Authorization: Bearer dmt_…`, um espaço depois de `Bearer`), ou o token foi revogado. Crie um novo. |
| `401` / "The API token "…" expired on …" | Crie um novo token; escolha uma validade mais longa se foi muito curta. |
| `403` / "cannot be used from …" | O token está restrito a outras redes. O endereço na mensagem é o que o servidor vê: detrás de um proxy reverso, declare o proxy em `DUMBMONIT_TRUSTED_PROXIES`. |
| `403` em toda chamada de um cliente baseado em navegador | Seu `Origin` não é permitido: adicione-o a `DUMBMONIT_API_CORS_ORIGINS`. |
| Uma ferramenta responde "needs a token with the "write" scope" | O token é somente leitura, ou seu criador não é mais administrador. Crie um token de escrita se você quer que o assistente aja. |
| `429` | 120 chamadas em um minuto: o assistente provavelmente está em loop. Espere por `Retry-After`, depois faça uma pergunta mais estreita. |
| "Unsupported protocol version" | O cliente fala uma versão que o DumbMonit não fala; atualize o cliente. |
| Claude Desktop mostra o servidor como falhado | O Node.js precisa estar instalado para `npx`; via HTTP simples, `--allow-http` precisa estar em `args`. |
| ChatGPT não consegue conectar | Precisa de uma URL HTTPS pública; um endereço LAN ou HTTP simples não funciona a partir dos servidores da OpenAI. |
