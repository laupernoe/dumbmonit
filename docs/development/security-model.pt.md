# Modelo de segurança

Esta página descreve o que o DumbMonit protege, em quem confia, como o desenho
limita os danos de um erro e onde olhar no código e nos testes. Para comunicar
uma vulnerabilidade, siga a
[política de segurança](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## O que está em jogo

O DumbMonit corre dentro da sua rede. Guarda as credenciais de que precisa para
ler cada monitor (comunidades SNMP, tokens de API, palavras-passe), recebe
medições dos agentes nas suas máquinas e envia pedidos para endereços que
configura. Um comprometimento exporia esses segredos e daria a um atacante um
ponto de apoio na rede monitorizada.

## Fronteiras de confiança

| Fronteira | Quem está do outro lado | Confiança |
|---|---|---|
| Navegador ou script para `/api/*` | Administradores, operadores e leitores | Autenticado por um cookie de sessão ou por um token de API; três funções limitam o que cada um pode fazer. |
| Agente para `/api/ingest` | Máquinas onde instalou um agente | Um token por máquina; um token divulgado pode ser revogado isoladamente. |
| Servidor para monitores e serviços | Sistemas de terceiros | Sem confiança: as respostas são analisadas de forma defensiva e os redirecionamentos ficam no mesmo anfitrião. |
| Servidor para canais de notificação | Serviços externos | As credenciais são cifradas em repouso e nunca devolvidas pela API. |
| Servidor para VictoriaMetrics | Um processo filho em loopback, ou uma instância externa | O embutido não tem autenticação e escuta em loopback; uma instância externa é da responsabilidade do operador. |
| Volume `/data` e anfitrião | O operador | Confiável. Um atacante que controle o anfitrião ou o volume está fora do âmbito. |

## Princípios de conceção segura aplicados

- **Privilégio mínimo.** O contentor corre como utilizador sem privilégios, sem
  qualquer capability, com sistema de ficheiros raiz só de leitura e
  `no-new-privileges`. Três funções (leitor, operador, administrador) limitam o
  que um utilizador pode alterar (`crates/server/src/auth/users.rs`; teste
  `operator_role_api.rs`).
- **Seguro por predefinição.** Numa instância nova, só as rotas de estado de
  saúde, configuração inicial, início de sessão e estado de autenticação
  respondem até existir o primeiro administrador (teste
  `api_open_security_api.rs`).
- **Economia de mecanismo.** Um binário, um ficheiro SQLite, um processo filho em
  loopback e uma única porta de entrada para todos os clientes: `/api/*`.
- **Predefinições à prova de falha.** Uma verificação de segurança cujos dados
  faltam é `unknown`, nunca uma aprovação; um monitor inacessível e um erro
  de configuração são estados diferentes.
- **Mediação completa.** Um único middleware de autenticação
  (`crates/server/src/auth/middleware.rs`) protege as rotas da API.
- **Separação de privilégios.** Sessões, tokens de API e tokens de agente são
  credenciais diferentes, com alcances diferentes.
- **Defesa em profundidade.** Cifragem em repouso por cima do controlo de
  acesso, um cabeçalho CSRF por cima das flags do cookie, um limite de
  tentativas por cima do hash das palavras-passe.
- **Sem segredos de ida e volta.** A API devolve `has_secret` e
  `credential_kind`, nunca o segredo; omitir um segredo num `PUT` mantém-no.
- **Apenas criptografia comprovada.** As palavras-passe usam hash Argon2, os
  segredos são cifrados com AES-256-GCM (`crypto.rs`), a autenticação de dois
  fatores usa TOTP com códigos de recuperação de utilização única guardados
  como hash, o OpenID Connect usa state, nonce e PKCE. Nada é feito em casa.

## Contramedidas por classe de fraqueza

| Classe (tema do OWASP Top 10) | Medidas |
|---|---|
| Controlo de acesso quebrado | Um único middleware de autenticação; funções; uma instância nova recusa tudo exceto configuração inicial, início de sessão, estado de saúde e estado de autenticação. |
| Falhas criptográficas | AES-256-GCM em repouso, chave guardada fora da base de dados; hashes Argon2 das palavras-passe; o servidor integrado fala HTTP simples, por isso está documentado um proxy inverso com TLS. |
| Injeção | O SQL passa pelo `sqlx` com parâmetros associados; o proxy do VictoriaMetrics só reencaminha consultas de leitura (`tsdb/mod.rs`, `proxy_read`); os analisadores têm testes unitários. |
| Conceção insegura | O modelo de ameaças acima; os segredos nunca fazem ida e volta; uma instância de finalidade única. |
| Configuração de segurança incorreta | Ficheiros Compose reforçados, VictoriaMetrics só em loopback, notas de reforço para o operador na política de segurança. |
| Componentes vulneráveis | O `cargo deny` verifica licenças, proibições e avisos RustSec; uma análise Trivy semanal das imagens publicadas; Dependabot; GitHub Actions fixadas por commit. |
| Falhas de identificação e autenticação | Limite de tentativas de início de sessão por endereço de cliente e por conta (`auth/rate_limit.rs`), autenticação de dois fatores opcional, OpenID Connect, um administrador pode repor o segundo fator de um utilizador e terminar-lhe as sessões. |
| Integridade do software e dos dados | Os instaladores do agente verificam o SHA-256 do binário descarregado; as imagens de versão são construídas pela CI. |
| Registo e monitorização | Um registo de segurança dos inícios de sessão e das alterações sensíveis (`auth/audit.rs`). |
| Server-side request forgery | As integrações e os canais de notificação só seguem redirecionamentos para o mesmo anfitrião, para que um monitor não possa reencaminhar um pedido e a sua chave de API para outro lado. |
| Cross-site request forgery | Os pedidos que alteram estado e dependem do cookie têm de levar `X-Requested-With: DumbMonit` ou metadados de fetch do mesmo domínio, caso contrário `403`; os pedidos autenticados por token não levam cookie. |

## Verificação

Cada push e cada pull request executa `clippy -D warnings`, a suite de testes
Rust (incluindo testes de API, autenticação e funções em
`crates/server/tests/`), os testes web em `web/tests/`, verificações de
dependências e a análise CodeQL. O OpenSSF Scorecard corre de forma agendada.
As correções de segurança trazem um teste de regressão e são listadas, com o
respetivo identificador CVE ou GHSA, nas notas de versão.

O [argumentário de garantia](assurance-case.md) apresenta o modelo de ameaças
completo, o argumento de conceção segura e as provas por trás de cada
afirmação.

## Limites conhecidos

- O servidor integrado fala HTTP simples; coloque TLS à sua frente.
- As opções dos monitores são guardadas sem cifragem. Ponha os segredos nos
  campos de credenciais.
- O VictoriaMetrics embutido não tem autenticação.
- O DumbMonit está antes da 1.0; as rotas e os campos ainda podem mudar.
