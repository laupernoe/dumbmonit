# Caso de garantia

Esta página argumenta que o DumbMonit atende aos seus requisitos de segurança:
ela expõe as ameaças, os limites onde a confiança muda, por que o design limita
os danos de um erro, como cada classe comum de fraqueza é combatida e quais
evidências sustentam cada afirmação. Ela estende o
[modelo de segurança](security-model.md), que lista as mesmas medidas de forma
mais resumida. Para relatar uma vulnerabilidade, siga a
[política de segurança](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

Os caminhos de arquivo abaixo são relativos à raiz do repositório.

## Requisitos de segurança

1. Somente um usuário autenticado, com um papel que o permita, pode ler ou
   alterar a configuração, os alertas ou as medições.
2. As credenciais que o DumbMonit armazena para os seus dispositivos não podem
   ser lidas a partir do arquivo do banco de dados, de um pacote de backup sem
   sua senha, nem pela API.
3. Somente uma máquina com um token válido pode enviar medições, e um token
   vazado pode ser revogado isoladamente.
4. O DumbMonit não pode ser transformado em um meio de alcançar serviços que não
   deveria, em particular o seu próprio processo filho VictoriaMetrics, sem
   autenticação.
5. Uma página servida a um navegador não pode executar script que ela não
   trouxe, e um site de terceiros não pode fazer um navegador autenticado
   alterar nada.

## Modelo de ameaças

| Atacante | Capacidade | No escopo |
|---|---|---|
| Cliente de rede anônimo | Alcança a porta HTTP | Sim |
| Usuário autenticado com papel baixo | Sessão válida ou token de API somente leitura | Sim (escalada de privilégios) |
| Página web maliciosa | Executa no navegador de um admin | Sim (CSRF, XSS, clickjacking) |
| Dispositivo monitorado comprometido ou malicioso | Responde às sondagens com conteúdo hostil | Sim (parsers, redirecionamentos) |
| Agente malicioso ou roubado | Possui um token de agente | Sim (alcance de um único token) |
| Admin que insere uma URL hostil | Configura um webhook ou endereço de dispositivo | Sim, para os endereços do próprio servidor |
| Quem tem o host, o volume `/data` ou o runtime de contêineres | Controle total | Não: fora do escopo, como na política de segurança |
| Vulnerabilidades no VictoriaMetrics ou nos dispositivos monitorados | Código de terceiros | Não: relate ao projeto original |

## Limites de confiança

| Limite | O que o atravessa | Como é protegido | Evidência |
|---|---|---|---|
| Navegador para `/api/*` | Cookie de sessão ou token de API, JSON | Um middleware de autenticação, três papéis, cabeçalho CSRF nas escritas autenticadas por cookie | `crates/server/src/auth/middleware.rs`; testes `auth_api.rs`, `operator_role_api.rs`, `api_open_security_api.rs`, `cors_policy_api.rs` |
| Navegador para as páginas servidas | HTML, script, estilo | Content Security Policy com nonce por resposta, `nosniff`, `Referrer-Policy: same-origin`, `X-Frame-Options: DENY` | `crates/server/src/api/mod.rs` (camada de cabeçalhos de segurança) |
| Provedor de identidade para o servidor | Redirecionamento OpenID Connect e ID token | State, nonce e PKCE (S256); a vinculação de contas locais é explícita | `crates/server/src/auth/oidc/`; teste `oidc_api.rs` |
| Agente para `/api/ingest` | Medições, resultados de comandos | Token por agente, armazenado como hash SHA-256 (prefixo `dmon_`) | `crates/server/src/api/ingest.rs`, `crates/server/src/collectors/agent/token.rs`; teste `agent_binding_api.rs` |
| Agente relay para o servidor | Resultados de sondagens de outra rede | Mesma verificação de token de agente (`agent_commands::authenticate`) | `crates/server/src/api/relay.rs`; teste `relay_api.rs` |
| Servidor para dispositivos monitorados | Requisições de sondagem, respostas hostis | Redirecionamentos seguidos apenas no mesmo host, número de redirecionamentos limitado, parsers de resposta com testes unitários, verificação de certificado ativada por padrão | `crates/collectors/src/http.rs` (`same_host_redirects`) |
| Servidor para canais de notificação | Requisições de webhook e SMTP | Mesma política de redirecionamento, mais uma proteção que recusa os endereços do próprio servidor, verificada antes do envio, a cada redirecionamento e na resolução DNS | `crates/server/src/notify/guard.rs`; teste `notify_guard_api.rs` |
| Servidor para VictoriaMetrics | Escritas e consultas de leitura | O processo filho embutido escuta em `127.0.0.1:8428` por padrão; o proxy compatível com Prometheus repassa apenas as rotas de leitura (query, query_range, series, labels) | `crates/server/src/config.rs` (`DUMBMONIT_VM_LISTEN`), `crates/server/src/api/prometheus.rs` (`readable_segments`) |
| Servidor para SQLite | Configuração e estado | Parâmetros vinculados através do `sqlx` | `crates/server/src/db/` |
| Servidor para `/data` | Banco de dados, chave secreta | Segredos criptografados com AES-256-GCM; o usuário do contêiner é dono apenas de `/data` | `crates/server/src/crypto.rs`, `Dockerfile` |

## Argumento de design seguro

O argumento segue os princípios de design de Saltzer e Schroeder.

- **Economia de mecanismo.** Um binário, um arquivo SQLite, um processo filho em
  loopback, uma única entrada para todos os clientes (`/api/*`).
- **Padrões à prova de falhas.** Em uma instância nova, toda rota sob a proteção
  de autenticação responde `401`; apenas saúde, estado de autenticação,
  configuração inicial e login respondem, de modo que ninguém pode preparar a
  instância antes do seu dono (`auth/middleware.rs`, teste
  `api_open_security_api.rs`). O VictoriaMetrics escuta em loopback, a menos que
  o operador altere isso. Os certificados TLS dos serviços monitorados são
  verificados, a menos que o operador defina `insecure_tls`
  (`crates/collectors/src/http.rs`).
- **Mediação completa.** O middleware de autenticação roda em toda rota da API,
  em vez de cada handler verificar por conta própria.
- **Design aberto.** O código, a descrição da API
  (`crates/server/src/api/openapi.yaml`) e esta página são públicos. A
  segurança não depende do sigilo do design.
- **Separação de privilégios.** Sessões, tokens de API e tokens de agente são
  credenciais diferentes. Os tokens de API têm escopo `read` ou `write`
  (`auth/token.rs`) e os usuários têm papel de visualizador, operador ou admin.
- **Privilégio mínimo.** O servidor roda com o UID 65532 (ou `PUID`): iniciado
  como root, apenas entrega o diretório de dados a esse usuário, sem seguir
  links, e depois abandona o root de vez e verifica que não consegue
  recuperá-lo, antes de abrir qualquer outra coisa (`privileges.rs`). O arquivo
  Compose mantém só as quatro capabilities de que este passo precisa, define um
  sistema de arquivos raiz somente leitura e `no-new-privileges`
  (`docker-compose.yml`).
- **Mínimo de mecanismo comum.** Os agentes se autenticam com seu próprio token,
  então revogar uma máquina não afeta nenhum usuário nem outro agente.
- **Aceitabilidade psicológica.** O comportamento seguro é o padrão e não exige
  nenhum passo extra: o cookie é `HttpOnly` e `SameSite=Lax`, o cabeçalho CSRF é
  adicionado pelo cliente web, os segredos são somente de escrita na interface.
- **Defesa em profundidade.** Criptografia em repouso além do controle de
  acesso; o cabeçalho CSRF além do `SameSite`; o limite de taxa de login além do
  hash Argon2; uma política de redirecionamento além da proteção contra
  endereços do próprio servidor.

### Criptografia

- Senhas: Argon2 (`auth/password.rs`). Os segredos de sessão e de token de API
  são armazenados como hashes SHA-256 de valores aleatórios (`auth/session.rs`,
  `auth/token.rs`).
- Segredos em repouso: AES-256-GCM com um nonce novo a cada criptografia
  (`crypto.rs`). A chave é derivada com Argon2id a partir de `DUMBMONIT_SECRET`
  ou do arquivo de chave gerado, com um salt persistente; um segredo com menos
  de 16 caracteres é recusado, e um valor canário detecta uma chave errada na
  inicialização.
- Backups: a chave é derivada da senha do operador com Argon2id, com os
  parâmetros armazenados no pacote, de modo que o algoritmo pode mudar sem
  quebrar pacotes antigos (`backup/bundle.rs`).
- Segundo fator: TOTP, com códigos de recuperação de uso único armazenados como
  hash (`auth/totp.rs`).
- OpenID Connect: state, nonce e PKCE `S256` (`auth/oidc/pkce.rs`).
- Todas as primitivas vêm de bibliotecas mantidas (`aes-gcm`, `argon2`, `sha2`);
  nenhuma é caseira. O listener embutido fala HTTP simples; a documentação exige
  um proxy reverso TLS, e os cookies ganham `Secure` quando a requisição chega
  por HTTPS (`auth/cookie.rs`).

## Contramedidas para fraquezas comuns

| Fraqueza (OWASP Top 10, CWE) | Contramedida | Evidência |
|---|---|---|
| Quebra de controle de acesso, CWE-862, CWE-285 | Middleware de autenticação, papéis, escopos de token | `operator_role_api.rs`, `api_tokens_api.rs`, `api_open_security_api.rs` |
| Falhas criptográficas, CWE-327, CWE-312 | AES-256-GCM em repouso, Argon2, nenhum segredo retornado pela API | testes unitários de `crypto.rs`; `has_secret` / `credential_kind` no lugar do segredo |
| Injeção de SQL, CWE-89 | Parâmetros vinculados através do `sqlx` | `crates/server/src/db/` |
| Cross-site scripting, CWE-79 | O Svelte escapa a saída; CSP com `default-src 'none'`, scripts apenas de `'self'` e um nonce; `base-uri 'none'` | `api/mod.rs` |
| Cross-site request forgery, CWE-352 | Escritas por cookie exigem `X-Requested-With: DumbMonit` ou metadados fetch de mesma origem; o CORS não concede o cabeçalho a ninguém; `SameSite=Lax` | `auth/middleware.rs`, `auth/cors.rs`; teste `cors_policy_api.rs` |
| Clickjacking, CWE-1021 | `X-Frame-Options: DENY` e `frame-ancestors 'none'`, exceto nas páginas de status públicas opcionais destinadas a serem incorporadas | `api/mod.rs` |
| Server-side request forgery, CWE-918 | Apenas redirecionamentos para o mesmo host; a proteção de notificações recusa os endereços de escuta do próprio servidor e do VictoriaMetrics, inclusive após a resolução DNS (rebinding). Loopback e faixas privadas permanecem acessíveis de propósito, porque os serviços de homelab vivem lá | `notify/guard.rs`; teste `notify_guard_api.rs` |
| Falhas de autenticação, CWE-307, CWE-287 | Limite de taxa de login com atraso crescente por endereço e por conta, TOTP opcional, OIDC com PKCE, redefinição do segundo fator por um admin | `auth/rate_limit.rs`, `auth/totp.rs`, `auth/oidc/` |
| Desserialização insegura, esgotamento de recursos, CWE-400 | JSON tipado via `serde`, corpos de requisição limitados a 16 MiB, redirecionamentos limitados | `api/mod.rs`, `collectors/src/http.rs` |
| Segurança de memória, CWE-787, CWE-416 | Rust; `unsafe` aparece apenas no código de contadores de desempenho do agente Windows (`crates/agent/src/collect/perf_counters.rs`) | grep por `unsafe` |
| Componentes vulneráveis, CWE-1104 | `cargo deny` (licenças, bans, avisos); Dependabot; varredura Trivy das imagens publicadas; actions fixadas por commit; builds com `--locked` | `.github/workflows/ci.yml`, `security.yml`, `.github/dependabot.yml`, `deny.toml` |
| Integridade de software e dados, CWE-494 | Os instaladores do agente verificam o SHA-256 do binário baixado | `crates/agent/install/install.sh`, `install.ps1` |
| Falhas de registro e monitoramento, CWE-778 | Log de segurança de logins e alterações sensíveis | `auth/audit.rs`; teste `security_audit_api.rs` |

Os riscos residuais conhecidos estão listados no modelo de segurança (listener
HTTP simples, opções de dispositivo sem criptografia, VictoriaMetrics embutido
sem autenticação). A CSP permite estilos inline (`style-src 'unsafe-inline'`), o
que a enfraquece contra injeção de estilo, mas não contra execução de script.
Existe um aviso aceito, `RUSTSEC-2023-0071`, ignorado em `deny.toml` com o seu
motivo: o crate `rsa` verifica assinaturas RS256 com a chave pública do
provedor e nunca usa uma chave privada.

## Evidências

- **Integração contínua** (`.github/workflows/ci.yml`): `cargo fmt --check`,
  `clippy -D warnings`, a suíte completa de testes Rust,
  `cargo deny check licenses bans` e `cargo deny check advisories`, verificação
  de tipos web, testes e build web, e um build da imagem Docker, a cada push e
  pull request.
- **Varreduras agendadas** (`.github/workflows/security.yml`): Trivy nas imagens
  publicadas do servidor e do agente, e o OpenSSF Scorecard, semanalmente e após
  cada publicação `:edge`. Os resultados aparecem na aba Security do
  repositório, onde o code scanning do GitHub (CodeQL) também reporta.
- **Testes**: testes de API, autenticação, papéis, CORS, OIDC, proteção e
  auditoria em `crates/server/tests/`, testes unitários junto ao código e
  testes web em `web/tests/`. A cobertura de linhas Rust medida com
  `cargo-llvm-cov` é de 87,9 %.
- **Política de regressão**: uma correção vem com um teste que falha sem ela
  (veja a política de testes em `CONTRIBUTING.md`).
- **Registro público**: as correções de segurança são publicadas como avisos e
  listadas nas notas de lançamento.
