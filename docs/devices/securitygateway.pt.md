# SecurityGateway for Email Servers {#securitygateway-for-email-servers}

Os serviços de um gateway de email SecurityGateway, verificados do exterior,
e no SecurityGateway 12.5 ou posterior, a versão e os contadores de
desempenho que a sua API REST publica: fila de entrega, quarentena, sessões
SMTP.

!!! warning "Validado apenas contra a documentação"

    Esta integração foi construída a partir da documentação publicada pela
    MDaemon Technologies do SecurityGateway e da sua API REST, não contra um
    gateway real. Toda chamada é uma leitura. A documentação não publica o
    caminho nem os nomes dos contadores de desempenho, por isso o DumbMonit
    encontra o caminho na própria descrição da API e mantém cada contador
    sob o nome que a API lhe dá. Ainda ninguém a executou contra um gateway
    real. Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_securitygateway_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `service_up` | `service`, `port` | 1 se a porta aceitou a ligação e, para SMTP, saudou com `220`. 0 se não respondeu, ou respondeu para recusar o serviço (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Tempo até à saudação, ou até à ligação para uma porta sem uma. |
| `api_up` | | Só com uma chave de API: 1 se a API REST respondeu, 0 se não respondeu. |
| `api_response_seconds` | | Tempo que a API REST demorou a responder. |
| `counters_available` | | 1 se os contadores de desempenho foram lidos; 0 se a API não descreve nenhum, ou a conta da chave não os pode ler. |
| `counter` | `counter` | Uma série por valor numérico dos contadores de desempenho, nomeada como a API a nomeia, em `snake_case`: `deliveryQueue` torna-se `delivery_queue`. No máximo 64. |
| `info` (valor 1) | `version`, `source` | A versão dada pela descrição da API (`source="openapi"`), ou senão pela saudação SMTP quando o gateway a anuncia aí (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tempo que toda a verificação demorou. |

**Como os contadores são encontrados.** A API REST do SecurityGateway
descreve-se a si própria em `/api/v1/openapi`. O DumbMonit lê essa
descrição, escolhe o caminho mais curto só de leitura sem parâmetros cujo
nome mencione contadores, lê-o, e mantém todos os números na sua resposta.
Nada é renomeado e nada é assumido: se uma versão futura renomear um
contador, a série segue o novo nome. Os contadores listados na ajuda do
SecurityGateway são as sessões SMTP de entrada e saída ativas, a fila de
entrega, a quarentena administrativa, o tempo de atividade, as contagens de
domínios e utilizadores e o pool de ligações à base de dados; alguns deles
só mudam uma vez por minuto.

**Uma falha parcial continua a ser uma verificação bem-sucedida.** Um
serviço parado dá `service_up 0` para esse serviço e os outros continuam a
ser medidos. Só quando nenhum serviço e nenhuma API responde de todo é que
o monitor está *inacessível*. Uma chave recusada é um erro de
configuração, mostrado na página do monitor e não notificado.

**O que não é lido.** A licença e a sua expiração, o espaço em disco e as
estatísticas de mensagens do painel não são descritos pela documentação
publicada da API REST. Para monitorizar o próprio serviço Windows do
SecurityGateway e os discos do servidor, instale o [agente](agent.md) no
servidor e liste o serviço na sua definição `services`.

As [regras incorporadas](../alerting/rules.md#mdaemon-and-securitygateway)
que se aplicam: SecurityGateway service down, SecurityGateway API not
answering, SecurityGateway delivery queue growing, mais Monitor unreachable.

## A página do monitor {#the-device-page}

Um monitor SecurityGateway mostra um painel **Mail services** acima dos
gráficos, lido da última medição guardada: abrir a página nunca liga ao
gateway. Lista cada serviço monitorizado com a sua porta, uma palavra e uma
cor (Answering, Down) e o seu tempo de resposta, a versão, e, com uma chave
de API, cada contador de desempenho com o seu valor mais recente.

## Monitorizar os serviços do gateway, e opcionalmente a API REST {#watch-the-gateway-services-and-optionally-the-rest-api}

1. Nada para instalar no servidor. Sem credencial, o DumbMonit liga-se ao SMTP (25) e à interface web (4000) do exterior e lê a saudação SMTP. Altere a lista na opção Services, por exemplo para verificar HTTPS em vez de HTTP.

    ```
    smtp,web_https
    ```

2. Para também ler os contadores de desempenho, é necessário o SecurityGateway 12.5 ou posterior. Crie uma conta dedicada com o seguinte nome, com uma senha longa não usada em nenhum outro lugar, e dê-lhe a função Domain Administrator. Se os contadores continuarem vazios na página do monitor, a sua versão reserva-os para a função Global Administrator, e a conta precisa dessa função em vez disso.

    ```
    dumbmonit
    ```

3. Inicie sessão como essa conta e crie uma chave em Setup/Users → Accounts → API Keys, com uma data de expiração. Uma chave transporta os direitos da conta que a cria, por isso tem de ser criada a partir desta conta e não da sua.

4. Restrinja a conta ao endereço de onde o DumbMonit se liga nas suas restrições de IP: as chaves de API também as obedecem.

5. Verifique a partir do anfitrião do DumbMonit que a chave funciona. O comando imprime a descrição da API, que também lista os contadores que o DumbMonit lê.

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. No DumbMonit, introduza o endereço do gateway, por exemplo «sg.example.com», e cole a chave. A API é alcançada na porta HTTPS da interface web, 4443 por predefinição, sob /api/v1.

!!! warning
    O SecurityGateway não tem função só de leitura nem chave mais restrita: a chave pode fazer tudo o que a sua conta pode. Nunca a crie a partir da sua própria conta, mantenha-a só no DumbMonit e dê-lhe uma data de expiração. Versões anteriores à 12.5 não têm API REST: deixe a credencial vazia e o DumbMonit só verifica os serviços. A interface web usa frequentemente um certificado autoassinado na 4443: se a ligação for recusada por esse motivo, marque «Accept an unverifiable certificate» nas opções.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Só serviços | Nada: as portas são verificadas sem iniciar sessão. |
| Chave de API (12.5 e posterior) | A chave, enviada como `Authorization: Bearer …`. |

Endereço: o nome de anfitrião ou IP do gateway, por exemplo
`sg.example.com`. Uma porta escrita no endereço (`sg.example.com:8443`) é
tomada como a porta da API.

## Opções {#options}

| Opção | Predefinição | Efeito |
|---|---|---|
| `services` | `smtp,web` | O que verificar, separado por vírgulas: `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `nome:porta` altera uma porta; qualquer outro nome com a sua porta adiciona um serviço próprio, verificado só por ligação. `none` não verifica nenhuma porta. |
| `request_timeout_seconds` | `10` | Tempo permitido para cada ligação, saudação e chamada de API, de 1 a 60. |
| `api_port` | `4443` | A porta da interface web que serve `/api/v1`. Usada só com uma chave de API. |
| `api_tls` | `true` | HTTPS para a API REST. Desmarque só para a interface web via HTTP simples (porta 4000 por predefinição): a chave atravessa então a rede em claro. |
| `insecure_tls` | `false` | Aceitar um certificado autoassinado na API REST. |
| `counters` | `true` | Ler os contadores de desempenho. |
