# MikroTik RouterOS

Roteadores e switches rodando RouterOS 7: se o RouterOS e o firmware
RouterBOOT ficaram atrasados, CPU, memória e armazenamento, temperaturas,
ventoinhas e fontes de alimentação, e o estado, tráfego, erros e perdas de
link de toda interface.

O DumbMonit lê a API REST que o RouterOS 7 serve sob `/rest`, com um usuário
em um grupo que só pode ler. Ele envia requisições `GET` e nada mais.

Um MikroTik raramente quebra; ele envelhece. Uma versão do RouterOS que não
é mais a mais recente de seu canal, um firmware RouterBOOT deixado atrás
após uma atualização (ele só segue quando solicitado, no próximo
reinício), uma fonte de alimentação redundante que morreu sem ninguém
notar, uma porta que continua registrando erros ou perdendo seu link. É
isso que esta página observa, junto com a carga.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_mikrotik_`. Os totais são
contadores: gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `info` | valor 1 | `version`, `release` (`long-term`, `stable`…), `board`, `architecture`, `identity` |
| `uptime_seconds` | tempo desde a última inicialização | |
| `cpu_load_percent`, `cpu_count` | carga de CPU, todos os núcleos | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | memória | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | a flash ou disco em que o RouterOS vive | |
| `update_checked` | 1 quando o roteador verificou um RouterOS mais novo e obteve uma resposta | |
| `update_available` | 1 quando a versão mais recente do canal difere da instalada | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 1 em hardware RouterBOARD, 0 em um Cloud Hosted Router ou uma máquina x86 | |
| `firmware_upgrade_pending` | 1 quando o RouterBOOT é mais antigo que o firmware empacotado com o RouterOS | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | cada sensor que a placa tem | `sensor` (`cpu-temperature`, `fan1-speed`…) |
| `health_ok` | 1 quando um sensor `psu*-state` ou `fan*-state` lê `ok`, 0 caso contrário | `sensor` |
| `interface_running` | 1 quando a interface tem link | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | tráfego | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | erros e descartes | `interface`, `type` |
| `interface_link_downs_total` | vezes que o link foi perdido | `interface`, `type` |
| `interfaces_skipped` | interfaces deixadas de fora além do limite definido nas opções | |
| `scrape_errors` | leituras secundárias que falharam durante a última sondagem | |

Apenas interfaces ativadas são lidas. Dinâmicas (um cliente PPPoE ou L2TP,
cujo nome muda a cada conexão) e o loopback são deixados de fora.

Sensores dependem da placa: um hAP tem uma tensão e uma temperatura, um CCR
adiciona ventoinhas e fontes de alimentação, um Cloud Hosted Router não tem
nenhum. Um sensor que a placa não tem não produz série.

A versão mais nova do RouterOS só é conhecida quando o próprio roteador
verifica atualizações: um usuário somente leitura não tem permissão para
iniciar essa verificação. O passo opcional abaixo a agenda no roteador. Sem
ele, `update_checked` permanece em 0 e a regra de atualização nunca dispara.

As [regras integradas](../alerting/rules.md#mikrotik-routeros) que se
aplicam: MikroTik CPU high, memory high, storage almost full, temperature
high, power supply or fan failed, interface accumulating errors, link
flapping, RouterOS update available e firmware upgrade pending, além de
Device unreachable.

## A página do dispositivo

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca se conecta ao roteador. Ele mostra a versão do RouterOS e se uma mais
nova está disponível, o firmware RouterBOOT, o tempo de atividade, CPU,
memória e armazenamento, cada sensor com seu estado, e uma tabela das
interfaces com seu link, tráfego, erros e perdas de link.

## Criar um usuário somente leitura no roteador

1. Abra um terminal no roteador (WinBox → New Terminal, ou SSH com a conta
   padrão) e crie um grupo que só pode ler, através da API REST. O
   RouterOS precisa das três políticas para uma leitura REST: rest-api abre
   a API, api e read permitem a própria leitura.

    ```
    /user group add name=dumbmonit-read policy=read,api,rest-api comment=DumbMonit
    ```

2. Crie o usuário nesse grupo com uma senha longa e aleatória própria. Para
   aceitar a conta apenas do DumbMonit, adicione address= seguido do
   endereço do host DumbMonit.

    ```
    /user add name=dumbmonit group=dumbmonit-read password=REPLACE-WITH-A-LONG-RANDOM-PASSWORD
    ```

3. Ative o serviço web HTTPS, que o RouterOS entrega desativado e sem
   certificado. Pule isso se www-ssl já tem um. Caso contrário, estes
   comandos fazem um certificado no próprio roteador; depois marque
   "Aceitar um certificado não verificável" nas opções abaixo.

    ```
    /certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
    /certificate sign local-ca
    /certificate add name=https-cert common-name=router.lan days-valid=825
    /certificate sign https-cert ca=local-ca
    /ip service set www-ssl certificate=https-cert disabled=no
    ```

4. Opcional: para ser avisado sobre novas versões do RouterOS, deixe o
   roteador verificá-las na inicialização e uma vez por dia. Um usuário
   somente leitura não pode iniciar essa verificação, então o DumbMonit só
   lê seu resultado.

    ```
    /system scheduler add name=check-for-updates interval=1d start-time=startup on-event="/system package update check-for-updates once"
    ```

5. No DumbMonit, informe o endereço do roteador, por exemplo
   "192.168.88.1", com o usuário dumbmonit e sua senha. O DumbMonit só
   envia requisições GET para /rest: ele nunca altera uma configuração,
   nunca inicia uma atualização e nunca reinicia o roteador.

!!! warning
    O RouterOS 7.1 ou posterior é necessário: o RouterOS 6 não tem API
    REST. Não use a conta padrão: seu grupo completo pode alterar tudo,
    enquanto dumbmonit-read não tem política de escrita e toda alteração é
    recusada. O serviço www (HTTP simples) também responde à API REST, mas
    envia a senha em claro em cada requisição.

As três políticas foram verificadas uma a uma no RouterOS 7.23.7, cada uma
com um usuário novo: sem `rest-api` o roteador responde 401, sem `api`
responde "not allowed", sem `read` "not enough permissions". O DumbMonit
reporta cada caso como um problema de credencial nomeando a política
faltante, não como um roteador fora do ar. Com as três, uma alteração, um
reinício ou uma verificação de atualização é recusada com "not enough
permissions".

O RouterOS pode continuar aplicando as políticas anteriores de um grupo a
um usuário por um tempo depois que o grupo é editado. Se você mudar as
políticas de um grupo existente e o erro persistir, remova e recrie o
usuário.

## Credenciais

| Credencial | Campos |
|---|---|
| Usuário somente leitura | O usuário `dumbmonit` e sua senha, enviados como autenticação básica HTTP em cada requisição. A API REST não tem sessão. |

Endereço: um nome de host ou IP (`192.168.88.1`), `host:porta`, ou uma URL
completa (`https://router.lan:8443`). Opções: protocolo (HTTPS por padrão;
HTTP usa o serviço `www`), porta (443 para HTTPS, 80 para HTTP),
verificação de certificado, tempo limite da requisição (10 s), se deve ler
as interfaces (ativado), e quantas no máximo (64).
