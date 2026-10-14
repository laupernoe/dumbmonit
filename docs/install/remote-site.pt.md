# Monitorizar um site remoto {#monitor-a-remote-site}

Um DumbMonit, várias redes. Um **agente de retransmissão** instalado no site
remoto executa, em nome do servidor, as sondas dos dispositivos que lhe
atribuir, e reporta os resultados pela mesma ligação de saída que já usa para
enviar as suas próprias métricas. Nada a abrir no site remoto, sem VPN, sem
segunda instância para vigiar.

```
 head office                              remote site (NAT, firewall)
 ┌──────────────────┐   HTTPS, outbound   ┌──────────────────────────────┐
 │ DumbMonit server │◀────────────────────│ relay agent                  │
 │  scheduler       │  probes to run  ──▶ │  runs SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alerting, UI    │◀── samples, verdict │  pushes its own metrics too  │    hypervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Publicar o servidor por HTTPS {#1-publish-the-server-over-https}

O retransmissor obtém os seus trabalhos do servidor: o servidor tem de ser
alcançável a partir do site remoto, tipicamente `https://monitor.example.org`
atrás de um proxy inverso. Duas coisas viajam nessa ligação:

- o **token de registo** (`dmon_…`), no cabeçalho `Authorization` de cada
  pedido — como para qualquer agente;
- as **credenciais dos dispositivos** que o retransmissor sonda (community SNMP,
  token da API Proxmox…), decifradas, dentro de cada trabalho de sonda.

Em HTTP simples pela internet ambos seriam legíveis pelo caminho: ponha o
servidor atrás de HTTPS antes de retransmitir o que quer que seja para fora da
sua própria rede. Numa LAN ou numa VPN, HTTP é aceitável e o token é o único
segredo em trânsito até atribuir dispositivos ao retransmissor. As credenciais
nunca são escritas em disco de nenhum dos lados: um trabalho de sonda vive na
memória do servidor até o retransmissor o reportar, e o agente não guarda nada
entre duas sondas.

O proxy inverso tem de reencaminhar `/api/agent/relay` como `/api/ingest`,
permitir corpos de pedido até 16 MB (um hipervisor ocupado reporta milhares de
amostras), e manter as ligações inativas abertas durante pelo menos 60
segundos: o servidor retém um pedido do retransmissor até 25 segundos enquanto
espera trabalho (long polling).

## 2. Criar um token de registo {#2-create-an-enrollment-token}

**Definições → Agentes → Novo token**, ou adicione um dispositivo do tipo
*agent*. Copie o valor `dmon_…`: é mostrado uma só vez.

## 3. Iniciar o retransmissor no site remoto {#3-start-the-relay-at-the-remote-site}

Qualquer máquina no site remoto com Docker, usando `docker-compose.agent.yml`
do repositório:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

Ou o agente normal instalado com o instalador de uma linha, com mais duas
chaves em `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

A máquina aparece no DumbMonit em segundos, como um dispositivo de agente
normal, e a sua página diz *Relay · site Lyon office · relays 0 devices*. O
registo do agente diz `relay mode enabled: waiting for probes from the server`
assim que o seu primeiro lote é aceite.

!!! note "O que o retransmissor precisa de alcançar"
    O retransmissor sonda dispositivos a partir da sua própria rede: precisa do
    mesmo acesso de que o servidor precisaria se lá estivesse — UDP 161 para
    SNMP, 8006 para Proxmox, 5001 para DSM, e assim por diante. Para monitores
    de **ping**, o contentor precisa do sysctl `net.ipv4.ping_group_range`, já definido
    no ficheiro compose (veja [Ping ICMP sem capability](docker.md#icmp-ping-without-a-capability)). Para dispositivos numa VLAN ou rede link-local, ou para alvos
    `localhost`, use `network_mode: host`.

## 4. Atribuir dispositivos ao retransmissor {#4-assign-devices-to-the-relay}

Adicione os dispositivos remotos como de costume — comutador SNMP, nó Proxmox,
verificação HTTP — com o seu endereço **como visto a partir do site remoto**
(`192.168.10.1`, `https://pve.lan:8006`). No formulário do dispositivo, em
*More options*, defina **Reached through** para o agente de retransmissão em
vez de *Direct*. Os dispositivos existentes podem ser mudados da mesma forma;
voltar a *Direct* faz o servidor sondá-los de novo.

A partir daí:

- o servidor enfileira uma sonda para o dispositivo em cada intervalo; o
  retransmissor apanha-a em milissegundos (espera trabalho no servidor),
  executa o coletor e reporta;
- **Sondar agora** e a deteção de perfil na página do dispositivo passam também
  pelo retransmissor;
- a *última sonda* e o erro do dispositivo seguem o veredicto do retransmissor.
  Quando o retransmissor não apanha uma sonda a tempo, o dispositivo mostra
  *Timed out: relay agent … did not pick up the probe* — verifique que o agente
  corre com `relay: true`;
- a página do agente mostra *Relay for N devices*, e `GET /api/relays` lista os
  retransmissores com o seu site e contagem.

## Alertas quando o retransmissor está em baixo {#alerting-when-the-relay-is-down}

Um retransmissor que fica em silêncio leva consigo os seus dispositivos: as
sondas param, e todos ficariam *inacessíveis* de uma vez. O retransmissor é
tratado como **pai** de todos os dispositivos que sonda (além do pai que possa
ter definido): quando o próprio retransmissor está *inacessível*, os alertas dos
seus dispositivos são mostrados como `suppressed` com `suppressed_by` = o
retransmissor, e sai apenas uma notificação — a do retransmissor. O pai que
definir explicitamente mantém o seu papel e prevalece quando ambos estão em
baixo.

## Limites {#limits}

- Um retransmissor por dispositivo. Um dispositivo sondado pelo retransmissor
  não é também sondado pelo servidor.
- Os próprios dispositivos de agente não podem ser retransmitidos: enviam as
  suas próprias métricas.
- O retransmissor executa no máximo 8 sondas de cada vez; uma sonda é limitada
  pelo tempo limite de sonda do servidor (`DUMBMONIT_PROBE_TIMEOUT_SECS`),
  limitado a 120 s no agente.
- Um agente de retransmissão precisa dos coletores compilados: agentes
  anteriores a esta funcionalidade ignoram `relay: true`, e o servidor reporta
  as suas sondas como expiradas. Reinstale com o binário ou imagem atual.
