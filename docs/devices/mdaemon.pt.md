# MDaemon Email Server {#mdaemon-email-server}

Os serviços de email de um servidor MDaemon, verificados do exterior: SMTP,
IMAP, POP3, webmail e Remote Administration aceitam ligações e saúdam como
deveriam. Com uma conta, o DumbMonit também lê a versão exata através da
API XML. Com o [agente](agent.md) instalado no servidor MDaemon, a mesma
página também mostra as filas de email, sessões e totais de mensagens, que
o MDaemon só publica como contadores de desempenho do Windows.

!!! warning "Validado apenas contra a documentação"

    Esta integração foi construída a partir da documentação publicada pela
    MDaemon Technologies e do formato de resposta da API XML aí mostrado,
    não contra um servidor MDaemon real. Toda chamada é uma leitura, e as
    verificações das portas de email seguem os padrões do protocolo, mas
    ainda ninguém a executou contra um MDaemon real. O mesmo se aplica aos
    contadores de fila lidos pelo agente: os seus nomes vêm de uma lista
    publicada dos contadores de desempenho do MDaemon, não de um servidor
    em funcionamento. Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_mdaemon_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `service_up` | `service`, `port` | 1 se a porta aceitou a ligação e, para SMTP, POP3 e IMAP, saudou com `220`, `+OK` ou `* OK`. 0 se não respondeu, ou respondeu para recusar serviço (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Tempo até à saudação, ou até à ligação para uma porta sem uma. |
| `api_up` | | Só com uma conta: 1 se a API XML respondeu, 0 se não respondeu. |
| `api_response_seconds` | | Tempo que a API XML demorou a responder. |
| `api_operation_ok` | | 1 se a conta foi autorizada a executar a chamada, 0 se o MDaemon respondeu com um estado de erro. |
| `info` (valor 1) | `version`, `build`, `source` | A versão: da API XML (`source="xml_api"`, `build` é a compilação do serviço), ou senão da saudação SMTP quando o servidor a anuncia aí (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tempo que toda a verificação demorou. |

**Uma falha parcial continua a ser uma verificação bem-sucedida.** Um
serviço parado dá `service_up 0` para esse serviço e os outros continuam a
ser medidos. Só quando nenhum serviço e nenhuma API responde de todo é que
o monitor está *inacessível*.

**Uma conta recusada é um erro de configuração, não uma indisponibilidade.**
Uma senha errada, um endereço que a API XML não permite, ou uma porta
errada é mostrada na página do monitor com o motivo e não é notificada.

**As filas vêm do agente, não deste monitor.** O MDaemon publica os
tamanhos das suas filas (inbound, local, remote, retry, bad, holding,
quarantine), as suas contagens de sessão, as suas estatísticas de mensagens
e o seu tempo de atividade como contadores de desempenho do Windows, no
objeto chamado MDaemon, e no seu próprio Queue and Statistics Manager.
Nenhuma documentação publicada do MDaemon descreve uma chamada de API XML
que os devolva, por isso este monitor não arrisca adivinhar uma: o
[agente](agent.md), instalado no servidor MDaemon, lê os contadores de
desempenho em vez disso. Ver [Filas de email através do agente](#mail-queues-through-the-agent).
A licença e o espaço em disco do armazenamento de email não são expostos
através de uma interface documentada nem de um contador de desempenho, e
não são lidos. O agente também monitoriza o disco que contém o
armazenamento de email; liste o serviço MDaemon na sua definição `services`
para monitorizar o próprio serviço Windows.

As [regras incorporadas](../alerting/rules.md#mdaemon-and-securitygateway)
que se aplicam: MDaemon mail service down, MDaemon XML API not answering,
mais Monitor unreachable; com o agente, MDaemon mail queue growing, MDaemon
Bad queue not empty e MDaemon Retry queue high.

## A página do monitor {#the-device-page}

Um monitor MDaemon mostra um painel **Mail services** acima dos
gráficos, lido da última medição guardada: abrir a página nunca liga ao
servidor. Lista cada serviço monitorizado com a sua porta, uma palavra e
uma cor (Answering, Down), e o seu tempo de resposta; depois a versão e de
onde foi lida. Os serviços parados vêm primeiro.

Quando o agente no mesmo servidor reporta os contadores do MDaemon, um
painel **Mail queues** segue-se: o número de mensagens em cada fila (uma
fila congelada, uma fila Bad não vazia e mais de cinquenta mensagens em
Retry são marcadas em palavras), as sessões ativas, as mensagens e os
veredictos de spam, vírus e DNSBL das últimas 24 horas, o tempo de
atividade do MDaemon, e os servidores internos que o MDaemon reporta como
inativos. O mesmo painel aparece na própria página do agente.

## Filas de email através do agente {#mail-queues-through-the-agent}

1. Instale o [agente](agent.md#install) no servidor Windows que executa o
   MDaemon. Nada mais é necessário: o agente nota o serviço Windows chamado
   `MDaemon` e lê os contadores do objeto de desempenho MDaemon em cada
   período de amostragem. Para forçá-lo a ligar ou desligar, defina
   `mdaemon: true` ou `mdaemon: false` em `agent.yaml`.
2. As filas aparecem na página do agente. Para também as mostrar neste
   monitor MDaemon, o DumbMonit procura o agente na mesma máquina: o
   agente definido como pai deste monitor, ou o agente relay que o
   sonda, ou o agente cujo nome de anfitrião corresponde ao endereço deste
   monitor (`mail` para `mail.example.com`), ou, quando há um
   monitor MDaemon e um agente a reportar contadores MDaemon, esse
   agente. Se nenhum corresponder, edite este monitor e escolha o
   agente como o seu **Parent device**: o agente e o MDaemon partilham a
   máquina, por isso quando o agente fica silencioso os alertas deste
   monitor também são retidos.

As séries pertencem ao agente, com a sua etiqueta `target`. Todas têm o
prefixo `dumbmonit_mdaemon_`:

| Métrica | Etiquetas | Contadores de desempenho lidos |
|---|---|---|
| `queue_messages` | `queue`: `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | `Inbound queue messages`, `Local queue messages`, `Remote queue messages`, `Retry queue messages`, `Bad queue messages`, `Holding queue messages`, `LAN queue messages`, `Quarantine queue messages`, `RAW queue messages` |
| `queue_frozen` (1 congelada) | `queue`: `inbound`, `local`, `remote` | `Inbound queue frozen`, `Local queue frozen`, `Remote queue frozen` |
| `sessions_active` | `protocol`: `smtp_in`, `smtp_out`, `pop3_in`, `pop3_out`, `imap`, `webmail` | `Active SMTP (in) sessions`, `Active SMTP (out) sessions`, `Active POP3 (in) sessions`, `Active POP3 (out) sessions`, `Active IMAP sessions`, `Active Webmail sessions` |
| `sessions_total` (contador) | `protocol`: `smtp_in`, `smtp_out`, `pop3`, `imap` | `SMTP sessions (in) total`, `SMTP sessions (out) total`, `POP3 sessions total`, `IMAP sessions total` |
| `messages_total` (contador) | `protocol`: `smtp_in`, `smtp_out`, `domainpop_in` | `SMTP messages (in) total`, `SMTP messages (out) total`, `DomainPOP messages (in) total` |
| `filtered_messages_total` (contador) | `filter`: `spam`, `virus`, `dnsbl`; `verdict`: `accepted`, `refused` | `spam accepted total`, `spam refused total`, `Viruses accepted total`, `Viruses refused total`, `DNSBL accepted total`, `DNSBL refused total` |
| `server_active` (1 ativo) | `server`: `smtp`, `pop3`, `imap`, `webmail`, `webadmin`, `activesync`, `antispam`, `antivirus`, `minger`, `multipop`, `domainpop` | `SMTP server state`, `POP3 server state`, `IMAP server state`, `Web Mail server state`, `Web Admin server state`, `ActiveSync server state`, `AntiSpam server state`, `AntiVirus server state`, `Minger server state`, `MultiPOP server state`, `DomainPOP server state` |
| `running` (1 em execução) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

Os totais contam desde que o MDaemon arrancou por último: aplique
`increase()` ou `rate()` nas consultas, como a página faz para os seus
valores de 24 horas. Os contadores por segundo (`SMTP messages (in)/sec`…)
não são lidos, já que os totais dão as mesmas taxas. Um contador que a sua
versão do MDaemon não tem, ou nomeia de forma diferente, é reportado uma
vez no registo do agente (`performance counter not available`) e os outros
continuam a ser lidos; para ler um que a predefinição não tem, liste-o em
`perf_counters` em `agent.yaml` (ver
[Contadores de desempenho do Windows](agent.md#windows-performance-counters)).

## Monitorizar os serviços de email, e opcionalmente a API XML {#watch-the-mail-services-and-optionally-the-xml-api}

1. Nada para instalar no servidor. Sem credencial, o DumbMonit liga-se às portas de email do exterior e lê cada saudação: SMTP (25), IMAP (143) e Webmail (3000) por predefinição. Adicione POP3, MSA, as portas TLS ou Remote Administration na opção Services, como uma lista desta forma.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. Para também ler a versão através da API XML, crie uma conta dedicada no MDaemon, nomeada por exemplo como se segue, com uma senha longa não usada em nenhum outro lugar. Dê-lhe o nível mais baixo que o seu MDaemon aceita para a API XML, e nada mais: o DumbMonit só chama GetVersionInfo, que não lê nenhuma caixa de correio e não altera nada.

    ```
    dumbmonit@example.com
    ```

3. Desde o MDaemon 24, a API XML só responde aos endereços que permite. Abra Setup → XML API Service → Address Restrictions (Setup → XML API Management no MDaemon 26) e permita o endereço de onde o DumbMonit se liga.

4. Verifique a partir do anfitrião do DumbMonit que a conta alcança a API. O comando pede a senha e imprime uma resposta que começa com `<MDaemon><API productversion=…>`.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. No DumbMonit, introduza o endereço do servidor, por exemplo «mail.example.com», depois o endereço de email completo da conta como nome de utilizador, e a sua senha. A API XML é alcançada na porta HTTPS do Remote Administration, 444 por predefinição.

!!! warning
    O MDaemon não tem função só de leitura para a API XML: nunca reutilize a sua própria conta, e mantenha esta senha fora de qualquer outra ferramenta. O Dynamic Screening pode bloquear um endereço depois de inícios de sessão falhados repetidos, por isso uma senha errada aqui pode bloquear o anfitrião do DumbMonit: teste primeiro com o comando acima. O Remote Administration usa frequentemente um certificado autoassinado: se a ligação for recusada por esse motivo, marque «Accept an unverifiable certificate» nas opções. Os tamanhos das filas de email não são lidos por este monitor: o MDaemon só os publica como contadores de desempenho do Windows, que o agente DumbMonit lê quando está instalado no servidor de email.

A API XML é servida pelo servidor web do Remote Administration: porta 1000
via HTTP e 444 via HTTPS por predefinição, em `/MdMgmtWS/`, ou pelo IIS
quando o Remote Administration corre aí. A sua referência completa vem com
o servidor: abra `https://mail.example.com:444/MdMgmtWS` num navegador, ou
procure em `\MDaemon\Docs\API\XML API\`.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Só serviços | Nada: as portas de email são verificadas sem iniciar sessão. |
| Conta da API XML | O endereço de email completo da conta, e a sua senha, enviados como autenticação HTTP básica. |

Endereço: o nome de anfitrião ou IP do servidor, por exemplo
`mail.example.com`. Uma porta escrita no endereço (`mail.example.com:8444`)
é tomada como a porta da API XML.

## Opções {#options}

| Opção | Predefinição | Efeito |
|---|---|---|
| `services` | `smtp,imap,webmail` | O que verificar, separado por vírgulas: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `nome:porta` altera uma porta; qualquer outro nome com a sua porta adiciona um serviço próprio, verificado só por ligação. `none` não verifica nenhuma porta. |
| `request_timeout_seconds` | `10` | Tempo permitido para cada ligação, saudação e chamada de API, de 1 a 60. |
| `api_port` | `444` | A porta do Remote Administration que serve `/MdMgmtWS/`. Usada só com uma conta. |
| `api_tls` | `true` | HTTPS para a API XML. Desmarque só para o Remote Administration via HTTP simples (porta 1000 por predefinição): a senha atravessa então a rede em claro. |
| `insecure_tls` | `false` | Aceitar um certificado autoassinado na API XML. |

As portas TLS (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) são
verificadas só por ligação. Para monitorizar a expiração dos seus
certificados, adicione um [certificado TLS](services.md#tls) para cada uma.
