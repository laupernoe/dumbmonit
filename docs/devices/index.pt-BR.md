# Tipos de dispositivo {#device-types}

O DumbMonit chama tudo que observa de *dispositivo* (a API diz *target*),
mesmo quando é uma página web ou um nome DNS. Cada tipo é um coletor no
servidor; o servidor descreve cada tipo para a interface
(`GET /api/collectors`), então o formulário sempre corresponde ao que esta
versão pode monitorar.

| Tipo | `kind` | Credencial | Uso típico |
|---|---|---|---|
| [Dispositivo SNMP](snmp.md) | `snmp` | comunidade ou SNMP v3 | Switch, roteador, NAS, nobreak, impressora |
| [Proxmox VE](proxmox.md) | `proxmox` | token de API ou usuário/senha | Hipervisor, cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | token de API ou usuário/senha | Servidor de backup |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | token de API | Console federando vários clusters PVE e instâncias PBS |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | usuário/senha | Gateway de e-mail filtrando spam e vírus |
| [VMware vSphere](vsphere.md) | `vsphere` | usuário/senha | vCenter ou ESXi autônomo: hosts, VMs, datastores, alarmes |
| [Synology DSM](synology.md) | `synology` | usuário/senha | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | chave e segredo de API | Firewall, roteador, borda multi-WAN |
| [pfSense](pfsense.md) | `pfsense` | chave da API REST (usuário somente leitura) | Firewall: gateways fora do ar ou com perdas, interfaces sem link, serviços parados |
| [FortiGate](fortigate.md) | `fortigate` | token da API REST (perfil somente leitura, host confiável) | Firewall: túneis IPsec, links, sincronização HA, modo de conservação, licenças FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | usuário/senha (perfil somente leitura) | Firewall: apenas links de interface (a API XML não expõe estado de túnel, HA ou licença) |
| [UniFi Network](unifi.md) | `unifi` | chave de API ou usuário/senha | Console UniFi ou servidor autoalojado: dispositivos, WAN, clientes |
| [Home Assistant](homeassistant.md) | `homeassistant` | token de acesso de longa duração | Casa inteligente: entidades indisponíveis, baterias baixas, atualizações, reparos |
| [TrueNAS](truenas.md) | `truenas` | chave de API | Servidor de armazenamento ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | chave de API (papel Viewer) | NAS: array, discos, verificações de paridade, pools de cache, contêineres, VMs |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | usuário/senha (grupo somente leitura) | Roteador ou switch: versões e firmware atrasados, CPU, memória, sensores, erros de interface |
| [UPS com NUT](nut.md) | `nut` | nenhuma, ou usuário/senha | UPS em um NAS, Raspberry Pi ou servidor rodando Network UPS Tools |
| [Hardware de servidor (Redfish)](redfish.md) | `redfish` | usuário/senha | Ventoinhas, temperaturas, fontes de alimentação e discos do servidor, lidos de seu BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | nenhuma, usuário/senha ou token | Banco de dados de séries temporais: ingestão, amostras recusadas, espaço livre em disco |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | nenhuma, usuário/senha ou token | Banco de dados de logs: ingestão, linhas recusadas, espaço livre em disco |
| [Grafana Loki](loki.md) | `loki` | nenhuma, usuário/senha ou token | Servidor de logs: prontidão, linhas recusadas, falhas de flush |
| [Graylog](graylog.md) | `graylog` | token de acesso ou usuário/senha | Servidor de logs e seu cluster OpenSearch: journal, buffers, vazão, entradas |
| [Pi-hole](pihole.md) | `pihole` | senha de app, ou nenhuma | Bloqueador de anúncios DNS: estado de bloqueio, parcela bloqueada, idade da lista de bloqueio, atualizações |
| [AdGuard Home](adguard.md) | `adguard` | usuário/senha | Filtro DNS: proteção, bloqueios, listas de filtro, servidores upstream, atualizações |
| [Servidor de e-mail MDaemon](mdaemon.md) | `mdaemon` | nenhuma, ou endereço de e-mail/senha | Servidor de e-mail: SMTP, IMAP, POP3, webmail, versão |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | nenhuma, ou chave de API | Gateway de e-mail: serviços, fila de entrega e outros contadores |
| [Redis / Valkey](redis.md) | `redis` | usuário ACL, só senha, ou nenhuma | Armazenamento em memória: memória contra maxmemory, replicação, persistência, chaves |
| [MongoDB](mongodb.md) | `mongodb` | usuário/senha (clusterMonitor) ou nenhuma | Banco de dados de documentos: replica set, atraso, conexões, cache WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | usuário/senha (marca monitoring) | Message broker: alarmes de recursos, nós, partições, filas sem consumidor |
| [CrowdSec](crowdsec.md) | `crowdsec` | nenhuma, ou chave de API de bouncer | Motor de segurança: decisões, alertas, bouncers que pararam de buscar, leitura de logs |
| [Traefik](traefik.md) | `traefik` | nenhuma, ou usuário basicAuth | Proxy reverso: roteadores desativados, servidores backend fora do ar, certificados não obtidos ou não renovados, parcela de 5xx |
| [Caddy](caddy.md) | `caddy` | usuário view somente leitura, ou nenhuma | Servidor web e proxy reverso: saúde de upstream, reloads com falha, erros de handler, parcela de 5xx |
| [Nginx](nginx.md) | `nginx` | nenhuma, ou autenticação básica | Servidor web: conexões ativas/em espera, taxas de accept/handled/request, saúde de upstream do NGINX Plus |
| [Apache httpd](apache.md) | `apache` | nenhuma, ou autenticação básica | Servidor web: workers ocupados/ociosos, placar por estado, taxas de requisição e bytes |
| [Nginx Proxy Manager](npm.md) | `npm` | e-mail e senha do usuário (somente visualização) | Gerenciador de proxy: hosts que o nginx recusou, hosts desativados, certificados servidos |
| [Expiração de domínio](domain.md) | `domain` | nenhuma | Domínio registrado via RDAP: dias antes da expiração, retenção, período de resgate, registrador |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | token de conta de serviço (ClusterRole somente leitura) | Cluster: nós não prontos, pods em crash loop ou pendentes, workloads sem réplicas, reivindicações de volume travadas |
| [Active Directory](activedirectory.md) | `activedirectory` | usuário/senha (usuário de domínio simples) | Domínio Windows via LDAPS: controladores de domínio e papéis FSMO, membros de grupos privilegiados, política de senha, achados de segurança |
| [Veeam Backup & Replication](veeam.md) | `veeam` | usuário/senha (Veeam Backup Viewer) | Servidor de backup: jobs com falha, sessões com falha, repositórios enchendo, licença |
| [Tailscale](tailscale.md) | `tailscale` | cliente OAuth (devices:core:read) ou token de acesso à API | Tailnet: servidores offline, chaves de nó expirando, dispositivos aguardando aprovação |
| [Nextcloud](nextcloud.md) | `nextcloud` | token de monitoramento (NC-Token) | Sincronização de arquivos: modo de manutenção, atualização pendente, atualizações, usuários ativos, OPcache |
| [Immich](immich.md) | `immich` | chave de API | Biblioteca de fotos: filas de jobs, atualizações, disco, contagens de fotos |
| [Paperless-ngx](paperless.md) | `paperless` | token de API ou usuário/senha | Documentos: Redis, Celery, índice, importações com falha, atualizações |
| [Jellyfin](jellyfin.md) | `jellyfin` | chave de API | Servidor de mídia: tarefas agendadas com falha, plugins, streams e transcodificações |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, ou nenhuma de uma rede permitida | Servidor de mídia: streams, transcodificações, bibliotecas, atualizações |
| [GitLab (self-managed)](gitlab.md) | `gitlab` | token de acesso pessoal | Forge: fila do Sidekiq, executores de CI, migrações pendentes, configurações de cadastro/2FA, licença |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | token de acesso | Forge: verificação de saúde, tarefas agendadas, executores de Actions, contagens de repositórios/contas |
| [Servidor com agente](agent.md) | `agent` | nenhuma (token de cadastro) | Linux, Windows, Raspberry Pi, [host Hyper-V](agent.md#hyper-v-hosts) |
| [Site ou API web](services.md#http) | `http` | nenhuma, usuário/senha ou token | Página de saúde, API REST |
| [Porta de rede](services.md#tcp) | `tcp` | nenhuma | SSH, SMB, banco de dados |
| [Nome de domínio](services.md#dns) | `dns` | nenhuma | Seu domínio, um nome interno |
| [Host alcançável](services.md#ping) | `ping` | nenhuma | Gateway, ponto de acesso, impressora |
| [Certificado TLS](services.md#tls) | `tls` | nenhuma | IMAPS, LDAPS, proxy reverso |
| [Servidor de hora NTP](services.md#ntp) | `ntp` | nenhuma | Roteador, NAS, servidor de hora interno, servidor de pool público |
| [Heartbeat](push.md) | `push` | nenhuma (URL secreta) | Job cron, script de backup, automação que precisa dar sinal |
| [Mudanças no site](webchange.md) | `webchange` | nenhuma | Uma página ou um site inteiro: mudanças de texto, páginas novas e removidas, antes/depois com capturas de tela |
| [Dispositivo de demonstração](demo.md) | `dummy` | nenhuma | Explorar a interface sem hardware |

## Como um dispositivo é lido {#how-a-device-is-read}

O agendador executa todo dispositivo ativado em seu próprio intervalo (60 s
por padrão, mínimo de 10 s) através do coletor de seu tipo, com no máximo
`DUMBMONIT_MAX_CONCURRENT_PROBES` sondagens simultâneas e um tempo limite
rígido de `DUMBMONIT_PROBE_TIMEOUT_SECS` por sondagem. Cada sondagem bem
sucedida grava suas amostras mais `dumbmonit_up = 1`; uma sondagem com falha
não grava nada, e é o silêncio da série que a regra "Device unreachable"
detecta.

Falhas são classificadas. Um dispositivo que não responde está *inacessível*
e gera um alerta. Uma comunidade errada, um certificado recusado ou uma
capacidade faltando é um *erro de configuração*: é mostrado na página do
dispositivo, em vermelho, com o motivo, e não é notificado. A distinção
importa: um dispositivo mal configurado não deveria acordar ninguém à noite.

## Opções {#options}

Alguns tipos leem opções das tags do dispositivo: a porta da API, se deve
aceitar um certificado autoassinado, quais nós monitorar. O formulário mostra
um campo tipado por opção sob **Mais opções**, com seu padrão e texto de
ajuda. As páginas de cada tipo as listam literalmente. Opções são copiadas em
toda série como rótulos `tag_<key>`, então nunca coloque um segredo em uma:
use o campo de credencial, que é criptografado.

## Dispositivos pai {#parent-devices}

Qualquer dispositivo pode declarar outro como seu pai. Quando o pai está
inacessível, alertas de seus descendentes são marcados como *Suprimido pelo
pai* em vez de serem enviados: um switch que cai produz uma notificação, não
trinta. Na página Dispositivos, os filhos se agrupam sob seu pai e escurecem
quando ele está inacessível.
