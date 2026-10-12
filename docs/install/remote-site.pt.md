# Monitorizar um site remoto {#monitor-a-remote-site}

Um DumbMonit, várias redes. Um **agente relay** instalado no site
remoto executa, em nome do servidor, as verificações dos dispositivos
que lhe atribui, e reporta os resultados através da mesma ligação de
saída que já usa para enviar as suas próprias métricas. Nada para abrir
no site remoto, nenhuma VPN, nenhuma segunda instância a vigiar.

```
 head office                              remote site (NAT, firewall)
 ┌──────────────────┐   HTTPS, outbound   ┌──────────────────────────────┐
 │ DumbMonit server │◀────────────────────│ relay agent                  │
 │  scheduler       │  probes to run  ──▶ │  runs SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alerting, UI    │◀── samples, verdict │  pushes its own metrics too  │    hypervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Publicar o servidor via HTTPS {#1-publish-the-server-over-https}

O relay obtém as suas tarefas do servidor: o servidor tem de estar
acessível a partir do site remoto, tipicamente
`https://monitor.example.org` atrás de um proxy inverso. Duas coisas
viajam nessa ligação:

- o **token de registo** (`dmon_…`), no cabeçalho `Authorization` de
  cada pedido — como para qualquer agente;
- as **credenciais de dispositivo** dos dispositivos que o relay
  sonda (comunidade SNMP, token de API Proxmox…), desencriptadas,
  dentro de cada tarefa de verificação.

Via HTTP simples através da internet ambas seriam legíveis pelo
caminho: coloque o servidor atrás de HTTPS antes de fazer relay de
qualquer coisa fora da sua própria rede. Numa LAN ou VPN, HTTP é
aceitável e o token é o único segredo em trânsito até atribuir
dispositivos ao relay. As credenciais nunca são escritas em disco em
nenhum dos lados: uma tarefa de verificação vive na memória do
servidor até o relay a reportar, e o agente não mantém nada entre duas
verificações.

O proxy inverso tem de encaminhar `/api/agent/relay` como
`/api/ingest`, permitir corpos de pedido até 16 MB (um hipervisor
ocupado reporta milhares de amostras), e manter ligações inativas
abertas durante pelo menos 60 segundos: o servidor retém um pedido de
relay até 25 segundos enquanto espera por trabalho (long polling).

## 2. Criar um token de registo {#2-create-an-enrollment-token}

**Settings → Agents → New token**, ou adicione um dispositivo do tipo
*agent*. Copie o valor `dmon_…`: é mostrado uma vez.

## 3. Iniciar o relay no site remoto {#3-start-the-relay-at-the-remote-site}

Qualquer máquina no site remoto com Docker, usando
`docker-compose.agent.yml` do repositório:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

Ou o agente comum instalado com o instalador de uma linha, com mais
duas chaves em `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

A máquina aparece no DumbMonit dentro de segundos, como um
dispositivo de agente comum, e a sua página diz *Relay · site Lyon
office · relays 0 devices*. O registo do agente diz `relay mode
enabled: waiting for probes from the server` uma vez o seu primeiro
lote aceite.

!!! note "O que o relay precisa de alcançar"
    O relay sonda dispositivos a partir da sua própria rede: precisa
    do mesmo acesso que o servidor precisaria se estivesse lá — UDP
    161 para SNMP, 8006 para Proxmox, 5001 para DSM, e assim por
    diante. Para monitores **ping**, o contentor precisa de
    `cap_add: [NET_RAW]` (ver o ficheiro compose). Para dispositivos
    numa VLAN ou rede link-local, ou para targets `localhost`, use
    `network_mode: host`.

## 4. Atribuir dispositivos ao relay {#4-assign-devices-to-the-relay}

Adicione os dispositivos remotos normalmente — comutador SNMP, nó
Proxmox, verificação HTTP — com o seu endereço **como visto a partir
do site remoto** (`192.168.10.1`, `https://pve.lan:8006`). No
formulário do dispositivo, em *More options*, defina **Reached
through** para o agente relay em vez de *Direct*. Os dispositivos
existentes podem ser mudados da mesma forma; mudar de volta para
*Direct* faz o servidor voltar a sondá-los.

A partir daí:

- o servidor enfilera uma verificação para o dispositivo a cada
  intervalo; o relay recolhe-a dentro de milissegundos (espera no
  servidor por trabalho), executa o coletor e reporta;
- **Probe now** e a deteção de perfil na página do dispositivo também
  passam pelo relay;
- a *última verificação* e erro do dispositivo seguem o veredicto do
  relay. Quando o relay não recolhe uma verificação a tempo, o
  dispositivo mostra *Timed out: relay agent … did not pick up the
  probe* — verifique que o agente corre com `relay: true`;
- a página do agente mostra *Relay for N devices*, e
  `GET /api/relays` lista os relays com o seu site e contagem.

## Alertar quando o relay está em baixo {#alerting-when-the-relay-is-down}

Um relay que fica silencioso leva os seus dispositivos consigo: as
suas verificações param, e todos se tornariam *unreachable* de uma
vez. O relay é tratado como um **pai** de todo dispositivo que sonda
(além do pai que possa ter definido): quando o próprio relay está
*unreachable*, os alertas dos seus dispositivos são mostrados como
`suppressed` com `suppressed_by` = o relay, e só uma notificação sai —
para o relay. O pai que definiu explicitamente mantém o seu papel e
ganha quando ambos estão em baixo.

## Limites {#limits}

- Um relay por dispositivo. Um dispositivo sondado pelo relay não é
  também sondado pelo servidor.
- Os próprios dispositivos de agente não podem ser relayed: enviam as
  suas próprias métricas.
- O relay executa no máximo 8 verificações de cada vez; uma
  verificação é limitada pelo tempo limite de verificação do servidor
  (`DUMBMONIT_PROBE_TIMEOUT`), limitado a 120 s no agente.
- Um agente relay precisa dos coletores compilados: agentes mais
  antigos do que esta funcionalidade ignoram `relay: true`, e o
  servidor reporta as suas verificações como esgotadas. Reinstale com
  o binário ou imagem atual.
