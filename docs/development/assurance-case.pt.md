# Argumentário de garantia

Esta página defende que o DumbMonit cumpre os seus requisitos de segurança:
enuncia as ameaças, as fronteiras onde a confiança muda, porque é que o desenho
limita os danos de um erro, como se contrapõe cada classe comum de fraqueza e
que provas sustentam cada afirmação. Prolonga o [modelo de segurança](security-model.md),
que lista as mesmas medidas de forma mais breve. Para comunicar uma
vulnerabilidade, siga a
[política de segurança](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

Os caminhos de ficheiros abaixo são relativos à raiz do repositório.

## Requisitos de segurança

1. Só um utilizador autenticado, com uma função que o permita, pode ler ou
   alterar a configuração, os alertas ou as medições.
2. As credenciais que o DumbMonit guarda para os seus monitores não são
   legíveis a partir do ficheiro da base de dados, de um pacote de cópia de
   segurança sem a sua frase-passe, nem a partir da API.
3. Só uma máquina com um token válido pode enviar medições, e um token
   divulgado pode ser revogado isoladamente.
4. O DumbMonit não pode ser transformado num meio de alcançar serviços que não
   deveria, em particular o seu próprio processo filho VictoriaMetrics, sem
   autenticação.
5. Uma página servida a um navegador não pode executar scripts que não trouxe
   consigo, e um site de terceiros não pode fazer com que um navegador com
   sessão iniciada altere seja o que for.

## Modelo de ameaças

| Atacante | Capacidade | No âmbito |
|---|---|---|
| Cliente de rede anónimo | Alcança a porta HTTP | Sim |
| Utilizador autenticado com uma função baixa | Sessão válida ou token de API só de leitura | Sim (escalada de privilégios) |
| Página web maliciosa | Executa-se no navegador de um administrador | Sim (CSRF, XSS, clickjacking) |
| Monitor comprometido ou malicioso | Responde às sondas com conteúdo hostil | Sim (analisadores, redirecionamentos) |
| Agente malicioso ou roubado | Possui um token de agente | Sim (alcance de um único token) |
| Administrador que introduz um URL hostil | Configura um webhook ou o endereço de um monitor | Sim, para os endereços do próprio servidor |
| Quem tem o anfitrião, o volume `/data` ou o runtime de contentores | Controlo total | Não: fora do âmbito, como na política de segurança |
| Vulnerabilidades no VictoriaMetrics ou nos monitores | Código a montante | Não: comunique a montante |

## Fronteiras de confiança

| Fronteira | O que a atravessa | Como é protegida | Prova |
|---|---|---|---|
| Navegador para `/api/*` | Cookie de sessão ou token de API, JSON | Um único middleware de autenticação, três funções, cabeçalho CSRF nas escritas autenticadas por cookie | `crates/server/src/auth/middleware.rs`; testes `auth_api.rs`, `operator_role_api.rs`, `api_open_security_api.rs`, `cors_policy_api.rs` |
| Navegador para as páginas servidas | HTML, script, estilo | Content Security Policy com nonce por resposta, `nosniff`, `Referrer-Policy: same-origin`, `X-Frame-Options: DENY` | `crates/server/src/api/mod.rs` (camada de cabeçalhos de segurança) |
| Fornecedor de identidade para o servidor | Redirecionamento OpenID Connect e token de ID | State, nonce e PKCE (S256); a associação a contas locais é explícita | `crates/server/src/auth/oidc/`; teste `oidc_api.rs` |
| Agente para `/api/ingest` | Medições, resultados de comandos | Token por agente, guardado como hash SHA-256 (prefixo `dmon_`) | `crates/server/src/api/ingest.rs`, `crates/server/src/collectors/agent/token.rs`; teste `agent_binding_api.rs` |
| Agente relay para o servidor | Resultados de sondas de outra rede | Mesma verificação do token de agente (`agent_commands::authenticate`) | `crates/server/src/api/relay.rs`; teste `relay_api.rs` |
| Servidor para monitores | Pedidos de sonda, respostas hostis | Redirecionamentos seguidos apenas no mesmo anfitrião, número de redirecionamentos limitado, analisadores de respostas com testes unitários, verificação de certificados ativa por predefinição | `crates/collectors/src/http.rs` (`same_host_redirects`) |
| Servidor para canais de notificação | Pedidos de webhook e SMTP | Mesma política de redirecionamentos, mais uma salvaguarda que recusa os endereços do próprio servidor, verificada antes do envio, em cada redirecionamento e na resolução DNS | `crates/server/src/notify/guard.rs`; teste `notify_guard_api.rs` |
| Servidor para VictoriaMetrics | Escritas e consultas de leitura | O processo filho embutido escuta em `127.0.0.1:8428` por predefinição; o proxy compatível com Prometheus só retransmite as rotas de leitura (query, query_range, series, labels) | `crates/server/src/config.rs` (`DUMBMONIT_VM_LISTEN`), `crates/server/src/api/prometheus.rs` (`readable_segments`) |
| Servidor para SQLite | Configuração e estado | Parâmetros associados através do `sqlx` | `crates/server/src/db/` |
| Servidor para `/data` | Base de dados, chave secreta | Segredos cifrados com AES-256-GCM; o utilizador do contentor é dono apenas de `/data` | `crates/server/src/crypto.rs`, `Dockerfile` |

## Argumento de conceção segura

O argumento segue os princípios de conceção de Saltzer e Schroeder.

- **Economia de mecanismo.** Um binário, um ficheiro SQLite, um processo filho
  em loopback, uma única entrada para todos os clientes (`/api/*`).
- **Predefinições à prova de falha.** Numa instância nova, todas as rotas sob a
  proteção de autenticação respondem `401`; só respondem o estado de saúde, o
  estado de autenticação, a configuração inicial e o início de sessão, pelo que
  ninguém pode preparar a instância antes do seu dono (`auth/middleware.rs`,
  teste `api_open_security_api.rs`). O VictoriaMetrics escuta em loopback, a
  menos que o operador o altere. Os certificados TLS dos serviços monitorizados
  são verificados, a menos que o operador defina `insecure_tls`
  (`crates/collectors/src/http.rs`).
- **Mediação completa.** O middleware de autenticação corre em todas as rotas da
  API, em vez de cada handler fazer a sua própria verificação.
- **Desenho aberto.** O código, a descrição da API
  (`crates/server/src/api/openapi.yaml`) e esta página são públicos. A
  segurança não depende do segredo do desenho.
- **Separação de privilégios.** Sessões, tokens de API e tokens de agente são
  credenciais diferentes. Os tokens de API têm um âmbito `read` ou `write`
  (`auth/token.rs`) e os utilizadores têm a função de leitor, operador ou
  administrador.
- **Privilégio mínimo.** A imagem corre com o UID 65532 (`Dockerfile`). Arrancado como root (`user:
  "0:0"`, para os bind mounts), o servidor limita-se a entregar a diretoria de
  dados a esse utilizador, sem seguir ligações, e depois abandona o root de vez
  e verifica que não o consegue recuperar, antes de abrir qualquer outra coisa
  (`privileges.rs`). O ficheiro Compose mantém apenas as quatro capabilities de
  que este passo precisa, define um sistema de ficheiros raiz só de leitura e
  `no-new-privileges` (`docker-compose.yml`).
- **Menor mecanismo comum.** Os agentes autenticam-se com o seu próprio token,
  pelo que revogar uma máquina não afeta nenhum utilizador nem outro agente.
- **Aceitabilidade psicológica.** O comportamento seguro é o predefinido e não
  exige passos extra: o cookie é `HttpOnly` e `SameSite=Lax`, o cabeçalho CSRF é
  acrescentado pelo cliente web, os segredos são só de escrita na interface.
- **Defesa em profundidade.** Cifragem em repouso por cima do controlo de
  acesso; o cabeçalho CSRF por cima do `SameSite`; o limite de tentativas de
  início de sessão por cima do hash Argon2; uma política de redirecionamentos
  por cima da salvaguarda de endereços próprios.

### Criptografia

- Palavras-passe: Argon2 (`auth/password.rs`). Os segredos de sessão e de token
  de API são guardados como hashes SHA-256 de valores aleatórios
  (`auth/session.rs`, `auth/token.rs`).
- Segredos em repouso: AES-256-GCM com um nonce novo em cada cifragem
  (`crypto.rs`). A chave é derivada com Argon2id a partir de `DUMBMONIT_SECRET`
  ou do ficheiro de chave gerado, com um salt persistente; um segredo com menos
  de 16 caracteres é recusado, e um valor-canário deteta uma chave errada no
  arranque.
- Cópias de segurança: a chave deriva da frase-passe do operador com Argon2id,
  com os parâmetros guardados no pacote, de modo que o algoritmo possa mudar
  sem quebrar pacotes antigos (`backup/bundle.rs`).
- Segundo fator: TOTP, com códigos de recuperação de utilização única guardados
  como hash (`auth/totp.rs`).
- OpenID Connect: state, nonce e PKCE `S256` (`auth/oidc/pkce.rs`).
- Todas as primitivas vêm de bibliotecas mantidas (`aes-gcm`, `argon2`,
  `sha2`); nenhuma é feita em casa. O listener integrado fala HTTP simples; a
  documentação exige um proxy inverso com TLS, e os cookies ganham `Secure`
  quando o pedido chega por HTTPS (`auth/cookie.rs`).

## Contramedidas às fraquezas comuns

| Fraqueza (OWASP Top 10, CWE) | Contramedida | Prova |
|---|---|---|
| Controlo de acesso quebrado, CWE-862, CWE-285 | Middleware de autenticação, funções, âmbitos dos tokens | `operator_role_api.rs`, `api_tokens_api.rs`, `api_open_security_api.rs` |
| Falhas criptográficas, CWE-327, CWE-312 | AES-256-GCM em repouso, Argon2, nenhum segredo devolvido pela API | Testes unitários de `crypto.rs`; `has_secret` / `credential_kind` em vez do segredo |
| Injeção de SQL, CWE-89 | Parâmetros associados através do `sqlx` | `crates/server/src/db/` |
| Cross-site scripting, CWE-79 | O Svelte escapa a saída; CSP com `default-src 'none'`, scripts apenas de `'self'` e de um nonce; `base-uri 'none'` | `api/mod.rs` |
| Cross-site request forgery, CWE-352 | As escritas por cookie exigem `X-Requested-With: DumbMonit` ou metadados de fetch do mesmo domínio; o CORS não concede o cabeçalho a ninguém; `SameSite=Lax` | `auth/middleware.rs`, `auth/cors.rs`; teste `cors_policy_api.rs` |
| Clickjacking, CWE-1021 | `X-Frame-Options: DENY` e `frame-ancestors 'none'`, exceto nas páginas de estado públicas, opcionais, concebidas para serem incorporadas | `api/mod.rs` |
| Server-side request forgery, CWE-918 | Apenas redirecionamentos no mesmo anfitrião; a salvaguarda das notificações recusa os endereços de escuta do próprio servidor e do VictoriaMetrics, inclusive após a resolução DNS (rebinding). O loopback e os intervalos privados continuam acessíveis de propósito, porque é aí que vivem os serviços de homelab | `notify/guard.rs`; teste `notify_guard_api.rs` |
| Falhas de autenticação, CWE-307, CWE-287 | Limite de tentativas de início de sessão com atraso crescente por endereço e por conta, TOTP opcional, OIDC com PKCE, reposição do segundo fator por um administrador | `auth/rate_limit.rs`, `auth/totp.rs`, `auth/oidc/` |
| Desserialização insegura, esgotamento de recursos, CWE-400 | JSON tipado via `serde`, corpos de pedidos limitados a 16 MiB, redirecionamentos limitados | `api/mod.rs`, `collectors/src/http.rs` |
| Segurança de memória, CWE-787, CWE-416 | Rust; o `unsafe` só aparece no código dos contadores de desempenho do agente Windows (`crates/agent/src/collect/perf_counters.rs`) | grep por `unsafe` |
| Componentes vulneráveis, CWE-1104 | `cargo deny` (licenças, proibições, avisos); Dependabot; análise Trivy das imagens publicadas; actions fixadas por commit; compilações com `--locked` | `.github/workflows/ci.yml`, `security.yml`, `.github/dependabot.yml`, `deny.toml` |
| Integridade do software e dos dados, CWE-494 | Os instaladores do agente verificam o SHA-256 do binário descarregado | `crates/agent/install/install.sh`, `install.ps1` |
| Falhas de registo e monitorização, CWE-778 | Registo de segurança dos inícios de sessão e das alterações sensíveis | `auth/audit.rs`; teste `security_audit_api.rs` |

Os riscos residuais conhecidos estão listados no modelo de segurança (listener
HTTP simples, opções de monitores sem cifragem, VictoriaMetrics embutido sem
autenticação). A CSP permite estilos inline (`style-src 'unsafe-inline'`), o
que a enfraquece contra a injeção de estilos, mas não contra a execução de
scripts. Existe um aviso aceite, `RUSTSEC-2023-0071`, ignorado em `deny.toml`
com o respetivo motivo: o crate `rsa` verifica assinaturas RS256 com a chave
pública do fornecedor e nunca usa uma chave privada.

## Provas

- **Integração contínua** (`.github/workflows/ci.yml`): `cargo fmt --check`,
  `clippy -D warnings`, a suite completa de testes Rust,
  `cargo deny check licenses bans` e `cargo deny check advisories`, verificação
  de tipos da web, testes e compilação da web, e uma construção da imagem
  Docker, em cada push e pull request.
- **Análises agendadas** (`.github/workflows/security.yml`): Trivy sobre as
  imagens publicadas do servidor e do agente, e o OpenSSF Scorecard, todas as
  semanas e após cada publicação de `:edge`. Os resultados aparecem no separador
  Security do repositório, onde o GitHub code scanning (CodeQL) também reporta.
- **Testes**: testes de API, autenticação, funções, CORS, OIDC, salvaguarda e
  auditoria em `crates/server/tests/`, testes unitários junto ao código e
  testes web em `web/tests/`. A cobertura de linhas de Rust medida com
  `cargo-llvm-cov` é de 91 %.
- **Política de regressão**: uma correção traz um teste que falha sem ela
  (consulte a política de testes em `CONTRIBUTING.md`).
- **Registo público**: as correções de segurança são publicadas como avisos e
  listadas nas notas de versão.
