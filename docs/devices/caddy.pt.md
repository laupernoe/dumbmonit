# Caddy {#caddy}

O servidor web e proxy inverso: upstreams a falhar as suas verificações de
saúde, uma recarga de configuração que falhou, erros de handler e a fração
de respostas 5xx.

O DumbMonit lê três coisas da API de administração do Caddy, todas com
`GET`: `/config/apps/http/servers` (a configuração HTTP que o Caddy executa,
nunca a secção `tls` onde vivem os tokens dos fornecedores de DNS),
`/reverse_proxy/upstreams` (os backends e as falhas que a verificação de
saúde passiva memoriza) e `/metrics` (saúde dos upstreams, recargas, pedidos
e erros).

A API de administração do Caddy não é só de leitura: quem a alcançar pode
carregar uma nova configuração ou parar o Caddy. Escuta em `localhost:2019`
por essa razão. A configuração abaixo mantém-a aí e deixa o próprio Caddy
publicar uma vista só de leitura dela, protegida por senha, que responde a
`GET` nos três caminhos acima e recusa tudo o resto.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_caddy_`. Os totais são
contadores: os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `config_loaded` | 1 quando o Caddy executa pelo menos um servidor HTTP | |
| `servers`, `routes` | servidores HTTP e rotas na configuração | |
| `config_last_reload_ok` | 0 quando a última recarga de configuração falhou | |
| `config_last_reload_timestamp_seconds` | hora da última recarga bem-sucedida | |
| `upstreams`, `upstreams_unhealthy` | backends de cada `reverse_proxy`, e os que estão a falhar | |
| `upstream_healthy` | 1 quando nem a verificação de saúde ativa nem a passiva exclui o backend | `upstream` |
| `upstream_fails` | pedidos falhados que a verificação de saúde passiva memoriza atualmente | `upstream` |
| `upstream_requests` | pedidos em curso para o backend | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | com a opção `metrics`: pedidos, respostas 5xx e erros de handler | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | o mesmo por servidor (`srv0`, `srv1`… ou os nomes que deu) | `server` |

`upstream_healthy` precisa de `/metrics`: a verificação de saúde ativa só é
visível aí. Uma vista só de leitura que não deixa passar `/metrics` continua
a reportar os backends e as suas falhas memorizadas. O Caddy não publica a
sua versão através da API de administração.

As [regras incorporadas](../alerting/rules.md#reverse-proxies-and-domains)
que se aplicam: Caddy upstream unhealthy, Caddy configuration reload failed,
Reverse proxy answering 5xx, mais Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o Caddy. Diz o que está mal numa frase (sem configuração, uma
recarga falhada, upstreams não saudáveis, uma fração alta de 5xx), depois os
servidores, rotas, upstreams, taxa de pedidos, fração de 5xx e erros de
handler ao longo de 24 horas, depois cada upstream com as suas falhas
memorizadas.

## Publicar uma vista só de leitura da API de administração do Caddy {#publish-a-read-only-view-of-caddys-admin-api}

1. A API de administração do Caddy pode carregar uma nova configuração e parar o Caddy: tem de ficar fora de alcance de tudo excepto o DumbMonit. Deixe-a onde o Caddy a coloca, em localhost:2019, e deixe o Caddy publicar uma vista só de leitura dela: só GET, nos três caminhos que o DumbMonit lê, atrás de uma senha.

2. Crie o hash da senha para o utilizador dumbmonit.

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

3. Adicione este site ao Caddyfile, com esse hash em vez de HASH, depois recarregue o Caddy. Responde na porta 2020 e recusa tudo exceto essas três leituras.

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

4. Opcional: adicione metrics ao bloco de opções globais no topo do Caddyfile para contar pedidos, erros e respostas 5xx por servidor. A saúde dos upstreams e as recargas são reportadas sem isso.

    ```
    metrics
    ```

5. Filtre a porta 2020 para que só o anfitrião do DumbMonit a alcance. No DumbMonit, introduza o anfitrião do Caddy com essa porta, por exemplo «caddy.lan:2020», e o utilizador dumbmonit com a sua senha.

!!! warning
    Nunca publique a própria API de administração na rede sem uma firewall em frente: quem alcançar a porta 2019 pode substituir a configuração ou parar o Caddy, e a opção origins não impede um cliente que não envia nenhum cabeçalho Origin. Se a expuser numa rede privada, filtre a porta para que só o anfitrião do DumbMonit a alcance, e escolha No password.

O Caddyfile mantém o seu significado sem indentação; `caddy fmt` indenta-o.
`header_up Host localhost:2019` é necessário porque a API de administração
só responde a pedidos dirigidos ao endereço em que escuta. Com a imagem do
contentor Caddy, execute o comando de hash com `docker exec`.

Uma configuração carregada como JSON em vez de a partir de um Caddyfile
recebe o mesmo site como um servidor extra com um handler `basic_auth`, um
correspondente `method` e `path` e um `reverse_proxy` para `localhost:2019`.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Vista só de leitura | O utilizador e a senha do bloco `basic_auth` acima (recomendado). |
| Sem senha | Nada: a própria API de administração, filtrada para que só o DumbMonit a alcance. |

Endereço: um nome de anfitrião ou IP com a porta da vista só de leitura
(`caddy.lan:2020`), ou um URL completo. Opções: protocolo (HTTP por
predefinição), porta (2019, a própria API de administração), verificação
de certificado e tempo limite do pedido (10 s).

A verificação de saúde passiva (`fail_duration`) só memoriza as falhas
durante essa duração: um backend que falha uma vez e recupera é reportado
como não saudável até então. A regra espera cinco minutos antes de disparar.
