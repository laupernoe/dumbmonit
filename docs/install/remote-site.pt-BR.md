# Monitorar um site remoto

Um DumbMonit, várias redes. Um **agente relay** instalado no site remoto
executa, em nome do servidor, as sondagens dos dispositivos que você
atribui a ele, e reporta os resultados pela mesma conexão de saída que já
usa para enviar suas próprias métricas. Nada para abrir no site remoto,
nenhuma VPN, nenhuma segunda instância para observar.

```
 matriz                                    site remoto (NAT, firewall)
 ┌──────────────────┐   HTTPS, saída      ┌──────────────────────────────┐
 │ servidor DumbMonit│◀────────────────────│ agente relay                 │
 │  agendador        │  sondagens a rodar──▶│  roda SNMP / Proxmox / HTTP…│──▶ switch, NAS,
 │  alertas, UI      │◀── amostras, veredito│  também envia suas métricas │    hipervisor…
 └──────────────────┘                      └──────────────────────────────┘
```

## 1. Publicar o servidor via HTTPS

O relay busca seus jobs do servidor: o servidor precisa ser acessível a
partir do site remoto, tipicamente `https://monitor.example.org` detrás de
um proxy reverso. Duas coisas viajam nesse link:

- o **token de cadastro** (`dmon_…`), no cabeçalho `Authorization` de toda
  requisição — como para qualquer agente;
- as **credenciais de dispositivo** dos dispositivos que o relay sonda
  (comunidade SNMP, token de API do Proxmox…), descriptografadas, dentro de
  cada job de sondagem.

Em HTTP simples pela internet, ambos seriam legíveis no caminho: coloque o
servidor detrás de HTTPS antes de fazer relay de qualquer coisa fora da sua
própria rede. Em uma LAN ou VPN, HTTP é aceitável e o token é o único
segredo em trânsito até você atribuir dispositivos ao relay. Credenciais
nunca são escritas em disco em nenhum dos lados: um job de sondagem vive na
memória do servidor até o relay reportá-lo, e o agente não mantém nada
entre duas sondagens.

O proxy reverso precisa encaminhar `/api/agent/relay` como `/api/ingest`,
permitir corpos de requisição de até 16 MB (um hipervisor ocupado reporta
milhares de amostras), e manter conexões ociosas abertas por pelo menos 60
segundos: o servidor mantém uma requisição de relay por até 25 segundos
enquanto espera por trabalho (long polling).

## 2. Criar um token de cadastro

**Configurações → Agentes → Novo token**, ou adicione um dispositivo do
tipo *agent*. Copie o valor `dmon_…`: ele é mostrado uma vez.

## 3. Iniciar o relay no site remoto

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

Ou o agente comum instalado com o instalador de uma linha, com duas chaves
mais em `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

A máquina aparece no DumbMonit em segundos, como um dispositivo de agente
comum, e sua página diz *Relay · site Lyon office · faz relay de 0
dispositivos*. O log do agente diz `relay mode enabled: waiting for probes
from the server` uma vez que seu primeiro lote é aceito.

!!! note "O que o relay precisa alcançar"
    O relay sonda dispositivos a partir de sua própria rede: ele precisa do
    mesmo acesso que o servidor precisaria se estivesse lá — UDP 161 para
    SNMP, 8006 para Proxmox, 5001 para DSM, e assim por diante. Para
    monitores de **ping**, o contêiner precisa de `cap_add: [NET_RAW]`
    (veja o arquivo compose). Para dispositivos em uma VLAN ou rede
    link-local, ou para alvos `localhost`, use `network_mode: host`.

## 4. Atribuir dispositivos ao relay

Adicione os dispositivos remotos como de costume — switch SNMP, nó Proxmox,
verificação HTTP — com seu endereço **como visto do site remoto**
(`192.168.10.1`, `https://pve.lan:8006`). No formulário do dispositivo, sob
*Mais opções*, defina **Alcançado através de** para o agente relay em vez
de *Direto*. Dispositivos existentes podem ser trocados da mesma forma;
trocar de volta para *Direto* faz o servidor sondá-los de novo.

A partir de então:

- o servidor enfileira uma sondagem para o dispositivo em cada intervalo; o
  relay a pega dentro de milissegundos (ele espera no servidor por
  trabalho), executa o coletor e reporta;
- **Sondar agora** e a detecção de perfil na página do dispositivo também
  passam pelo relay;
- a *última sondagem* e erro do dispositivo seguem o veredito do relay.
  Quando o relay não pega uma sondagem a tempo, o dispositivo mostra
  *Tempo esgotado: o agente relay … não pegou a sondagem* — verifique que o
  agente roda com `relay: true`;
- a página do agente mostra *Relay para N dispositivos*, e
  `GET /api/relays` lista os relays com seu site e contagem.

## Alertando quando o relay está fora do ar

Um relay que fica silencioso leva seus dispositivos com ele: suas sondagens
param, e todos ficariam *inacessíveis* de uma vez. O relay é tratado como
um **pai** de todo dispositivo que sonda (além do pai que você pode ter
definido): quando o próprio relay está *inacessível*, os alertas de seus
dispositivos são mostrados como `suppressed` com `suppressed_by` = o relay,
e apenas uma notificação sai — para o relay. O pai que você definiu
explicitamente mantém seu papel e vence quando ambos estão fora do ar.

## Limites

- Um relay por dispositivo. Um dispositivo sondado pelo relay não é também
  sondado pelo servidor.
- Dispositivos de agente em si não podem ser retransmitidos: eles enviam
  suas próprias métricas.
- O relay executa no máximo 8 sondagens por vez; uma sondagem é limitada
  pelo tempo limite de sondagem do servidor (`DUMBMONIT_PROBE_TIMEOUT`),
  limitado a 120 s no agente.
- Um agente relay precisa dos coletores compilados: agentes mais antigos
  que esta funcionalidade ignoram `relay: true`, e o servidor reporta suas
  sondagens como expiradas. Reinstale com o binário ou imagem atual.
