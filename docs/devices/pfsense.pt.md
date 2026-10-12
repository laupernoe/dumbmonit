# pfSense {#pfsense}

Uma firewall pfSense através do pacote REST API da comunidade
(`pfSense-pkg-RESTAPI`, versão 2): gateways em baixo, a perder pacotes ou a
responder lentamente, interfaces ativadas sem ligação, serviços ativados
que pararam, e CPU, memória, disco e temperatura.

O DumbMonit envia uma chave de API no cabeçalho `X-API-Key`. Uma chave
transporta exactamente os privilégios do utilizador pfSense a que
pertence: o utilizador `dumbmonit` abaixo detém só os privilégios `GET` das
cinco páginas que o DumbMonit lê, pelo que a chave pode ler essas páginas e
não alterar nada. A cada minuto o DumbMonit lê, em paralelo,
`/api/v2/status/system`, `/api/v2/status/gateways`,
`/api/v2/status/interfaces`, `/api/v2/status/services` e
`/api/v2/system/version`, e, se o privilégio foi dado,
`/api/v2/system/restapi/version`.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir da documentação publicada do
    pacote REST API e do código-fonte dos seus modelos (nomes de campos,
    unidades, o envelope de resposta e os seus identificadores de erro),
    não contra um pfSense em funcionamento. Toda chamada é uma leitura.
    Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_pfsense_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | Como o painel os mostra, em percentagem. O valor de CPU é a própria estimativa do pfSense a partir da média de carga. |
| `temperature_celsius` | | Quando o pfSense consegue ler um sensor. |
| `load_average` | `period`: `1m`, `5m`, `15m` | Média de carga. |
| `cpu_count` | | Núcleos do processador. |
| `version_info` | `version` | Valor 1: a versão do pfSense, `2.7.2-RELEASE`. |
| `gateway_up` | `gateway` | 0 quando o pfSense marca o gateway como em baixo, 1 caso contrário. |
| `gateway_status_info` | `gateway`, `status` | Valor 1: a própria palavra do pfSense (`none` quando tudo está bem, `down`, `loss`, `delay`). |
| `gateway_loss_percent` | `gateway` | Perda de pacotes até o endereço de monitorização. |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | Tempo de ida e volta até o endereço de monitorização e o seu desvio; ausente enquanto o gateway está em baixo. |
| `gateways`, `gateways_down` | | Contagens. |
| `interface_enabled`, `interface_up` | `interface`, `descr` | Ativada na configuração; ligação ativa. |
| `interface_down` | `interface`, `descr` | 1 quando a interface está ativada e não tem ligação. Uma interface desativada nunca está em baixo. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | Contadores. |
| `interfaces_down` | | Contagem. |
| `service_running`, `service_enabled` | `service`, `description` | Como Status > Services os mostra. |
| `service_stopped` | `service`, `description` | 1 quando o serviço está ativado e não está em execução. |
| `services_stopped` | | Contagem. |
| `restapi_update_available` | | Com o privilégio `/api/v2/system/restapi/version`: 1 quando o pacote REST API tem uma versão mais recente. |
| `restapi_version_info` | `version`, `latest` | Valor 1: versão instalada e mais recente do pacote. |

O pfSense não diz através desta API se uma atualização do pfSense está
disponível: a única rota que toca nas atualizações do sistema
(`/api/v2/system/update`) inicia uma. O DumbMonit só reporta as
atualizações do próprio pacote.

As [regras incorporadas](../alerting/rules.md#pfsense) que se aplicam:

- **pfSense gateway down**: um gateway está marcado como em baixo há três
  minutos (Warning).
- **pfSense gateway losing packets**: mais de 10% de perda durante dez
  minutos, num gateway que não está já em baixo (Advisory).
- **pfSense gateway slow**: tempo de ida e volta acima de 500 ms durante
  quinze minutos (Advisory). Suba-o para uma ligação por satélite.
- **pfSense interface without link**: uma interface ativada sem ligação
  durante cinco minutos (Advisory).
- **pfSense service stopped**: um serviço ativado não em execução durante
  cinco minutos (Advisory).
- **pfSense disk nearly full**: mais de 90% durante trinta minutos
  (Advisory).

## A página do dispositivo {#the-device-page}

O painel Firewall diz o que está mal numa frase, mostra CPU, memória, disco
e temperatura, depois lista cada gateway com a sua latência e perda, as
interfaces sem ligação, os serviços parados, e uma atualização disponível
do pacote REST API.

## Criar uma chave REST API só de leitura para o DumbMonit {#create-a-read-only-rest-api-key-for-dumbmonit}

1. Instale o pacote REST API da comunidade (pfSense-pkg-RESTAPI, versão 2) a partir de Diagnostics > Command Prompt ou de uma shell SSH, com o ficheiro da página de versões do projeto que corresponde à sua versão do pfSense (2.7.2 abaixo). Depois, em System > REST API > Settings, marque tanto BasicAuth como Key entre os métodos de autenticação.

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. Em System > User Manager, adicione um utilizador chamado dumbmonit com uma senha longa e aleatória. Em Effective Privileges, adicione só estes privilégios REST API: /api/v2/status/system GET, /api/v2/status/gateways GET, /api/v2/status/interfaces GET, /api/v2/status/services GET, /api/v2/system/version GET, mais /api/v2/system/restapi/version GET para saber das próprias atualizações do pacote. Só para o passo seguinte, adicione também /api/v2/auth/key POST.

3. Crie a chave como dumbmonit, a partir de qualquer máquina que alcance o pfSense: a chave transporta exactamente os privilégios desse utilizador. Copie o valor da chave na resposta, depois remova de novo o privilégio /api/v2/auth/key POST.

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

4. No DumbMonit, introduza o endereço do pfSense, por exemplo «pfsense.lan», e cole a chave como API key. O pfSense vem com um certificado autoassinado: marque Accept an unverifiable certificate a menos que o tenha substituído.

!!! warning
    A REST API não diz se uma atualização do pfSense está disponível, só se o próprio pacote tem uma. O pacote é um projeto da comunidade, não parte do pfSense: reinstale-o depois de cada atualização do pfSense.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave REST API | A chave criada para `dumbmonit` no passo 3, enviada como `X-API-Key`. |

Endereço: o nome de anfitrião ou IP do pfSense, `anfitrião:porta`, ou um
URL. Opções:

- **Protocol** (`https`) e **Port** (443): os do webConfigurator, definidos
  em System > Advanced > Admin Access.
- **Accept an unverifiable certificate**: o próprio certificado do pfSense
  é autoassinado.
- **Timeout per request** (15 s).

## Resolução de problemas {#troubleshooting}

«pfSense refused the API key (401)»: a chave está errada, ou Key não está
marcado entre os métodos de autenticação em System > REST API > Settings.

«pfSense accepted the API key but refused … (403)»: o utilizador
`dumbmonit` não tem o privilégio dessa página; a mensagem nomeia-o.

«… not found: is the REST API package installed?»: o pacote está em falta,
tipicamente depois de uma atualização do pfSense, que o remove.
Reinstale-o como no passo 1.
