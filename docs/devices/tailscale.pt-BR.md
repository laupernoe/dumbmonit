# Tailscale {#tailscale}

Os dispositivos de uma tailnet através da API do Tailscale: servidores que
ficaram offline, chaves de nó próximas de expirar, dispositivos esperando
aprovação, e clientes com atualização disponível.

O DumbMonit lê `GET /api/v2/tailnet/{tailnet}/devices` em
`api.tailscale.com`, uma vez por minuto. A credencial recomendada é um
cliente OAuth limitado ao escopo `devices:core:read`: o DumbMonit troca seu
ID e segredo por um token de uma hora (`/api/v2/oauth/token`) e o mantém até
expirar. Ele pode listar os dispositivos e não pode alterar nada. Um token
de acesso à API (`tskey-api-…`) também funciona, mas carrega todos os
direitos do usuário que o criou e expira em no máximo 90 dias.

Esta é uma API de internet: o endereço do dispositivo é o da API, não de uma
máquina. O DumbMonit precisa de HTTPS de saída para `api.tailscale.com`;
nada na tailnet é contatado.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir da documentação da API do
    Tailscale e do modelo de dispositivo de seu cliente Go oficial, não
    contra uma tailnet real. Ela só lê. Avise-nos o que não funcionar.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_tailscale_`. Um dispositivo é
nomeado pelo primeiro rótulo de seu nome MagicDNS (`nas` para
`nas.tail1234.ts.net`). Dispositivos compartilhados na tailnet de outra são
contados, não descritos.

| Métrica | Rótulos | Significado |
|---|---|---|
| `device_online` | `device` | 1 enquanto o dispositivo está conectado ao servidor de coordenação. |
| `device_offline_seconds` | `device` | 0 enquanto online, senão o tempo desde que foi visto por último. |
| `device_watched` | `device` | 1 quando o dispositivo deve permanecer online: todo dispositivo marcado por padrão, ou os nomeados na opção abaixo. |
| `device_key_expiry_seconds` | `device` | Até a chave do nó expirar; ausente quando a expiração de chave está desativada. |
| `device_authorized` | `device` | 0 enquanto o dispositivo espera aprovação. |
| `device_update_available` | `device` | 1 quando o Tailscale oferece um cliente mais novo. |
| `device_info` | `device`, `os`, `client_version`, `tags` | Valor 1. |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | Contagens. |

Apenas dispositivos observados geram um alerta quando ficam offline: um
celular ou laptop que dorme não é uma indisponibilidade. Dispositivos
marcados são, na prática, os servidores, roteadores de subrede e nós de
saída; liste outros na opção **Dispositivos a manter online**, ou informe
`all`.

As [regras integradas](../alerting/rules.md#tailscale) que se aplicam:

- **Tailscale device offline**: um dispositivo observado desconectado por
  mais de 15 minutos (Advisory).
- **Tailscale node key expiring**: menos de 14 dias restantes (Advisory,
  lembrada diariamente). Quando expira, o dispositivo deixa a tailnet até
  alguém fazer login nele de novo.
- **Tailscale device awaiting approval** (Advisory): aprove-o se é
  esperado, remova-o caso contrário.
- **Tailscale client update**: um dispositivo observado tem uma
  atualização de cliente há seis horas (Info, lembrada semanalmente).

## Dispositivos {#devices}

Todo dispositivo da tailnet, observado ou não (exceto dispositivos
compartilhados), é publicado sob a família genérica compartilhada por toda
integração do DumbMonit que rastreia dispositivos clientes —
`dumbmonit_client_device_*`, com prefixo sem `tailscale`, documentada
completamente em [Dispositivos clientes](../alerting/rules.md#client-devices):

| Métrica | O que é | Rótulos |
|---|---|---|
| `last_seen_timestamp_seconds` | Última conexão: "agora" enquanto `connectedToControl` é verdadeiro, senão `lastSeen` | `device`, `type` (`Tagged` ou `Personal`), `os`, `user`, `kind` |
| `stale_seconds` | Positivo uma vez que um dispositivo passou `device_stale_days` (3 por padrão) sem se conectar; aciona a regra [Dispositivo cliente desatualizado](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: o Tailscale não tem backup para reportar) |

Um celular ou laptop que viaja pode passar vários dias sem reconectar: isso
não tem relação com a configuração **Dispositivos a manter online** acima,
que só gera alerta para dispositivos observados ficando offline. Aumente
**Desatualização de dispositivo** para uma tailnet com dispositivos que
circulam.

## A página do dispositivo {#the-device-page}

O painel Tailnet diz o que está errado em uma frase, mostra os dispositivos
online, os dispositivos observados offline, as atualizações disponíveis e
os dispositivos compartilhados, depois lista os dispositivos observados e
qualquer dispositivo esperando aprovação ou cuja chave expira em breve,
depois a tabela de todo dispositivo com sua última conexão.

## Criar um cliente OAuth somente leitura para o DumbMonit {#create-a-read-only-oauth-client-for-dumbmonit}

1. No console Tailscale (login.tailscale.com), abra Settings > Trust
   credentials (antigamente OAuth clients) e gere um cliente OAuth apenas
   com o escopo devices:core:read. Ele lista os dispositivos e não pode
   alterar nada.

2. Copie o ID do cliente e o segredo do cliente (tskey-client-…), mostrados
   uma vez. No DumbMonit, informe api.tailscale.com como endereço e cole-os
   como Client ID e Client secret. Deixe a opção Tailnet vazia: a tailnet
   do cliente é lida.

!!! warning
    Apenas dispositivos marcados (servidores, roteadores de subrede, nós de
    saída) devem permanecer online: um celular ou laptop que dorme não é
    uma indisponibilidade. Nomeie os dispositivos a observar na opção
    Dispositivos a manter online, ou informe all. Todo dispositivo da
    tailnet aparece na tabela Dispositivos abaixo com sua última conexão,
    observado ou não; um celular que viaja pode passar dias sem uma, então
    aumente Desatualização de dispositivo para esses.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Cliente OAuth | ID do cliente e segredo do cliente (`tskey-client-…`) de um cliente com o escopo `devices:core:read`. |
| Token de acesso à API | Um token `tskey-api-…`, enviado como token bearer. |

Endereço: `api.tailscale.com`. Opções:

- **Tailnet**: vazio para a tailnet das credenciais, ou o nome da tailnet
  mostrado no console Tailscale.
- **Dispositivos a manter online**: nomes de dispositivo ou tags separados
  por vírgulas (`nas, tag:router`), ou `all`. Vazio: todo dispositivo
  marcado.
- **Tempo limite por requisição** (15 s).
- **Desatualização de dispositivo** (`device_stale_days`, 3 dias por
  padrão).

## Solução de problemas {#troubleshooting}

"Tailscale refused the OAuth client": o ID ou o segredo está errado, ou o
cliente foi excluído.

"Tailscale refused to list the devices (403)": o cliente não tem o escopo
`devices:core:read`.

"Tailscale knows no tailnet …": deixe a opção Tailnet vazia.
