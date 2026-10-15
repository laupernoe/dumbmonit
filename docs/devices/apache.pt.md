# Apache httpd {#apache-httpd}

O servidor web: trabalhadores ocupados e inativos, o quadro de pontuação dos
trabalhadores por estado, e taxas de pedidos/bytes quando o ExtendedStatus
está ativado.

O DumbMonit lê o `mod_status` incorporado do Apache, consultado com `?auto`
para que responda em linhas `Key: Value` simples em vez da página HTML
destinada a um navegador.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_apache_`. As métricas `_total` são
contadores: os gráficos e as regras convertem-nas em taxas. Tudo abaixo de
`workers_busy`/`workers_idle` e `scoreboard` precisa de `ExtendedStatus On`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `workers_busy`, `workers_idle` | processos ou threads de trabalho, por estado | |
| `scoreboard` | lugares de trabalho por estado do scoreboard | `state` |
| `requests_total` | pedidos servidos desde que o Apache arrancou | |
| `sent_bytes_total` | bytes enviados desde que o Apache arrancou | |
| `uptime_seconds` | tempo desde que o Apache arrancou | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | próprias médias correntes do Apache | |
| `cpu_load_ratio` | próprio valor `CPULoad` do Apache | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | contagens de ligações assíncronas (MPMs event e worker) | |
| `version_info` | presença; o cabeçalho de resposta `Server` | `version` |

`scoreboard` soma sempre o número configurado de lugares de trabalho: os
estados são waiting, open_slot, starting, reading, sending, keepalive,
dns_lookup, closing, logging, finishing e idle_cleanup.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o Apache. Mostra trabalhadores ocupados e inativos, a
discriminação do scoreboard, e, com ExtendedStatus, as taxas de pedidos e
bytes ao longo de 24 horas.

## Ativar o mod_status do Apache {#enable-apaches-mod_status}

1. Certifique-se de que o mod_status está carregado (vem com o Apache e normalmente está ativado por predefinição).

    ```
    a2enmod status
    ```

2. Adicione uma localização de estado, restrita ao anfitrião do DumbMonit; ajuste o endereço em conformidade. No Debian e Ubuntu isto vai num ficheiro próprio, noutros na configuração principal.

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Ative os contadores de pedidos e bytes (opcional, mas vale a pena): sem isto, só os trabalhadores ocupados/inativos e o scoreboard são reportados.

    ```
    ExtendedStatus On
    ```

4. Recarregue o Apache e verifique que responde antes de configurar o DumbMonit.

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. No DumbMonit, introduza o anfitrião do Apache, por exemplo «apache.lan», e o caminho se não for «/server-status» (o DumbMonit adiciona «?auto» por si próprio).

!!! warning
    Sem ExtendedStatus, os pedidos e bytes não são reportados de forma alguma, não como zero: o gráfico fica vazio em vez de mostrar uma linha plana enganosa. O Apache não publica a sua própria versão no mod_status; o DumbMonit lê-a do cabeçalho de resposta Server, que algumas distribuições cortam de propósito.

No Debian e Ubuntu, a localização de estado normalmente já existe em
`/etc/apache2/mods-available/status.conf`; editar a linha `Require` aí basta,
sem necessidade de um novo ficheiro. Onde a configuração é um único ficheiro
(a maioria das outras distribuições), adicione o bloco ao virtual host
principal em vez disso.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem senha | Nada: a página de estado é filtrada para que só o DumbMonit a alcance. |
| Autenticação básica | O utilizador e a senha do bloco que protege a página de estado, se existir. |

Endereço: um nome de anfitrião ou IP, com a porta se não for 80. Opções:
protocolo (HTTP por predefinição), porta, caminho de estado
(`/server-status` por predefinição, `?auto` adicionado automaticamente),
verificação de certificado e tempo limite do pedido (10 s).
