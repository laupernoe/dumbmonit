# Tipos de dispositivo {#device-types}

O DumbMonit chama *dispositivo* a tudo o que monitoriza (a API diz *target*),
mesmo quando é uma página web ou um nome DNS. Cada tipo é um coletor no
servidor; o servidor descreve cada tipo à interface
(`GET /api/collectors`), pelo que o formulário corresponde sempre ao que esta
versão pode monitorizar.

| Tipo | `kind` | Credencial | Utilização típica |
|---|---|---|---|
| [Dispositivo SNMP](snmp.md) | `snmp` | comunidade ou SNMP v3 | Comutador, router, NAS, UPS, impressora |
| [Proxmox VE](proxmox.md) | `proxmox` | token de API ou utilizador/senha | Hipervisor, cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | token de API ou utilizador/senha | Servidor de cópias de segurança |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | token de API | Consola que federa vários clusters PVE e instâncias PBS |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | utilizador/senha | Gateway de email que filtra spam e vírus |
| [VMware vSphere](vsphere.md) | `vsphere` | utilizador/senha | vCenter ou ESXi autónomo: anfitriões, VMs, datastores, alarmes |
| [Synology DSM](synology.md) | `synology` | utilizador/senha | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | chave de API e segredo | Firewall, router, ponto de entrada multi-WAN |
| [pfSense](pfsense.md) | `pfsense` | chave de API REST (utilizador só de leitura) | Firewall: gateways em baixo ou com perdas, interfaces sem ligação, serviços parados |
| [FortiGate](fortigate.md) | `fortigate` | token de API REST (perfil só de leitura, anfitrião de confiança) | Firewall: túneis IPsec, ligações, sincronização HA, modo de conservação, licenças FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | utilizador/senha (perfil só de leitura) | Firewall: apenas ligações de interface (a API XML não expõe o estado de túnel, HA ou licença) |
| [UniFi Network](unifi.md) | `unifi` | chave de API ou utilizador/senha | Consola UniFi ou servidor autoalojado: dispositivos, WAN, clientes |
| [Home Assistant](homeassistant.md) | `homeassistant` | token de acesso de longa duração | Casa inteligente: entidades indisponíveis, baterias baixas, atualizações, reparações |
| [TrueNAS](truenas.md) | `truenas` | chave de API | Servidor de armazenamento ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | chave de API (função Viewer) | NAS: array, discos, verificações de paridade, pools de cache, contentores, VMs |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | utilizador/senha (grupo só de leitura) | Router ou comutador: versões e firmware em atraso, CPU, memória, sensores, erros de interface |
| [UPS com NUT](nut.md) | `nut` | nenhuma, ou utilizador/senha | UPS num NAS, Raspberry Pi ou servidor com Network UPS Tools |
| [Hardware de servidor (Redfish)](redfish.md) | `redfish` | utilizador/senha | Ventoinhas, temperaturas, fontes de alimentação e discos do servidor, lidos do seu BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | nenhuma, utilizador/senha ou token | Base de dados de séries temporais: ingestão, amostras recusadas, espaço livre em disco |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | nenhuma, utilizador/senha ou token | Base de dados de logs: ingestão, linhas recusadas, espaço livre em disco |
| [Grafana Loki](loki.md) | `loki` | nenhuma, utilizador/senha ou token | Servidor de logs: estado de preparação, linhas recusadas, falhas de flush |
| [Graylog](graylog.md) | `graylog` | token de acesso ou utilizador/senha | Servidor de logs e o seu cluster OpenSearch: journal, buffers, caudal, entradas |
| [Pi-hole](pihole.md) | `pihole` | senha de aplicação, ou nenhuma | Bloqueador de anúncios DNS: estado de bloqueio, fração bloqueada, idade da lista de bloqueio, atualizações |
| [AdGuard Home](adguard.md) | `adguard` | utilizador/senha | Filtro DNS: proteção, bloqueios, listas de filtros, servidores a montante, atualizações |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | nenhuma, ou endereço de email/senha | Servidor de email: SMTP, IMAP, POP3, webmail, versão |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | nenhuma, ou chave de API | Gateway de email: serviços, fila de entrega e outros contadores |
| [Redis / Valkey](redis.md) | `redis` | utilizador ACL, só senha, ou nenhuma | Armazenamento em memória: memória contra maxmemory, replicação, persistência, chaves |
| [MongoDB](mongodb.md) | `mongodb` | utilizador/senha (clusterMonitor) ou nenhuma | Base de dados de documentos: replica set, atraso, ligações, cache WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | utilizador/senha (etiqueta monitoring) | Broker de mensagens: alarmes de recursos, nós, partições, filas sem consumidor |
| [CrowdSec](crowdsec.md) | `crowdsec` | nenhuma, ou chave de API de bouncer | Motor de segurança: decisões, alertas, bouncers que deixaram de obter, leitura de logs |
| [Traefik](traefik.md) | `traefik` | nenhuma, ou utilizador basicAuth | Proxy inverso: routers desativados, servidores de backend em baixo, certificados não obtidos ou não renovados, fração de 5xx |
| [Caddy](caddy.md) | `caddy` | utilizador de visualização só de leitura, ou nenhuma | Servidor web e proxy inverso: saúde dos upstreams, recargas falhadas, erros de handler, fração de 5xx |
| [Nginx](nginx.md) | `nginx` | nenhuma, ou autenticação básica | Servidor web: ligações ativas/em espera, taxas de aceitação/tratamento/pedidos, saúde dos upstreams do NGINX Plus |
| [Apache httpd](apache.md) | `apache` | nenhuma, ou autenticação básica | Servidor web: trabalhadores ocupados/inativos, scoreboard por estado, taxas de pedidos e bytes |
| [Nginx Proxy Manager](npm.md) | `npm` | email e senha do utilizador (só visualização) | Gestor de proxy: anfitriões que o nginx recusou, anfitriões desativados, certificados servidos |
| [Expiração de domínio](domain.md) | `domain` | nenhuma | Domínio registado via RDAP: dias até à expiração, retenção, período de recuperação, registador |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | token de conta de serviço (ClusterRole só de leitura) | Cluster: nós não prontos, pods em crash loop ou pendentes, workloads sem réplicas, reivindicações de volume bloqueadas |
| [Active Directory](activedirectory.md) | `activedirectory` | utilizador/senha (utilizador de domínio simples) | Domínio Windows via LDAPS: controladores de domínio e funções FSMO, membros de grupos privilegiados, política de senhas, descobertas de segurança |
| [Veeam Backup & Replication](veeam.md) | `veeam` | utilizador/senha (Veeam Backup Viewer) | Servidor de cópias de segurança: tarefas falhadas, sessões falhadas, repositórios a enchê-se, licença |
| [Tailscale](tailscale.md) | `tailscale` | cliente OAuth (devices:core:read) ou token de acesso de API | Tailnet: servidores offline, chaves de nó a expirar, dispositivos à espera de aprovação |
| [Nextcloud](nextcloud.md) | `nextcloud` | token de monitorização (NC-Token) | Sincronização de ficheiros: modo de manutenção, atualização pendente, atualizações, utilizadores ativos, OPcache |
| [Immich](immich.md) | `immich` | chave de API | Biblioteca de fotos: filas de tarefas, atualizações, disco, contagens de fotos |
| [Paperless-ngx](paperless.md) | `paperless` | token de API ou utilizador/senha | Documentos: Redis, Celery, índice, importações falhadas, atualizações |
| [Jellyfin](jellyfin.md) | `jellyfin` | chave de API | Servidor de media: tarefas agendadas falhadas, plugins, streams e transcodificações |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, ou nenhuma de uma rede permitida | Servidor de media: streams, transcodificações, bibliotecas, atualizações |
| [GitLab (autoalojado)](gitlab.md) | `gitlab` | token de acesso pessoal | Forja: atraso do Sidekiq, executores de CI, migrações pendentes, definições de registo/2FA, licença |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | token de acesso | Forja: verificação de saúde, tarefas agendadas, executores de Actions, contagens de repositórios/contas |
| [Servidor com agente](agent.md) | `agent` | nenhuma (token de registo) | Linux, Windows, Raspberry Pi, [anfitrião Hyper-V](agent.md#hyper-v-hosts) |
| [Site ou API web](services.md#http) | `http` | nenhuma, utilizador/senha ou token | Página de saúde, API REST |
| [Porta de rede](services.md#tcp) | `tcp` | nenhuma | SSH, SMB, base de dados |
| [Nome de domínio](services.md#dns) | `dns` | nenhuma | O seu domínio, um nome interno |
| [Anfitrião alcançável](services.md#ping) | `ping` | nenhuma | Gateway, ponto de acesso, impressora |
| [Certificado TLS](services.md#tls) | `tls` | nenhuma | IMAPS, LDAPS, proxy inverso |
| [Servidor de hora NTP](services.md#ntp) | `ntp` | nenhuma | Router, NAS, servidor de hora interno, servidor de pool público |
| [Heartbeat](push.md) | `push` | nenhuma (URL secreto) | Tarefa cron, script de cópia de segurança, automação que tem de chamar |
| [Alterações de site](webchange.md) | `webchange` | nenhuma | Uma página ou um site inteiro: alterações de texto, páginas novas e removidas, antes/depois com capturas de ecrã |
| [Dispositivo de demonstração](demo.md) | `dummy` | nenhuma | Explorar a interface sem hardware |

## Como um dispositivo é lido {#how-a-device-is-read}

O escalonador executa cada dispositivo ativado no seu próprio intervalo
(60 s por predefinição, mínimo de 10 s) através do coletor do seu tipo, com
no máximo `DUMBMONIT_MAX_CONCURRENT_PROBES` verificações em curso e um tempo
limite rígido de `DUMBMONIT_PROBE_TIMEOUT_SECS` por verificação. Cada
verificação bem-sucedida escreve as suas amostras mais `dumbmonit_up = 1`;
uma verificação falhada não escreve nada, e é o silêncio da série que a
regra «Device unreachable» deteta.

As falhas são classificadas. Um dispositivo que não responde está
*inacessível* e gera um alerta. Uma comunidade errada, um certificado
recusado ou uma capacidade em falta é um *erro de configuração*: é mostrado
na página do dispositivo, em vermelho, com o motivo, e não é notificado. A
distinção importa: um dispositivo mal configurado não deve acordar ninguém
de noite.

## Opções {#options}

Alguns tipos lêem opções a partir das etiquetas do dispositivo: a porta da
API, se deve aceitar um certificado autoassinado, quais nós monitorizar. O
formulário mostra um campo tipado por opção em **More options**, com a sua
predefinição e texto de ajuda. As páginas por tipo listam-nas literalmente.
As opções são copiadas em cada série como etiquetas `tag_<key>`, por isso
nunca coloque um segredo numa delas: use o campo de credencial, que é
encriptado.

## Dispositivos pai {#parent-devices}

Qualquer dispositivo pode declarar outro como seu pai. Quando o pai está
inacessível, os alertas dos seus descendentes são marcados como *Suppressed
by parent* em vez de serem enviados: um comutador a cair produz uma
notificação, não trinta. Na página Devices, os filhos empilham-se sob o seu
pai e esbatem-se quando este está inacessível.
