# OPNsense {#opnsense}

Firewall e roteador de código aberto: o estado de cada gateway com seu
tempo de ida e volta e perda de pacotes, a tabela de estados pf,
contadores e endereços de interface, túneis VPN, leases DHCP, o resolver,
serviços, CARP e atualizações pendentes.

!!! warning "Testado apenas contra uma instância de teste"

    Esta integração foi construída e verificada contra o OPNsense 26.1, a
    imagem oficial iniciada em uma máquina virtual para esse fim, com a
    chave de API restrita descrita abaixo — não contra um firewall que
    roteia o tráfego de uma casa há anos. Toda chamada é uma leitura e
    toda forma vem desse OPNsense real, mas seus gateways não tinham
    endereço de monitor, ele não fazia parte de um par CARP e não
    executava nenhum túnel OpenVPN ou IPsec: essas partes seguem o
    código-fonte do OPNsense em vez de uma resposta observada. Avise-nos
    o que não funcionar.

## A página do monitor {#the-device-page}

Além dos gráficos genéricos, um monitor OPNsense mostra três painéis,
lidos do que a sondagem armazenou — abrir a página nunca consulta o
próprio firewall:

* **WAN e gateways** — o endereço público atual de toda interface que
  carrega um gateway, depois cada gateway com seu estado em uma palavra,
  seu atraso de ida e volta e sua perda de pacotes. Gateways fora do ar
  vêm primeiro. Este é o painel pelo qual a integração existe: em um
  firewall multi-WAN, o link de backup assume silenciosamente e nada
  mais na rede muda.
* **Tráfego e tabela de estados** — quantas conexões o pf está
  rastreando contra seu limite configurado, depois cada interface com seu
  estado de link, seus endereços, seus contadores de bytes e pacotes, e
  seus erros e descartes quando há algum. Depois os leases DHCP,
  contados por servidor.
* **Saúde do firewall** — serviços parados primeiro, depois os túneis
  VPN com o número de pares conectados, depois o CARP quando o firewall
  é metade de um par, depois o firmware (versão em execução, o que o
  mirror oferece, pacotes esperando, e se um reinício está pendente),
  depois o resolver, depois a máquina: tempo de atividade, carga,
  memória, swap, buffers de rede, partições e sensores de temperatura.

Apenas contagens, nunca tráfego. O DumbMonit não lê nenhuma regra de
firewall, nenhum estado de conexão, nenhum endereço de cliente e nenhum
nome de host. Leases DHCP são contados, nunca listados; sessões VPN são
contadas, nunca nomeadas.

## "none" significa "online" {#none-means-online}

`GET /api/routes/gateway/status` escreve `"none"` no campo `status` de um
gateway quando **tudo está bem** — uma palavra que por si só parece "sem
informação". O DumbMonit a traduz para `online`, então a página, as
séries e as regras dizem a mesma coisa que a própria interface do
OPNsense. `down` e `force_down` são as falhas; `loss` e `delay` são
avisos do dpinger, e o gateway ainda responde.

A mesma chamada retorna atraso, perda e desvio padrão como **strings com
sua unidade** (`"8.4 ms"`, `"0.0 %"`), e como `"~"` quando o gateway não
tem endereço de monitor. `"~"` produz uma série menos, nunca um zero: um
gateway que ninguém pinga não tem zero milissegundos de latência, não tem
nenhuma.

## A verificação de atualização nunca é acionada {#the-update-check-is-never-triggered}

`GET /api/core/firmware/status` retorna o resultado da *última*
verificação do firewall — a que o próprio painel do OPNsense mostra. O
DumbMonit nunca envia a forma `POST` dessa chamada, nem
`/api/core/firmware/check`, que enviaria o firewall ao mirror a cada
medição, de toda instalação DumbMonit.

Um firewall que nunca verificou — ou acabou de instalar uma atualização,
o que limpa o cache — responde `"status": "none"`, exatamente como um
atualizado faz. O DumbMonit distingue os dois, e até uma verificação
rodar ele publica `firmware_checked = 0` e nada mais: nenhum "zero
atualizações pendentes", nenhum "nenhum reinício necessário". Execute uma
verificação uma vez em System → Firmware, ou deixe a programação própria
do firewall fazer isso.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_opnsense_`.

| Família | Métricas | Rótulos |
|---|---|---|
| Identidade | `version_info` (rótulos carregam a versão do OPNsense, a versão do FreeBSD e a linha de produto) | |
| Máquina | `uptime_seconds`, `load1/5/15`, `cpu_count`, `memory_used/total_bytes`, `memory_used_percent` (memória inclui o cache ZFS, que o FreeBSD conta como usado), `swap_used/total_bytes`, `swap_used_percent` (ausente em uma máquina sem swap), `mbuf_used` e `mbuf_total` (clusters de buffer de rede em uso contra `kern.ipc.nmbclusters`), `mbuf_used_percent`, `mbuf_failures` (alocações recusadas desde o boot, um contador) | |
| Armazenamento | `disk_used_bytes`, `disk_total_bytes`, `disk_used_percent` | `device`, `mountpoint` |
| Sensores | `temperature_celsius` | `sensor` |
| Gateways | `gateway_up` (0 apenas em `down` e `force_down`), `gateway_monitored`, `gateway_delay_seconds`, `gateway_stddev_seconds`, `gateway_loss_percent`, `gateways_total`, `gateways_down` | `gateway`, `address` |
| Interfaces | `interface_up`, `interface_bytes_in/out`, `interface_packets_in/out`, `interface_errors_in/out`, `interface_drops`, `interface_collisions` (contadores, armazenados em bruto), `interface_address_info` (o endereço do momento, em um rótulo) | `interface`, `device`, e `address` para o último |
| Firewall | `pf_states`, `pf_state_limit`, `pf_states_used_percent` | |
| DHCP | `dhcp_leases`, `dhcp_leases_active` | `backend` (`kea`, `isc`, `dnsmasq`) |
| VPN | `vpn_tunnel_up`, `vpn_peers_total`, `vpn_peers_connected` (WireGuard: handshake dentro de três minutos), `vpn_handshake_age_seconds` (WireGuard, o par mais recente; ausente enquanto nenhum par jamais se conectou), `vpn_bytes_in/out`, `vpn_tunnels_down` | `kind` (`wireguard`, `openvpn`, `ipsec`), `tunnel` (o monitor WireGuard, `wg0`, ou a descrição IPsec e OpenVPN) |
| Resolver | `unbound_running`, `unbound_queries` (um contador), `unbound_cache_hit_percent` | |
| Serviços | `service_running` (1 ou 0, uma série por serviço que o firewall lista) | `service` |
| CARP | `carp_enabled`, `carp_maintenance_mode`, `carp_vip_master` (todos ausentes em um firewall que não faz parte de um par) | `interface`, `vhid`, `address` |
| Firmware | `firmware_checked`, depois — só uma vez que uma verificação rodou — `firmware_updates_pending`, `firmware_upgrade_available`, `firmware_reboot_required`, `firmware_connection_ok` | |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Contadores de interface são armazenados em bruto, como contadores:
envolva-os em `rate()` para obter vazão. Fazer isso assim significa que um
reinício de firewall aparece como uma lacuna, não como um pico de vários
gigabytes por segundo.

`gateway_up` é 1 para um gateway que não é monitorado de forma alguma,
porque não há nada a dizer o contrário; `gateway_monitored` é o que
distingue os dois. Um gateway sem endereço de monitor não produz nem
série de atraso nem de perda.

Um firewall que não faz parte de um par de alta disponibilidade não tem
endereço virtual CARP, e nenhuma série CARP de forma alguma. Isso é um
firewall sem cluster, não um cluster degradado: nenhuma regra pode
disparar.

WireGuard, OpenVPN, IPsec, Unbound e Kea são plugins. Um firewall que não
tem um responde `404`, que não é nem um erro de coleta nem um erro de
autenticação: uma métrica menos, e nada na interface.

Regras integradas que se aplicam: Monitor unreachable, Internet gateway
down, Gateway losing packets, Gateway slow, Firewall state table filling
up, Firewall network buffers exhausted, VPN tunnel down, Firewall resolver
stopped, Firewall core service stopped, Firewall too hot, Firewall left
in CARP maintenance mode, Firewall reboot pending, Firewall updates
pending. Notificações nomeiam o gateway, túnel, serviço ou sensor
envolvido.

## O que preparar no OPNsense {#what-to-prepare-in-opnsense}

Os passos abaixo são os que o aviso ao lado do formulário mostra. O
princípio: um grupo apenas com os privilégios de status que o DumbMonit
precisa, um usuário que nunca entra, e uma chave de API que não pertence
a nenhuma de suas próprias contas. Esse grupo exato foi verificado contra
toda chamada que o DumbMonit faz, no OPNsense 26.1.

1. Na interface web: System → Access → Groups → Add. Nomeie o grupo como
   abaixo.

    ```
    dumbmonit
    ```

2. Marque estes privilégios no grupo, exatamente como o OPNsense os
   nomeia: Lobby: Dashboard (sistema, memória, discos, temperaturas,
   tabela de estados), System: Gateways, Status: Interfaces, Status:
   Services, System: Firmware, Interfaces: Virtual IPs: Status (CARP) e
   Services: Unbound (MVC). Depois um por recurso que você executa:
   Services: DHCP: Kea(v4) ou Services: Dnsmasq DNS/DHCP: Settings para
   os leases, VPN: WireGuard: Status, Status: OpenVPN, Status: IPsec. Um
   privilégio que você deixa de fora custa exatamente as métricas que
   carrega, nada mais.

3. System → Access → Users → Add. Nomeie a conta como abaixo, deixe o
   OPNsense gerar uma senha embaralhada (essa conta nunca entra: ela
   responde com sua chave), e a torne membro do grupo dumbmonit.

    ```
    dumbmonit
    ```

4. Edite esse usuário de novo e, sob API keys, clique em +. O OPNsense
   baixa um pequeno arquivo de texto com duas linhas, key= e secret=, e
   mostra o secret apenas esta vez. Copie a key para o campo chave de API
   do DumbMonit e o secret para seu campo segredo de API.

5. No DumbMonit, informe o endereço do firewall, por exemplo
   "192.168.1.1" ou "fw.lan:8443". O firewall nunca é solicitado a
   verificar atualizações: o DumbMonit lê o resultado da verificação que
   o OPNsense roda em sua própria programação, ou que você inicia de
   System → Firmware.

!!! warning

    Não reutilize a conta com a qual você entra: uma chave de API é uma
    senha que nunca expira. O OPNsense não tem variante somente leitura
    de alguns privilégios acima: Status: Services também permite iniciar
    e parar um serviço, System: Firmware também permite iniciar uma
    atualização, e os privilégios Unbound, Kea e Dnsmasq também cobrem
    suas configurações. O DumbMonit sempre só lê, mas para limitar o que
    uma chave exposta poderia fazer, restrinja as redes de origem do
    grupo ao endereço do host DumbMonit. Nunca conceda All pages. O
    OPNsense também vem com um certificado autoassinado: se a conexão for
    recusada por esse motivo, marque "Aceitar um certificado não
    verificável" nas opções.

Endereço: `192.168.1.1`, `fw.lan`, `fw.lan:8443`, `[fd00::1]` ou uma URL
completa. O esquema `https` e a porta 443 são adicionados se faltando.

Alguns privilégios cobrem mais de uma chamada. `Lobby: Dashboard` carrega
os endpoints de informação do sistema, hora, memória, disco, swap,
buffer de rede e temperatura e a tabela de estados; `Status: Interfaces`
carrega a visão geral de interfaces com seus contadores e endereços. Um
privilégio que você deixa de fora custa exatamente as métricas que
carrega: a sondagem ainda tem sucesso, e `scrape_errors` permanece em
zero, porque um `403` em uma chamada opcional é lido como "esta conta não
pode ver isso", não como uma falha. Só `Lobby: Dashboard` é obrigatório:
sem ele a sondagem não pode nem ler o nome e versão do firewall, e
reporta um erro de autenticação.

### camelCase ou snake_case {#camelcase-or-snake_case}

O OPNsense 25.7 renomeou seus caminhos de API de `systemResources` para
`system_resources`. Ambas as grafias ainda respondem, mas os privilégios
comparam o caminho letra por letra: na 25.7 e posteriores uma chave
restrita recebe `403` na grafia antiga, e na 25.1 em algumas das novas. O
DumbMonit pede a grafia atual primeiro e recorre à outra em um `403`,
então o mesmo grupo funciona em qualquer lado da renomeação.

## Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `scheme` | Protocolo | `https` | HTTPS se encaixa em um firewall pronto para uso. HTTP apenas se a interface web é servida em texto claro. |
| `port` | Porta da interface web | `443` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | O OPNsense vem com um certificado autoassinado por padrão. |
| `request_timeout_seconds` | Tempo limite por requisição (segundos) | `15` | Tempo permitido para cada chamada de API, de 1 a 120. |
| `gateways` | Observar os gateways | `true` | Lê o que o dpinger diz sobre cada gateway: ativo ou fora do ar, atraso de ida e volta e perda de pacotes. |
| `interfaces` | Observar as interfaces | `true` | Lê o estado de link, endereços e contadores de bytes, pacotes, erros e descartes de cada interface. |
| `firewall` | Observar a tabela de estados | `true` | Lê quantas conexões o pf está rastreando e o limite configurado. |
| `dhcp` | Contar os leases DHCP | `true` | Conta os leases que o firewall está distribuindo, qualquer que seja o servidor que executa. Apenas contagens. |
| `vpn` | Observar os túneis VPN | `true` | Lê WireGuard, OpenVPN e IPsec: quais túneis existem e quais estão ativos. |
| `unbound` | Observar o resolver | `true` | Lê se o Unbound está respondendo. |
| `services` | Observar os serviços | `true` | Lê a lista de serviços que o firewall gerencia e quais estão rodando. |
| `carp` | Observar o CARP | `true` | Lê os endereços virtuais de um par de alta disponibilidade, e o modo de manutenção persistente. |
| `firmware` | Observar atualizações | `true` | Lê o resultado da última verificação de atualização do firewall e se um reinício está pendente. |
| `temperature` | Ler os sensores de temperatura | `true` | Lê os sensores de CPU e placa que o firewall expõe. |

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de autenticação | A key e o secret estão trocados, ou a key foi excluída do usuário. O OPNsense mostra o secret apenas uma vez: se foi perdido, crie outra key e exclua a antiga. |
| Uma página web em vez de JSON | A chave de API é recusada, ou a conta não pode alcançar aquele endpoint. O OPNsense então serve sua página de login com um `200`; o DumbMonit a reporta como um problema de autenticação em vez de uma resposta ilegível. |
| Erro de certificado | Certificado autoassinado: instale um confiável (ACME é um plugin) ou ative `insecure_tls`. |
| Nenhuma métrica de gateway de forma alguma | A opção `gateways` está desligada, ou o grupo não tem `System: Gateways`. Um firewall sem gateway definido genuinamente não tem nenhum. |
| Um gateway não mostra atraso nem perda | Ele não tem endereço de monitor, então o dpinger não o pinga. O OPNsense escreve `~` e o DumbMonit não publica série em vez de um zero. |
| "Gateway down" dispara para um link que funciona | O endereço de monitor é inacessível mesmo com o link ativo — um caso comum com um ISP que descarta ICMP. Defina um endereço de monitor diferente em System → Gateways. |
| Nenhuma métrica de tabela de estados | O grupo não tem `Lobby: Dashboard`. |
| Erro de autenticação em uma chave que funciona em uma ferramenta de navegador | A ferramenta usou a outra grafia do caminho. Com uma chave restrita, o OPNsense 25.7 e posteriores respondem `403` para `systemInformation` e `200` para `system_information`. |
| Nenhuma métrica de atualização exceto `firmware_checked` | O firewall nunca verificou atualizações, ou acabou de instalar uma. Execute uma verificação de System → Firmware. |
| Nenhuma métrica de VPN | O plugin não está instalado, ou nenhum túnel está configurado. Uma conexão IPsec desativada deliberadamente não é reportada como fora do ar. |
| "Firewall resolver stopped" em um firewall que resolve bem | O Unbound está instalado mas desligado porque o firewall resolve com algo mais. Desligue essa regra. |
| A versão do firmware parece desatualizada | O firewall não verificou o mirror recentemente. O DumbMonit lê a última verificação, ele nunca aciona uma. |
| `scrape_errors` acima de zero | Uma chamada opcional falhou por um motivo que não é um privilégio faltando ou um plugin faltando — um tempo esgotado, geralmente. As outras métricas não são afetadas. |
