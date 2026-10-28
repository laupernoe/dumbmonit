# Monitorizar um site remoto {#monitor-a-remote-site}

Um DumbMonit, várias redes. Um **agente de retransmissão** instalado no site
remoto executa, em nome do servidor, as sondas dos monitores que lhe
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
- as **credenciais dos monitores** que o retransmissor sonda (community SNMP,
  token da API Proxmox…), decifradas, dentro de cada trabalho de sonda.

Em HTTP simples pela internet ambos seriam legíveis pelo caminho: ponha o
servidor atrás de HTTPS antes de retransmitir o que quer que seja para fora da
sua própria rede. Numa LAN ou numa VPN, HTTP é aceitável e o token é o único
segredo em trânsito até atribuir monitores ao retransmissor. As credenciais
nunca são escritas em disco de nenhum dos lados: um trabalho de sonda vive na
memória do servidor até o retransmissor o reportar, e o agente não guarda nada
entre duas sondas.

O proxy inverso tem de reencaminhar `/api/agent/relay` como `/api/ingest`,
permitir corpos de pedido até 16 MB (um hipervisor ocupado reporta milhares de
amostras), e manter as ligações inativas abertas durante pelo menos 60
segundos: o servidor retém um pedido do retransmissor até 25 segundos enquanto
espera trabalho (long polling).

## 2. Criar um token de registo {#2-create-an-enrollment-token}

Adicione uma monitorização, escolha **Servidor com o agente** e marque *Este agente é um relé para
um site remoto* (ou abra a entrada *Relay* da página de adição). Dê um nome ao site e crie o token:
leva o modo relé e o site, por isso não há nada a editar na máquina. Copie o valor `dmon_…`: só é
mostrado uma vez; a página espera depois pelo primeiro envio.

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

A máquina aparece no DumbMonit em segundos, como um monitor de agente
normal, e a sua página diz *Relay · site Lyon office · relays 0 devices*. O
registo do agente diz `relay mode enabled: waiting for probes from the server`
assim que o seu primeiro lote é aceite.

!!! note "O que o retransmissor precisa de alcançar"
    O retransmissor sonda monitores a partir da sua própria rede: precisa do
    mesmo acesso de que o servidor precisaria se lá estivesse — UDP 161 para
    SNMP, 8006 para Proxmox, 5001 para DSM, e assim por diante. Para monitores
    de **ping**, o contentor precisa do sysctl `net.ipv4.ping_group_range`, já definido
    no ficheiro compose (veja [Ping ICMP sem capability](docker.md#icmp-ping-without-a-capability)). Para monitores numa VLAN ou rede link-local, ou para alvos
    `localhost`, use `network_mode: host`.

## 4. Atribuir monitores ao retransmissor {#4-assign-devices-to-the-relay}

Adicione os monitores remotos como de costume — comutador SNMP, nó Proxmox,
verificação HTTP — com o seu endereço **como visto a partir do site remoto**
(`192.168.10.1`, `https://pve.lan:8006`). No formulário do monitor, em
*More options*, defina **Reached through** para o agente de retransmissão em
vez de *Direct*. Os monitores existentes podem ser mudados da mesma forma;
voltar a *Direct* faz o servidor sondá-los de novo.

A partir daí:

- o servidor enfileira uma sonda para o monitor em cada intervalo; o
  retransmissor apanha-a em milissegundos (espera trabalho no servidor),
  executa o coletor e reporta;
- **Sondar agora** e a deteção de perfil na página do monitor passam também
  pelo retransmissor;
- a *última sonda* e o erro do monitor seguem o veredicto do retransmissor.
  Quando o retransmissor não apanha uma sonda a tempo, o monitor mostra
  *Timed out: relay agent … did not pick up the probe* — verifique que o agente
  corre com `relay: true`;
- a página do agente mostra *Relay for N devices*, e `GET /api/relays` lista os
  retransmissores com o seu site e contagem.

## 5. Instalar agentes atrás do retransmissor {#5-install-agents-behind-the-relay}

Um site atrás de uma firewall tem muitas vezes servidores que não conseguem chegar ao DumbMonit:
só o retransmissor sai. Esses servidores podem mesmo assim correr o agente completo (sistema,
discos, serviços, contentores, cópias de segurança): ligam-se ao **gateway** do retransmissor, e
este leva a ligação até ao servidor.

```
 remote site (only the relay goes out)                     head office
 ┌────────────┐  TLS 1.3, mTLS   ┌──────────────┐  tunnel inside the  ┌──────────────────┐
 │ agent      │═════════════════▶│ relay agent  │═ relay's own mTLS ═▶│ DumbMonit server │
 │ (server A) │  to the relay's  │  gateway     │  connection, TCP    │  agent port 8443 │
 └────────────┘  port 8443       └──────────────┘  8443 outbound      └──────────────────┘
        └──────────── one TLS session, end to end, agent ⇄ server ─────────────┘
```

**No DumbMonit**, adicione uma monitorização, escolha **Servidor com o agente** e, em *Como chega a
máquina a este servidor?*, escolha **Através de &lt;retransmissor&gt; (site)**. Indique o endereço
pelo qual o site chega ao retransmissor (um IP ou um nome local). Os comandos mostrados —
Linux/macOS, Windows, Docker — apontam então para o gateway do retransmissor
(`--url=http://192.168.10.5:8443 --tls-url=https://192.168.10.5:8443`) e continuam a levar a
impressão digital da autoridade de certificação do **servidor**. O fluxo espera pela primeira
ligação e deixa-o dar nome à máquina, arrumá-la numa pasta e colocá-la num site sem sair da
página. A partir de um site em *Adicionar por site*, **Servidor com o agente** pré-seleciona o
retransmissor desse site; a página do próprio retransmissor tem **Instalar um agente através
dele**.

Escolher um retransmissor para uma instalação pede-lhe que abra o seu gateway (na porta 8443 por
predefinição) no próximo envio. O painel *Relay* da página do retransmissor mostra o gateway — *À
escuta na porta 8443 — 3 agentes ligados* — e permite mudar a porta e o endereço, ou fechá-lo.
Abra essa porta na firewall do próprio retransmissor, **só para a rede do site**. Num
retransmissor Docker, publique a porta (`-p 8443:8443`, ou os `ports:` comentados de
`docker-compose.agent.yml`). `gateway_listen: 0.0.0.0:9443` no `agent.yaml` do retransmissor
(`DUMBMONIT_AGENT_GATEWAY_LISTEN`) prevalece sobre a definição na interface.

O que passa pelo retransmissor:

- **O canal cifrado do agente**, intocado. O retransmissor reconhece um ClientHello TLS e copia os
  bytes, nos dois sentidos, para um túnel que abre até à porta de agentes do servidor com o seu
  *próprio* certificado de cliente (um upgrade HTTP em `/api/agent/relay/tunnel`). O servidor faz
  então o handshake TLS 1.3 com o agente do site como se este se tivesse ligado diretamente: o
  certificado de cliente do agente, a autoridade fixada, o token e o segredo de associação são
  verificados pelo servidor, de ponta a ponta. O registo segue o mesmo caminho.
- **O instalador e o binário**, que o retransmissor vai buscar ao servidor pelo seu próprio canal
  e serve na mesma porta (só `/install.sh`, `/install.ps1`, `/download/<ficheiro>`). O comando de
  instalação leva o SHA-256 de cada binário: o script recusa qualquer outro.

O servidor regista, para cada agente, o retransmissor pelo qual se liga. A página do agente diz
*Chega a este servidor através do retransmissor …*; o mapa de topologia desenha o retransmissor
como seu pai; e quando o retransmissor cai, os agentes atrás dele aparecem como `suppressed` pelo
retransmissor, e não como muitas falhas separadas. Um agente que mais tarde se ligue diretamente
(ou por outro retransmissor) é atualizado no envio seguinte.

O kit Active Directory oferece a mesma escolha: em **Route**, escolha o retransmissor, e os
servidores do domínio descarregam o agente a partir dele e reportam através dele.

## Alertas quando o retransmissor está em baixo {#alerting-when-the-relay-is-down}

Um retransmissor que fica em silêncio leva consigo os seus monitores: as
sondas param, e todos ficariam *inacessíveis* de uma vez. O retransmissor é
tratado como **pai** de todos os monitores que sonda (além do pai que possa
ter definido): quando o próprio retransmissor está *inacessível*, os alertas dos
seus monitores são mostrados como `suppressed` com `suppressed_by` = o
retransmissor, e sai apenas uma notificação — a do retransmissor. O pai que
definir explicitamente mantém o seu papel e prevalece quando ambos estão em
baixo.

## Limites {#limits}

- Um retransmissor por monitor. Um monitor sondado pelo retransmissor
  não é também sondado pelo servidor.
- As monitorizações de agente nunca são sondadas através de um retransmissor (enviam as suas
  próprias métricas); um agente pode, em vez disso, *ligar-se* através do gateway de um
  retransmissor (ver acima). Um retransmissor liga-se sempre diretamente: não há retransmissor
  atrás de um retransmissor.
- O gateway de um retransmissor aceita por predefinição 64 ligações em simultâneo e 4 MiB/s por
  túnel e sentido (`gateway_max_connections`, `gateway_rate_kib`); o servidor aceita 256 túneis
  por retransmissor.
- O retransmissor executa no máximo 8 sondas de cada vez; uma sonda é limitada
  pelo tempo limite de sonda do servidor (`DUMBMONIT_PROBE_TIMEOUT_SECS`),
  limitado a 120 s no agente.
- Um agente de retransmissão precisa dos coletores compilados: agentes
  anteriores a esta funcionalidade ignoram `relay: true`, e o servidor reporta
  as suas sondas como expiradas. Reinstale com o binário ou imagem atual.
