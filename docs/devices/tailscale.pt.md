# Tailscale {#tailscale}

Os dispositivos de uma tailnet através da API do Tailscale: servidores que
ficaram offline, chaves de nó a expirar em breve, dispositivos à espera de
aprovação, e clientes com uma atualização disponível.

O DumbMonit lê `GET /api/v2/tailnet/{tailnet}/devices` em
`api.tailscale.com`, uma vez por minuto. A credencial recomendada é um
cliente OAuth limitado ao âmbito `devices:core:read`: o DumbMonit troca o
seu ID e segredo por um token de uma hora (`/api/v2/oauth/token`) e
mantém-o até expirar. Pode listar os dispositivos e não pode alterar nada.
Um token de acesso de API (`tskey-api-…`) também funciona, mas transporta
todos os direitos do utilizador que o criou e expira no máximo após 90
dias.

Esta é uma API da Internet: o endereço do monitor é o da API, não de
uma máquina. O DumbMonit precisa de HTTPS de saída para `api.tailscale.com`;
nada na tailnet é contactado.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir da documentação da API do
    Tailscale e do modelo de dispositivo do seu cliente Go oficial, não
    contra uma tailnet real. Só lê. Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_tailscale_`. Um dispositivo é
nomeado pela primeira etiqueta do seu nome MagicDNS (`nas` para
`nas.tail1234.ts.net`). Os dispositivos partilhados na tailnet a partir de
outra são contados, não descritos.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `device_online` | `device` | 1 enquanto o dispositivo está ligado ao servidor de coordenação. |
| `device_offline_seconds` | `device` | 0 enquanto online, caso contrário o tempo desde que foi visto por último. |
| `device_watched` | `device` | 1 quando o dispositivo tem de ficar online: todo dispositivo etiquetado por predefinição, ou os nomeados na opção abaixo. |
| `device_key_expiry_seconds` | `device` | Até a chave de nó expirar; ausente quando a expiração de chave está desativada. |
| `device_authorized` | `device` | 0 enquanto o dispositivo espera por aprovação. |
| `device_update_available` | `device` | 1 quando o Tailscale oferece um cliente mais recente. |
| `device_info` | `device`, `os`, `client_version`, `tags` | Valor 1. |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | Contagens. |

Só os dispositivos monitorizados geram um alerta quando ficam offline: um
telefone ou um portátil que dorme não é uma indisponibilidade. Os
dispositivos etiquetados são, na prática, os servidores, routers de
sub-rede e nós de saída; liste outros na opção **Devices to keep online**,
ou introduza `all`.

As [regras incorporadas](../alerting/rules.md#tailscale) que se aplicam:

- **Tailscale device offline**: um dispositivo monitorizado desligou-se
  durante mais de 15 minutos (Advisory).
- **Tailscale node key expiring**: menos de 14 dias restantes (Advisory,
  lembrado diariamente). Quando expira, o dispositivo sai da tailnet até
  alguém iniciar sessão nele de novo.
- **Tailscale device awaiting approval** (Advisory): aprove-o se for
  esperado, remova-o caso contrário.
- **Tailscale client update**: um dispositivo monitorizado tem uma
  atualização de cliente há seis horas (Info, lembrado semanalmente).

## Dispositivos {#devices}

Todo dispositivo da tailnet, monitorizado ou não (exceto os partilhados), é
publicado sob a família genérica partilhada por toda integração do
DumbMonit que acompanha dispositivos cliente — `dumbmonit_client_device_*`,
prefixada sem `tailscale`, documentada na íntegra em
[Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | O quê | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última ligação: «agora» enquanto `connectedToControl` é verdadeiro, caso contrário `lastSeen` | `device`, `type` (`Tagged` ou `Personal`), `os`, `user`, `kind` |
| `stale_seconds` | Positivo uma vez que um dispositivo passou `device_stale_days` (3 por predefinição) sem se ligar; alimenta a regra [Dispositivo obsoleto](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: o Tailscale não tem nenhuma cópia de segurança a reportar) |

Um telefone ou um portátil que viaja pode passar vários dias sem se ligar:
isto não está relacionado com a definição **Devices to keep online** acima,
que só gera um alerta para dispositivos monitorizados a ficar offline. Suba
**Device staleness** para uma tailnet com dispositivos que circulam.

## A página do monitor {#the-device-page}

O painel Tailnet diz o que está mal numa frase, mostra os dispositivos
online, os dispositivos monitorizados offline, as atualizações disponíveis
e os dispositivos partilhados, depois lista os dispositivos monitorizados e
qualquer dispositivo à espera de aprovação ou cuja chave expira em breve,
depois a tabela de todo dispositivo com a sua última ligação.

## Criar um cliente OAuth só de leitura para o DumbMonit {#create-a-read-only-oauth-client-for-dumbmonit}

1. Na consola Tailscale (login.tailscale.com), abra Settings > Trust credentials (anteriormente OAuth clients) e gere um cliente OAuth só com o âmbito devices:core:read. Lista os dispositivos e não pode alterar nada.

2. Copie o ID do cliente e o segredo do cliente (tskey-client-…), mostrados uma vez. No DumbMonit, introduza api.tailscale.com como endereço e cole-os como Client ID e Client secret. Deixe a opção Tailnet vazia: a tailnet do cliente é lida.

!!! warning
    Só os dispositivos etiquetados (servidores, routers de sub-rede, nós de saída) devem ficar online: um telefone ou um portátil que dorme não é uma indisponibilidade. Nomeie os dispositivos a monitorizar na opção Devices to keep online, ou introduza all. Todo dispositivo da tailnet aparece na tabela Devices abaixo com a sua última ligação, monitorizado ou não; um telefone que viaja pode passar dias sem uma, por isso suba Device staleness para esses.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Cliente OAuth | ID e segredo do cliente (`tskey-client-…`) de um cliente com o âmbito `devices:core:read`. |
| Token de acesso de API | Um token `tskey-api-…`, enviado como token bearer. |

Endereço: `api.tailscale.com`. Opções:

- **Tailnet**: vazio para a tailnet das credenciais, ou o nome da tailnet
  mostrado na consola Tailscale.
- **Devices to keep online**: nomes de dispositivo ou etiquetas separados
  por vírgulas (`nas, tag:router`), ou `all`. Vazio: todo dispositivo
  etiquetado.
- **Timeout per request** (15 s).
- **Device staleness** (`device_stale_days`, 3 dias por predefinição).

## Resolução de problemas {#troubleshooting}

«Tailscale refused the OAuth client»: o ID ou o segredo está errado, ou o
cliente foi eliminado.

«Tailscale refused to list the devices (403)»: o cliente não tem o âmbito
`devices:core:read`.

«Tailscale knows no tailnet …»: deixe a opção Tailnet vazia.
