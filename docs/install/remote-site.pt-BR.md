# Monitorar um site remoto {#monitor-a-remote-site}

Um DumbMonit, várias redes. Um **agente de retransmissão** instalado no site remoto
executa, em nome do servidor, as sondagens dos monitores que você atribui a ele, e
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
- as **credenciais de monitor** dos monitores que o agente de retransmissão sonda (comunidade SNMP,
  token API Proxmox…), descriptografadas, dentro de cada trabalho de sonda.

Sobre HTTP simples através da internet, ambas seriam legíveis no caminho: coloque o
servidor atrás de HTTPS antes de retransmitir nada fora de sua própria rede. Em uma
LAN ou VPN, HTTP é aceitável e o token é o único segredo em trânsito
até que você atribua monitores ao agente de retransmissão. As credenciais nunca são escritas em disco
em nenhum lado: um trabalho de sonda vive na memória do servidor até o agente de retransmissão
relatá-lo, e o agente não mantém nada entre duas sondas.

O proxy reverso deve encaminhar `/api/agent/relay` como `/api/ingest`, permitir
corpos de solicitação até 16 MB (um hipervisor ocupado relata milhares de amostras),
e manter conexões ociosas abertas por pelo menos 60 segundos: o servidor mantém uma
solicitação de agente de retransmissão por até 25 segundos enquanto aguarda trabalho (enquete longa).

## 2. Criar um token de inscrição {#2-create-an-enrollment-token}

Adicione um monitoramento, escolha **Servidor com o agente** e marque *Este agente é um relay para
um site remoto* (ou abra a entrada *Relay* da página de adição). Dê um nome ao site e crie o
token: ele leva o modo relay e o site, então não há nada para editar na máquina. Copie o valor
`dmon_…`: ele é mostrado só uma vez; a página então espera o primeiro envio.

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

A máquina aparece em DumbMonit em segundos, como um monitor de agente comum,
e sua página diz *Retransmissão · site Escritório Lyon · retransmite 0 monitores*. O
log do agente diz `relay mode enabled: waiting for probes from the server` uma vez que seu
primeiro lote é aceito.

!!! note "O que o agente de retransmissão precisa alcançar"
    O agente de retransmissão sonda monitores de sua própria rede: ele precisa do mesmo acesso
    que o servidor precisaria se estivesse lá — UDP 161 para SNMP, 8006 para
    Proxmox, 5001 para DSM, e assim por diante. Para sondagens de **ping**, o contêiner
    precisa do sysctl `net.ipv4.ping_group_range`, já definido no arquivo
    compose (consulte [Ping ICMP sem capability](docker.md#icmp-ping-without-a-capability)). Para monitores em uma VLAN
    ou rede link-local, ou para alvos `localhost`, use
    `network_mode: host`.

## 4. Atribuir monitores ao agente de retransmissão {#4-assign-devices-to-the-relay}

Adicione os monitores remotos como usual — switch SNMP, nó Proxmox, verificação HTTP —
com seu endereço **visto do site remoto** (`192.168.10.1`,
`https://pve.lan:8006`). No formulário do monitor, em *Mais opções*, defina
**Alcançado através de** para o agente de retransmissão em vez de *Direto*. Monitores existentes
podem ser alternados da mesma forma; alternar de volta para *Direto* faz o servidor
consultá-los novamente.

De então em diante:

- o servidor enfileira uma sonda para o monitor em cada intervalo; o agente de retransmissão
  a pega dentro de milissegundos (aguarda no servidor para trabalho), executa o
  coletor e relata;
- **Sondar agora** e detecção de perfil na página do monitor também passam pelo
  agente de retransmissão;
- a *última sonda* e erro do monitor seguem o veredicto do agente de retransmissão. Quando o
  agente de retransmissão não pega uma sonda no tempo, o monitor mostra *Tempo esgotado: agente de retransmissão
  … não pegou a sonda* — verifique se o agente está executando com
  `relay: true`;
- a página do agente mostra *Retransmissão para N monitores*, e `GET /api/relays` lista
  os agentes de retransmissão com seu site e contagem.

## 5. Instalar agentes atrás do relay {#5-install-agents-behind-the-relay}

Um site atrás de um firewall muitas vezes tem servidores que não conseguem chegar ao DumbMonit:
só o relay sai. Esses servidores ainda podem rodar o agente completo (sistema, discos, serviços,
contêineres, backups): eles se conectam ao **gateway** do relay, e o relay leva a conexão até o
servidor.

```
 remote site (only the relay goes out)                     head office
 ┌────────────┐  TLS 1.3, mTLS   ┌──────────────┐  tunnel inside the  ┌──────────────────┐
 │ agent      │═════════════════▶│ relay agent  │═ relay's own mTLS ═▶│ DumbMonit server │
 │ (server A) │  to the relay's  │  gateway     │  connection, TCP    │  agent port 8443 │
 └────────────┘  port 8443       └──────────────┘  8443 outbound      └──────────────────┘
        └──────────── one TLS session, end to end, agent ⇄ server ─────────────┘
```

**No DumbMonit**, adicione um monitoramento, escolha **Servidor com o agente** e, em *Como a
máquina chega a este servidor?*, escolha **Por meio de &lt;relay&gt; (site)**. Informe o endereço
pelo qual o site chega ao relay (um IP ou um nome local). Os comandos mostrados — Linux/macOS,
Windows, Docker — passam a apontar para o gateway do relay
(`--url=http://192.168.10.5:8443 --tls-url=https://192.168.10.5:8443`) e continuam levando a
impressão digital da autoridade certificadora do **servidor**. O fluxo então espera a primeira
conexão e deixa você nomear a máquina, colocá-la em uma pasta e em um local sem sair da página.
A partir de um site em *Adicionar por site*, **Servidor com o agente** pré-seleciona o relay
desse site; a página do próprio relay tem **Instalar um agente por meio dele**.

Escolher um relay para uma instalação pede que ele abra o gateway (na porta 8443 por padrão) no
próximo envio. O painel *Relay* da página do relay mostra o gateway — *Escutando na porta 8443 —
3 agentes conectados* — e permite mudar a porta e o endereço, ou fechá-lo. Abra essa porta no
firewall do próprio relay, **só para a rede do site**. Em um relay Docker, publique a porta
(`-p 8443:8443`, ou os `ports:` comentados de `docker-compose.agent.yml`).
`gateway_listen: 0.0.0.0:9443` no `agent.yaml` do relay (`DUMBMONIT_AGENT_GATEWAY_LISTEN`) tem
prioridade sobre a configuração na interface.

O que passa pelo relay:

- **O canal criptografado do agente**, intocado. O relay reconhece um ClientHello TLS e copia os
  bytes, nos dois sentidos, para um túnel que ele abre até a porta de agentes do servidor com seu
  *próprio* certificado de cliente (um upgrade HTTP em `/api/agent/relay/tunnel`). O servidor faz
  então o handshake TLS 1.3 com o agente do site como se ele tivesse se conectado diretamente: o
  certificado de cliente do agente, a autoridade fixada, o token e o segredo de vínculo são
  verificados pelo servidor, de ponta a ponta. O cadastro segue o mesmo caminho.
- **O instalador e o binário**, que o relay busca no servidor pelo próprio canal e serve na mesma
  porta (só `/install.sh`, `/install.ps1`, `/download/<arquivo>`). O comando de instalação leva o
  SHA-256 de cada binário: o script recusa qualquer outro.

O servidor registra, para cada agente, o relay pelo qual ele se conecta. A página do agente diz
*Chega a este servidor por meio do relay …*; o mapa de topologia desenha o relay como pai dele; e
quando o relay cai, os agentes atrás dele aparecem como `suppressed` pelo relay, não como várias
quedas separadas. Um agente que depois se conecta diretamente (ou por outro relay) é atualizado
no envio seguinte.

O kit do Active Directory oferece a mesma escolha: em **Route**, escolha o relay, e os servidores
do domínio baixam o agente dele e reportam por meio dele.

## Alertar quando o agente de retransmissão está inativo {#alerting-when-the-relay-is-down}

Um agente de retransmissão que fica silencioso leva seus monitores com ele: suas sondas param, e
todas se tornariam *inacessíveis* de uma vez. O agente de retransmissão é tratado como um
**pai** de cada monitor que sonda (além do pai que você pode ter
definido): quando o agente de retransmissão está *inacessível*, os alertas de seus monitores são
mostrados como `suppressed` com `suppressed_by` = o agente de retransmissão, e apenas uma
notificação sai — para o agente de retransmissão. O pai que você definiu explicitamente mantém seu
papel e vence quando ambos estão desativados.

## Limites {#limits}

- Um agente de retransmissão por monitor. Um monitor sondado pelo agente de retransmissão não é também sondado pelo
  servidor.
- Monitoramentos de agente nunca são sondados por meio de um relay (eles enviam as próprias
  métricas); em vez disso, um agente pode se *conectar* pelo gateway de um relay (veja acima). Um
  relay sempre se conecta diretamente: não existe relay atrás de relay.
- O gateway de um relay aceita por padrão 64 conexões ao mesmo tempo e 4 MiB/s por túnel e
  sentido (`gateway_max_connections`, `gateway_rate_kib`); o servidor aceita 256 túneis por
  relay.
- O agente de retransmissão executa no máximo 8 sondas por vez; uma sonda é limitada pelo
  tempo limite de sonda do servidor (`DUMBMONIT_PROBE_TIMEOUT_SECS`), limitado a 120 s no
  agente.
- Um agente de retransmissão precisa dos coletores compilados: agentes mais antigos que este
  recurso ignoram `relay: true`, e o servidor relata suas sondas como expiradas.
  Reinstale com o binário ou imagem atual.
