# Apache httpd

O servidor web: workers ocupados e ociosos, o placar de workers por estado, e
taxas de requisição/bytes quando ExtendedStatus está ativo.

O DumbMonit lê o `mod_status` embutido do Apache, consultado com `?auto` para
que ele responda em linhas simples `Chave: Valor` em vez da página HTML
pensada para um navegador.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_apache_`. As métricas `_total` são
contadores: gráficos e regras as transformam em taxas. Tudo abaixo de
`workers_busy`/`workers_idle` e `scoreboard` precisa de `ExtendedStatus On`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `workers_busy`, `workers_idle` | processos ou threads de worker, por estado | |
| `scoreboard` | slots de worker por estado do placar | `state` |
| `requests_total` | requisições servidas desde que o Apache iniciou | |
| `sent_bytes_total` | bytes enviados desde que o Apache iniciou | |
| `uptime_seconds` | tempo desde que o Apache iniciou | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | médias próprias do Apache em execução | |
| `cpu_load_ratio` | valor `CPULoad` próprio do Apache | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | contagens de conexões assíncronas (MPMs event e worker) | |
| `version_info` | presença; o cabeçalho de resposta `Server` | `version` |

`scoreboard` sempre soma o número configurado de slots de worker: os estados
são waiting, open_slot, starting, reading, sending, keepalive, dns_lookup,
closing, logging, finishing e idle_cleanup.

## A página do dispositivo

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Apache. Ele mostra workers ocupados e ociosos, a divisão do
placar, e, com ExtendedStatus, as taxas de requisição e bytes nas últimas 24
horas.

## Ativar o mod_status do Apache

1. Certifique-se de que o mod_status está carregado (ele vem com o Apache e
   geralmente é ativado por padrão).

    ```
    a2enmod status
    ```

2. Adicione um local de status, restrito ao host DumbMonit; ajuste o
   endereço conforme necessário. No Debian e Ubuntu isso vai em um arquivo
   próprio, em outros lugares na configuração principal.

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Ative os contadores de requisição e bytes (opcional, mas vale a pena): sem
   isso, só workers ocupados/ociosos e o placar são reportados.

    ```
    ExtendedStatus On
    ```

4. Recarregue o Apache e verifique que ele responde antes de configurar o
   DumbMonit.

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. No DumbMonit, informe o host do Apache, por exemplo "apache.lan", e o
   caminho se não for "/server-status" (o DumbMonit adiciona "?auto" por
   conta própria).

!!! warning
    Sem ExtendedStatus, requisições e bytes não são reportados de forma
    alguma, não como zero: o gráfico fica vazio em vez de mentir reto. O
    Apache não publica sua própria versão no mod_status; o DumbMonit a lê do
    cabeçalho de resposta Server em vez disso, que algumas distribuições
    reduzem de propósito.

No Debian e Ubuntu, o local de status geralmente já existe em
`/etc/apache2/mods-available/status.conf`; editar a linha `Require` ali é o
suficiente, nenhum arquivo novo é necessário. Onde a configuração é um único
arquivo (a maioria das outras distribuições), adicione o bloco ao virtual
host principal em vez disso.

## Credenciais

| Credencial | Campos |
|---|---|
| Sem senha | Nada: a página de status é filtrada para que só o DumbMonit a alcance. |
| Autenticação básica | O usuário e senha do bloco que protege a página de status, se houver. |

Endereço: um nome de host ou IP, com a porta se não for 80. Opções:
protocolo (HTTP por padrão), porta, caminho de status (`/server-status` por
padrão, `?auto` adicionado automaticamente), verificação de certificado e
tempo limite da requisição (10 s).
