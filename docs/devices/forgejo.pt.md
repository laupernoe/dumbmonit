# Forgejo / Gitea {#forgejo-gitea}

O Forgejo e o Gitea partilham a mesma API REST, por isso o DumbMonit lê ambos
com um único coletor: se a instância responde, a sua verificação de saúde,
quantos repositórios, contas e organizações possui, se as suas tarefas
agendadas (verificações de repositório, limpezas, o verificador de
atualizações) ainda correm no seu próprio intervalo, e se os seus executores
(runners) de Actions estão online.

Duas chamadas têm de ter sucesso: `/api/v1/version`, pública, e
`/api/v1/user`, a prova de que o **token de acesso** é válido. Tudo o resto —
a verificação de saúde, as contagens de repositórios/contas/organizações, as
tarefas agendadas e os executores de Actions — é lido a partir de endpoints
que o Forgejo e o Gitea reservam a contas de administrador do site, e é
ignorado sem fazer a verificação falhar quando o token pertence a uma conta
comum.

A falha a vigiar aqui: **uma tarefa agendada que deixou de correr**. O
Forgejo e o Gitea continuam a servir páginas e operações git enquanto o seu
agendador interno está bloqueado; ninguém repara até o verificador de
atualizações ou uma verificação de consistência de repositório não correr há
semanas.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_forgejo_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `token_is_admin` | 1 quando a conta do token é administradora do site | |
| `healthz_ok` | 1 a menos que a verificação de saúde reporte `fail` | |
| `healthz_check` | 1 por componente em falha (base de dados, cache) | `check` |
| `repos` | repositórios visíveis para a conta do token | |
| `users`, `orgs` | contas e organizações na instância | |
| `cron_tasks` | tarefas agendadas definidas na instância | |
| `cron_task_overdue` | 1 por tarefa cuja próxima execução já está vencida, 0 caso contrário | `task` |
| `runners_total`, `runners_online`, `runners_offline` | executores de Actions conhecidos pela instância | |
| `runner_offline` | valor 1 por executor offline (no máximo 32) | `runner` |
| `scrape_errors` | chamadas só de administrador que falharam na última verificação (0 com as de um token comum, nunca tentadas) | |

As [regras incorporadas](../alerting/rules.md#self-hosted-forges) que se
aplicam: Forgejo/Gitea runners all offline, Forgejo/Gitea scheduled task
overdue, mais Device unreachable. Uma pontuação de segurança verifica as
tarefas agendadas: ver
[Atualizações do fornecedor e outras verificações](../using/security-score.md).

## O que não monitoriza {#what-it-does-not-watch}

O Forgejo e o Gitea não têm função de administrador só de leitura: toda
chamada reservada a «admin» precisa de uma conta com direitos totais, não de
um delegado limitado por âmbito. A sua API também não tem equivalente às
definições de registo ou de dois fatores do GitLab, nem uma marca de
«atualização disponível» autorreportada — compare a versão acima com as
[notas de versão](https://forgejo.org/releases/) do projeto manualmente.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca chama o Forgejo nem o Gitea. Diz primeiro o que está mal (a verificação
de saúde a falhar, executores todos offline, uma tarefa agendada vencida),
depois a versão, depois as contagens de repositórios, contas e organizações.

## Criar um token de acesso numa conta de administrador do site {#create-an-access-token-on-a-site-administrator-account}

1. Crie um token de acesso numa conta de administrador do site: avatar → Settings → Applications → Generate new token. Dê-lhe o seguinte nome e marque os âmbitos read:admin, read:repository e read:user.

    ```
    dumbmonit
    ```

2. read:admin abre as contagens de administração, as tarefas agendadas e os executores de Actions; sem ele, só a versão e a própria conta do token são lidas, e o resto é ignorado sem fazer a verificação falhar. O Forgejo e o Gitea também não têm função de administrador só de leitura.

3. No DumbMonit, introduza o endereço, por exemplo «forgejo.lan» (porta 3000) ou «https://git.example.com» atrás de um proxy inverso, e cole o token.

!!! warning
    Um token de acesso numa conta de administrador do site pode ler e alterar tudo o que a conta pode, quaisquer que sejam os âmbitos marcados: o Forgejo e o Gitea só controlam quais rotas da API um âmbito abre, não quanto a própria conta de administrador pode fazer. Dedique-lhe uma conta de bot em vez de reutilizar a sua própria.

Verificar o token manualmente, a partir do anfitrião do DumbMonit:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de acesso | Enviado como `Authorization: token …`. |

Endereço: um nome de anfitrião (`forgejo.lan`) ou `anfitrião:porta`. Opções:
protocolo (HTTP por predefinição — coloque um proxy inverso com TLS em frente
para qualquer coisa além de uma LAN), porta (3000), verificação de
certificado e tempo limite do pedido (15 s).

Validado com o Forgejo 9.0 (fonte oficial da API e definições Swagger,
partilhadas com o Gitea).
