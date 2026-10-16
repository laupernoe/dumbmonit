# Forgejo / Gitea {#forgejo--gitea}

O Forgejo e o Gitea compartilham a mesma API REST, então o DumbMonit lê ambos
com um único coletor: se a instância responde, sua verificação de saúde,
quantos repositórios, contas e organizações ela possui, se suas tarefas
agendadas (verificações de repositório, limpezas, o verificador de
atualizações) ainda rodam em seu próprio intervalo, e se seus executores
(runners) de Actions estão online.

Duas chamadas precisam ter sucesso: `/api/v1/version`, pública, e
`/api/v1/user`, a prova de que o **token de acesso** é válido. Todo o resto —
a verificação de saúde, as contagens de repositórios/contas/organizações, as
tarefas agendadas e os executores de Actions — é lido de endpoints que o
Forgejo e o Gitea reservam a contas de administrador do site, e é ignorado
sem falhar a sondagem quando o token pertence a uma conta comum.

A falha que vale a pena observar aqui: **uma tarefa agendada que parou de
rodar**. O Forgejo e o Gitea continuam servindo páginas e operações git
enquanto seu agendador interno está travado; ninguém percebe até o
verificador de atualizações ou uma verificação de consistência de
repositório não rodar há semanas.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_forgejo_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `token_is_admin` | 1 quando a conta do token é administradora do site | |
| `healthz_ok` | 1 a menos que a verificação de saúde reporte `fail` | |
| `healthz_check` | 1 por componente com falha (banco de dados, cache) | `check` |
| `repos` | repositórios visíveis para a conta do token | |
| `users`, `orgs` | contas e organizações na instância | |
| `cron_tasks` | tarefas agendadas definidas na instância | |
| `cron_task_overdue` | 1 por tarefa cuja próxima execução já está atrasada, 0 caso contrário | `task` |
| `runners_total`, `runners_online`, `runners_offline` | executores de Actions conhecidos pela instância | |
| `runner_offline` | valor 1 por executor offline (no máximo 32) | `runner` |
| `scrape_errors` | chamadas restritas a administrador que falharam na última sondagem (0 com as de um token comum nunca tentadas) | |

As [regras integradas](../alerting/rules.md#self-hosted-forges) que se
aplicam: Forgejo/Gitea runners all offline, Forgejo/Gitea scheduled task
overdue, além de Monitor unreachable. Uma pontuação de segurança verifica as
tarefas agendadas: veja
[Atualizações de fornecedor e outras verificações](../using/security-score.md).

## O que ele não observa {#what-it-does-not-watch}

O Forgejo e o Gitea não têm papel de administrador somente leitura: toda
chamada reservada a "admin" exige uma conta com direitos totais, não um
delegado de escopo limitado. Sua API também não tem equivalente às
configurações de cadastro ou de dois fatores do GitLab, nem um indicador
próprio de "atualização disponível" — compare a versão acima com as
[notas de lançamento](https://forgejo.org/releases/) do projeto manualmente.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca chama o Forgejo ou o Gitea. Ele diz primeiro o que está errado (a
verificação de saúde falhando, executores todos offline, uma tarefa agendada
atrasada), depois a versão, depois as contagens de repositórios, contas e
organizações.

## Criar um token de acesso em uma conta de administrador do site {#create-an-access-token-on-a-site-administrator-account}

1. Crie um token de acesso em uma conta de administrador do site: avatar →
   Settings → Applications → Generate new token. Nomeie-o como abaixo e
   marque os escopos read:admin, read:repository e read:user.

    ```
    dumbmonit
    ```

2. read:admin abre as contagens de administração, as tarefas agendadas e os
   executores de Actions; sem ele, apenas a versão e a própria conta do
   token são lidas, e o resto é ignorado sem falhar a sondagem. O Forgejo e
   o Gitea também não têm papel de administrador somente leitura.

3. No DumbMonit, informe o endereço, por exemplo "forgejo.lan" (porta 3000)
   ou "https://git.example.com" detrás de um proxy reverso, e cole o token.

!!! warning
    Um token de acesso em uma conta de administrador do site pode ler e
    alterar tudo que a conta pode, quaisquer que sejam os escopos marcados:
    o Forgejo e o Gitea só controlam quais rotas de API um escopo abre, não
    o quanto a própria conta de administrador pode fazer. Dedique uma conta
    bot a isso em vez de reutilizar a sua própria.

Verificando o token manualmente, a partir do host DumbMonit:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de acesso | Enviado como `Authorization: token …`. |

Endereço: um nome de host (`forgejo.lan`) ou `host:porta`. Opções: protocolo
(HTTP por padrão — coloque um proxy reverso com TLS na frente para qualquer
coisa além de uma LAN), porta (3000), verificação de certificado e tempo
limite da requisição (15 s).

Validado contra o Forgejo 9.0 (fonte da API oficial e definições Swagger,
compartilhadas com o Gitea).
