# Tipos de dispositivo {#device-types}

O DumbMonit chama *dispositivo* a tudo o que monitoriza (a API diz *target*),
mesmo quando é uma página web ou um nome DNS. Cada tipo é um coletor no
servidor; o servidor descreve cada tipo à interface (`GET /api/collectors`),
pelo que o formulário corresponde sempre ao que esta versão pode monitorizar.

| Tipo | `kind` | Credencial | Utilização típica |
|---|---|---|---|
| [Dispositivo SNMP](snmp.md) | `snmp` | community ou SNMP v3 | Comutador, router, NAS, UPS, impressora |
| [Proxmox VE](proxmox.md) | `proxmox` | token de API ou utilizador/palavra-passe | Hipervisor, cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | token de API ou utilizador/palavra-passe | Servidor de cópias de segurança |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | token de API | Consola que federa vários clusters PVE e instâncias PBS |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | utilizador/palavra-passe | Gateway de correio que filtra spam e vírus |
| [VMware vSphere](vsphere.md) | `vsphere` | utilizador/palavra-passe | vCenter ou ESXi autónomo: anfitriões, VM, datastores, alarmes |
| [XCP-ng / XenServer](xcpng.md) | `xcpng` | utilizador/palavra-passe (função XAPI só de leitura) | Pool através do XAPI: anfitriões, VM, repositórios de armazenamento, alta disponibilidade |
| [Nutanix AHV](nutanix.md) | `nutanix` | utilizador/palavra-passe (Prism Viewer) | API v4 do Prism Central: clusters, anfitriões, VM AHV, contentores de armazenamento, alertas |
| [Synology DSM](synology.md) | `synology` | utilizador/palavra-passe | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | chave de API e segredo | Firewall, router, borda multi-WAN |
| [pfSense](pfsense.md) | `pfsense` | chave de API REST (utilizador só de leitura) | Firewall: gateways em baixo ou com perdas, interfaces sem ligação, serviços parados |
| [FortiGate](fortigate.md) | `fortigate` | token de API REST (perfil só de leitura, anfitrião de confiança) | Firewall: túneis IPsec, ligações, sincronização HA, modo conserve, licenças FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | utilizador/palavra-passe (perfil só de leitura) | Firewall: apenas ligações de interface (a API XML não expõe estado de túneis, HA nem licenças) |
| [Servidor OpenVPN](openvpn.md) | `openvpn` | nenhuma, ou palavra-passe de gestão | Servidor OpenVPN através da sua interface de gestão: versão, estado, clientes ligados |
| [UniFi Network](unifi.md) | `unifi` | chave de API ou utilizador/palavra-passe | Consola UniFi ou servidor auto-alojado: dispositivos, WAN, clientes |
| [Home Assistant](homeassistant.md) | `homeassistant` | token de acesso de longa duração | Casa inteligente: entidades indisponíveis, baterias fracas, atualizações, reparações |
| [TrueNAS](truenas.md) | `truenas` | chave de API | Servidor de armazenamento ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | chave de API (papel Viewer) | NAS: array, discos, verificações de paridade, pools de cache, contentores, VM |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | utilizador/palavra-passe (grupo só de leitura) | Router ou comutador: versões e firmware em atraso, CPU, memória, sensores, erros de interface |
| [UPS com NUT](nut.md) | `nut` | nenhuma, ou utilizador/palavra-passe | UPS num NAS, Raspberry Pi ou servidor com Network UPS Tools |
| [Hardware de servidor (Redfish)](redfish.md) | `redfish` | utilizador/palavra-passe | Ventoinhas, temperaturas, fontes de alimentação e unidades do servidor, lidas do seu BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | nenhuma, utilizador/palavra-passe ou token | Base de dados de séries temporais: ingestão, amostras recusadas, margem de disco |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | nenhuma, utilizador/palavra-passe ou token | Base de dados de registos: ingestão, linhas recusadas, margem de disco |
| [Grafana Loki](loki.md) | `loki` | nenhuma, utilizador/palavra-passe ou token | Servidor de registos: prontidão, linhas recusadas, falhas de flush |
| [Graylog](graylog.md) | `graylog` | token de acesso ou utilizador/palavra-passe | Servidor de registos e o seu cluster OpenSearch: journal, buffers, débito, inputs |
| [Pi-hole](pihole.md) | `pihole` | palavra-passe de aplicação, ou nenhuma | Bloqueador de anúncios DNS: estado de bloqueio, parte bloqueada, idade das listas de bloqueio, atualizações |
| [AdGuard Home](adguard.md) | `adguard` | utilizador/palavra-passe | Filtro DNS: proteção, bloqueios, listas de filtros, servidores a montante, atualizações |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | nenhuma, ou endereço de email/palavra-passe | Servidor de correio: SMTP, IMAP, POP3, webmail, versão |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | nenhuma, ou chave de API | Gateway de correio: serviços, fila de entrega e outros contadores |
| [Redis / Valkey](redis.md) | `redis` | utilizador ACL, só palavra-passe, ou nenhuma | Armazenamento em memória: memória contra maxmemory, replicação, persistência, chaves |
| [MongoDB](mongodb.md) | `mongodb` | utilizador/palavra-passe (clusterMonitor) ou nenhuma | Base de dados de documentos: replica set, atraso, ligações, cache WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | utilizador/palavra-passe (tag monitoring) | Message broker: alarmes de recursos, nós, partições, filas sem consumidor |
| [CrowdSec](crowdsec.md) | `crowdsec` | nenhuma, ou chave de API de bouncer | Motor de segurança: decisões, alertas, bouncers que pararam de obter, leitura de registos |
| [Traefik](traefik.md) | `traefik` | nenhuma, ou utilizador basicAuth | Proxy inverso: routers desativados, servidores de backend em baixo, certificados não obtidos ou não renovados, parte de 5xx |
| [Caddy](caddy.md) | `caddy` | utilizador de visualização só de leitura, ou nenhuma | Servidor web e proxy inverso: saúde dos upstreams, recarregamentos falhados, erros de handlers, parte de 5xx |
| [Nginx](nginx.md) | `nginx` | nenhuma, ou autenticação básica | Servidor web: ligações ativas/em espera, taxas de accept/handled/request, saúde dos upstreams do NGINX Plus |
| [Apache httpd](apache.md) | `apache` | nenhuma, ou autenticação básica | Servidor web: workers ocupados/inativos, scoreboard por estado, taxas de pedidos e bytes |
| [Nginx Proxy Manager](npm.md) | `npm` | email e palavra-passe do utilizador (só visualização) | Gestor de proxy: anfitriões que o nginx recusou, anfitriões desativados, certificados servidos |
| [Expiração de domínio](domain.md) | `domain` | nenhuma | Domínio registado via RDAP: dias até à expiração, retenção, período de resgate, registador |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | token de conta de serviço (ClusterRole só de leitura) | Cluster: nós não prontos, pods em crash loop ou pendentes, workloads sem réplicas, claims de volume bloqueados |
| [Active Directory](activedirectory.md) | `activedirectory` | utilizador/palavra-passe (utilizador de domínio comum) | Domínio Windows via LDAPS: controladores de domínio e papéis FSMO, membros de grupos privilegiados, política de palavras-passe, deteções de segurança |
| [Veeam Backup & Replication](veeam.md) | `veeam` | utilizador/palavra-passe (Veeam Backup Viewer) | Servidor de cópias de segurança: tarefas falhadas, sessões falhadas, repositórios a encher, licença |
| [Tailscale](tailscale.md) | `tailscale` | cliente OAuth (devices:core:read) ou token de acesso de API | Tailnet: servidores offline, chaves de nó a expirar, dispositivos à espera de aprovação |
| [Nextcloud](nextcloud.md) | `nextcloud` | token de monitorização (NC-Token) | Sincronização de ficheiros: modo de manutenção, atualização pendente, atualizações, utilizadores ativos, OPcache |
| [Immich](immich.md) | `immich` | chave de API | Biblioteca de fotografias: filas de tarefas, atualizações, disco, contagens de fotografias |
| [Paperless-ngx](paperless.md) | `paperless` | token de API ou utilizador/palavra-passe | Documentos: Redis, Celery, índice, importações falhadas, atualizações |
| [Jellyfin](jellyfin.md) | `jellyfin` | chave de API | Servidor multimédia: tarefas agendadas falhadas, plugins, streams e transcodificações |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, ou nenhuma a partir de uma rede permitida | Servidor multimédia: streams, transcodificações, bibliotecas, atualizações |
| [GitLab (auto-alojado)](gitlab.md) | `gitlab` | token de acesso pessoal | Forja: backlog do Sidekiq, runners de CI, migrações pendentes, definições de registo/2FA, licença |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | token de acesso | Forja: verificação de saúde, tarefas agendadas, runners de Actions, contagens de repositórios/contas |
| [Servidor com agente](agent.md) | `agent` | nenhuma (token de registo) | Linux, Windows, Raspberry Pi, [anfitrião Hyper-V](agent.md#hyper-v-hosts) |
| [Site ou API web](services.md#http) | `http` | nenhuma, utilizador/palavra-passe ou token | Página de saúde, API REST |
| [Porta de rede](services.md#tcp) | `tcp` | nenhuma | SSH, SMB, base de dados |
| [Nome de domínio](services.md#dns) | `dns` | nenhuma | O seu domínio, um nome interno |
| [Anfitrião alcançável](services.md#ping) | `ping` | nenhuma | Gateway, ponto de acesso, impressora |
| [Certificado TLS](services.md#tls) | `tls` | nenhuma | IMAPS, LDAPS, proxy inverso |
| [Servidor SMTP](services.md#smtp) | `smtp` | nenhuma, ou utilizador/palavra-passe | Servidor de correio: saudação, STARTTLS, certificado, início de sessão opcional |
| [PostgreSQL](services.md#postgresql) | `postgres` | utilizador/palavra-passe | Base de dados: ligação e uma consulta à sua escolha |
| [MySQL ou MariaDB](services.md#mysql-and-mariadb) | `mysql` | utilizador/palavra-passe | Base de dados: ligação e uma consulta à sua escolha |
| [Broker MQTT](services.md#mqtt) | `mqtt` | nenhuma, ou utilizador/palavra-passe | Broker: ligação, subscrição, mensagem retida |
| [WebSocket](services.md#websocket) | `websocket` | nenhuma, utilizador/palavra-passe ou token | Endpoint WebSocket: handshake, troca de mensagens opcional |
| [Servidor de hora NTP](services.md#ntp) | `ntp` | nenhuma | Router, NAS, servidor de hora interno, servidor de pool público |
| [Heartbeat](push.md) | `push` | nenhuma (URL secreto) | Tarefa cron, script de cópia de segurança, automação que tem de se anunciar |
| [Alterações em sites](webchange.md) | `webchange` | nenhuma | Uma página ou um site inteiro: alterações de texto, páginas novas e removidas, antes/depois com capturas de ecrã |
| [Saúde e SEO de sites](website.md) | `website` | nenhuma | Sites em massa: em baixo, página em branco, redirecionamentos, noindex, robots.txt, alterações de título/descrição/canónico, plataforma (importação CSV) |
| [Dispositivo de demonstração](demo.md) | `dummy` | nenhuma | Explorar a interface sem hardware |

## Como um dispositivo é lido {#how-a-device-is-read}

O agendador executa cada dispositivo ativado no seu próprio intervalo
(predefinição 60 s, mínimo 10 s) através do coletor do seu tipo, com no máximo
`DUMBMONIT_MAX_CONCURRENT_PROBES` sondas em curso e um tempo limite rígido de
`DUMBMONIT_PROBE_TIMEOUT_SECS` por sonda. Cada sonda bem-sucedida escreve as
suas amostras mais `dumbmonit_up = 1`; uma sonda falhada não escreve nada, e é
o silêncio da série que a regra «Dispositivo inacessível» deteta. A página do
dispositivo mostra a falha na sonda seguinte, mas o alerta espera de propósito:
dispara cerca de quatro minutos após a última sonda bem-sucedida (três minutos
de silêncio, depois uma espera de um minuto), pelo que uma única sonda perdida
nunca acorda ninguém.

As falhas são classificadas. Um dispositivo que não responde está
*inacessível* e levanta um alerta. Uma community errada, um certificado
recusado ou uma capacidade em falta é um *erro de configuração*: é mostrado na
página do dispositivo, a vermelho, com o motivo, e não é notificado. A
distinção importa: um dispositivo mal configurado não deve acordar ninguém de
noite.

## Opções {#options}

Alguns tipos lêem opções das etiquetas do dispositivo: a porta da API, se
aceita um certificado autoassinado, que nós monitorizar. O formulário mostra um
campo tipado por opção em **Mais opções**, com a sua predefinição e texto de
ajuda. As páginas de cada tipo listam-nas textualmente. As opções são
definições, não etiquetas: não são copiadas para as séries. As etiquetas livres
(qualquer outra chave) são copiadas para cada série como etiquetas
`tag_<key>`, exceto chaves que nomeiam um segredo (comparadas por palavra:
`password`, `token`, `key`, `cert`, `secret`, `pass`, `private`, `pem`…, pelo
que `primary_key` é ignorada mas `keyboard` não) e valores com mais de 256
caracteres. Nunca ponha um segredo numa etiqueta de qualquer forma: use o campo
de credencial, que é cifrado.

## Dispositivos pai {#parent-devices}

Qualquer dispositivo pode declarar outro como seu pai. Quando o pai está
inacessível, os alertas dos seus descendentes são marcados *Suprimido pelo pai*
em vez de serem enviados: um comutador que cai produz uma notificação, não
trinta. Na página Dispositivos, os filhos empilham-se sob o pai e esbatem-se
quando este está inacessível.
