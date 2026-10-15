# Modelo de segurança

Esta página descreve o que o DumbMonit protege, em quem ele confia, como o
design limita os danos de um erro e onde olhar no código e nos testes. Para
relatar uma vulnerabilidade, siga a
[política de segurança](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## O que está em jogo

O DumbMonit roda dentro da sua rede. Ele armazena as credenciais de que precisa
para ler cada dispositivo (communities SNMP, tokens de API, senhas), recebe
medições de agentes nas suas máquinas e envia requisições para endereços que
você configura. Um comprometimento exporia esses segredos e daria a um atacante
um ponto de apoio na rede monitorada.

## Limites de confiança

| Limite | Quem está do outro lado | Confiança |
|---|---|---|
| Navegador ou script para `/api/*` | Admins, operadores e visualizadores | Autenticado por cookie de sessão ou token de API; três papéis limitam o que cada um pode fazer. |
| Agente para `/api/ingest` | Máquinas em que você instalou um agente | Um token por máquina; um token vazado pode ser revogado isoladamente. |
| Servidor para dispositivos e serviços | Sistemas de terceiros | Sem confiança: as respostas são analisadas de forma defensiva e os redirecionamentos permanecem no mesmo host. |
| Servidor para canais de notificação | Serviços externos | As credenciais são criptografadas em repouso e nunca retornadas pela API. |
| Servidor para VictoriaMetrics | Um processo filho em loopback, ou uma instância externa | O embutido não tem autenticação e escuta em loopback; uma instância externa é responsabilidade do operador. |
| Volume `/data` e host | O operador | Confiável. Um atacante que controle o host ou o volume está fora do escopo. |

## Princípios de design seguro aplicados

- **Privilégio mínimo.** O contêiner roda como usuário sem privilégios, sem
  nenhuma capability, com sistema de arquivos raiz somente leitura e
  `no-new-privileges`. Três papéis (visualizador, operador, admin) restringem o
  que um usuário pode alterar (`crates/server/src/auth/users.rs`; teste
  `operator_role_api.rs`).
- **Seguro por padrão.** Em uma instância nova, apenas as rotas de saúde,
  configuração inicial, login e estado de autenticação respondem até que o
  primeiro admin exista (teste `api_open_security_api.rs`).
- **Economia de mecanismo.** Um binário, um arquivo SQLite, um processo filho em
  loopback e uma única porta de entrada para todos os clientes: `/api/*`.
- **Padrões à prova de falhas.** Uma verificação de segurança cujos dados estão
  ausentes é `unknown`, nunca uma aprovação; um dispositivo inacessível e um
  erro de configuração são estados diferentes.
- **Mediação completa.** Um único middleware de autenticação
  (`crates/server/src/auth/middleware.rs`) protege as rotas da API.
- **Separação de privilégios.** Sessões, tokens de API e tokens de agente são
  credenciais diferentes, com alcances diferentes.
- **Defesa em profundidade.** Criptografia em repouso além do controle de
  acesso, um cabeçalho CSRF além dos flags do cookie, um limite de taxa além do
  hash de senhas.
- **Nenhum segredo faz o caminho de volta.** A API retorna `has_secret` e
  `credential_kind`, nunca o segredo; omitir um segredo em um `PUT` o mantém.
- **Apenas criptografia consolidada.** As senhas são protegidas com Argon2, os
  segredos são criptografados com AES-256-GCM (`crypto.rs`), a autenticação de
  dois fatores usa TOTP com códigos de recuperação de uso único armazenados como
  hash, e o OpenID Connect usa state, nonce e PKCE. Nada é caseiro.

## Contramedidas por classe de fraqueza

| Classe (tema do OWASP Top 10) | Medidas |
|---|---|
| Quebra de controle de acesso | Um middleware de autenticação; papéis; uma instância nova recusa tudo, exceto configuração inicial, login, saúde e estado de autenticação. |
| Falhas criptográficas | AES-256-GCM em repouso, chave mantida fora do banco de dados; hashes de senha Argon2; o servidor embutido fala HTTP simples, por isso um proxy reverso TLS está documentado. |
| Injeção | O SQL passa pelo `sqlx` com parâmetros vinculados; o proxy do VictoriaMetrics só encaminha consultas de leitura (`tsdb/mod.rs`, `proxy_read`); os parsers têm testes unitários. |
| Design inseguro | O modelo de ameaças acima; segredos nunca fazem o caminho de volta; uma instância de propósito único. |
| Configuração incorreta de segurança | Arquivos Compose reforçados, VictoriaMetrics somente em loopback, notas de hardening para o operador na política de segurança. |
| Componentes vulneráveis | O `cargo deny` verifica licenças, bans e avisos do RustSec; uma varredura semanal do Trivy nas imagens publicadas; Dependabot; GitHub Actions fixadas por commit. |
| Falhas de identificação e autenticação | Limite de taxa de login por endereço de cliente e por conta (`auth/rate_limit.rs`), autenticação de dois fatores opcional, OpenID Connect, um admin pode redefinir o segundo fator de um usuário e encerrar suas sessões. |
| Falhas de integridade de software e dados | Os instaladores do agente verificam o SHA-256 do binário baixado; as imagens de lançamento são construídas pela CI. |
| Falhas de registro e monitoramento | Um log de segurança de logins e alterações sensíveis (`auth/audit.rs`). |
| Server-side request forgery | Integrações e canais de notificação seguem redirecionamentos apenas para o mesmo host, de modo que um dispositivo não pode redirecionar uma requisição e sua chave de API para outro lugar. |
| Cross-site request forgery | Requisições que alteram estado e dependem do cookie devem carregar `X-Requested-With: DumbMonit` ou metadados fetch de mesma origem, caso contrário `403`; requisições autenticadas por token não carregam cookie. |

## Verificação

Cada push e pull request executa `clippy -D warnings`, a suíte de testes Rust
(incluindo testes de API, autenticação e papéis em `crates/server/tests/`), os
testes web em `web/tests/`, verificações de dependências e a análise CodeQL. O
OpenSSF Scorecard roda de forma agendada. As correções de segurança vêm com um
teste de regressão e são listadas, com seu identificador CVE ou GHSA, nas notas
de lançamento.

O [caso de garantia](assurance-case.md) apresenta o modelo de ameaças completo,
o argumento de design seguro e as evidências por trás de cada afirmação.

## Limites conhecidos

- O servidor embutido fala HTTP simples; coloque TLS na frente dele.
- As opções dos dispositivos são armazenadas sem criptografia. Coloque segredos
  nos campos de credencial.
- O VictoriaMetrics embutido não tem autenticação.
- O DumbMonit está antes da 1.0; rotas e campos ainda podem mudar.
