# OPNsense {#opnsense}

Firewall e router de código aberto: o estado de cada gateway com o seu
tempo de ida e volta e perda de pacotes, a tabela de estados do pf,
contadores e endereços de interface, túneis VPN, atribuições DHCP, o
resolvedor, serviços, CARP e atualizações pendentes.

!!! warning "Testado apenas contra uma instância de teste"

    Esta integração foi construída e verificada contra o OPNsense 26.1, a
    imagem oficial iniciada numa máquina virtual para esse fim, com a
    chave de API restrita descrita abaixo — não contra uma firewall que
    encaminha o tráfego de uma casa há anos. Toda chamada é uma leitura e
    toda forma vem desse OPNsense real, mas os seus gateways não tinham
    endereço de monitorização, não fazia parte de um par CARP e não
    executava nenhum túnel OpenVPN ou IPsec: essas partes seguem o
    código-fonte do OPNsense em vez de uma resposta observada. Diga-nos o
    que falha.

## A página do monitor {#the-device-page}

Além dos gráficos genéricos, um monitor OPNsense mostra três painéis,
lidos do que a verificação guardou — abrir a página nunca consulta a
própria firewall:

* **WAN and gateways** — o endereço público atual de cada interface que
  transporta um gateway, depois cada gateway com o seu estado numa
  palavra, o seu atraso de ida e volta e a sua perda de pacotes. Os
  gateways em baixo vêm primeiro. Este é o painel para o qual a integração
  existe: numa firewall multi-WAN, a ligação de reserva assume
  silenciosamente e nada mais na rede muda.
* **Traffic and state table** — quantas ligações o pf está a acompanhar
  contra o seu limite configurado, depois cada interface com o seu estado
  de ligação, os seus endereços, os seus contadores de bytes e pacotes, e
  os seus erros e descartes quando existem. Depois as atribuições DHCP,
  contadas por servidor.
* **Firewall health** — serviços parados primeiro, depois os túneis VPN
  com o número de pares ligados, depois CARP quando a firewall é metade
  de um par, depois o firmware (versão em execução, o que o espelho
  oferece, pacotes em espera, e se um reinício está pendente), depois o
  resolvedor, depois a máquina: tempo de atividade, carga, memória, swap,
  buffers de rede, partições e sensores de temperatura.

Só contagens, nunca tráfego. O DumbMonit não lê nenhuma regra de firewall,
nenhum estado de ligação, nenhum endereço de cliente e nenhum nome de
anfitrião. As atribuições DHCP são contadas, nunca listadas; as sessões
VPN são contadas, nunca nomeadas.

## «none» significa «online» {#none-means-online}

`GET /api/routes/gateway/status` escreve `"none"` no campo `status` de um
gateway quando **tudo está bem** — uma palavra que isoladamente se lê como
«sem informação». O DumbMonit traduz-a para `online`, para que a página,
as séries e as regras digam a mesma coisa que a própria interface do
OPNsense. `down` e `force_down` são as falhas; `loss` e `delay` são os
avisos do dpinger, e o gateway continua a responder.

A mesma chamada devolve o atraso, perda e desvio padrão como **cadeias com
a sua unidade** (`"8.4 ms"`, `"0.0 %"`), e como `"~"` quando o gateway não
tem endereço de monitorização. `"~"` produz uma série a menos, nunca um
zero: um gateway que ninguém faz ping não tem zero milissegundos de
latência, não tem nenhuma.

## A verificação de atualização nunca é acionada {#the-update-check-is-never-triggered}

`GET /api/core/firmware/status` devolve o resultado da *última*
verificação da firewall — a que o próprio painel do OPNsense mostra. O
DumbMonit nunca envia a forma `POST` dessa chamada, nem
`/api/core/firmware/check`, que enviaria a firewall ao espelho em cada
medição, a partir de cada instalação DumbMonit.

Uma firewall que nunca verificou — ou acabou de instalar uma atualização,
o que limpa a cache — responde `"status": "none"`, exatamente como uma
atualizada. O DumbMonit distingue as duas, e até uma verificação ter
corrido publica `firmware_checked = 0` e nada mais: nenhum «zero updates
pending», nenhum «no reboot needed». Execute uma verificação uma vez em
System → Firmware, ou deixe o próprio calendário da firewall fazê-lo.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_opnsense_`.

| Família | Métricas | Etiquetas |
|---|---|---|
| Identidade | `version_info` (as etiquetas transportam a versão do OPNsense, a versão do FreeBSD e a linha de produto) | |
| Máquina | `uptime_seconds`, `load1/5/15`, `cpu_count`, `memory_used/total_bytes`, `memory_used_percent` (a memória inclui a cache ZFS, que o FreeBSD conta como usada), `swap_used/total_bytes`, `swap_used_percent` (ausente numa máquina sem swap), `mbuf_used` e `mbuf_total` (clusters de buffer de rede em uso contra `kern.ipc.nmbclusters`), `mbuf_used_percent`, `mbuf_failures` (alocações recusadas desde o arranque, um contador) | |
| Armazenamento | `disk_used_bytes`, `disk_total_bytes`, `disk_used_percent` | `device`, `mountpoint` |
| Sensores | `temperature_celsius` | `sensor` |
| Gateways | `gateway_up` (0 só em `down` e `force_down`), `gateway_monitored`, `gateway_delay_seconds`, `gateway_stddev_seconds`, `gateway_loss_percent`, `gateways_total`, `gateways_down` | `gateway`, `address` |
| Interfaces | `interface_up`, `interface_bytes_in/out`, `interface_packets_in/out`, `interface_errors_in/out`, `interface_drops`, `interface_collisions` (contadores, guardados em bruto), `interface_address_info` (o endereço do momento, numa etiqueta) | `interface`, `device`, e `address` para o último |
| Firewall | `pf_states`, `pf_state_limit`, `pf_states_used_percent` | |
| DHCP | `dhcp_leases`, `dhcp_leases_active` | `backend` (`kea`, `isc`, `dnsmasq`) |
| VPN | `vpn_tunnel_up`, `vpn_peers_total`, `vpn_peers_connected` (WireGuard: handshake dentro de três minutos), `vpn_handshake_age_seconds` (WireGuard, o par mais recente; ausente enquanto nenhum par nunca se ligou), `vpn_bytes_in/out`, `vpn_tunnels_down` | `kind` (`wireguard`, `openvpn`, `ipsec`), `tunnel` (o monitor WireGuard, `wg0`, ou a descrição IPsec e OpenVPN) |
| Resolvedor | `unbound_running`, `unbound_queries` (um contador), `unbound_cache_hit_percent` | |
| Serviços | `service_running` (1 ou 0, uma série por serviço que a firewall lista) | `service` |
| CARP | `carp_enabled`, `carp_maintenance_mode`, `carp_vip_master` (todas ausentes numa firewall que não faz parte de um par) | `interface`, `vhid`, `address` |
| Firmware | `firmware_checked`, depois — só depois de uma verificação ter corrido — `firmware_updates_pending`, `firmware_upgrade_available`, `firmware_reboot_required`, `firmware_connection_ok` | |
| Coleta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Os contadores de interface são guardados em bruto, como contadores:
envolva-os em `rate()` para obter o caudal. Fazê-lo assim significa que um
reinício da firewall se mostra como uma lacuna, não como um pico de vários
gigabytes por segundo.

`gateway_up` é 1 para um gateway que não é monitorizado de todo, porque
não há nada que diga o contrário; `gateway_monitored` é o que distingue os
dois. Um gateway sem endereço de monitorização não produz nem uma série de
atraso nem de perda.

Uma firewall que não faz parte de um par de alta disponibilidade não tem
endereço virtual CARP, e nenhuma série CARP de todo. Essa é uma firewall
sem cluster, não um cluster degradado: nenhuma regra pode disparar.

WireGuard, OpenVPN, IPsec, Unbound e Kea são plugins. Uma firewall que não
tem um responde `404`, que não é nem um erro de coleta nem um erro de
autenticação: uma métrica a menos, e nada na interface.

As regras incorporadas que se aplicam: Monitor unreachable, Internet
gateway down, Gateway losing packets, Gateway slow, Firewall state table
filling up, Firewall network buffers exhausted, VPN tunnel down, Firewall
resolver stopped, Firewall core service stopped, Firewall too hot,
Firewall left in CARP maintenance mode, Firewall reboot pending, Firewall
updates pending. As notificações nomeiam o gateway, túnel, serviço ou
sensor em causa.

## O que preparar no OPNsense {#what-to-prepare-in-opnsense}

Os passos abaixo são os que o aviso junto ao formulário mostra. O
princípio: um grupo só com os privilégios de estado que o DumbMonit
precisa, um utilizador que nunca inicia sessão, e uma chave de API que não
pertence a nenhuma das suas próprias contas. Este grupo exato foi
verificado contra cada chamada que o DumbMonit faz, no OPNsense 26.1.

1. Na interface web: System → Access → Groups → Add. Dê o seguinte nome ao grupo.

    ```
    dumbmonit
    ```

2. Marque estes privilégios no grupo, exatamente como o OPNsense os nomeia: Lobby: Dashboard (sistema, memória, discos, temperaturas, tabela de estados), System: Gateways, Status: Interfaces, Status: Services, System: Firmware, Interfaces: Virtual IPs: Status (CARP) e Services: Unbound (MVC). Depois um por funcionalidade que executa: Services: DHCP: Kea(v4) ou Services: Dnsmasq DNS/DHCP: Settings para as atribuições, VPN: WireGuard: Status, Status: OpenVPN, Status: IPsec. Um privilégio que deixe de fora custa exatamente as métricas que transporta, nada mais.

3. System → Access → Users → Add. Dê o seguinte nome à conta, deixe o OPNsense gerar uma senha embaralhada (esta conta nunca inicia sessão: responde com a sua chave), e torne-a membro do grupo dumbmonit.

    ```
    dumbmonit
    ```

4. Edite esse utilizador de novo e, em API keys, clique em +. O OPNsense descarrega um pequeno ficheiro de texto com duas linhas, key= e secret=, e mostra o segredo só esta vez. Copie a chave para o campo API key do DumbMonit e o segredo para o seu campo API secret.

5. No DumbMonit, introduza o endereço da firewall, por exemplo «192.168.1.1» ou «fw.lan:8443». Nunca se pede à firewall para verificar atualizações: o DumbMonit lê o resultado da verificação que o OPNsense executa no seu próprio calendário, ou que inicia em System → Firmware.

!!! warning

    Não reutilize a conta com que inicia sessão: uma chave de API é uma senha que nunca expira. O OPNsense não tem uma variante só de leitura de alguns privilégios acima: Status: Services também permite iniciar e parar um serviço, System: Firmware também permite iniciar uma atualização, e os privilégios Unbound, Kea e Dnsmasq também cobrem as suas definições. O DumbMonit só lê, mas para limitar o que uma chave exposta poderia fazer, restrinja as redes de origem do grupo ao endereço do anfitrião do DumbMonit. Nunca conceda All pages. O OPNsense também vem com um certificado autoassinado: se a ligação for recusada por esse motivo, marque «Accept an unverifiable certificate» nas opções.

Endereço: `192.168.1.1`, `fw.lan`, `fw.lan:8443`, `[fd00::1]` ou um URL
completo. O esquema `https` e a porta 443 são adicionados se estiverem em
falta.

Alguns privilégios cobrem mais do que uma chamada. `Lobby: Dashboard`
transporta os endpoints de informação do sistema, hora, memória, disco,
swap, buffer de rede e temperatura e a tabela de estados; `Status:
Interfaces` transporta a vista geral de interfaces com os seus contadores
e endereços. Um privilégio que deixe de fora custa exatamente as métricas
que transporta: a verificação continua a ter sucesso, e `scrape_errors`
fica em zero, porque um `403` numa chamada opcional é lido como «esta
conta não pode ver isso», não como uma falha. Só `Lobby: Dashboard` é
obrigatório: sem ele a verificação nem consegue ler o nome e a versão da
firewall, e reporta um erro de autenticação.

### camelCase ou snake_case {#camelcase-or-snake_case}

O OPNsense 25.7 renomeou os seus caminhos de API de `systemResources` para
`system_resources`. Ambas as grafias continuam a responder, mas os
privilégios comparam o caminho letra a letra: no 25.7 e posterior uma
chave restrita recebe `403` na grafia antiga, e no 25.1 em algumas das
novas. O DumbMonit pede primeiro a grafia atual e recua para a outra num
`403`, pelo que o mesmo grupo funciona em ambos os lados da renomeação.

## Opções {#options}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `scheme` | Protocol | `https` | HTTPS adequa-se a uma firewall de fábrica. HTTP só se a interface web for servida em texto simples. |
| `port` | Web interface port | `443` | Usada se o endereço não der uma porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | O OPNsense vem com um certificado autoassinado por predefinição. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tempo permitido para cada chamada de API, de 1 a 120. |
| `gateways` | Watch the gateways | `true` | Lê o que o dpinger diz sobre cada gateway: ativo ou em baixo, atraso de ida e volta e perda de pacotes. |
| `interfaces` | Watch the interfaces | `true` | Lê o estado de ligação, endereços e contadores de bytes, pacotes, erros e descartes de cada interface. |
| `firewall` | Watch the state table | `true` | Lê quantas ligações o pf está a acompanhar e o limite configurado. |
| `dhcp` | Count the DHCP leases | `true` | Conta as atribuições que a firewall está a distribuir, qualquer que seja o servidor que executa. Só contagens. |
| `vpn` | Watch the VPN tunnels | `true` | Lê WireGuard, OpenVPN e IPsec: quais túneis existem e quais estão ativos. |
| `unbound` | Watch the resolver | `true` | Lê se o Unbound está a responder. |
| `services` | Watch the services | `true` | Lê a lista de serviços que a firewall gere e quais estão em execução. |
| `carp` | Watch CARP | `true` | Lê os endereços virtuais de um par de alta disponibilidade, e o modo de manutenção persistente. |
| `firmware` | Watch for updates | `true` | Lê o resultado da última verificação de atualização da firewall e se um reinício está pendente. |
| `temperature` | Read the temperature sensors | `true` | Lê os sensores de CPU e placa que a firewall expõe. |

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Erro de autenticação | A chave e o segredo estão trocados, ou a chave foi eliminada do utilizador. O OPNsense mostra o segredo só uma vez: se foi perdido, crie outra chave e elimine a antiga. |
| Uma página web em vez de JSON | A chave de API é recusada, ou a conta não pode alcançar esse endpoint. O OPNsense serve então a sua página de início de sessão com um `200`; o DumbMonit reporta-o como um problema de autenticação em vez de uma resposta ilegível. |
| Erro de certificado | Certificado autoassinado: instale um de confiança (ACME é um plugin) ou ative `insecure_tls`. |
| Nenhuma métrica de gateway | A opção `gateways` está desligada, ou o grupo não tem `System: Gateways`. Uma firewall sem nenhum gateway definido genuinamente não tem nenhuma. |
| Um gateway não mostra atraso nem perda | Não tem endereço de monitorização, por isso o dpinger não lhe faz ping. O OPNsense escreve `~` e o DumbMonit não publica nenhuma série em vez de um zero. |
| «Gateway down» dispara para uma ligação que funciona | O endereço de monitorização está inacessível ainda que a ligação esteja ativa — um caso comum com um ISP que descarta ICMP. Defina um endereço de monitorização diferente em System → Gateways. |
| Nenhuma métrica da tabela de estados | O grupo não tem `Lobby: Dashboard`. |
| Erro de autenticação numa chave que funciona numa ferramenta de navegador | A ferramenta usou a outra grafia do caminho. Com uma chave restrita, o OPNsense 25.7 e posterior respondem `403` a `systemInformation` e `200` a `system_information`. |
| Nenhuma métrica de atualização exceto `firmware_checked` | A firewall nunca verificou atualizações, ou acabou de instalar uma. Execute uma verificação em System → Firmware. |
| Nenhuma métrica VPN | O plugin não está instalado, ou nenhum túnel está configurado. Uma ligação IPsec desativada não é deliberadamente reportada como em baixo. |
| «Firewall resolver stopped» numa firewall que resolve bem | O Unbound está instalado mas desligado porque a firewall resolve com outra coisa. Desligue essa regra. |
| A versão do firmware parece desatualizada | A firewall não verificou o espelho recentemente. O DumbMonit lê a última verificação, nunca aciona uma. |
| `scrape_errors` acima de zero | Uma chamada opcional falhou por um motivo que não um privilégio em falta ou um plugin em falta — um tempo limite, geralmente. As outras métricas não são afetadas. |
