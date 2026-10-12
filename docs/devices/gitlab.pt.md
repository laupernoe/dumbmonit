# GitLab (autoalojado) {#gitlab-self-managed}

Se uma instância GitLab autoalojada está a servir, e se as partes que
silenciosamente sustentam tudo o resto estão a acompanhar: as filas do
Sidekiq, os executores de CI/CD, migrações de base de dados deixadas de uma
atualização, e — para o GitLab Enterprise Edition — a licença.

O DumbMonit lê `/api/v4/version` com um **token de acesso pessoal**, a única
chamada que tem de ter sucesso: é a prova de que o GitLab responde e de que
o token é válido. Tudo o resto — Sidekiq, executores, migrações,
estatísticas da instância, as definições de registo e de dois fatores, a
licença, e a verificação de preparação em detalhe — é lido a partir de
endpoints que o GitLab reserva a contas de administrador, e é ignorado sem
fazer a verificação falhar quando o token pertence a uma conta comum.

A falha a vigiar aqui: **o Sidekiq a ficar atrasado**. O GitLab continua a
servir páginas enquanto a sua fila de trabalhadores em segundo plano cresce,
e nada na página o diz até uma notificação, um webhook ou uma atualização de
pipeline de CI chegar com minutos ou horas de atraso.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_gitlab_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `revision` |
| `readiness_check` | 1 quando um componente de preparação (base de dados, cache, filas, estado partilhado, Gitaly) não está `ok` | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | tarefas à espera e a sua idade, por fila do Sidekiq | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | executores de CI/CD conhecidos pela instância | |
| `runner_offline` | valor 1 por executor offline (no máximo 32) | `runner` |
| `migrations_pending` | migrações de base de dados ainda por aplicar | |
| `migration_pending` | valor 1 por migração pendente (no máximo 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | contagens de toda a instância | |
| `signup_enabled` | 1 enquanto o registo público está aberto | |
| `two_factor_required` | 1 enquanto todos os utilizadores têm de configurar a autenticação de dois fatores | |
| `license_info` | valor 1 (só Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | se a licença expirou, e o tempo restante | |
| `pipelines_failed_recent`, `pipelines_pending` | pipelines falhadas na janela de observação, e pipelines pendentes ou a correr agora — só para os projetos listados na opção «Projects watched for CI health» | `project` |
| `scrape_errors` | chamadas só de administrador que falharam na última verificação (0 com as de um token comum, nunca tentadas) | |

As [regras incorporadas](../alerting/rules.md#self-hosted-forges) que se
aplicam: GitLab runners all offline, GitLab Sidekiq backlog high, GitLab
migrations pending, GitLab license expiring, mais Device unreachable. Uma
pontuação de segurança verifica as definições de dois fatores e de registo,
e as migrações pendentes: ver
[Atualizações do fornecedor e outras verificações](../using/security-score.md).

## O que não monitoriza {#what-it-does-not-watch}

A própria API REST do GitLab não tem nenhum endpoint que diga «há uma versão
mais recente do GitLab disponível»: ao contrário do Nextcloud ou do
Paperless-ngx, não há nenhuma marca de atualização autorreportada para ler,
por isso o DumbMonit não a afirma. Compare a versão acima com a
[página de versões](https://about.gitlab.com/releases/) do GitLab
manualmente, ou acompanhe o próprio aviso de verificação de atualização do
GitLab na área de administração.

A saúde dos pipelines de CI só é lida para os projetos que listar: a API do
GitLab não tem nenhum endpoint de «todas as pipelines falhadas» a nível de
instância, apenas um por projeto.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca chama o GitLab. Diz primeiro o que está mal (um componente de
preparação falhado, executores todos offline, um grande atraso no Sidekiq),
depois a versão, depois as contagens da instância e as definições lidas da
API de administração.

## Criar um token de acesso pessoal numa conta de administrador {#create-a-personal-access-token-on-an-administrator-account}

1. Crie um token de acesso pessoal numa conta de administrador: avatar → Edit profile → Access tokens → Add new token. Dê-lhe o seguinte nome, defina uma data de expiração com que se sinta confortável a renovar, e marque apenas o âmbito read_api: lê toda a API e não escreve nada.

    ```
    dumbmonit
    ```

2. As filas do Sidekiq, executores de CI, migrações pendentes, estatísticas da instância, as definições de registo e de dois fatores e a licença são cada uma lida de um endpoint só de administrador: o GitLab não tem uma delegação mais fina para elas. Um token de uma conta comum ainda funciona; só a versão é então lida, e o resto é ignorado sem fazer a verificação falhar.

3. No DumbMonit, introduza o endereço com que abre o GitLab, por exemplo «gitlab.example.com» (HTTPS na 443) ou «https://git.example.com» atrás de um proxy inverso, e cole o token.

4. Para também monitorizar pipelines de CI, liste os IDs numéricos dos projetos (mostrados na página de resumo de cada projeto, sob o seu nome) na opção «Projects watched for CI health», separados por vírgulas.

!!! warning
    Um token de acesso pessoal numa conta de administrador pode ler tudo o que a conta pode ler, mesmo com o âmbito read_api: o GitLab não tem função de administrador só de leitura. Dedique-lhe uma conta de bot em vez de reutilizar a sua própria, e revogue o token se alguma vez for exposto.

Verificar o token manualmente, a partir do anfitrião do DumbMonit:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de acesso pessoal | Enviado como cabeçalho `PRIVATE-TOKEN`. |

Endereço: um nome de anfitrião (`gitlab.example.com`) ou `anfitrião:porta`.
Opções: protocolo (HTTPS por predefinição), porta (443), verificação de
certificado, tempo limite do pedido (15 s), os projetos monitorizados para a
saúde de CI (nenhum por predefinição) e a janela de observação de pipelines
falhadas (24 h).

Validado com o GitLab Enterprise Edition 17.5 (documentação oficial da API).
