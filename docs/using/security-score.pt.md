# Pontuação de segurança {#security-score}

Todo dispositivo suportado recebe uma **pontuação de segurança** de 0
a 100 e uma nota de **A** a **F**, no espírito do PingCastle ou do
Microsoft Secure Score: uma lista de verificações extraídas das
próprias orientações de boas práticas do fornecedor, cada uma a
passar, a falhar ou não avaliada.

A pontuação aparece como um cartão **Security** na página do
dispositivo, e na página **Security**, que lista todo dispositivo
avaliado, o pior primeiro.

## Como a pontuação é calculada {#how-the-score-is-computed}

O DumbMonit nunca pede nada extra ao dispositivo para calcular a
pontuação. As verificações lêem os últimos valores que os coletores já
registaram (olhando até 6 horas atrás), por isso uma pontuação não
custa nada ao dispositivo e funciona offline a partir da última
verificação.

Cada verificação tem:

| Campo | Significado |
|---|---|
| Category | exposição, correção, autenticação, encriptação, cópia de segurança ou configuração |
| Severity | critical, high, medium ou low — o seu peso na pontuação: 10, 6, 3, 1 |
| Result | **pass**, **fail**, ou **unknown** quando os dados não são coletados |
| Evidence | o que foi observado: `pve1: 3 security updates`, `Expires in 9 days`… |
| Remediation | o que fazer, nos termos do fornecedor |
| Vendor guidance | a página do fornecedor (ou padrão) em que a verificação se baseia |

**Score** = 100 × (peso das verificações a passar) ÷ (peso das
verificações a passar + a falhar). **As verificações desconhecidas são
mostradas mas nunca contadas**, nem a favor nem contra: uma pontuação
não deve descer porque um token de monitorização não tem um
privilégio, nem subir porque nada é conhecido. Um dispositivo cujas
verificações são todas desconhecidas *não é avaliado*.

| Nota | Pontuação | Palavra |
|---|---|---|
| A | 90–100 | Strong |
| B | 75–89 | Good |
| C | 60–74 | Fair |
| D | 40–59 | Weak |
| F | 0–39 | Poor |

**Uma verificação crítica falhada limita a nota a C**, qualquer que
seja a pontuação: uma firewall que já não filtra, ou um firmware
ativamente explorado deixado sem correção, não é compensado por dez
definições menores boas. O cartão diz isso quando acontece.

## Histórico e alertas {#history-and-alerts}

O servidor recalcula toda pontuação a cada 15 minutos e guarda-a como
`dumbmonit_security_score{target="<id>"}` (e o número de verificações
falhadas como `dumbmonit_security_checks_failed`), para que a possa
representar em gráfico e alertar sobre ela como qualquer outra
métrica. Duas regras incorporadas usam-na:

- **Security score dropped** — a pontuação caiu 10 pontos ou mais em
  comparação com o seu melhor valor das últimas 24 horas: algo novo
  está a falhar.
- **Security score low** — a pontuação está abaixo de 40 (nota F).
  Edite a regra para mover o limiar para a sua própria barra.

Ver [Regras de alerta](../alerting/rules.md#security-score).

## Verificações por tipo {#checks-per-kind}

### Proxmox VE {#proxmox-ve}

| Verificação | Severidade | Dados |
|---|---|---|
| Security updates installed | high | atualizações de segurança pendentes por nó (precisa de `Sys.Modify` no token, caso contrário desconhecido) |
| Proxmox VE packages up to date | medium | pacotes Proxmox atualizáveis por nó |
| Running the installed kernel | medium | kernel mais recente instalado do que em execução |
| No test repository in use | medium | repositórios padrão ativados |
| Package repositories correctly configured | medium | erros e avisos que o Proxmox VE levanta sobre os repositórios (por exemplo repositório enterprise sem subscrição) |
| Enterprise repository subscription | low | estado de subscrição por nó |
| Web interface certificate valid for 14+ days | medium | expiração do certificado do nó |
| Every guest is in a backup job | high | convidados não selecionados por nenhuma tarefa de cópia de segurança |
| Backup jobs succeed | high | último resultado de cada tarefa de cópia de segurança |

Fontes: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server {#proxmox-backup-server}

| Verificação | Severidade | Dados |
|---|---|---|
| System updates installed | high | atualizações pendentes (precisa de `Sys.Modify` em `/system`) |
| Running the installed Backup Server version | medium | daemons ainda a correr a versão pré-atualização |
| Verify job scheduled | high | número de tarefas de verificação |
| Prune job scheduled | medium | número de tarefas de prune |
| Backups copied elsewhere (sync or tape) | medium | tarefas de cópia de segurança sync e tape (regra 3-2-1) |
| Garbage collection clean, no bad chunks | high | último resultado de GC e chunks corrompidos por datastore |
| Certificate valid for 14+ days | medium | expiração do certificado |

Fontes: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html),
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html),
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html).

### Synology DSM {#synology-dsm}

| Verificação | Severidade | Dados |
|---|---|---|
| DSM 7 or later | high | versão do DSM |
| Hyper Backup task configured | medium | número de tarefas Hyper Backup |
| Hyper Backup tasks succeed | high | último resultado de cada tarefa |

Fontes: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense e pfSense {#opnsense-and-pfsense}

| Verificação | Severidade | Dados |
|---|---|---|
| OPNsense: firmware updates installed | high | atualizações de pacote pendentes (desconhecido até o OPNsense ter verificado atualizações) |
| OPNsense: on the current major release series | medium | nova série maior disponível |
| OPNsense: no reboot pending after an update | medium | reinício necessário |
| OPNsense: packet filter enabled | **critical** | estado do `pf` |
| pfSense: REST API package up to date | low | atualização do pacote REST API disponível |

Fontes: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html),
[pfSense REST API](https://pfrest.org/).

### UniFi {#unifi}

| Verificação | Severidade | Dados |
|---|---|---|
| Device firmware up to date | high | marca de atualizável de cada dispositivo adotado |

Fonte: [UniFi releases](https://community.ui.com/releases).

### Monitores TLS e HTTPS {#tls-and-https-monitors}

Para monitores `tls`, `http`, `smtp`, `mqtt` e `websocket` que negoceiam
TLS. Um monitor em texto simples não tem cartão Security.

| Verificação | Severidade | Dados |
|---|---|---|
| Certificate valid for 14+ days | high | dias até à expiração |
| Certificate chain trusted | high | verificação da cadeia |
| TLS 1.3 negotiated | low | versão de protocolo negociada |

Fontes: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final),
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/).

### Servidores de hora NTP {#ntp-time-servers}

| Verificação | Severidade | Dados |
|---|---|---|
| Server reports a synchronized clock | medium | stratum e indicador leap |
| Clock offset within 100 ms | low | desvio medido |

Um stratum de 16 (ou 0, um «kiss-o'-death») e um indicador leap
«unsynchronized» ambos falham a primeira verificação independentemente
do desvio: um servidor que nunca sincronizou não tem nada de confiança
para oferecer. O limiar de 100 ms está fixo aqui e não segue a opção
«Tolerated offset» por dispositivo, da mesma forma que a verificação de
certificado avisa sempre a catorze dias independentemente das próprias
definições de um monitor.

Fonte: [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905).

### Nginx e Apache httpd {#nginx-and-apache-httpd}

| Verificação | Severidade | Dados |
|---|---|---|
| Server header does not disclose the version | low | o cabeçalho de resposta `Server` capturado junto com a página de estado |

Um cabeçalho que ainda nomeia o produto («nginx», «Apache») sem um
número de versão passa: é isso que `server_tokens off` e
`ServerTokens Prod` produzem, e um cabeçalho ausente também passa. Um
cabeçalho como «nginx/1.25.3» ou «Apache/2.4.58 (Unix)» falha: diz a um
scanner exatamente que CVEs tentar.

Fontes: [Nginx core module reference](https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens),
[Apache core module reference](https://httpd.apache.org/docs/2.4/mod/core.html#servertokens).

### Máquinas com o agente (e os seus contentores Docker) {#machines-with-the-agent-and-their-docker-containers}

| Verificação | Severidade | Dados |
|---|---|---|
| Security updates installed | high | atualizações de segurança pendentes (onde o gestor de pacotes as distingue) |
| No pending updates | low | atualizações pendentes |
| No reboot pending | medium | reinício necessário |
| SELinux enforcing | medium | modo do SELinux (desconhecido onde o SELinux está ausente) |
| Container images up to date | medium | imagem mais recente no registo (só em máquinas com contentores) |
| No container image older than 180 days | low | idade da imagem (só em máquinas com contentores) |

Fontes: [CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index),
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often).

### Atualizações de fornecedor para outras integrações {#vendor-updates-for-other-integrations}

Uma verificação de correção por produto, a partir da informação de
atualização que cada coletor já lê:

| Tipo | Verificação | Severidade |
|---|---|---|
| FortiGate | FortiOS patch installed | **critical** |
| FortiGate | FortiGuard licences valid | high |
| MikroTik | RouterOS up to date | high |
| MikroTik | RouterBOOT firmware matches RouterOS | low |
| Home Assistant | Home Assistant components up to date | medium |
| Nextcloud | Nextcloud server up to date | high |
| Nextcloud | Nextcloud apps up to date | medium |
| AdGuard Home | AdGuard Home up to date | medium |
| Pi-hole | Pi-hole up to date | medium |
| Plex | Plex Media Server up to date | medium |
| Paperless-ngx | Paperless-ngx up to date | medium |
| Tailscale | Tailscale clients up to date | medium |

### Active Directory {#active-directory}

Um dispositivo Active Directory é avaliado a partir das descobertas
estilo PingCastle do seu coletor: contas Kerberoastable, contas sem
pré-autenticação Kerberos, delegação não restrita, idade da senha do
`krbtgt`, tamanho de grupos privilegiados, cobertura LAPS, política de
senhas, contas obsoletas. Cada descoberta é uma verificação que passa
quando afeta zero objetos; a severidade que o coletor reporta é a
usada. Descobertas ainda não reportadas são listadas como desconhecidas.

Fontes: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview),
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password).

### GitLab {#gitlab}

Lido a partir da API de administração, com um token de acesso pessoal
de administrador; desconhecido com um comum.

| Verificação | Severidade | Dados |
|---|---|---|
| Two-factor authentication required | high | definição `require_two_factor_authentication` |
| Public sign-up disabled | medium | definição `signup_enabled` |
| Database migrations applied | medium | migrações pendentes depois de uma atualização |

Fonte: [GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html).

### Forgejo e Gitea {#forgejo-and-gitea}

| Verificação | Severidade | Dados |
|---|---|---|
| Scheduled tasks run on time | medium | tarefas cuja próxima execução já está vencida |

Desconhecido sem um token de acesso de administrador do site: o
Forgejo e o Gitea reservam a lista de tarefas agendadas à
administração.

Fonte: [Forgejo API](https://forgejo.org/docs/latest/user/api/).

## Limites {#limits}

A pontuação só reflete o que o DumbMonit pode ver. É um impulso para
olhar, não uma auditoria, e uma nota A não significa que um
dispositivo está seguro.

Boas práticas que **não** são verificadas ainda porque os coletores não
lêem os dados hoje:

| Tipo | Não verificado (precisa de dados novos) |
|---|---|
| Proxmox VE | firewall de datacenter/nó ativada; autenticação de dois fatores em `root@pam` e outras contas de administrador |
| Proxmox Backup Server | encriptação do lado do cliente das cópias de segurança (precisa de ler os manifestos de cópia de segurança) |
| Synology DSM | verificação em 2 passos para administradores; conta `admin` predefinida desativada; bloqueio automático; firewall do DSM; redirecionamento HTTP para HTTPS; exposição QuickConnect; se uma atualização DSM mais recente está disponível |
| OPNsense / pfSense | interface web ou SSH acessível pela WAN; credenciais predefinidas; atualizações do sistema pfSense (não da REST API) |
| UniFi | SSH do dispositivo com credenciais predefinidas; definições do site |
| Monitores TLS | TLS 1.0/1.1 ou conjuntos de cifras fracos ainda oferecidos — a verificação só negocia TLS 1.2 e 1.3, por isso um servidor limitado a versões mais antigas mostra-se como uma verificação falhada em vez disso |
| Anfitriões de agente | estado da firewall do anfitrião; configuração SSH; estado do Windows Defender |
| Docker | contentores em execução privilegiados, como root, ou com o socket Docker montado |

Outros limites:

- Uma verificação só é tão fresca quanto os seus dados: alguns
  coletores relêem atualizações ou subscrições a cada poucas horas.
- Os pesos são uma escolha de produto, a mesma para toda instalação.
- Tipos sem nenhuma verificação não têm cartão Security e não aparecem
  na página Security.
