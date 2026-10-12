# Home Assistant

A casa inteligente por trás do painel: se o Home Assistant roda ou iniciou
em modo de recuperação, entidades que estão indisponíveis ou desconhecidas
por domínio, baterias baixas, atualizações esperando (core, sistema
operacional, add-ons, firmware de dispositivo) e os reparos abertos.

O DumbMonit lê a API REST do Home Assistant com o token de acesso de longa
duração de um usuário dedicado **sem direitos de administrador**, e a lista
de reparos via API WebSocket. Ele nunca chama um serviço, nunca dispara um
evento e nunca altera um estado.

A falha que vale a pena observar não é "o Home Assistant não responde" (uma
[verificação de site](services.md#http) já vê isso) mas **o Home Assistant
responde e algo detrás dele parou de funcionar**: uma integração cujas
entidades todas ficaram indisponíveis depois de uma atualização, um stick
Zigbee que caiu do barramento USB, um sensor de porta cuja bateria chegou a
5%, um reparo esperando há semanas.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_homeassistant_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `info` | valor 1 | `version` |
| `running` | 1 quando o estado do core é `RUNNING`, 0 enquanto inicia ou para | |
| `recovery_mode` | 1 em modo de recuperação (ou modo seguro em versões mais antigas): a configuração não pôde ser carregada | |
| `entities` | entidades por domínio | `domain` |
| `entities_unavailable`, `entities_unknown` | entidades nesse estado, por domínio | `domain` |
| `entity_unavailable` | valor 1 por entidade indisponível, no máximo 50 | `entity`, `name`, `domain` |
| `batteries` | sensores de bateria e sensores binários de bateria | |
| `batteries_low` | baterias abaixo do limite, e sensores binários de bateria que estão ativos | |
| `battery_low` | uma série por bateria baixa, valor = seu nível em porcentagem (0 para um sensor binário), no máximo 50 | `entity`, `name` |
| `updates_available` | entidades `update.*` que estão ativas | |
| `update_available` | valor 1 por atualização esperando, no máximo 50 | `entity`, `name`, `installed`, `latest` |
| `repairs` | reparos abertos (nem ignorados nem descartados) por severidade: `critical`, `error`, `warning` | `severity` |
| `repair` | valor 1 por reparo aberto, no máximo 50 | `issue`, `domain`, `severity` |
| `scrape_errors` | chamadas que falharam na última sondagem | |

Alguns domínios permanecem no estado `unknown` por natureza: um botão nunca
pressionado, um evento nunca recebido, um serviço de notificação ou fala.
Eles não são contados como unknown: `button`, `input_button`, `event`,
`scene`, `notify`, `tts`, `stt`, `conversation`, `wake_word`, `ai_task`,
`person`. A opção **Domínios deixados de fora** remove outros domínios (por
exemplo `device_tracker` ou `media_player` para dispositivos que
frequentemente estão desligados) das contagens de indisponível e
desconhecido.

Reparos não estão na API REST: o DumbMonit abre a API WebSocket, envia
`repairs/list_issues` e a fecha. Um usuário sem direitos de administrador
pode listá-los. A opção **Ler os reparos** desativa isso.

As [regras integradas](../alerting/rules.md#home-assistant) que se aplicam:
Home Assistant in recovery mode, Home Assistant not running, Home Assistant
entities went unavailable, Home Assistant battery low, Home Assistant
update available, Home Assistant repair to address, além de Device
unreachable.

## Dispositivos

Todo celular e tablet conectado ao aplicativo companion (a integração
`mobile_app`) é publicado sob a família genérica compartilhada por toda
integração do DumbMonit que rastreia dispositivos clientes —
`dumbmonit_client_device_*`, com prefixo sem `homeassistant`, documentada
completamente em [Dispositivos clientes](../alerting/rules.md#client-devices):

| Métrica | O que é | Rótulos |
|---|---|---|
| `last_seen_timestamp_seconds` | Última atualização da entidade `device_tracker` do dispositivo | `device`, `type` (`Mobile`), `os` (vazio), `user` (a pessoa a quem está vinculado, se houver), `kind` |
| `stale_seconds` | Positivo uma vez que um dispositivo passou `device_stale_days` (3 por padrão) sem uma atualização; aciona a regra [Dispositivo cliente desatualizado](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: o Home Assistant não tem backup para reportar) |

**Como um dispositivo é reconhecido**: `/api/states` não diz qual
integração criou uma entidade — isso precisa do registro de entidades, que
a API REST não serve. O `device_tracker` do aplicativo companion é o único
que define um atributo `battery_level`; integrações de localização de
terceiros (Life360, OwnTracks, a integração de demonstração usada em
testes) definem no máximo `battery`. Uma integração de terceiros que por
acaso copiasse esse nome exato de atributo seria confundida com o
aplicativo companion; inversamente, um dispositivo do aplicativo companion
com relato de bateria desligado fica faltando na tabela em vez de mostrado
incorretamente. A conta do dispositivo, quando conhecida, vem de uma
entidade `person.*` cujo atributo `device_trackers` o nomeia.

## A página do dispositivo

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Home Assistant. Ele diz primeiro o que está errado (modo
de recuperação, reparos de severidade de erro, entidades indisponíveis),
depois as baterias baixas pelo nível, as atualizações esperando com suas
versões, as entidades por domínio, e a tabela de dispositivos do aplicativo
companion com sua última conexão.

## Como foi validado

Contra o Home Assistant 2026.9.4, com o token de longa duração de um
usuário sem direitos de administrador: configuração, estados de entidade da
integração de demonstração e de sensores de template (níveis de bateria, um
sensor binário de bateria, um sensor indisponível), entidades de atualização
e a lista de reparos via API WebSocket. Os atributos `device_tracker` do
aplicativo companion (`battery_level`, `source_type`) foram verificados
contra o código-fonte da integração `mobile_app`, não contra um celular
real.

## Criar um usuário dedicado e um token de longa duração no Home Assistant

1. No Home Assistant: Settings → People → Users → Add user (ative o modo
   Avançado no seu perfil se a aba Users estiver faltando). Nomeie o
   usuário como abaixo, dê a ele uma senha longa e aleatória e deixe o
   botão Administrator desligado. Um usuário comum pode ler toda entidade e
   a lista de reparos, e não pode alterar a configuração.

    ```
    dumbmonit
    ```

2. Entre no Home Assistant uma vez como esse usuário, abra seu perfil (o
   nome na parte inferior da barra lateral) → Security → Long-lived access
   tokens → Create token. Nomeie-o após o DumbMonit e copie-o agora: ele é
   mostrado apenas uma vez.

3. No DumbMonit, informe o endereço do Home Assistant, por exemplo
   "homeassistant.local" (porta 8123) ou "https://ha.example.net" detrás
   de um proxy reverso, e cole o token. Baterias abaixo de 20% contam como
   baixas; a opção Limite de bateria baixa muda isso.

4. O DumbMonit só lê /api/config, /api/states e, via API WebSocket, a
   lista de reparos. Ele nunca chama um serviço, nunca dispara um evento e
   nunca altera um estado.

!!! warning
    Um token de longa duração carrega todos os direitos de seu usuário e
    permanece válido por dez anos: crie-o para o usuário dedicado, nunca
    para o seu próprio. O Home Assistant serve HTTP simples a menos que TLS
    esteja configurado, e o token viaja em cada requisição: em uma rede não
    confiável, use HTTPS. Todo celular e tablet conectado ao aplicativo
    companion (a integração mobile_app) aparece na tabela Dispositivos
    abaixo com sua última atividade; é opcional e silencioso sem esse
    aplicativo.

## Credenciais

| Credencial | Campos |
|---|---|
| Token de acesso de longa duração | Um token do usuário dedicado, enviado como `Authorization: Bearer`. |

Endereço: um nome de host ou IP (`homeassistant.local`), `host:porta`, ou
uma URL completa (`https://ha.example.net`, com um prefixo de caminho
detrás de um proxy reverso); um `/api` final é aceito. Opções: protocolo
(HTTP por padrão), porta (8123), verificação de certificado, tempo limite
da requisição (10 s), limite de bateria baixa (20%), domínios deixados de
fora (nenhum), reparos (lido) e desatualização de dispositivo
(`device_stale_days`, 3 dias por padrão).
