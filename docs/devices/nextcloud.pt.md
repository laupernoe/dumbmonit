# Nextcloud {#nextcloud}

Se o Nextcloud está a servir ou bloqueado: modo de manutenção deixado
ativo, uma atualização de base de dados em espera depois de uma
atualização, atualizações do Nextcloud e de apps, utilizadores ativos,
espaço livre para o diretório de dados, o tamanho da base de dados, e o
OPcache do PHP.

O DumbMonit lê o `status.php` público e as estatísticas da app
**serverinfo**, que vem com o Nextcloud, com um **token de monitorização**
(`NC-Token`). O token abre essas estatísticas e nada mais: não pode
iniciar sessão, listar utilizadores, abrir um ficheiro nem alterar uma
definição.

A falha a vigiar: **uma atualização que não terminou**. Depois de uma
atualização, o Nextcloud pode ficar em modo de manutenção, ou esperar por
`occ upgrade`; a página web mostra então um aviso que ninguém vê até
alguém tentar sincronizar. Ambos são lidos de `status.php`, que responde
mesmo nesse caso.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nextcloud_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `maintenance` | 1 enquanto o modo de manutenção está ativo | |
| `needs_db_upgrade` | 1 enquanto o Nextcloud espera por `occ upgrade` | |
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de atualização do Nextcloud encontrou uma versão mais recente | `available_version` |
| `apps_installed`, `app_updates_available` | apps instaladas, e quantas têm uma versão mais recente na loja de apps | |
| `app_update_available` | valor 1 por app com uma atualização (no máximo 32) | `app`, `available_version` |
| `active_users` | utilizadores vistos nos últimos 5 minutos, hora e dia | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | contas, ficheiros e partilhas na instância | |
| `free_space_bytes` | espaço livre onde o Nextcloud guarda os seus dados | |
| `database_size_bytes` | tamanho da base de dados | |
| `platform_info` | valor 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | `memory_limit` do PHP | |
| `opcache_enabled`, `opcache_full` | OPcache ativo; OPcache sem espaço | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | quão cheio o OPcache está e quão bem serve | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | pool do PHP-FPM, quando o Nextcloud corre atrás do PHP-FPM (as imagens `-fpm`, AIO) | |

Enquanto o Nextcloud está em manutenção ou espera por uma atualização, a
sua API responde 503: só `maintenance`, `needs_db_upgrade` e
`version_info` são escritas então, e o dispositivo não é reportado como
inacessível.

As [regras incorporadas](../alerting/rules.md#self-hosted-applications) que
se aplicam: Nextcloud stuck in maintenance, Nextcloud database upgrade
pending, Nextcloud update available, Nextcloud OPcache full, Nextcloud
free space low, mais Device unreachable.

## Tarefas de fundo (cron) {#background-jobs-cron}

O Nextcloud não diz ao token de monitorização quando as tarefas de fundo
correram por último: só uma conta com todos os direitos na instância pode
ler isso, e o DumbMonit não pede uma. Monitorize o cron com um
[Heartbeat](push.md) em vez disso: crie um no DumbMonit, depois chame o
seu URL no fim da linha cron que executa as tarefas de fundo do
Nextcloud, para que só chame quando `cron.php` tiver sucesso:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Com um intervalo esperado de 5 minutos, o heartbeat alerta assim que o
cron para, seja qual for o motivo (um contentor reiniciado sem o seu
cron, um erro PHP, tarefas de fundo deixadas em AJAX).

## Dispositivos cliente {#client-devices}

O Nextcloud não expõe uma lista a nível de administrador de clientes de
sincronização ou dispositivos móveis através de nenhuma API. A página
pessoal «Devices & sessions» (Settings → Security) só lista a conta
atualmente com sessão iniciada, suportada por uma sessão de navegador em
vez de um token de API, e o único equivalente a nível de administrador é a
linha de comandos (`occ user:auth-tokens:list <user>`, uma conta de cada
vez) — não algo que o token de monitorização possa alcançar, e não algo
que o DumbMonit peça a um administrador para incluir num script de
verificação. A API `serverinfo` que o DumbMonit lê também não tem nenhuma
lista de dispositivos: dá contagens de utilizadores ativos
(`active_users`, acima), nunca quem ou o que são.

Este é um limite deliberado, não uma funcionalidade em falta: dar ao
DumbMonit uma lista de dispositivos significaria alargar o token de
monitorização para uma conta de administração completa, o que o resto
desta página evita a todo custo. Se o Nextcloud algum dia expuser isto via
OCS, o DumbMonit seguirá o mesmo padrão de
[dispositivos cliente](../alerting/rules.md) já usado para o Immich,
Proxmox Backup Server e Veeam.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca chama o Nextcloud. Diz primeiro o que está mal (manutenção,
atualização em espera, OPcache cheio ou desligado), depois atualizações,
depois utilizadores ativos, espaço livre, tamanho da base de dados,
utilizadores, ficheiros e valores do OPcache, depois as apps a atualizar
pelo nome e a plataforma (base de dados, PHP).

## Dar ao Nextcloud um token de monitorização {#give-nextcloud-a-monitoring-token}

1. A app serverinfo do Nextcloud, incluída e ativada com o Nextcloud, responde a um token de monitorização que abre as suas estatísticas e nada mais: nenhum ficheiro, nenhum utilizador, nenhuma partilha. No servidor Nextcloud, defina um token longo e aleatório e imprima-o de volta. Execute o occ como o utilizador do servidor web; no Docker, coloque docker exec -u www-data e o nome do contentor antes.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Se o occ disser que o serverinfo está desativado, ative-o primeiro.

    ```
    php occ app:enable serverinfo
    ```

3. No DumbMonit, introduza o endereço com que abre o Nextcloud, por exemplo «cloud.example.com» (HTTPS na 443) ou «http://10.0.0.5:8080», e cole o token. O anfitrião tem de ser um dos domínios de confiança do Nextcloud, ou o Nextcloud recusa o pedido.

4. O DumbMonit só lê status.php e as estatísticas do serverinfo. Nunca abre um ficheiro, nunca lista utilizadores e nunca altera uma definição, e o token não pode iniciar sessão em lado nenhum.

!!! warning
    O Nextcloud não diz ao token quando as tarefas de fundo (cron) correram por último: só uma conta com todos os direitos na instância pode ler isso. Para ser avisado quando o cron para, adicione um Heartbeat no DumbMonit e chame o seu URL no fim da linha cron, como a documentação mostra.

Com o Nextcloud AIO, execute o occ no contentor `nextcloud-aio-nextcloud`:
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Porque não um
grupo de administração delegada: o Nextcloud só abre a API serverinfo ao
token ou a membros do grupo `admin` completo, e a delegação não altera
isso.

Verificar o token manualmente, a partir do anfitrião do DumbMonit:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de monitorização (NC-Token) | O token do serverinfo. O DumbMonit envia-o no cabeçalho `NC-Token`, com `OCS-APIRequest: true`. |

Endereço: um nome de anfitrião (`cloud.example.com`), `anfitrião:porta`, ou
um URL completo com um prefixo de caminho (`https://example.com/nextcloud`);
um `/index.php` ou `/login` final é aceite. Opções: protocolo (HTTPS por
predefinição), porta (443), verificação de certificado e tempo limite do
pedido (15 s).

A contagem de atualizações de apps consulta a cache da loja de apps do
Nextcloud; numa instância que não consegue alcançar a loja de apps, essa
contagem fica em 0.

Validado com o Nextcloud 35.0.1 (imagem oficial).
