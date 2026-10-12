# Nextcloud

Se o Nextcloud está servindo ou travado: modo de manutenção deixado
ativado, uma atualização de banco de dados esperando após uma atualização,
atualizações do Nextcloud e de apps, usuários ativos, espaço livre para o
diretório de dados, o tamanho do banco de dados, e o OPcache do PHP.

O DumbMonit lê o `status.php` público e as estatísticas do app
**serverinfo**, que vem com o Nextcloud, com um **token de monitoramento**
(`NC-Token`). O token abre essas estatísticas e nada mais: ele não pode
fazer login, listar usuários, abrir um arquivo ou alterar uma
configuração.

A falha que vale a pena observar: **uma atualização que não terminou**.
Depois de uma atualização o Nextcloud pode permanecer em modo de
manutenção, ou esperar por `occ upgrade`; a página web então mostra um
aviso que ninguém vê até alguém tentar sincronizar. Ambos são lidos de
`status.php`, que responde mesmo então.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_nextcloud_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `maintenance` | 1 enquanto o modo de manutenção está ativo | |
| `needs_db_upgrade` | 1 enquanto o Nextcloud espera por `occ upgrade` | |
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de atualização do Nextcloud encontrou uma versão mais nova | `available_version` |
| `apps_installed`, `app_updates_available` | apps instalados, e quantos têm uma versão mais nova na loja de apps | |
| `app_update_available` | valor 1 por app com uma atualização (no máximo 32) | `app`, `available_version` |
| `active_users` | usuários vistos nos últimos 5 minutos, hora e dia | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | contas, arquivos e compartilhamentos na instância | |
| `free_space_bytes` | espaço livre onde o Nextcloud mantém seus dados | |
| `database_size_bytes` | tamanho do banco de dados | |
| `platform_info` | valor 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | `memory_limit` do PHP | |
| `opcache_enabled`, `opcache_full` | OPcache ativo; OPcache sem espaço | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | quão cheio o OPcache está e quão bem ele serve | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | pool do PHP-FPM, quando o Nextcloud roda detrás do PHP-FPM (as imagens `-fpm`, AIO) | |

Enquanto o Nextcloud está em manutenção ou espera por uma atualização, sua
API responde 503: apenas `maintenance`, `needs_db_upgrade` e
`version_info` são escritos então, e o dispositivo não é reportado como
inacessível.

As [regras integradas](../alerting/rules.md#self-hosted-applications) que
se aplicam: Nextcloud stuck in maintenance, Nextcloud database upgrade
pending, Nextcloud update available, Nextcloud OPcache full, Nextcloud
free space low, além de Device unreachable.

## Jobs em segundo plano (cron)

O Nextcloud não informa ao token de monitoramento quando jobs em segundo
plano rodaram por último: só uma conta com todos os direitos na instância
pode ler isso, e o DumbMonit não pede uma. Observe o cron com um
[Heartbeat](push.md) em vez disso: crie um no DumbMonit, depois chame sua
URL no final da linha cron que executa os jobs em segundo plano do
Nextcloud, para que só chame quando `cron.php` tiver sucesso:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Com um intervalo esperado de 5 minutos, o heartbeat alerta assim que o
cron para, qualquer que seja o motivo (um contêiner reiniciado sem seu
cron, um erro de PHP, jobs em segundo plano deixados no AJAX).

## Dispositivos clientes

O Nextcloud não expõe uma lista de clientes de sincronização ou
dispositivos móveis em nível de administrador através de nenhuma API. A
página pessoal "Dispositivos e sessões" (Settings → Security) lista apenas
a conta atualmente conectada, baseada em uma sessão de navegador em vez de
um token de API, e o único equivalente em nível de administrador é a linha
de comando (`occ user:auth-tokens:list <user>`, uma conta por vez) — não
algo que o token de monitoramento pode alcançar, e não algo que o
DumbMonit pede a um administrador para colocar em um script de sondagem. A
API `serverinfo` que o DumbMonit lê também não tem lista de dispositivos:
ela dá contagens de usuários ativos (`active_users`, acima), nunca quem ou
o que eles são.

Esta é uma limitação deliberada, não uma funcionalidade faltando: dar ao
DumbMonit uma lista de dispositivos significaria expandir o token de
monitoramento para uma conta de administração completa, o que o resto
desta página evita com cuidado. Se o Nextcloud algum dia expor isso via
OCS, o DumbMonit seguirá o mesmo padrão de
[dispositivos clientes](../alerting/rules.md) já usado para Immich, Proxmox
Backup Server e Veeam.

## A página do dispositivo

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca chama o Nextcloud. Ele diz primeiro o que está errado (manutenção,
atualização esperando, OPcache cheio ou desligado), depois atualizações,
depois usuários ativos, espaço livre, tamanho do banco de dados, usuários,
arquivos e números do OPcache, depois os apps a atualizar pelo nome e a
plataforma (banco de dados, PHP).

## Dar ao Nextcloud um token de monitoramento

1. O app serverinfo do Nextcloud, entregue e ativado com o Nextcloud,
   responde a um token de monitoramento que abre suas estatísticas e nada
   mais: nenhum arquivo, nenhum usuário, nenhum compartilhamento. No
   servidor Nextcloud, defina um token longo e aleatório e o imprima de
   volta. Execute occ como o usuário do servidor web; no Docker, coloque
   docker exec -u www-data e o nome do contêiner antes.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Se occ disser que serverinfo está desativado, ative-o primeiro.

    ```
    php occ app:enable serverinfo
    ```

3. No DumbMonit, informe o endereço com o qual você abre o Nextcloud, por
   exemplo "cloud.example.com" (HTTPS na 443) ou "http://10.0.0.5:8080", e
   cole o token. O host precisa ser um dos domínios confiáveis do
   Nextcloud, ou o Nextcloud recusa a requisição.

4. O DumbMonit lê apenas status.php e as estatísticas do serverinfo. Ele
   nunca abre um arquivo, nunca lista usuários e nunca altera uma
   configuração, e o token não pode fazer login em nenhum lugar.

!!! warning
    O Nextcloud não informa ao token quando jobs em segundo plano (cron)
    rodaram por último: só uma conta com todos os direitos na instância
    pode ler isso. Para ser avisado quando o cron para, adicione um
    Heartbeat no DumbMonit e chame sua URL no final da linha cron, como a
    documentação mostra.

Com o Nextcloud AIO, execute occ no contêiner `nextcloud-aio-nextcloud`:
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Por que não um
grupo de administração delegada: o Nextcloud só abre a API serverinfo para
o token ou para membros do grupo `admin` completo, e a delegação não muda
isso.

Verificando o token manualmente, a partir do host DumbMonit:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Credenciais

| Credencial | Campos |
|---|---|
| Token de monitoramento (NC-Token) | O token serverinfo. O DumbMonit o envia no cabeçalho `NC-Token`, com `OCS-APIRequest: true`. |

Endereço: um nome de host (`cloud.example.com`), `host:porta`, ou uma URL
completa com um prefixo de caminho (`https://example.com/nextcloud`); um
`/index.php` ou `/login` final é aceito. Opções: protocolo (HTTPS por
padrão), porta (443), verificação de certificado e tempo limite da
requisição (15 s).

A contagem de atualizações de app consulta o cache da loja de apps do
Nextcloud; em uma instância que não consegue alcançar a loja de apps, essa
contagem permanece em 0.

Validado contra o Nextcloud 35.0.1 (imagem oficial).
