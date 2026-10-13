# CrowdSec {#crowdsec}

O motor de segurança: decisões ativas e de onde elas vêm, alertas, bouncers
que pararam de buscar, linhas de log lidas e as que nenhum analisador
reconheceu, e se a Local API responde.

O DumbMonit lê as métricas Prometheus do CrowdSec (porta 6060) e verifica
`/health` da Local API (porta 8080). Com uma chave de bouncer opcional, ele
também pede à Local API as decisões sobre 192.0.2.1, um endereço de
documentação: a resposta é sempre vazia, e prova que a autenticação e o
banco de dados respondem. Ele nunca cria ou exclui uma decisão e nunca baixa
a lista de banimento.

A falha que vale a pena observar é um bouncer que parou de buscar. O
firewall ou proxy reverso continua rodando com a lista que tinha, novos
atacantes passam, e nada reclama. O CrowdSec não publica quando um bouncer
buscou por último; o DumbMonit lê isso a partir do contador de requisições do
bouncer, que para de se mover. A segunda é uma aquisição que quebrou (um
arquivo de log renomeado, um contêiner recriado): o CrowdSec não lê nada, não
vê nenhum ataque, e permanece verde.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_crowdsec_`. Os totais são
contadores: gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `lapi_up` | 1 quando a Local API responde `/health` | |
| `lapi_decisions_ok` | com uma chave de bouncer: 1 quando a Local API respondeu à consulta de decisão | |
| `decisions` | decisões ativas, incluindo blocklists da comunidade | |
| `decisions_by_origin` | | `origin`: `crowdsec`, `cscli`, `CAPI`, `lists`… |
| `alerts` | alertas mantidos pela Local API, excluindo os da comunidade | |
| `bouncers_seen` | bouncers que chamaram a Local API desde que o CrowdSec iniciou | |
| `bouncer_requests_total` | requisições de cada bouncer; quando para de se mover, o bouncer parou de buscar | `bouncer` |
| `machine_requests_total` | requisições de cada agente (máquina) | `machine` |
| `lapi_requests_total` | todas as requisições à Local API | |
| `scenario_overflows_total` | cenários disparados pelo agente local: cada um é um alerta | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | linhas de log lidas pelo agente, e as analisadas ou não | |
| `source_lines_total`, `source_lines_unparsed_total` | o mesmo por fonte de log | `source` |
| `buckets` | buckets de cenário atualmente abertos | |

As famílias da Local API aparecem onde a Local API roda, as famílias de log
onde um agente lê logs; a imagem oficial e uma instalação padrão rodam
ambos.

As [regras integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: CrowdSec Local API down, CrowdSec bouncer stopped pulling,
CrowdSec reads no logs, além de Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o CrowdSec. Ele diz o que está errado em uma frase (Local API,
bouncers, leitura de log e a parcela de linhas que nenhum analisador
reconheceu), depois as decisões ativas, alertas mantidos, cenários disparados
nas últimas 24 horas e linhas lidas por segundo, depois o tempo desde que
cada bouncer buscou por último, as decisões por origem e a parcela não
analisada de cada fonte de log.

## Deixar o DumbMonit ler as métricas do CrowdSec {#let-dumbmonit-read-crowdsecs-metrics}

1. O CrowdSec publica suas métricas em 127.0.0.1:6060 por padrão. Defina
   listen_addr na seção prometheus de /etc/crowdsec/config.yaml para um
   endereço que o DumbMonit possa alcançar, depois reinicie o CrowdSec. A
   imagem Docker oficial já escuta em toda interface.

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. As métricas não precisam de credencial: elas contêm contagens, nomes de
   bouncer e máquina e caminhos de arquivo de log, nunca um endereço IP.
   Filtre a porta 6060 para que só o host DumbMonit a alcance, e verifique
   que ela responde.

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. O DumbMonit também verifica /health da Local API, na porta 8080 do mesmo
   host: listen_uri na seção api.server precisa então estar acessível também.
   Desmarque Verificar a Local API para um host que só roda um agente.

4. Opcional: crie uma chave de bouncer para o DumbMonit e cole-a como Chave
   de API de bouncer abaixo. Uma chave de bouncer só pode ler decisões. O
   DumbMonit pede as decisões sobre 192.0.2.1, um endereço de documentação,
   então a resposta é sempre vazia e a lista de banimento nunca é baixada.

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. No DumbMonit, informe o host CrowdSec, por exemplo "crowdsec.lan". Onde a
   Local API e os agentes rodam em hosts diferentes, adicione cada host:
   decisões, alertas e bouncers são contados onde a Local API roda, linhas
   de log onde cada agente as lê.

!!! warning
    Um bouncer que para de buscar continua aplicando uma lista congelada:
    novos atacantes passam e nada reclama. O DumbMonit avisa quando as
    requisições de um bouncer param por 30 minutos. Ele só conhece bouncers
    que buscaram pelo menos uma vez desde a última inicialização do
    CrowdSec.

Um bouncer excluído com `cscli bouncers delete` mantém seu contador até o
CrowdSec reiniciar, e é reportado como parado até então. Um bouncer que só
chama a Local API quando atende uma requisição (um bouncer de proxy reverso
em modo live) pode ficar quieto por meia hora em uma noite calma; mude-o
para o modo stream, ou aumente a janela da regra.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Apenas métricas | Nada: lê `/metrics`, e `/health` da Local API. |
| Chave de API de bouncer | A chave impressa por `cscli bouncers add dumbmonit`, enviada como `X-Api-Key` à Local API. |

Endereço: um nome de host ou IP (`crowdsec.lan`), `host:porta` para as
métricas, ou uma URL completa. Opções: protocolo (HTTP por padrão), porta
das métricas (6060), Verificar a Local API (ativado), porta da Local API
(8080), verificação de certificado e tempo limite da requisição (10 s). Itens
do Hub (analisadores, cenários, coleções) que estão corrompidos ou
desatualizados não são visíveis através das métricas, apenas com `cscli hub
list` no host.
