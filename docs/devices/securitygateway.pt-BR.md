# SecurityGateway for Email Servers

Os serviços de um gateway de e-mail SecurityGateway, verificados de fora, e
no SecurityGateway 12.5 ou posterior, a versão e os contadores de desempenho
que sua API REST publica: fila de entrega, quarentena, sessões SMTP.

!!! warning "Validado apenas contra a documentação"

    Esta integração foi construída a partir da documentação publicada da
    MDaemon Technologies sobre o SecurityGateway e sua API REST, não contra
    um gateway real. Toda chamada é uma leitura. A documentação não publica
    o caminho ou os nomes dos contadores de desempenho, então o DumbMonit
    encontra o caminho na própria descrição da API e mantém cada contador
    sob o nome que a API lhe dá. Ninguém ainda o executou contra um gateway
    em funcionamento. Avise-nos o que não funcionar.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_securitygateway_`.

| Métrica | Rótulos | Significado |
|---|---|---|
| `service_up` | `service`, `port` | 1 se a porta aceitou a conexão e, para SMTP, saudou com `220`. 0 se não respondeu, ou respondeu para recusar o serviço (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Tempo até a saudação, ou até a conexão para uma porta sem uma. |
| `api_up` | | Apenas com uma chave de API: 1 se a API REST respondeu, 0 se não. |
| `api_response_seconds` | | Tempo que a API REST levou para responder. |
| `counters_available` | | 1 se os contadores de desempenho foram lidos; 0 se a API não descreve nenhum, ou a conta da chave não pode lê-los. |
| `counter` | `counter` | Uma série por valor numérico dos contadores de desempenho, nomeada como a API o nomeia, em `snake_case`: `deliveryQueue` se torna `delivery_queue`. No máximo 64. |
| `info` (valor 1) | `version`, `source` | A versão dada pela descrição da API (`source="openapi"`), ou senão pela saudação SMTP quando o gateway a anuncia lá (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tempo que toda a verificação levou. |

**Como os contadores são encontrados.** A API REST do SecurityGateway se
descreve em `/api/v1/openapi`. O DumbMonit lê essa descrição, pega o caminho
somente leitura mais curto sem parâmetros cujo nome mencione contadores, o
lê, e mantém todo número em sua resposta. Nada é renomeado e nada é
assumido: se uma versão futura renomear um contador, a série segue o novo
nome. Os contadores listados na ajuda do SecurityGateway são as sessões SMTP
de entrada e saída ativas, a fila de entrega, a quarentena administrativa, o
tempo de atividade, as contagens de domínios e usuários e o pool de conexões
do banco de dados; alguns deles só mudam uma vez por minuto.

**Uma indisponibilidade parcial ainda é uma verificação bem sucedida.** Um
serviço parado dá `service_up 0` para aquele serviço e os outros ainda são
medidos. Só quando nenhum serviço e nenhuma API responde é que o dispositivo
está *inacessível*. Uma chave recusada é um erro de configuração, mostrado
na página do dispositivo e não notificado.

**O que não é lido.** A licença e sua expiração, o espaço em disco e as
estatísticas de mensagens do painel não são descritos pela documentação
publicada da API REST. Para observar o próprio serviço Windows do
SecurityGateway e os discos do servidor, instale o [agente](agent.md) no
servidor e liste o serviço em sua configuração `services`.

As [regras integradas](../alerting/rules.md#mdaemon-and-securitygateway) que
se aplicam: SecurityGateway service down, SecurityGateway API not
answering, SecurityGateway delivery queue growing, além de Device
unreachable.

## A página do dispositivo

Um dispositivo SecurityGateway mostra um painel **Serviços de e-mail** acima
dos gráficos, lido da última medição armazenada: abrir a página nunca se
conecta ao gateway. Ele lista todo serviço observado com sua porta, uma
palavra e uma cor (Respondendo, Fora do ar) e seu tempo de resposta, a
versão, e com uma chave de API, todo contador de desempenho com seu valor
mais recente.

## Observar os serviços do gateway, e opcionalmente a API REST

1. Nada para instalar no servidor. Sem credencial, o DumbMonit se conecta ao
   SMTP (25) e à interface web (4000) de fora e lê a saudação SMTP. Mude a
   lista na opção Services, por exemplo para verificar HTTPS em vez de
   HTTP.

    ```
    smtp,web_https
    ```

2. Para também ler os contadores de desempenho, o SecurityGateway 12.5 ou
   posterior é necessário. Crie uma conta dedicada nomeada como abaixo, com
   uma senha longa usada em nenhum outro lugar, e dê a ela o papel Domain
   Administrator. Se os contadores permanecerem vazios na página do
   dispositivo, sua versão os reserva para o papel Global Administrator, e a
   conta precisa desse papel em vez disso.

    ```
    dumbmonit
    ```

3. Entre como essa conta e crie uma chave em Setup/Users → Accounts → API
   Keys, com uma data de expiração. Uma chave carrega os direitos da conta
   que a cria, por isso ela precisa ser criada a partir desta conta e não da
   sua.

4. Restrinja a conta ao endereço do qual o DumbMonit se conecta em suas
   restrições de IP: chaves de API também as obedecem.

5. Verifique a partir do host DumbMonit que a chave funciona. O comando
   imprime a descrição da API, que também lista os contadores que o
   DumbMonit lê.

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. No DumbMonit, informe o endereço do gateway, por exemplo
   "sg.example.com", e cole a chave. A API é alcançada na porta HTTPS da
   interface web, 4443 por padrão, sob /api/v1.

!!! warning
    O SecurityGateway não tem papel somente leitura nem chave mais
    restrita: a chave pode fazer tudo que sua conta pode. Nunca a crie a
    partir da sua própria conta, mantenha-a só no DumbMonit e dê a ela uma
    data de expiração. Versões anteriores à 12.5 não têm API REST: deixe a
    credencial vazia e o DumbMonit verifica apenas os serviços. A interface
    web frequentemente usa um certificado autoassinado na 4443: se a
    conexão for recusada por esse motivo, marque "Aceitar um certificado não
    verificável" nas opções.

## Credenciais

| Credencial | Campos |
|---|---|
| Apenas serviços | Nada: as portas são verificadas sem fazer login. |
| Chave de API (12.5 e posteriores) | A chave, enviada como `Authorization: Bearer …`. |

Endereço: o nome de host ou IP do gateway, por exemplo `sg.example.com`.
Uma porta escrita no endereço (`sg.example.com:8443`) é tomada como a porta
da API.

## Opções

| Opção | Padrão | Efeito |
|---|---|---|
| `services` | `smtp,web` | O que verificar, separado por vírgulas: `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `nome:porta` muda uma porta; qualquer outro nome com sua porta adiciona um serviço próprio, verificado apenas por conexão. `none` não verifica nenhuma porta. |
| `request_timeout_seconds` | `10` | Tempo permitido para cada conexão, saudação e chamada de API, de 1 a 60. |
| `api_port` | `4443` | A porta da interface web que serve `/api/v1`. Usada apenas com uma chave de API. |
| `api_tls` | `true` | HTTPS para a API REST. Desmarque apenas para a interface web sobre HTTP simples (porta 4000 por padrão): a chave então cruza a rede em claro. |
| `insecure_tls` | `false` | Aceitar um certificado autoassinado na API REST. |
| `counters` | `true` | Ler os contadores de desempenho. |
