# CrowdSec {#crowdsec}

O motor de segurança: decisões ativas e de onde vêm, alertas, bouncers que
deixaram de obter atualizações, linhas de log lidas e as que nenhum parser
reconheceu, e se a Local API responde.

O DumbMonit lê as métricas Prometheus do CrowdSec (porta 6060) e verifica
`/health` da Local API (porta 8080). Com uma chave de bouncer opcional,
também pergunta à Local API as decisões sobre 192.0.2.1, um endereço de
documentação: a resposta é sempre vazia, e prova que a autenticação e a
base de dados respondem. Nunca cria ou elimina uma decisão e nunca descarrega
a lista de banimento.

A falha a vigiar é um bouncer que deixou de obter atualizações. A firewall
ou o proxy inverso continua a correr com a lista que tinha, novos atacantes
passam, e nada se queixa. O CrowdSec não publica quando um bouncer obteve
pela última vez; o DumbMonit lê-o a partir do contador de pedidos do
bouncer, que para de se mover. A segunda é uma aquisição que quebrou (um
ficheiro de log renomeado, um contentor recriado): o CrowdSec não lê nada,
não vê nenhum ataque, e permanece verde.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_crowdsec_`. Os totais são
contadores: os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `lapi_up` | 1 quando a Local API responde a `/health` | |
| `lapi_decisions_ok` | com uma chave de bouncer: 1 quando a Local API respondeu à consulta de decisão | |
| `decisions` | decisões ativas, listas de bloqueio da comunidade incluídas | |
| `decisions_by_origin` | | `origin`: `crowdsec`, `cscli`, `CAPI`, `lists`… |
| `alerts` | alertas mantidos pela Local API, excluindo os da comunidade | |
| `bouncers_seen` | bouncers que chamaram a Local API desde que o CrowdSec arrancou | |
| `bouncer_requests_total` | pedidos de cada bouncer; quando para de se mover, o bouncer deixou de obter atualizações | `bouncer` |
| `machine_requests_total` | pedidos de cada agente (máquina) | `machine` |
| `lapi_requests_total` | todos os pedidos à Local API | |
| `scenario_overflows_total` | cenários disparados pelo agente local: cada um é um alerta | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | linhas de log lidas pelo agente, e as analisadas ou não | |
| `source_lines_total`, `source_lines_unparsed_total` | o mesmo por fonte de log | `source` |
| `buckets` | buckets de cenário atualmente abertos | |

As famílias da Local API aparecem onde a Local API corre, as famílias de
log onde um agente lê logs; a imagem oficial e uma instalação padrão
executam ambos.

As [regras incorporadas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: CrowdSec Local API down, CrowdSec bouncer stopped pulling,
CrowdSec reads no logs, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o CrowdSec. Diz o que está mal numa frase (Local API,
bouncers, leitura de logs e a fração de linhas que nenhum parser
reconheceu), depois as decisões ativas, alertas mantidos, cenários disparados
nas últimas 24 horas e linhas lidas por segundo, depois o tempo desde que
cada bouncer obteve pela última vez, as decisões por origem e a fração não
analisada de cada fonte de log.

## Deixar o DumbMonit ler as métricas do CrowdSec {#let-dumbmonit-read-crowdsecs-metrics}

1. O CrowdSec publica as suas métricas em 127.0.0.1:6060 por predefinição. Defina listen_addr na secção prometheus de /etc/crowdsec/config.yaml para um endereço que o DumbMonit possa alcançar, depois reinicie o CrowdSec. A imagem Docker oficial já escuta em todas as interfaces.

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. As métricas não precisam de nenhuma credencial: contêm contagens, nomes de bouncers e máquinas e caminhos de ficheiros de log, nunca um endereço IP. Filtre a porta 6060 para que só o anfitrião do DumbMonit a alcance, e verifique que responde.

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. O DumbMonit também verifica /health da Local API, na porta 8080 do mesmo anfitrião: listen_uri na secção api.server tem então de estar também alcançável. Desmarque Check the Local API para um anfitrião que só executa um agente.

4. Opcional: crie uma chave de bouncer para o DumbMonit e cole-a como Bouncer API key abaixo. Uma chave de bouncer só pode ler decisões. O DumbMonit pergunta pelas decisões sobre 192.0.2.1, um endereço de documentação, por isso a resposta é sempre vazia e a lista de banimento nunca é descarregada.

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. No DumbMonit, introduza o anfitrião do CrowdSec, por exemplo «crowdsec.lan». Onde a Local API e os agentes correm em anfitriões diferentes, adicione cada anfitrião: decisões, alertas e bouncers são contados onde a Local API corre, linhas de log onde cada agente as lê.

!!! warning
    Um bouncer que deixa de obter atualizações continua a impor uma lista congelada: novos atacantes passam e nada se queixa. O DumbMonit avisa quando os pedidos de um bouncer pararem durante 30 minutos. Só conhece bouncers que obtiveram pelo menos uma vez desde que o CrowdSec arrancou pela última vez.

Um bouncer eliminado com `cscli bouncers delete` mantém o seu contador até o
CrowdSec reiniciar, e é reportado como parado até então. Um bouncer que só
chama a Local API quando serve um pedido (um bouncer de proxy inverso em
modo live) pode ficar quieto durante meia hora numa noite calma; mude-o
para o modo stream, ou aumente a janela da regra.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Só métricas | Nada: lê `/metrics`, e `/health` da Local API. |
| Chave de API de bouncer | A chave impressa por `cscli bouncers add dumbmonit`, enviada como `X-Api-Key` à Local API. |

Endereço: um nome de anfitrião ou IP (`crowdsec.lan`), `anfitrião:porta`
para as métricas, ou um URL completo. Opções: protocolo (HTTP por
predefinição), porta das métricas (6060), Check the Local API (ativado),
porta da Local API (8080), verificação de certificado e tempo limite do
pedido (10 s). Itens do hub (parsers, cenários, coleções) corrompidos ou
desatualizados não são visíveis através das métricas, só com `cscli hub
list` no anfitrião.
