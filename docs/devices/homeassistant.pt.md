# Home Assistant {#home-assistant}

A casa inteligente por detrás do painel: se o Home Assistant corre ou
arrancou em modo de recuperação, entidades indisponíveis ou desconhecidas
por domínio, baterias baixas, atualizações em espera (core, sistema
operativo, add-ons, firmware de dispositivo) e as reparações abertas.

O DumbMonit lê a API REST do Home Assistant com o token de acesso de longa
duração de um utilizador dedicado **sem direitos de administrador**, e a
lista de reparações através da API WebSocket. Nunca chama um serviço, nunca
dispara um evento e nunca altera um estado.

A falha a vigiar não é «o Home Assistant não responde» (uma
[verificação de site](services.md#http) já vê isso) mas **o Home Assistant
responde e algo por detrás dele deixou de funcionar**: uma integração cujas
entidades todas ficaram indisponíveis depois de uma atualização, uma pen
Zigbee que caiu do barramento USB, um sensor de porta cuja bateria chegou a
5%, uma reparação à espera há semanas.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_homeassistant_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `info` | valor 1 | `version` |
| `running` | 1 quando o estado do core é `RUNNING`, 0 enquanto arranca ou para | |
| `recovery_mode` | 1 em modo de recuperação (ou modo seguro em versões mais antigas): a configuração não pôde ser carregada | |
| `entities` | entidades por domínio | `domain` |
| `entities_unavailable`, `entities_unknown` | entidades nesse estado, por domínio | `domain` |
| `entity_unavailable` | valor 1 por entidade indisponível, no máximo 50 | `entity`, `name`, `domain` |
| `batteries` | sensores de bateria e sensores binários de bateria | |
| `batteries_low` | baterias abaixo do limiar, e sensores binários de bateria que estão ativos | |
| `battery_low` | uma série por bateria baixa, valor = o seu nível em percentagem (0 para um sensor binário), no máximo 50 | `entity`, `name` |
| `updates_available` | entidades `update.*` que estão ativas | |
| `update_available` | valor 1 por atualização em espera, no máximo 50 | `entity`, `name`, `installed`, `latest` |
| `repairs` | reparações abertas (nem ignoradas nem dispensadas) por severidade: `critical`, `error`, `warning` | `severity` |
| `repair` | valor 1 por reparação aberta, no máximo 50 | `issue`, `domain`, `severity` |
| `scrape_errors` | chamadas que falharam na última verificação | |

Alguns domínios ficam no estado `unknown` por natureza: um botão nunca
premido, um evento nunca recebido, um serviço de notificação ou de voz. Não
são contados como desconhecidos: `button`, `input_button`, `event`,
`scene`, `notify`, `tts`, `stt`, `conversation`, `wake_word`, `ai_task`,
`person`. A opção **Domains left out** remove mais domínios (por exemplo
`device_tracker` ou `media_player` para dispositivos frequentemente
desligados) das contagens de indisponíveis e desconhecidas.

As reparações não estão na API REST: o DumbMonit abre a API WebSocket,
envia `repairs/list_issues` e fecha-a. Um utilizador sem direitos de
administrador pode listá-las. A opção **Read the repairs** desativa isto.

As [regras incorporadas](../alerting/rules.md#home-assistant) que se
aplicam: Home Assistant in recovery mode, Home Assistant not running, Home
Assistant entities went unavailable, Home Assistant battery low, Home
Assistant update available, Home Assistant repair to address, mais Device
unreachable.

## Dispositivos {#devices}

Todo telefone e tablet com sessão iniciada na app companheira (a
integração `mobile_app`) é publicado sob a família genérica partilhada por
toda integração do DumbMonit que acompanha dispositivos cliente —
`dumbmonit_client_device_*`, prefixada sem `homeassistant`, documentada na
íntegra em [Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | O quê | Etiquetas |
|---|---|---|
| `last_seen_timestamp_seconds` | Última atualização da entidade `device_tracker` do dispositivo | `device`, `type` (`Mobile`), `os` (vazio), `user` (a pessoa a que está ligado, se existir), `kind` |
| `stale_seconds` | Positivo uma vez que um dispositivo passou `device_stale_days` (3 por predefinição) sem uma atualização; alimenta a regra [Dispositivo obsoleto](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: o Home Assistant não tem nenhuma cópia de segurança a reportar) |

**Como um dispositivo é reconhecido**: `/api/states` não diz qual
integração criou uma entidade — isso precisa do registo de entidades, que a
API REST não serve. O `device_tracker` da app companheira é o único que
define um atributo `battery_level`; integrações de localização de terceiros
(Life360, OwnTracks, a integração de demonstração usada em testes) definem
no máximo `battery`. Uma integração de terceiros que por acaso copiasse
esse mesmo nome de atributo seria confundida com a app companheira;
inversamente, um dispositivo da app companheira com o reporte de bateria
desligado fica em falta na tabela em vez de ser mostrado incorretamente. A
conta do dispositivo, quando conhecida, vem de uma entidade `person.*` cujo
atributo `device_trackers` o nomeia.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o Home Assistant. Diz primeiro o que está mal (modo de
recuperação, reparações de severidade de erro, entidades indisponíveis),
depois as baterias baixas pelo nível, as atualizações em espera com as suas
versões, as entidades por domínio, e a tabela de dispositivos da app
companheira com a sua última ligação.

## Como foi validado {#how-it-was-validated}

Contra o Home Assistant 2026.9.4, com o token de longa duração de um
utilizador sem direitos de administrador: configuração, estados de
entidade da integração de demonstração e de sensores de modelo (níveis de
bateria, um sensor binário de bateria, um sensor indisponível), entidades
de atualização e a lista de reparações através da API WebSocket. Os
atributos do `device_tracker` da app companheira (`battery_level`,
`source_type`) foram verificados contra o código-fonte da integração
`mobile_app`, não contra um telefone real.

## Criar um utilizador dedicado e um token de longa duração no Home Assistant {#create-a-dedicated-user-and-a-long-lived-token-in-home-assistant}

1. No Home Assistant: Settings → People → Users → Add user (ative o modo Advanced no seu perfil se o separador Users estiver em falta). Dê o seguinte nome ao utilizador, uma senha longa e aleatória e deixe o interruptor Administrator desligado. Um utilizador comum pode ler toda entidade e a lista de reparações, e não pode alterar a configuração.

    ```
    dumbmonit
    ```

2. Inicie sessão no Home Assistant uma vez como esse utilizador, abra o seu perfil (o nome no fundo da barra lateral) → Security → Long-lived access tokens → Create token. Dê-lhe o nome do DumbMonit e copie-o agora: só é mostrado uma vez.

3. No DumbMonit, introduza o endereço do Home Assistant, por exemplo «homeassistant.local» (porta 8123) ou «https://ha.example.net» atrás de um proxy inverso, e cole o token. Baterias abaixo de 20% contam como baixas; a opção Low battery threshold altera isso.

4. O DumbMonit só lê /api/config, /api/states e, através da API WebSocket, a lista de reparações. Nunca chama um serviço, nunca dispara um evento e nunca altera um estado.

!!! warning
    Um token de longa duração transporta todos os direitos do seu utilizador e permanece válido durante dez anos: crie-o para o utilizador dedicado, nunca para o seu próprio. O Home Assistant serve HTTP simples a menos que o TLS esteja configurado, e o token viaja em cada pedido: numa rede não confiável, use HTTPS. Todo telefone e tablet com sessão iniciada na app companheira (a integração mobile_app) aparece na tabela Devices abaixo com a sua última atividade; é opcional e silencioso sem essa app.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de acesso de longa duração | Um token do utilizador dedicado, enviado como `Authorization: Bearer`. |

Endereço: um nome de anfitrião ou IP (`homeassistant.local`),
`anfitrião:porta`, ou um URL completo (`https://ha.example.net`, com um
prefixo de caminho atrás de um proxy inverso); um `/api` final é aceite.
Opções: protocolo (HTTP por predefinição), porta (8123), verificação de
certificado, tempo limite do pedido (10 s), limiar de bateria baixa (20%),
domínios deixados de fora (nenhum), reparações (lido) e obsolescência de
dispositivo (`device_stale_days`, 3 dias por predefinição).
