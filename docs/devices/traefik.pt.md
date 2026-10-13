# Traefik {#traefik}

O proxy inverso: routers desativados por um erro de configuração, servidores de
backend que falham a verificação de saúde, certificados que um resolver nunca
obteve ou que não são renovados, e a parte de respostas 5xx.

O DumbMonit lê a API do Traefik, que só lê: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (servido pelo
Traefik 3.7) e `/api/version`. Quando as métricas Prometheus do Traefik são
servidas na mesma porta, `/metrics` acrescenta a taxa de pedidos e a parte de
respostas 5xx. Nenhum método da API altera nada, e o DumbMonit não envia
nenhum.

As falhas a vigiar são as silenciosas. Um router cujo serviço ou middleware não
existe, ou cuja regra não é interpretada, é desativado: os seus sites
respondem 404, e o Traefik só o diz no seu registo. Um serviço cujos servidores
falham todos a verificação de saúde responde 503 «no available server». Um
router que pede a um resolver ACME um certificado que o resolver não consegue
obter recebe o certificado autoassinado predefinido do Traefik, que os
navegadores recusam.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_traefik_`. Os totais são
contadores: os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | routers de todos os protocolos, e os que estão em erro ou aviso | |
| `services`, `services_errors`, `middlewares_errors` | o mesmo para serviços e middlewares | |
| `router_status` | por router HTTP: 0 ativado, 1 aviso, 2 desativado | `router`, `provider` |
| `router_error_info` | valor 1, para um router que não está ativado: o primeiro erro do Traefik | `router`, `error` |
| `service_error_info` | valor 1, para um serviço que não está ativado: o primeiro erro do Traefik | `service`, `error` |
| `service_status` | por serviço: 0 ativado, 1 aviso, 2 desativado | `service` |
| `service_servers`, `service_servers_up` | servidores de um serviço com balanceamento de carga, e os que passam a verificação de saúde | `service` |
| `server_up` | 1 quando o servidor passa a verificação de saúde (ou não tem nenhuma) | `service`, `server` |
| `resolver_routers` | routers que pedem o seu certificado a um resolver de certificados | `resolver` |
| `resolver_routers_uncovered` | os cujos nomes de anfitrião nenhum certificado detido pelo Traefik cobre ainda (precisa de `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificados que o Traefik detém, e os dias que faltam para cada um expirar | `cert` |
| `requests_total`, `requests_5xx_total` | com métricas: pedidos de todos os pontos de entrada, e os respondidos com 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | com métricas e etiquetas de router: o mesmo por router | `router` |
| `config_reloads_total` | com métricas: recarregamentos da configuração | |

Os routers e serviços do próprio Traefik (`@internal`) ficam de fora. Os
routers, serviços e servidores são descritos um a um até 500 de cada; os totais
cobrem sempre tudo.

Um router, serviço ou servidor que o Traefik já não lista (o provider Docker
retira de imediato os routers de um contentor parado) simplesmente deixa de ser
reportado: o DumbMonit não guarda um valor «saudável» para ele, e as suas
séries terminam. Os gráficos e as regras podem ainda mostrar o seu último valor
durante alguns minutos, até o armazenamento de métricas descartar a série
obsoleta. O Traefik responde 404 para uma rota assim, o que nenhuma regra
reporta: só um router que está listado e desativado o faz.

As [regras incorporadas](../alerting/rules.md#reverse-proxies-and-domains) que
se aplicam: Traefik router disabled, Traefik backend server down, Traefik
service without a server, Traefik certificate not obtained, Proxy certificate
not renewed, Reverse proxy answering 5xx, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
consulta o Traefik. Diz numa frase o que está errado (routers desativados e o
motivo do Traefik, serviços sem servidor, resolvers que não obtiveram nada,
certificados perto de expirar, uma parte elevada de 5xx), depois os routers,
serviços, taxa de pedidos e parte de 5xx, depois os servidores ativos por
serviço, os certificados que expiram primeiro à cabeça, e as respostas 5xx por
router em 24 horas.

## Deixar o DumbMonit ler a API do Traefik {#let-dumbmonit-read-traefiks-api}

1. A API do Traefik só lê: routers, serviços e certificados, nunca uma alteração. Ative-a com as métricas Prometheus, que acrescentam a taxa de pedidos e a parte de respostas 5xx: adicione estes dois argumentos ao comando do Traefik (ou as mesmas chaves à sua configuração estática) e reinicie-o. Ambas são então servidas no ponto de entrada traefik, porta 8080.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. A porta 8080 responde sem palavra-passe: publique-a apenas num endereço que o anfitrião do DumbMonit alcance, filtre-a para que nada mais a alcance, e verifique que responde.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Para exigir antes uma palavra-passe, encaminhe /api e /metrics por um ponto de entrada seu com um middleware basicAuth, como a documentação mostra, e escolha utilizador basicAuth abaixo. Crie o utilizador dumbmonit com htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. No DumbMonit, introduza o anfitrião do Traefik, por exemplo «traefik.lan», com a porta desse ponto de entrada se não for a 8080. Desmarque Ler as métricas se /metrics for servido noutro sítio.

!!! warning
    Um router desativado por um erro de configuração deita abaixo os seus sites, e o Traefik só o diz no seu registo. A verificação de que um resolver de certificados obteve os seus certificados precisa de /api/certificates, que o Traefik 3.7 serve; sem ele, a expiração dos certificados vem apenas das métricas.

### Um ponto de entrada da API com palavra-passe {#an-api-entry-point-with-a-password}

Em vez de `--api.insecure=true`, dê à API e às métricas um ponto de entrada
próprio e ponha um middleware basicAuth à frente de ambos. Na configuração
estática:

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

e na configuração dinâmica (mostrado o provider de ficheiros; as labels Docker
funcionam da mesma forma), com a linha impressa pelo `htpasswd`:

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

Depois introduza `traefik.lan:8082` no DumbMonit com o utilizador `dumbmonit`.
`addRoutersLabels` é o que dá as respostas 5xx por router; sem ele só são
contados os totais por ponto de entrada.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem palavra-passe | Nada: a porta da API é filtrada para que só o DumbMonit a alcance. |
| Utilizador basicAuth | O utilizador e a palavra-passe do middleware basicAuth à frente de `/api` e `/metrics`. |

Endereço: um nome de anfitrião ou IP (`traefik.lan`), `host:port`, ou um URL
completo. Opções: protocolo (HTTP por predefinição), porta da API (8080), Ler as
métricas (ativado), verificação do certificado e tempo limite do pedido (10 s).

Um serviço sem verificação de saúde reporta todos os servidores como ativos: o
Traefik só marca um servidor como em baixo quando um `healthCheck` falha. Os
serviços ponderados e de espelhamento não têm servidores próprios e não são
contados.
