# Forgejo / Gitea {#forgejo-gitea}

O Forgejo e o Gitea partilham a mesma API REST, pelo que o DumbMonit lê ambos
com um só coletor: se a instância responde, a sua verificação de saúde,
quantos repositórios, contas e organizações contém, se as suas tarefas
agendadas (verificações de repositórios, limpezas, verificador de
atualizações) continuam a correr no seu próprio intervalo, e se os seus
runners de Actions estão online.

Duas chamadas têm de ter êxito: `/api/v1/version`, pública, e `/api/v1/user`,
a prova de que o **token de acesso** é válido. Tudo o resto — a verificação de
saúde, as contagens de repositórios/contas/organizações, as tarefas agendadas
e os runners de Actions — é lido de endpoints que o Forgejo e o Gitea reservam
às contas de administrador do site, e é ignorado sem fazer falhar a sonda
quando o token pertence a uma conta comum.

A falha que vale a pena vigiar aqui: **uma tarefa agendada que deixou de
correr**. O Forgejo e o Gitea continuam a servir páginas e operações git
enquanto o seu agendador interno está bloqueado; ninguém repara até o
verificador de atualizações ou uma verificação de consistência de repositório
não correr há semanas.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_forgejo_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `token_is_admin` | 1 quando a conta do token é administradora do site | |
| `healthz_ok` | 1 a menos que a verificação de saúde reporte `fail` | |
| `healthz_check` | 1 por componente com falha (base de dados, cache) | `check` |
| `repos` | repositórios visíveis para a conta do token | |
| `users`, `orgs` | contas e organizações na instância | |
| `cron_tasks` | tarefas agendadas definidas na instância | |
| `cron_task_overdue` | 1 por tarefa cuja próxima execução já está vencida, 0 caso contrário; as tarefas desativadas (agenda `-`) são ignoradas | `task` |
| `runners_total`, `runners_online`, `runners_offline` | runners de Actions conhecidos da instância; `online`, `idle` e `active` contam como online | |
| `runner_offline` | valor 1 por runner offline (no máximo 32) | `runner` |
| `scrape_errors` | chamadas só de administrador que falharam na última sonda (0 com um token comum, cujas chamadas nunca são tentadas) | |

As [regras incorporadas](../alerting/rules.md#self-hosted-forges) que se
aplicam: Forgejo/Gitea runners all offline, Forgejo/Gitea scheduled task
overdue, mais Device unreachable. Uma pontuação de segurança verifica as
tarefas agendadas: veja
[Atualizações de fornecedores e outras verificações](../using/security-score.md).

## O que não monitoriza {#what-it-does-not-watch}

O Forgejo e o Gitea não têm um papel de administrador só de leitura: cada
chamada reservada a «admin» precisa de uma conta com direitos totais, não de
um delegado com âmbito limitado. A sua API também não tem equivalente às
definições de registo ou de dois fatores do GitLab, nem um indicador próprio de
«atualização disponível» — compare a versão acima com as
[notas de lançamento](https://forgejo.org/releases/) do projeto à mão.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
chama o Forgejo nem o Gitea. Diz primeiro o que está errado (a verificação de
saúde a falhar, runners todos offline, uma tarefa agendada em atraso), depois a
versão, depois as contagens de repositórios, contas e organizações.

## Criar um token de acesso numa conta de administrador do site {#create-an-access-token-on-a-site-administrator-account}

1. Crie um token de acesso numa conta de administrador do site: avatar → Settings → Applications → Generate new token. Dê-lhe o nome seguinte e marque os âmbitos read:admin, read:repository e read:user.

    ```
    dumbmonit
    ```

2. O read:admin abre as contagens de administração, as tarefas agendadas e os runners de Actions; sem ele, só são lidos a versão e a própria conta do token, e o resto é ignorado sem fazer falhar a sonda. O Forgejo e o Gitea também não têm um papel de administrador só de leitura.

3. No DumbMonit, introduza o endereço, por exemplo «forgejo.lan» (porta 3000) ou «https://git.example.com» atrás de um proxy inverso, e cole o token.

!!! warning
    Um token de acesso numa conta de administrador do site pode ler e alterar tudo o que a conta pode, sejam quais forem os âmbitos marcados: o Forgejo e o Gitea só controlam que rotas da API um âmbito abre, não quanto a própria conta de administrador pode fazer. Dedique-lhe uma conta de bot em vez de reutilizar a sua.

Verificar o token à mão, a partir do anfitrião do DumbMonit:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de acesso | Enviado como `Authorization: token …`. |

Endereço: um nome de anfitrião (`forgejo.lan`) ou `host:port`. Opções:
protocolo (HTTP por predefinição — ponha um proxy inverso com TLS à frente para
tudo o que não seja uma LAN), porta (3000), verificação do certificado e tempo
limite do pedido (15 s).

Validado contra o Forgejo 9.0 (código-fonte oficial da API e definições
Swagger, partilhados com o Gitea).
