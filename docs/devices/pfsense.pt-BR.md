# pfSense {#pfsense}

Um firewall pfSense através do pacote de API REST da comunidade
(`pfSense-pkg-RESTAPI`, versão 2): gateways que estão fora do ar, perdem
pacotes ou respondem lentamente, interfaces ativadas sem link, serviços
ativados que pararam, e CPU, memória, disco e temperatura.

O DumbMonit envia uma chave de API no cabeçalho `X-API-Key`. Uma chave
carrega exatamente os privilégios do usuário pfSense a que pertence: o
usuário `dumbmonit` abaixo possui apenas os privilégios `GET` das cinco
páginas que o DumbMonit lê, então a chave pode ler essas páginas e não
alterar nada. A cada minuto o DumbMonit lê, em paralelo,
`/api/v2/status/system`, `/api/v2/status/gateways`,
`/api/v2/status/interfaces`, `/api/v2/status/services` e
`/api/v2/system/version`, e, se o privilégio foi dado,
`/api/v2/system/restapi/version`.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir da documentação publicada do
    pacote de API REST e do código-fonte de seus modelos (nomes de campos,
    unidades, o envelope de resposta e seus identificadores de erro), não
    contra um pfSense em execução. Toda chamada é uma leitura. Avise-nos o
    que não funcionar.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_pfsense_`.

| Métrica | Rótulos | Significado |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | Como o painel os mostra, em porcentagem. O número de CPU é a própria estimativa do pfSense a partir da média de carga. |
| `temperature_celsius` | | Quando o pfSense consegue ler um sensor. |
| `load_average` | `period`: `1m`, `5m`, `15m` | Média de carga. |
| `cpu_count` | | Núcleos de processador. |
| `version_info` | `version` | Valor 1: a versão do pfSense, `2.7.2-RELEASE`. |
| `gateway_up` | `gateway` | 0 quando o pfSense marca o gateway como fora do ar, 1 caso contrário. |
| `gateway_status_info` | `gateway`, `status` | Valor 1: a própria palavra do pfSense (`none` quando tudo está bem, `down`, `loss`, `delay`). |
| `gateway_loss_percent` | `gateway` | Perda de pacotes para o endereço de monitoramento. |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | Round trip para o endereço de monitoramento e seu desvio; ausente enquanto o gateway está fora do ar. |
| `gateways`, `gateways_down` | | Contagens. |
| `interface_enabled`, `interface_up` | `interface`, `descr` | Ativada na configuração; link ativo. |
| `interface_down` | `interface`, `descr` | 1 quando a interface está ativada e não tem link. Uma interface desativada nunca está fora do ar. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | Contadores. |
| `interfaces_down` | | Contagem. |
| `service_running`, `service_enabled` | `service`, `description` | Como Status > Services os mostra. |
| `service_stopped` | `service`, `description` | 1 quando o serviço está ativado e não rodando. |
| `services_stopped` | | Contagem. |
| `restapi_update_available` | | Com o privilégio `/api/v2/system/restapi/version`: 1 quando o pacote de API REST tem uma versão mais nova. |
| `restapi_version_info` | `version`, `latest` | Valor 1: versão instalada e mais recente do pacote. |

O pfSense não informa através desta API se uma atualização do pfSense está
disponível: a única rota que toca atualizações do sistema
(`/api/v2/system/update`) inicia uma. O DumbMonit reporta apenas as
atualizações do próprio pacote.

As [regras integradas](../alerting/rules.md#pfsense) que se aplicam:

- **pfSense gateway down**: um gateway está marcado como fora do ar por
  três minutos (Warning).
- **pfSense gateway losing packets**: mais de 10% de perda por dez
  minutos, em um gateway que não está já fora do ar (Advisory).
- **pfSense gateway slow**: round trip acima de 500 ms por quinze minutos
  (Advisory). Aumente para um link via satélite.
- **pfSense interface without link**: uma interface ativada sem link por
  cinco minutos (Advisory).
- **pfSense service stopped**: um serviço ativado não rodando por cinco
  minutos (Advisory).
- **pfSense disk nearly full**: mais de 90% por trinta minutos (Advisory).

## A página do monitor {#the-device-page}

O painel Firewall diz o que está errado em uma frase, mostra CPU, memória,
disco e temperatura, depois lista todo gateway com sua latência e perda, as
interfaces sem link, os serviços parados, e uma atualização disponível do
pacote de API REST.

## Criar uma chave de API REST somente leitura para o DumbMonit {#create-a-read-only-rest-api-key-for-dumbmonit}

1. Instale o pacote de API REST da comunidade (pfSense-pkg-RESTAPI, versão
   2) em Diagnostics > Command Prompt ou um shell SSH, com o arquivo da
   página de releases do projeto que corresponde à sua versão do pfSense
   (2.7.2 abaixo). Depois, em System > REST API > Settings, marque tanto
   BasicAuth quanto Key entre os métodos de autenticação.

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. Em System > User Manager, adicione um usuário chamado dumbmonit com uma
   senha longa e aleatória. Em Effective Privileges, adicione apenas estes
   privilégios de API REST: /api/v2/status/system GET,
   /api/v2/status/gateways GET, /api/v2/status/interfaces GET,
   /api/v2/status/services GET, /api/v2/system/version GET, mais
   /api/v2/system/restapi/version GET para saber das próprias atualizações
   do pacote. Apenas para o próximo passo, adicione também
   /api/v2/auth/key POST.

3. Crie a chave como dumbmonit, de qualquer máquina que alcance o pfSense:
   a chave carrega exatamente os privilégios desse usuário. Copie o valor
   da chave na resposta, depois remova o privilégio /api/v2/auth/key POST
   de novo.

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

4. No DumbMonit, informe o endereço do pfSense, por exemplo "pfsense.lan",
   e cole a chave como API key. O pfSense vem com um certificado
   autoassinado: marque Aceitar um certificado não verificável a menos que
   você o tenha substituído.

!!! warning
    A API REST não informa se uma atualização do pfSense está disponível,
    apenas se o próprio pacote tem uma. O pacote é um projeto da
    comunidade, não parte do pfSense: reinstale-o após cada atualização do
    pfSense.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API REST | A chave criada para `dumbmonit` no passo 3, enviada como `X-API-Key`. |

Endereço: o nome de host ou IP do pfSense, `host:porta`, ou uma URL.
Opções:

- **Protocolo** (`https`) e **Porta** (443): os do webConfigurator,
  definidos em System > Advanced > Admin Access.
- **Aceitar um certificado não verificável**: o próprio certificado do
  pfSense é autoassinado.
- **Tempo limite por requisição** (15 s).

## Solução de problemas {#troubleshooting}

"pfSense refused the API key (401)": a chave está errada, ou Key não está
marcado entre os métodos de autenticação em System > REST API > Settings.

"pfSense accepted the API key but refused … (403)": o usuário `dumbmonit`
não tem o privilégio dessa página; a mensagem o nomeia.

"… not found: is the REST API package installed?": o pacote está faltando,
tipicamente após uma atualização do pfSense, que o remove. Reinstale-o como
no passo 1.
