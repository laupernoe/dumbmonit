# Monitorar um site remoto {#monitor-a-remote-site}

Um DumbMonit, várias redes. Um **agente de retransmissão** instalado no site remoto
executa, em nome do servidor, as sondagens dos dispositivos que você atribui a ele, e
relata os resultados pela mesma conexão de saída que já usa para enviar
suas próprias métricas. Nada a abrir no site remoto, sem VPN, sem uma segunda
instância para consultar.

```
 escritório principal                    site remoto (NAT, firewall)
 ┌──────────────────┐   HTTPS, saída    ┌──────────────────────────────┐
 │ Servidor DumbMonit │◀────────────────────│ agente de retransmissão       │
 │  agendador       │  sondagens para executar  ──▶ │  executa SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alertas, UI     │◀── amostras, veredicto │  envia suas próprias métricas também  │    hipervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Publicar o servidor sobre HTTPS {#1-publish-the-server-over-https}

O agente de retransmissão busca seus trabalhos do servidor: o servidor deve ser acessível a partir do
site remoto, normalmente `https://monitor.example.org` atrás de um proxy reverso. Duas coisas viajam nesse link:

- o **token de inscrição** (`dmon_…`), no cabeçalho `Authorization` de cada
  solicitação — como para qualquer agente;
- as **credenciais de dispositivo** dos dispositivos que o agente de retransmissão sonda (comunidade SNMP,
  token API Proxmox…), descriptografadas, dentro de cada trabalho de sonda.

Sobre HTTP simples através da internet, ambas seriam legíveis no caminho: coloque o
servidor atrás de HTTPS antes de retransmitir nada fora de sua própria rede. Em uma
LAN ou VPN, HTTP é aceitável e o token é o único segredo em trânsito
até que você atribua dispositivos ao agente de retransmissão. As credenciais nunca são escritas em disco
em nenhum lado: um trabalho de sonda vive na memória do servidor até o agente de retransmissão
relatá-lo, e o agente não mantém nada entre duas sondas.

O proxy reverso deve encaminhar `/api/agent/relay` como `/api/ingest`, permitir
corpos de solicitação até 16 MB (um hipervisor ocupado relata milhares de amostras),
e manter conexões ociosas abertas por pelo menos 60 segundos: o servidor mantém uma
solicitação de agente de retransmissão por até 25 segundos enquanto aguarda trabalho (enquete longa).

## 2. Criar um token de inscrição {#2-create-an-enrollment-token}

**Configurações → Agentes → Novo token**, ou adicione um dispositivo do tipo *agente*. Copie o
valor `dmon_…`: é mostrado uma única vez.

## 3. Iniciar o agente de retransmissão no site remoto {#3-start-the-relay-at-the-remote-site}

Qualquer máquina no site remoto com Docker, usando `docker-compose.agent.yml`
do repositório:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Escritório Lyon" \
docker compose -f docker-compose.agent.yml up -d
```

Ou o agente regular instalado com o instalador de uma linha, com duas chaves adicionais
em `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Escritório Lyon
```

A máquina aparece em DumbMonit em segundos, como um dispositivo de agente comum,
e sua página diz *Retransmissão · site Escritório Lyon · retransmite 0 dispositivos*. O
log do agente diz `relay mode enabled: waiting for probes from the server` uma vez que seu
primeiro lote é aceito.

!!! note "O que o agente de retransmissão precisa alcançar"
    O agente de retransmissão sonda dispositivos de sua própria rede: ele precisa do mesmo acesso
    que o servidor precisaria se estivesse lá — UDP 161 para SNMP, 8006 para
    Proxmox, 5001 para DSM, e assim por diante. Para sondagens de **ping**, o contêiner
    precisa de `cap_add: [NET_RAW]` (consulte o arquivo compose). Para dispositivos em uma VLAN
    ou rede link-local, ou para alvos `localhost`, use
    `network_mode: host`.

## 4. Atribuir dispositivos ao agente de retransmissão {#4-assign-devices-to-the-relay}

Adicione os dispositivos remotos como usual — switch SNMP, nó Proxmox, verificação HTTP —
com seu endereço **visto do site remoto** (`192.168.10.1`,
`https://pve.lan:8006`). No formulário do dispositivo, em *Mais opções*, defina
**Alcançado através de** para o agente de retransmissão em vez de *Direto*. Dispositivos existentes
podem ser alternados da mesma forma; alternar de volta para *Direto* faz o servidor
consultá-los novamente.

De então em diante:

- o servidor enfileira uma sonda para o dispositivo em cada intervalo; o agente de retransmissão
  a pega dentro de milissegundos (aguarda no servidor para trabalho), executa o
  coletor e relata;
- **Sondar agora** e detecção de perfil na página do dispositivo também passam pelo
  agente de retransmissão;
- a *última sonda* e erro do dispositivo seguem o veredicto do agente de retransmissão. Quando o
  agente de retransmissão não pega uma sonda no tempo, o dispositivo mostra *Tempo esgotado: agente de retransmissão
  … não pegou a sonda* — verifique se o agente está executando com
  `relay: true`;
- a página do agente mostra *Retransmissão para N dispositivos*, e `GET /api/relays` lista
  os agentes de retransmissão com seu site e contagem.

## Alertar quando o agente de retransmissão está inativo {#alerting-when-the-relay-is-down}

Um agente de retransmissão que fica silencioso leva seus dispositivos com ele: suas sondas param, e
todas se tornariam *inacessíveis* de uma vez. O agente de retransmissão é tratado como um
**pai** de cada dispositivo que sonda (além do pai que você pode ter
definido): quando o agente de retransmissão está *inacessível*, os alertas de seus dispositivos são
mostrados como `suppressed` com `suppressed_by` = o agente de retransmissão, e apenas uma
notificação sai — para o agente de retransmissão. O pai que você definiu explicitamente mantém seu
papel e vence quando ambos estão desativados.

## Limites {#limits}

- Um agente de retransmissão por dispositivo. Um dispositivo sondado pelo agente de retransmissão não é também sondado pelo
  servidor.
- Dispositivos de agente em si não podem ser retransmitidos: eles enviam suas próprias métricas.
- O agente de retransmissão executa no máximo 8 sondas por vez; uma sonda é limitada pelo
  tempo limite de sonda do servidor (`DUMBMONIT_PROBE_TIMEOUT_SECS`), limitado a 120 s no
  agente.
- Um agente de retransmissão precisa dos coletores compilados: agentes mais antigos que este
  recurso ignoram `relay: true`, e o servidor relata suas sondas como expiradas.
  Reinstale com o binário ou imagem atual.
