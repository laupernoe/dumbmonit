# Traefik {#traefik}

O proxy reverso: roteadores desativados por um erro de configuração,
servidores backend falhando sua verificação de saúde, certificados que um
resolver nunca obteve ou que não são renovados, e a parcela de respostas
5xx.

O DumbMonit lê a API do Traefik, que só lê: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (servido
pelo Traefik 3.7) e `/api/version`. Quando as métricas Prometheus do
Traefik são servidas na mesma porta, `/metrics` adiciona a taxa de
requisição e a parcela de respostas 5xx. Nenhum método da API altera nada,
e o DumbMonit não envia nenhum.

As falhas que valem a pena observar são as silenciosas. Um roteador cujo
serviço ou middleware não existe, ou cuja regra não analisa, é desativado:
seus sites respondem 404, e o Traefik só avisa isso em seu log. Um serviço
cujos servidores todos falham sua verificação de saúde responde 503 "no
available server". Um roteador que pede a um resolver ACME um certificado
que o resolver não consegue obter recebe o certificado autoassinado padrão
do Traefik, que navegadores recusam.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_traefik_`. Os totais são
contadores: gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | roteadores de todo protocolo, e os em erro ou aviso | |
| `services`, `services_errors`, `middlewares_errors` | o mesmo para serviços e middlewares | |
| `router_status` | por roteador HTTP: 0 ativado, 1 aviso, 2 desativado | `router`, `provider` |
| `router_error_info` | valor 1, para um roteador não ativado: o primeiro erro do Traefik | `router`, `error` |
| `service_status` | por serviço: 0 ativado, 1 aviso, 2 desativado | `service` |
| `service_servers`, `service_servers_up` | servidores de um serviço com balanceamento de carga, e os que passam sua verificação de saúde | `service` |
| `server_up` | 1 quando o servidor passa sua verificação de saúde (ou não tem nenhuma) | `service`, `server` |
| `resolver_routers` | roteadores pedindo a um resolver de certificado seu certificado | `resolver` |
| `resolver_routers_uncovered` | aqueles cujos nomes de host nenhum certificado mantido pelo Traefik ainda cobre (precisa de `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificados que o Traefik mantém, e os dias restantes antes de cada um expirar | `cert` |
| `requests_total`, `requests_5xx_total` | com métricas: requisições de todo entry point, e as respondidas com 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | com métricas e rótulos de roteador: o mesmo por roteador | `router` |
| `config_reloads_total` | com métricas: recarregamentos de configuração | |

Os próprios roteadores e serviços do Traefik (`@internal`) são deixados de
fora. Roteadores, serviços e servidores são descritos um a um até 500
cada; os totais sempre cobrem tudo.

As [regras integradas](../alerting/rules.md#reverse-proxies-and-domains)
que se aplicam: Traefik router disabled, Traefik backend server down,
Traefik service without a server, Traefik certificate not obtained, Proxy
certificate not renewed, Reverse proxy answering 5xx, além de Device
unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Traefik. Ele diz o que está errado em uma frase
(roteadores desativados e o motivo do Traefik, serviços sem servidor,
resolvers que não obtiveram nada, certificados próximos da expiração, uma
alta parcela de 5xx), depois os roteadores, serviços, taxa de requisição e
parcela de 5xx, depois os servidores ativos por serviço, os certificados
que expiram primeiro, e as respostas 5xx por roteador nas últimas 24
horas.

## Deixar o DumbMonit ler a API do Traefik {#let-dumbmonit-read-traefiks-api}

1. A API do Traefik só lê: roteadores, serviços e certificados, nunca uma
   alteração. Ative-a com as métricas Prometheus, que adicionam a taxa de
   requisição e a parcela de respostas 5xx: adicione estes dois argumentos
   ao comando do Traefik (ou as mesmas chaves à sua configuração estática)
   e reinicie-o. Ambos são então servidos no entry point traefik, porta
   8080.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. A porta 8080 responde sem senha: publique-a apenas em um endereço que o
   host DumbMonit alcance, filtre-a para que nada mais o faça, e verifique
   que ela responde.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Para exigir uma senha em vez disso, roteie /api e /metrics através de um
   entry point próprio com um middleware basicAuth, como a documentação
   mostra, e escolha usuário basicAuth abaixo. Crie o usuário dumbmonit com
   htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. No DumbMonit, informe o host Traefik, por exemplo "traefik.lan", com a
   porta desse entry point se não for 8080. Desmarque Ler as métricas se
   /metrics for servido em outro lugar.

!!! warning
    Um roteador desativado por um erro de configuração derruba seus sites,
    e o Traefik só avisa isso em seu log. A verificação de que um resolver
    de certificado obteve seus certificados precisa de /api/certificates,
    que o Traefik 3.7 serve; sem ele, a expiração de certificado vem
    apenas das métricas.

### Um entry point de API com senha {#an-api-entry-point-with-a-password}

Em vez de `--api.insecure=true`, dê à API e às métricas um entry point
próprio e coloque um middleware basicAuth na frente de ambos. Na
configuração estática:

```yaml
entryPoints:
  monitor:
    address: ":8082"
api: {}
metrics:
  prometheus:
    manualRouting: true
    addRoutersLabels: true
```

e na configuração dinâmica (provedor de arquivo mostrado; labels Docker
funcionam da mesma forma), com a linha impressa por `htpasswd`:

```yaml
http:
  middlewares:
    dumbmonit-auth:
      basicAuth:
        users:
          - "dumbmonit:$2y$05$…"
  routers:
    monitor-api:
      entryPoints: [monitor]
      rule: "PathPrefix(`/api`)"
      service: api@internal
      middlewares: [dumbmonit-auth]
    monitor-metrics:
      entryPoints: [monitor]
      rule: "Path(`/metrics`)"
      service: prometheus@internal
      middlewares: [dumbmonit-auth]
```

Depois informe `traefik.lan:8082` no DumbMonit com o usuário `dumbmonit`.
`addRoutersLabels` é o que dá as respostas 5xx por roteador; sem ele só os
totais por entry point são contados.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem senha | Nada: a porta da API é filtrada para que só o DumbMonit a alcance. |
| Usuário basicAuth | O usuário e senha do middleware basicAuth na frente de `/api` e `/metrics`. |

Endereço: um nome de host ou IP (`traefik.lan`), `host:porta`, ou uma URL
completa. Opções: protocolo (HTTP por padrão), porta da API (8080), Ler as
métricas (ativado), verificação de certificado e tempo limite da
requisição (10 s).

Um serviço sem verificação de saúde reporta todo servidor como ativo: o
Traefik só marca um servidor como fora do ar quando um `healthCheck`
falha. Serviços ponderados e de espelhamento não têm servidores próprios e
não são contados.
