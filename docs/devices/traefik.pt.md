# Traefik {#traefik}

O proxy inverso: routers desativados por um erro de configuração,
servidores de backend a falhar a sua verificação de saúde, certificados
que um resolvedor nunca obteve ou que não são renovados, e a fração de
respostas 5xx.

O DumbMonit lê a API do Traefik, que só lê: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (servida
pelo Traefik 3.7) e `/api/version`. Quando as métricas Prometheus do
Traefik são servidas na mesma porta, `/metrics` adiciona a taxa de
pedidos e a fração de respostas 5xx. Nenhum método da API altera nada, e
o DumbMonit não envia nenhum.

As falhas a vigiar são as silenciosas. Um router cujo serviço ou
middleware não existe, ou cuja regra não analisa, é desativado: os seus
sites respondem 404, e o Traefik só o diz no seu registo. Um serviço
cujos servidores todos falham a sua verificação de saúde responde 503
«no available server». Um router que pede a um resolvedor ACME um
certificado que o resolvedor não consegue obter recebe o certificado
autoassinado predefinido do Traefik, que os navegadores recusam.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_traefik_`. Os totais são
contadores: os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | routers de todos os protocolos, e os em erro ou aviso | |
| `services`, `services_errors`, `middlewares_errors` | o mesmo para serviços e middlewares | |
| `router_status` | por router HTTP: 0 ativado, 1 aviso, 2 desativado | `router`, `provider` |
| `router_error_info` | valor 1, para um router não ativado: o primeiro erro do Traefik | `router`, `error` |
| `service_status` | por serviço: 0 ativado, 1 aviso, 2 desativado | `service` |
| `service_servers`, `service_servers_up` | servidores de um serviço com balanceamento de carga, e os que passam a sua verificação de saúde | `service` |
| `server_up` | 1 quando o servidor passa a sua verificação de saúde (ou não tem nenhuma) | `service`, `server` |
| `resolver_routers` | routers a pedir um certificado a um resolvedor de certificado | `resolver` |
| `resolver_routers_uncovered` | os cujos nomes de anfitrião nenhum certificado detido pelo Traefik cobre ainda (precisa de `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificados que o Traefik detém, e os dias restantes antes de cada um expirar | `cert` |
| `requests_total`, `requests_5xx_total` | com métricas: pedidos de todo ponto de entrada, e os respondidos 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | com métricas e etiquetas de router: o mesmo por router | `router` |
| `config_reloads_total` | com métricas: recargas de configuração | |

Os próprios routers e serviços do Traefik (`@internal`) são deixados de
fora. Routers, serviços e servidores são descritos um a um até 500 cada;
os totais cobrem sempre tudo.

As [regras incorporadas](../alerting/rules.md#reverse-proxies-and-domains)
que se aplicam: Traefik router disabled, Traefik backend server down,
Traefik service without a server, Traefik certificate not obtained,
Proxy certificate not renewed, Reverse proxy answering 5xx, mais Device
unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a
página nunca consulta o Traefik. Diz o que está mal numa frase (routers
desativados e o motivo do Traefik, serviços sem servidor, resolvedores
que não obtiveram nada, certificados perto de expirar, uma fração alta
de 5xx), depois os routers, serviços, taxa de pedidos e fração de 5xx,
depois os servidores ativos por serviço, os certificados a expirar mais
em breve primeiro, e as respostas 5xx por router ao longo de 24 horas.

## Deixar o DumbMonit ler a API do Traefik {#let-dumbmonit-read-traefiks-api}

1. A API do Traefik só lê: routers, serviços e certificados, nunca uma alteração. Ative-a com as métricas Prometheus, que adicionam a taxa de pedidos e a fração de respostas 5xx: adicione estes dois argumentos ao comando do Traefik (ou as mesmas chaves à sua configuração estática) e reinicie-o. Ambos são então servidos no ponto de entrada traefik, porta 8080.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. A porta 8080 responde sem senha: publique-a só num endereço que o anfitrião do DumbMonit alcance, filtre-a para que mais nada o faça, e verifique que responde.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Para exigir uma senha em vez disso, encaminhe /api e /metrics através de um ponto de entrada próprio com um middleware basicAuth, como a documentação mostra, e escolha basicAuth user abaixo. Crie o utilizador dumbmonit com htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. No DumbMonit, introduza o anfitrião do Traefik, por exemplo «traefik.lan», com a porta desse ponto de entrada se não for 8080. Desmarque Read the metrics se /metrics for servido noutro lugar.

!!! warning
    Um router desativado por um erro de configuração derruba os seus sites, e o Traefik só o diz no seu registo. A verificação de que um resolvedor de certificado obteve os seus certificados precisa de /api/certificates, que o Traefik 3.7 serve; sem ele, a expiração de certificado vem só das métricas.

### Um ponto de entrada de API com senha {#an-api-entry-point-with-a-password}

Em vez de `--api.insecure=true`, dê à API e às métricas um ponto de
entrada próprio e coloque um middleware basicAuth em frente de ambos. Na
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

e na configuração dinâmica (mostrado o provider de ficheiro; etiquetas
Docker funcionam da mesma forma), com a linha impressa por `htpasswd`:

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

Depois introduza `traefik.lan:8082` no DumbMonit com o utilizador
`dumbmonit`. `addRoutersLabels` é o que dá as respostas 5xx por router;
sem ele só os totais por ponto de entrada são contados.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem senha | Nada: a porta da API é filtrada para que só o DumbMonit a alcance. |
| Utilizador basicAuth | O utilizador e a senha do middleware basicAuth em frente de `/api` e `/metrics`. |

Endereço: um nome de anfitrião ou IP (`traefik.lan`), `anfitrião:porta`,
ou um URL completo. Opções: protocolo (HTTP por predefinição), porta da
API (8080), Read the metrics (ativado), verificação de certificado e
tempo limite do pedido (10 s).

Um serviço sem verificação de saúde reporta todo servidor como ativo: o
Traefik só marca um servidor como em baixo quando um `healthCheck`
falha. Os serviços weighted e mirroring não têm servidores próprios e não
são contados.
