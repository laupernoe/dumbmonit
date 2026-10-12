# MikroTik RouterOS {#mikrotik-routeros}

Routers e comutadores a correr RouterOS 7: se o RouterOS e o firmware
RouterBOOT ficaram atrasados, CPU, memória e armazenamento, temperaturas,
ventoinhas e fontes de alimentação, e o estado, tráfego, erros e perdas de
ligação de cada interface.

O DumbMonit lê a API REST que o RouterOS 7 serve sob `/rest`, com um
utilizador num grupo que só pode ler. Envia pedidos `GET` e nada mais.

Um MikroTik raramente se avaria; envelhece. Uma versão do RouterOS que já
não é a mais recente do seu canal, um firmware RouterBOOT deixado atrás
depois de uma atualização (só segue quando pedido, no reinício seguinte),
uma fonte de alimentação redundante que morreu sem ninguém notar, uma porta
que continua a registar erros ou a perder a sua ligação. Isso é o que esta
página monitoriza, juntamente com a carga.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_mikrotik_`. Os totais são
contadores: os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `info` | valor 1 | `version`, `release` (`long-term`, `stable`…), `board`, `architecture`, `identity` |
| `uptime_seconds` | tempo desde o último arranque | |
| `cpu_load_percent`, `cpu_count` | carga de CPU, todos os núcleos | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | memória | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | a flash ou disco em que o RouterOS vive | |
| `update_checked` | 1 quando o router verificou um RouterOS mais recente e obteve uma resposta | |
| `update_available` | 1 quando a última versão do canal difere da instalada | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 1 em hardware RouterBOARD, 0 num Cloud Hosted Router ou máquina x86 | |
| `firmware_upgrade_pending` | 1 quando o RouterBOOT é mais antigo do que o firmware incluído com o RouterOS | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | cada sensor que a placa tem | `sensor` (`cpu-temperature`, `fan1-speed`…) |
| `health_ok` | 1 quando um sensor `psu*-state` ou `fan*-state` lê `ok`, 0 caso contrário | `sensor` |
| `interface_running` | 1 quando a interface tem ligação | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | tráfego | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | erros e descartes | `interface`, `type` |
| `interface_link_downs_total` | vezes que a ligação se perdeu | `interface`, `type` |
| `interfaces_skipped` | interfaces deixadas de fora além do limite definido nas opções | |
| `scrape_errors` | leituras secundárias que falharam durante a última verificação | |

Só as interfaces ativadas são lidas. As dinâmicas (um cliente PPPoE ou
L2TP, cujo nome muda a cada ligação) e o loopback são deixados de fora.

Os sensores dependem da placa: um hAP tem uma tensão e uma temperatura, um
CCR adiciona ventoinhas e fontes de alimentação, um Cloud Hosted Router não
tem nenhum. Um sensor que a placa não tem não produz série.

A versão mais recente do RouterOS só é conhecida quando o próprio router
verifica atualizações: um utilizador só de leitura não pode iniciar essa
verificação. O passo opcional abaixo agenda-a no router. Sem ele,
`update_checked` fica em 0 e a regra de atualização nunca dispara.

As [regras incorporadas](../alerting/rules.md#mikrotik-routeros) que se
aplicam: MikroTik CPU high, memory high, storage almost full, temperature
high, power supply or fan failed, interface accumulating errors, link
flapping, RouterOS update available e firmware upgrade pending, mais
Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca liga ao router. Mostra a versão do RouterOS e se uma mais recente
está disponível, o firmware RouterBOOT, o tempo de atividade, CPU, memória
e armazenamento, cada sensor com o seu estado, e uma tabela das interfaces
com a sua ligação, tráfego, erros e perdas de ligação.

## Criar um utilizador só de leitura no router {#create-a-read-only-user-on-the-router}

1. Abra um terminal no router (WinBox → New Terminal, ou SSH com a conta predefinida) e crie um grupo que só pode ler, através da API REST. O RouterOS precisa das três políticas para uma leitura REST: rest-api abre a API, api e read permitem a leitura em si.

    ```
    /user group add name=dumbmonit-read policy=read,api,rest-api comment=DumbMonit
    ```

2. Crie o utilizador nesse grupo com uma senha longa e aleatória própria. Para aceitar a conta só a partir do DumbMonit, adicione address= seguido do endereço do anfitrião do DumbMonit.

    ```
    /user add name=dumbmonit group=dumbmonit-read password=REPLACE-WITH-A-LONG-RANDOM-PASSWORD
    ```

3. Ative o serviço web HTTPS, que o RouterOS vem desativado e sem certificado. Ignore isto se www-ssl já tiver um. Caso contrário estes comandos criam um certificado no próprio router; depois marque «Accept an unverifiable certificate» nas opções abaixo.

    ```
    /certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
    /certificate sign local-ca
    /certificate add name=https-cert common-name=router.lan days-valid=825
    /certificate sign https-cert ca=local-ca
    /ip service set www-ssl certificate=https-cert disabled=no
    ```

4. Opcional: para ser avisado de novas versões do RouterOS, deixe o router verificá-las no arranque e uma vez por dia. Um utilizador só de leitura não pode iniciar essa verificação, por isso o DumbMonit só lê o seu resultado.

    ```
    /system scheduler add name=check-for-updates interval=1d start-time=startup on-event="/system package update check-for-updates once"
    ```

5. No DumbMonit, introduza o endereço do router, por exemplo «192.168.88.1», com o utilizador dumbmonit e a sua senha. O DumbMonit só envia pedidos GET para /rest: nunca altera uma definição, nunca inicia uma atualização e nunca reinicia o router.

!!! warning
    É necessário o RouterOS 7.1 ou posterior: o RouterOS 6 não tem API REST. Não use a conta predefinida: o seu grupo completo pode alterar tudo, enquanto dumbmonit-read não tem nenhuma política de escrita e toda alteração é recusada. O serviço www (HTTP simples) também responde à API REST, mas envia a senha em claro em cada pedido.

As três políticas foram verificadas uma a uma no RouterOS 7.23.7, cada uma
com um utilizador novo: sem `rest-api` o router responde 401, sem `api`
responde «not allowed», sem `read` «not enough permissions». O DumbMonit
reporta cada caso como um problema de credencial nomeando a política em
falta, não como um router em baixo. Com as três, uma alteração, um
reinício ou uma verificação de atualização é recusada com «not enough
permissions».

O RouterOS pode continuar a aplicar as antigas políticas de um grupo a um
utilizador durante algum tempo depois de o grupo ser editado. Se alterar as
políticas de um grupo existente e o erro persistir, remova e recrie o
utilizador.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Utilizador só de leitura | O utilizador `dumbmonit` e a sua senha, enviados como autenticação HTTP básica em cada pedido. A API REST não tem sessão. |

Endereço: um nome de anfitrião ou IP (`192.168.88.1`), `anfitrião:porta`,
ou um URL completo (`https://router.lan:8443`). Opções: protocolo (HTTPS
por predefinição; HTTP usa o serviço `www`), porta (443 para HTTPS, 80 para
HTTP), verificação de certificado, tempo limite do pedido (10 s), se ler as
interfaces (ativado), e quantas no máximo (64).
