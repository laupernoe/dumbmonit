# GitLab (auto-alojado) {#gitlab-self-managed}

Se uma instância GitLab auto-alojada está a servir, e se as peças que
carregam tudo o resto em silêncio estão a acompanhar: as filas do Sidekiq, os
runners de CI/CD, as migrações de base de dados deixadas por uma atualização
e — no GitLab Enterprise Edition — a licença.

O DumbMonit lê `/api/v4/version` com um **token de acesso pessoal**, a única
chamada que tem de ter êxito: é a prova de que o GitLab responde e de que o
token é válido. Tudo o resto — Sidekiq, runners, migrações, estatísticas da
instância, as definições de registo e de dois fatores, a licença e a
verificação de prontidão em detalhe — é lido de endpoints que o GitLab reserva
às contas de administrador, e é ignorado sem fazer falhar a sonda quando o
token pertence a uma conta comum.

A falha que vale a pena vigiar aqui: **o Sidekiq a ficar para trás**. O GitLab
continua a servir páginas enquanto a fila do seu worker em segundo plano
cresce, e nada na página o diz até uma notificação, um webhook ou uma
atualização de pipeline de CI chegar atrasada minutos ou horas.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_gitlab_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `revision` |
| `readiness_check` | 1 quando um componente de prontidão (base de dados, cache, filas, estado partilhado, Gitaly) não está `ok`; ausente quando o GitLab não o expõe ao DumbMonit (veja abaixo) | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | tarefas em espera e a sua idade, por fila do Sidekiq | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | runners de CI/CD conhecidos da instância | |
| `runner_offline` | valor 1 por runner offline (no máximo 32) | `runner` |
| `migrations_pending` | migrações de base de dados por aplicar | |
| `migration_pending` | valor 1 por migração pendente (no máximo 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | contagens de toda a instância | |
| `signup_enabled` | 1 enquanto o registo público está aberto | |
| `two_factor_required` | 1 enquanto todos os utilizadores têm de configurar a autenticação de dois fatores | |
| `license_info` | valor 1 (apenas Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | se a licença expirou, e o tempo que resta | |
| `pipelines_failed_recent`, `pipelines_pending` | pipelines falhados na janela de retrospetiva, e pipelines pendentes ou em execução agora — apenas para os projetos listados na opção «Projetos vigiados para a saúde do CI» | `project` |
| `scrape_errors` | chamadas só de administrador que falharam na última sonda (0 com um token comum, cujas chamadas nunca são tentadas) | |

As [regras incorporadas](../alerting/rules.md#self-hosted-forges) que se
aplicam: GitLab runners all offline, GitLab Sidekiq backlog high, GitLab
migrations pending, GitLab license expiring, mais Monitor unreachable. Uma
pontuação de segurança verifica as definições de dois fatores e de registo, e as
migrações pendentes: veja
[Atualizações de fornecedores e outras verificações](../using/security-score.md).

## O que não monitoriza {#what-it-does-not-watch}

A API REST do próprio GitLab não tem nenhum endpoint que diga «está disponível
uma versão mais recente do GitLab»: ao contrário do Nextcloud ou do
Paperless-ngx, não há indicador de atualização autodeclarado para ler, por isso
o DumbMonit não finge ter um. Compare a versão acima com a
[página de lançamentos](https://about.gitlab.com/releases/) do GitLab à mão, ou
vigie o aviso de verificação de atualizações do próprio GitLab na área de
administração.

A saúde dos pipelines de CI só é lida para os projetos que listar: a API do
GitLab não tem um endpoint de toda a instância com «todos os pipelines
falhados», apenas um por projeto.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
chama o GitLab. Diz primeiro o que está errado (um componente de prontidão
falhou, runners todos offline, um grande backlog do Sidekiq), depois a versão,
depois as contagens da instância e as definições lidas da API de administração.

## Criar um token de acesso pessoal numa conta de administrador {#create-a-personal-access-token-on-an-administrator-account}

1. Crie um token de acesso pessoal numa conta de administrador: avatar → Edit profile → Access tokens → Add new token. Dê-lhe o nome seguinte, defina uma data de expiração que lhe seja cómodo renovar, e marque apenas o âmbito read_api: lê toda a API e não escreve nada.

    ```
    dumbmonit
    ```

2. As filas do Sidekiq, os runners de CI, as migrações pendentes, as estatísticas da instância, as definições de registo e de dois fatores e a licença são lidos cada um de um endpoint só para administradores: o GitLab não tem delegação mais fina para eles. Um token de uma conta comum continua a funcionar; só a versão é então lida, e o resto é ignorado sem fazer falhar a sonda.

3. No DumbMonit, introduza o endereço com que abre o GitLab, por exemplo «gitlab.example.com» (HTTPS na 443) ou «https://git.example.com» atrás de um proxy inverso, e cole o token.

4. Para vigiar também os pipelines de CI, liste os IDs numéricos dos projetos (mostrados na página de visão geral de cada projeto, sob o seu nome) na opção «Projetos vigiados para a saúde do CI», separados por vírgulas.

!!! warning
    Um token de acesso pessoal numa conta de administrador pode ler tudo o que a conta pode ler, mesmo com o âmbito read_api: o GitLab não tem um papel de administrador só de leitura. Dedique-lhe uma conta de bot em vez de reutilizar a sua, e revogue o token se alguma vez vazar.

### Verificação de prontidão {#readiness-check}

O GitLab responde a `/-/readiness` com 404 a qualquer endereço de cliente que
não esteja na sua lista de permissões de monitorização (por predefinição só
127.0.0.1; atrás do Docker ou de um proxy inverso o DumbMonit chega de outro
endereço). O DumbMonit trata esse 404 como «não exposto»: o painel de
prontidão fica vazio e não é contado em `scrape_errors`. Para o ativar,
adicione o endereço do DumbMonit em `gitlab.rb` e execute
`gitlab-ctl reconfigure`:

```
gitlab_rails['monitoring_whitelist'] = ['127.0.0.0/8', '::1/128', '192.168.1.10/32']
```

O endpoint da licença responde igualmente 404 na Community Edition, o que
também não é um erro.

Verificar o token à mão, a partir do anfitrião do DumbMonit:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de acesso pessoal | Enviado no cabeçalho `PRIVATE-TOKEN`. |

Endereço: um nome de anfitrião (`gitlab.example.com`) ou `host:port`. Opções:
protocolo (HTTPS por predefinição), porta (443), verificação do certificado,
tempo limite do pedido (15 s), os projetos vigiados para a saúde do CI
(nenhum por predefinição) e a janela de retrospetiva dos pipelines falhados
(24 h).

Validado contra o GitLab Enterprise Edition 17.5 (documentação oficial da API).
