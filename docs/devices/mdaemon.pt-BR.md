# MDaemon Email Server

Os serviços de e-mail de um servidor MDaemon, verificados de fora: SMTP,
IMAP, POP3, webmail e Remote Administration aceitam conexões e saúdam da
forma esperada. Com uma conta, o DumbMonit também lê a versão exata através
da API XML. Com o [agente](agent.md) instalado no servidor MDaemon, a mesma
página também mostra as filas de e-mail, sessões e totais de mensagens, que
o MDaemon publica apenas como contadores de desempenho do Windows.

!!! warning "Validado apenas contra a documentação"

    Esta integração foi construída a partir da documentação publicada da
    MDaemon Technologies e do formato de resposta da API XML mostrado lá,
    não contra um servidor MDaemon real. Toda chamada é uma leitura, e as
    verificações de porta de e-mail seguem os padrões de protocolo, mas
    ninguém ainda o executou contra um MDaemon em funcionamento. O mesmo
    vale para os contadores de fila lidos pelo agente: seus nomes vêm de
    uma lista publicada de contadores de desempenho do MDaemon, não de um
    servidor em execução. Avise-nos o que não funcionar.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_mdaemon_`.

| Métrica | Rótulos | Significado |
|---|---|---|
| `service_up` | `service`, `port` | 1 se a porta aceitou a conexão e, para SMTP, POP3 e IMAP, saudou com `220`, `+OK` ou `* OK`. 0 se não respondeu, ou respondeu para recusar o serviço (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Tempo até a saudação, ou até a conexão para uma porta sem uma. |
| `api_up` | | Apenas com uma conta: 1 se a API XML respondeu, 0 se não. |
| `api_response_seconds` | | Tempo que a API XML levou para responder. |
| `api_operation_ok` | | 1 se a conta tinha permissão para executar a chamada, 0 se o MDaemon respondeu com um status de erro. |
| `info` (valor 1) | `version`, `build`, `source` | A versão: da API XML (`source="xml_api"`, `build` é o build do serviço), ou senão da saudação SMTP quando o servidor a anuncia lá (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tempo que toda a verificação levou. |

**Uma indisponibilidade parcial ainda é uma verificação bem sucedida.** Um
serviço parado dá `service_up 0` para aquele serviço e os outros ainda são
medidos. Só quando nenhum serviço e nenhuma API responde é que o
dispositivo está *inacessível*.

**Uma conta recusada é um erro de configuração, não uma indisponibilidade.**
Uma senha errada, um endereço que a API XML não permite, ou uma porta
errada é mostrado na página do dispositivo com o motivo e não é
notificado.

**Filas vêm do agente, não deste dispositivo.** O MDaemon publica os
tamanhos de suas filas (entrada, local, remoto, retry, bad, holding,
quarentena), suas contagens de sessão, suas estatísticas de mensagem e seu
tempo de atividade como contadores de desempenho do Windows, no objeto
chamado MDaemon, e em seu próprio Queue and Statistics Manager. Nenhuma
documentação publicada do MDaemon descreve uma chamada da API XML que os
retorne, então este dispositivo não adivinha uma: o [agente](agent.md),
instalado no servidor MDaemon, lê os contadores de desempenho em vez
disso. Veja
[Filas de e-mail através do agente](#mail-queues-through-the-agent). A
licença e o espaço em disco do armazenamento de e-mail não são expostos
através de uma interface documentada ou um contador de desempenho, e não
são lidos. O agente também observa o disco que contém o armazenamento de
e-mail; liste o serviço MDaemon em sua configuração `services` para
observar o próprio serviço Windows.

As [regras integradas](../alerting/rules.md#mdaemon-and-securitygateway)
que se aplicam: MDaemon mail service down, MDaemon XML API not answering,
além de Device unreachable; com o agente, MDaemon mail queue growing,
MDaemon Bad queue not empty e MDaemon Retry queue high.

## A página do dispositivo

Um dispositivo MDaemon mostra um painel **Serviços de e-mail** acima dos
gráficos, lido da última medição armazenada: abrir a página nunca se
conecta ao servidor. Ele lista todo serviço observado com sua porta, uma
palavra e uma cor (Respondendo, Fora do ar), e seu tempo de resposta;
depois a versão e onde foi lida. Serviços parados vêm primeiro.

Quando o agente no mesmo servidor reporta os contadores do MDaemon, um
painel **Filas de e-mail** segue: o número de mensagens em cada fila (uma
fila congelada, uma fila Bad não vazia e mais de cinquenta mensagens em
Retry são marcadas em palavras), as sessões ativas, as mensagens e os
veredictos de spam, vírus e DNSBL das últimas 24 horas, o tempo de
atividade do MDaemon, e os servidores internos que o MDaemon reporta como
inativos. O mesmo painel aparece na própria página do agente.

## Filas de e-mail através do agente

1. Instale o [agente](agent.md#install) no servidor Windows que executa o
   MDaemon. Nada mais é necessário: o agente percebe o serviço Windows
   chamado `MDaemon` e lê os contadores do objeto de desempenho MDaemon a
   cada período de amostragem. Para forçá-lo ativado ou desativado, defina
   `mdaemon: true` ou `mdaemon: false` em `agent.yaml`.
2. As filas aparecem na página do agente. Para também mostrá-las neste
   dispositivo MDaemon, o DumbMonit procura o agente na mesma máquina: o
   agente definido como pai deste dispositivo, ou o agente relay que o
   sonda, ou o agente cujo nome de host corresponde ao endereço deste
   dispositivo (`mail` para `mail.example.com`), ou, quando há um
   dispositivo MDaemon e um agente reportando contadores do MDaemon, esse
   agente. Se nenhum corresponder, edite este dispositivo e escolha o
   agente como seu **Dispositivo pai**: o agente e o MDaemon compartilham
   a máquina, então quando o agente fica silencioso os alertas deste
   dispositivo também são retidos.

As séries pertencem ao agente, com seu rótulo `target`. Todas têm o
prefixo `dumbmonit_mdaemon_`:

| Métrica | Rótulos | Contadores de desempenho lidos |
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

Os totais contam desde a última inicialização do MDaemon: aplique
`increase()` ou `rate()` em consultas, como a página faz para seus
números de 24 horas. Os contadores por segundo (`SMTP messages
(in)/sec`…) não são lidos, já que os totais dão as mesmas taxas. Um
contador que sua versão do MDaemon não tem, ou nomeia diferente, é
reportado uma vez no log do agente (`performance counter not available`)
e os outros ainda são lidos; para ler um que o predefinido não tem, liste-o
sob `perf_counters` em `agent.yaml` (veja
[Contadores de desempenho do Windows](agent.md#windows-performance-counters)).

## Observar os serviços de e-mail, e opcionalmente a API XML

1. Nada para instalar no servidor. Sem credencial, o DumbMonit se conecta
   às portas de e-mail de fora e lê cada saudação: SMTP (25), IMAP (143) e
   Webmail (3000) por padrão. Adicione POP3, MSA, as portas TLS ou Remote
   Administration na opção Services, como uma lista desta.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. Para também ler a versão através da API XML, crie uma conta dedicada
   no MDaemon, nomeada por exemplo como abaixo, com uma senha longa usada
   em nenhum outro lugar. Dê a ela o nível mais baixo que seu MDaemon
   aceita para a API XML, e nada mais: o DumbMonit só chama
   GetVersionInfo, que não lê nenhuma caixa de correio e não altera nada.

    ```
    dumbmonit@example.com
    ```

3. Desde o MDaemon 24, a API XML só responde aos endereços que permite.
   Abra Setup → XML API Service → Address Restrictions (Setup → XML API
   Management no MDaemon 26) e permita o endereço do qual o DumbMonit se
   conecta.

4. Verifique a partir do host DumbMonit que a conta alcança a API. O
   comando pede a senha e imprime uma resposta que começa com
   `<MDaemon><API productversion=…>`.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. No DumbMonit, informe o endereço do servidor, por exemplo
   "mail.example.com", depois o endereço de e-mail completo da conta como
   nome de usuário, e sua senha. A API XML é alcançada na porta HTTPS do
   Remote Administration, 444 por padrão.

!!! warning
    O MDaemon não tem papel somente leitura para a API XML: nunca
    reutilize sua própria conta, e mantenha essa senha fora de qualquer
    outra ferramenta. O Dynamic Screening pode bloquear um endereço após
    logins falhados repetidos, então uma senha errada aqui pode bloquear o
    host DumbMonit: teste com o comando acima primeiro. O Remote
    Administration frequentemente usa um certificado autoassinado: se a
    conexão for recusada por esse motivo, marque "Aceitar um certificado
    não verificável" nas opções. Os tamanhos de fila de e-mail não são
    lidos por este dispositivo: o MDaemon os publica apenas como
    contadores de desempenho do Windows, que o agente DumbMonit lê quando
    instalado no servidor de e-mail.

A API XML é servida pelo servidor web do Remote Administration: porta 1000
via HTTP e 444 via HTTPS por padrão, em `/MdMgmtWS/`, ou pelo IIS quando o
Remote Administration roda ali. Sua referência completa vem com o
servidor: abra `https://mail.example.com:444/MdMgmtWS` em um navegador, ou
procure em `\MDaemon\Docs\API\XML API\`.

## Credenciais

| Credencial | Campos |
|---|---|
| Apenas serviços | Nada: as portas de e-mail são verificadas sem fazer login. |
| Conta da API XML | O endereço de e-mail completo da conta, e sua senha, enviados como autenticação básica HTTP. |

Endereço: o nome de host ou IP do servidor, por exemplo
`mail.example.com`. Uma porta escrita no endereço
(`mail.example.com:8444`) é tomada como a porta da API XML.

## Opções

| Opção | Padrão | Efeito |
|---|---|---|
| `services` | `smtp,imap,webmail` | O que verificar, separado por vírgulas: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `nome:porta` muda uma porta; qualquer outro nome com sua porta adiciona um serviço próprio, verificado apenas por conexão. `none` não verifica nenhuma porta. |
| `request_timeout_seconds` | `10` | Tempo permitido para cada conexão, saudação e chamada de API, de 1 a 60. |
| `api_port` | `444` | A porta do Remote Administration que serve `/MdMgmtWS/`. Usada apenas com uma conta. |
| `api_tls` | `true` | HTTPS para a API XML. Desmarque apenas para o Remote Administration via HTTP simples (porta 1000 por padrão): a senha então cruza a rede em claro. |
| `insecure_tls` | `false` | Aceitar um certificado autoassinado na API XML. |

As portas TLS (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) são
verificadas apenas por conexão. Para observar a expiração de seus
certificados, adicione um dispositivo de
[certificado TLS](services.md#tls) para cada uma.
