# GitLab (self-managed) {#gitlab-self-managed}

Se uma instância GitLab self-managed está servindo, e se as partes que
sustentam silenciosamente todo o resto estão se mantendo em dia: as filas do
Sidekiq, os executores de CI/CD, migrações de banco de dados deixadas de uma
atualização, e — para GitLab Enterprise Edition — a licença.

O DumbMonit lê `/api/v4/version` com um **token de acesso pessoal**, a única
chamada que precisa ter sucesso: é a prova de que o GitLab responde e que o
token é válido. Todo o resto — Sidekiq, executores, migrações, estatísticas
da instância, as configurações de cadastro e dois fatores, a licença, e a
verificação de prontidão em detalhe — é lido de endpoints que o GitLab
reserva a contas de administrador, e é ignorado sem falhar a sondagem quando
o token pertence a uma conta comum.

A falha que vale a pena observar aqui: **o Sidekiq ficando atrasado**. O
GitLab continua servindo páginas enquanto sua fila de workers em segundo
plano cresce, e nada na página avisa isso até que uma notificação, um
webhook ou uma atualização de pipeline de CI atrase minutos ou horas.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_gitlab_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version`, `revision` |
| `readiness_check` | 1 quando um componente de prontidão (banco de dados, cache, filas, estado compartilhado, Gitaly) não está `ok` | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | jobs esperando e sua idade, por fila do Sidekiq | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | executores de CI/CD conhecidos pela instância | |
| `runner_offline` | valor 1 por executor offline (no máximo 32) | `runner` |
| `migrations_pending` | migrações de banco de dados restantes para aplicar | |
| `migration_pending` | valor 1 por migração pendente (no máximo 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | contagens de toda a instância | |
| `signup_enabled` | 1 enquanto o cadastro público está aberto | |
| `two_factor_required` | 1 enquanto todo usuário precisa configurar autenticação de dois fatores | |
| `license_info` | valor 1 (apenas Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | se a licença expirou, e o tempo restante | |
| `pipelines_failed_recent`, `pipelines_pending` | pipelines com falha na janela de retrospecto, e pipelines pendentes ou em execução agora — apenas para os projetos listados na opção "Projetos observados para saúde de CI" | `project` |
| `scrape_errors` | chamadas restritas a administrador que falharam na última sondagem (0 com as de um token comum nunca tentadas) | |

As [regras integradas](../alerting/rules.md#self-hosted-forges) que se
aplicam: GitLab runners all offline, GitLab Sidekiq backlog high, GitLab
migrations pending, GitLab license expiring, além de Device unreachable. Uma
pontuação de segurança verifica as configurações de dois fatores e cadastro,
e migrações pendentes: veja
[Atualizações de fornecedor e outras verificações](../using/security-score.md).

## O que ele não observa {#what-it-does-not-watch}

A própria API REST do GitLab não tem um endpoint que diga "uma versão mais
nova do GitLab está disponível": diferente do Nextcloud ou Paperless-ngx, não
há um indicador de atualização autorreportado para ler, então o DumbMonit não
reivindica um. Compare a versão acima com a
[página de lançamentos](https://about.gitlab.com/releases/) do GitLab
manualmente, ou observe o próprio aviso de verificação de atualização do
GitLab na área de administração.

A saúde dos pipelines de CI é lida apenas para os projetos que você lista: a
API do GitLab não tem um endpoint "todos os pipelines com falha" para toda a
instância, apenas um por projeto.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca chama o GitLab. Ele diz primeiro o que está errado (um componente de
prontidão falhou, executores todos offline, um grande atraso no Sidekiq),
depois a versão, depois as contagens da instância e as configurações lidas da
API de administração.

## Criar um token de acesso pessoal em uma conta de administrador {#create-a-personal-access-token-on-an-administrator-account}

1. Crie um token de acesso pessoal em uma conta de administrador: avatar →
   Edit profile → Access tokens → Add new token. Nomeie-o como abaixo, defina
   uma data de expiração que você se sinta confortável em renovar, e marque
   apenas o escopo read_api: ele lê toda a API e não escreve nada.

    ```
    dumbmonit
    ```

2. Filas do Sidekiq, executores de CI, migrações pendentes, estatísticas da
   instância, as configurações de cadastro e dois fatores e a licença são
   cada uma lida de um endpoint restrito a administrador: o GitLab não tem
   delegação mais fina para elas. Um token de uma conta comum ainda funciona;
   apenas a versão é então lida, e o resto é ignorado sem falhar a sondagem.

3. No DumbMonit, informe o endereço com o qual você abre o GitLab, por
   exemplo "gitlab.example.com" (HTTPS na 443) ou "https://git.example.com"
   detrás de um proxy reverso, e cole o token.

4. Para também observar pipelines de CI, liste os IDs numéricos dos projetos
   (mostrados na página de visão geral de cada projeto, sob seu nome) na
   opção "Projetos observados para saúde de CI", separados por vírgulas.

!!! warning
    Um token de acesso pessoal em uma conta de administrador pode ler tudo
    que a conta pode ler, mesmo com o escopo read_api: o GitLab não tem
    papel de administrador somente leitura. Dedique uma conta bot a isso em
    vez de reutilizar a sua própria, e revogue o token se ele algum dia for
    exposto.

Verificando o token manualmente, a partir do host DumbMonit:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credenciais {#readiness-check}

| Credencial | Campos |
|---|---|
| Token de acesso pessoal | Enviado como o cabeçalho `PRIVATE-TOKEN`. |

Endereço: um nome de host (`gitlab.example.com`) ou `host:porta`. Opções:
protocolo (HTTPS por padrão), porta (443), verificação de certificado, tempo
limite da requisição (15 s), os projetos observados para saúde de CI (nenhum
por padrão) e a janela de retrospecto de pipeline com falha (24 h).

Validado contra o GitLab Enterprise Edition 17.5 (documentação oficial da
API).
