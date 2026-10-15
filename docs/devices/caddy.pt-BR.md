# Caddy {#caddy}

O servidor web e proxy reverso: upstreams falhando suas verificações de
saúde, um recarregamento de configuração que falhou, erros de handler e a
parcela de respostas 5xx.

O DumbMonit lê três coisas da API de administração do Caddy, todas com
`GET`: `/config/apps/http/servers` (a configuração HTTP que o Caddy
executa, nunca a seção `tls` onde vivem tokens de provedor DNS),
`/reverse_proxy/upstreams` (os backends e as falhas que a verificação de
saúde passiva lembra) e `/metrics` (saúde de upstream, recarregamentos,
requisições e erros).

A API de administração do Caddy não é somente leitura: quem a alcança pode
carregar uma nova configuração ou parar o Caddy. Ela escuta em
`localhost:2019` por esse motivo. A configuração abaixo a mantém lá e deixa
o próprio Caddy publicar uma visão somente leitura dela, protegida por
senha, que responde `GET` nos três caminhos acima e recusa tudo mais.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_caddy_`. Os totais são
contadores: gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `config_loaded` | 1 quando o Caddy executa ao menos um servidor HTTP | |
| `servers`, `routes` | servidores HTTP e rotas na configuração | |
| `config_last_reload_ok` | 0 quando o último recarregamento de configuração falhou | |
| `config_last_reload_timestamp_seconds` | horário do último recarregamento bem sucedido | |
| `upstreams`, `upstreams_unhealthy` | backends de todo `reverse_proxy`, e os que estão falhando | |
| `upstream_healthy` | 1 quando nem a verificação de saúde ativa nem a passiva descartam o backend | `upstream` |
| `upstream_fails` | requisições com falha que a verificação de saúde passiva lembra atualmente | `upstream` |
| `upstream_requests` | requisições em andamento para o backend | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | com a opção `metrics`: requisições, respostas 5xx e erros de handler | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | o mesmo por servidor (`srv0`, `srv1`… ou os nomes que você deu) | `server` |

`upstream_healthy` precisa de `/metrics`: a verificação de saúde ativa só é
visível lá. Uma visão somente leitura que não deixa passar `/metrics` ainda
reporta os backends e suas falhas lembradas. O Caddy não publica sua versão
através da API de administração.

As [regras integradas](../alerting/rules.md#reverse-proxies-and-domains) que
se aplicam: Caddy upstream unhealthy, Caddy configuration reload failed,
Reverse proxy answering 5xx, além de Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Caddy. Ele diz o que está errado em uma frase (nenhuma
configuração, um recarregamento com falha, upstreams não saudáveis, uma
alta parcela de 5xx), depois os servidores, rotas, upstreams, taxa de
requisição, parcela de 5xx e erros de handler nas últimas 24 horas, depois
cada upstream com suas falhas lembradas.

## Publicar uma visão somente leitura da API de administração do Caddy {#publish-a-read-only-view-of-caddys-admin-api}

1. A API de administração do Caddy pode carregar uma nova configuração e
   parar o Caddy: ela precisa permanecer inacessível a qualquer coisa além
   do DumbMonit. Deixe-a onde o Caddy a coloca, em localhost:2019, e deixe
   o Caddy publicar uma visão somente leitura dela: apenas GET, nos três
   caminhos que o DumbMonit lê, detrás de uma senha.

2. Crie o hash da senha para o usuário dumbmonit.

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

3. Adicione este site ao Caddyfile, com esse hash no lugar de HASH, depois
   recarregue o Caddy. Ele responde na porta 2020 e recusa tudo exceto
   essas três leituras.

    ```
    :2020 {
    basic_auth {
    dumbmonit HASH
    }
    @readonly {
    method GET
    path /config/apps/http/servers /reverse_proxy/upstreams /metrics
    }
    handle @readonly {
    reverse_proxy localhost:2019 {
    header_up Host localhost:2019
    }
    }
    respond 403
    }
    ```

4. Opcional: adicione metrics ao bloco de opções globais no topo do
   Caddyfile para contar requisições, erros e respostas 5xx por servidor. A
   saúde de upstream e os recarregamentos são reportados sem isso.

    ```
    metrics
    ```

5. Filtre a porta 2020 para que só o host DumbMonit a alcance. No DumbMonit,
   informe o host Caddy com essa porta, por exemplo "caddy.lan:2020", e o
   usuário dumbmonit com sua senha.

!!! warning
    Nunca publique a própria API de administração na rede sem um firewall
    na frente: qualquer um que alcance a porta 2019 pode substituir a
    configuração ou parar o Caddy, e a opção origins não impede um cliente
    que não envia cabeçalho Origin. Se você a expõe em uma rede privada,
    filtre a porta para que só o host DumbMonit a alcance, e escolha Sem
    senha.

O Caddyfile mantém seu significado sem indentação; `caddy fmt` o indenta.
`header_up Host localhost:2019` é necessário porque a API de administração
só responde a requisições endereçadas ao endereço em que escuta. Com a
imagem do contêiner Caddy, execute o comando de hash com `docker exec`.

Uma configuração carregada como JSON em vez de a partir de um Caddyfile
recebe o mesmo site como um servidor extra com um handler `basic_auth`, um
correspondente `method` e `path` e um `reverse_proxy` para
`localhost:2019`.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Visão somente leitura | O usuário e senha do bloco `basic_auth` acima (recomendado). |
| Sem senha | Nada: a própria API de administração, filtrada para que só o DumbMonit a alcance. |

Endereço: um nome de host ou IP com a porta da visão somente leitura
(`caddy.lan:2020`), ou uma URL completa. Opções: protocolo (HTTP por
padrão), porta (2019, a própria API de administração), verificação de
certificado e tempo limite da requisição (10 s).

A verificação de saúde passiva (`fail_duration`) só lembra falhas por
aquela duração: um backend que falha uma vez e se recupera é reportado não
saudável até então. A regra espera cinco minutos antes de disparar.
