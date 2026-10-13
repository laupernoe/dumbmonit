# Pontuação de segurança {#security-score}

Todo dispositivo suportado recebe uma **pontuação de segurança** de 0 a
100 e uma nota de **A** a **F**, no espírito do PingCastle ou Microsoft
Secure Score: uma lista de verificações extraídas da própria orientação
de boas práticas do fornecedor, cada uma passando, falhando ou não
avaliada.

A pontuação aparece como um cartão **Segurança** na página do
dispositivo, e na página **Segurança**, que lista todo dispositivo
avaliado, o pior primeiro.

## Como a pontuação é calculada {#how-the-score-is-computed}

O DumbMonit nunca pede nada extra ao dispositivo para calcular a
pontuação. As verificações leem os últimos valores que os coletores já
registraram (olhando para as últimas 6 horas), então uma pontuação não
custa nada ao dispositivo e funciona offline a partir da última
sondagem.

Cada verificação tem:

| Campo | Significado |
|---|---|
| Categoria | exposição, atualização, autenticação, criptografia, backup ou configuração |
| Severidade | crítica, alta, média ou baixa — seu peso na pontuação: 10, 6, 3, 1 |
| Resultado | **passou**, **falhou**, ou **desconhecido** quando os dados não são coletados |
| Evidência | o que foi observado: `pve1: 3 security updates`, `Expires in 9 days`… |
| Correção | o que fazer, nos termos do fornecedor |
| Orientação do fornecedor | a página do fornecedor (ou padrão) em que a verificação se baseia |

**Pontuação** = 100 × (peso das verificações que passaram) ÷ (peso das
que passaram + que falharam). **Verificações desconhecidas são
mostradas mas nunca contadas**, nem a favor nem contra: uma pontuação não
deve cair porque um token de monitoramento não tem um privilégio, nem
subir porque nada é conhecido. Um dispositivo cujas verificações são
todas desconhecidas *não é avaliado*.

| Nota | Pontuação | Palavra |
|---|---|---|
| A | 90–100 | Forte |
| B | 75–89 | Boa |
| C | 60–74 | Razoável |
| D | 40–59 | Fraca |
| F | 0–39 | Ruim |

**Uma verificação crítica falhando limita a nota a C**, qualquer que
seja a pontuação: um firewall que não filtra mais, ou um firmware
ativamente explorado deixado sem correção, não é compensado por dez
pequenas boas configurações. O cartão avisa isso quando acontece.

## Histórico e alertas {#history-and-alerts}

O servidor recalcula toda pontuação a cada 15 minutos e a armazena como
`dumbmonit_security_score{target="<id>"}` (e o número de verificações
falhando como `dumbmonit_security_checks_failed`), então você pode
colocá-la em gráfico e alertar sobre ela como qualquer outra métrica.
Duas regras integradas a usam:

- **Security score dropped** — a pontuação caiu 10 pontos ou mais
  comparada com seu melhor valor das últimas 24 horas: algo novo está
  falhando.
- **Security score low** — a pontuação está abaixo de 40 (nota F). Edite
  a regra para mover o limite para sua própria barra.

Veja [Regras de alerta](../alerting/rules.md#security-score).

## Verificações por tipo {#checks-per-kind}

### Proxmox VE {#proxmox-ve}

| Verificação | Severidade | Dados |
|---|---|---|
| Atualizações de segurança instaladas | alta | atualizações de segurança pendentes por nó (precisa de `Sys.Modify` no token, senão desconhecido) |
| Pacotes Proxmox VE atualizados | média | pacotes Proxmox que podem ser atualizados por nó |
| Executando o kernel instalado | média | kernel mais novo instalado que em execução |
| Nenhum repositório de teste em uso | média | repositórios padrão ativados |
| Repositórios de pacote configurados corretamente | média | erros e avisos que o Proxmox VE gera sobre os repositórios (ex. repositório enterprise sem assinatura) |
| Assinatura do repositório enterprise | baixa | status de assinatura por nó |
| Certificado da interface web válido por 14+ dias | média | expiração do certificado do nó |
| Todo guest está em um job de backup | alta | guests não selecionados por nenhum job de backup |
| Jobs de backup têm sucesso | alta | último resultado de cada job de backup |

Fontes: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server {#proxmox-backup-server}

| Verificação | Severidade | Dados |
|---|---|---|
| Atualizações do sistema instaladas | alta | atualizações pendentes (precisa de `Sys.Modify` em `/system`) |
| Executando a versão instalada do Backup Server | média | daemons ainda rodando a versão pré-atualização |
| Job de verificação agendado | alta | número de jobs de verificação |
| Job de prune agendado | média | número de jobs de prune |
| Backups copiados para outro lugar (sync ou fita) | média | jobs de backup de sync e fita (regra 3-2-1) |
| Coleta de lixo limpa, nenhum chunk corrompido | alta | último resultado de GC e chunks corrompidos por datastore |
| Certificado válido por 14+ dias | média | expiração do certificado |

Fontes: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html),
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html),
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html).

### Synology DSM {#synology-dsm}

| Verificação | Severidade | Dados |
|---|---|---|
| DSM 7 ou posterior | alta | versão DSM |
| Tarefa Hyper Backup configurada | média | número de tarefas Hyper Backup |
| Tarefas Hyper Backup têm sucesso | alta | último resultado de cada tarefa |

Fontes: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense e pfSense {#opnsense-and-pfsense}

| Verificação | Severidade | Dados |
|---|---|---|
| OPNsense: atualizações de firmware instaladas | alta | atualizações de pacote pendentes (desconhecido até o OPNsense verificar atualizações) |
| OPNsense: na série de lançamento principal atual | média | nova série principal disponível |
| OPNsense: nenhum reinício pendente depois de uma atualização | média | reinício necessário |
| OPNsense: filtro de pacotes ativado | **crítica** | status do `pf` |
| pfSense: pacote da API REST atualizado | baixa | atualização do pacote da API REST disponível |

Fontes: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html),
[pfSense REST API](https://pfrest.org/).

### UniFi {#unifi}

| Verificação | Severidade | Dados |
|---|---|---|
| Firmware do dispositivo atualizado | alta | flag de atualização possível de cada dispositivo adotado |

Fonte: [UniFi releases](https://community.ui.com/releases).

### Monitores TLS e HTTPS {#tls-and-https-monitors}

Para monitores `tls`, `http`, `smtp`, `mqtt` e `websocket` que negociam
TLS. Um monitor em texto claro não tem cartão Segurança.

| Verificação | Severidade | Dados |
|---|---|---|
| Certificado válido por 14+ dias | alta | dias até expirar |
| Cadeia de certificado confiável | alta | verificação da cadeia |
| TLS 1.3 negociado | baixa | versão de protocolo negociada |

Fontes: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final),
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/).

### Servidores de hora NTP {#ntp-time-servers}

| Verificação | Severidade | Dados |
|---|---|---|
| Servidor reporta um relógio sincronizado | média | stratum e indicador de salto |
| Desvio de relógio dentro de 100 ms | baixa | desvio medido |

Um stratum de 16 (ou 0, um "kiss-o'-death") e um indicador de salto "não
sincronizado" ambos falham a primeira verificação independente do
desvio: um servidor que nunca sincronizou não tem nada confiável a
oferecer. O limite de 100 ms é fixo aqui e não segue a opção "Desvio
tolerado" por dispositivo, da mesma forma que a verificação de
certificado sempre avisa em catorze dias independente das próprias
configurações de um monitor.

Fonte: [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905).

### Nginx e Apache httpd {#nginx-and-apache-httpd}

| Verificação | Severidade | Dados |
|---|---|---|
| Cabeçalho do servidor não revela a versão | baixa | o cabeçalho de resposta `Server` capturado junto com a página de status |

Um cabeçalho que ainda nomeia o produto ("nginx", "Apache") sem um
número de versão passa: é o que `server_tokens off` e `ServerTokens
Prod` produzem, e um cabeçalho ausente também passa. Um cabeçalho como
"nginx/1.25.3" ou "Apache/2.4.58 (Unix)" falha: ele diz exatamente a um
scanner quais CVEs tentar.

Fontes: [Nginx core module reference](https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens),
[Apache core module reference](https://httpd.apache.org/docs/2.4/mod/core.html#servertokens).

### Máquinas com o agente (e seus contêineres Docker) {#machines-with-the-agent-and-their-docker-containers}

| Verificação | Severidade | Dados |
|---|---|---|
| Atualizações de segurança instaladas | alta | atualizações de segurança pendentes (onde o gerenciador de pacotes os distingue) |
| Nenhuma atualização pendente | baixa | atualizações pendentes |
| Nenhum reinício pendente | média | reinício necessário |
| SELinux enforcing | média | modo SELinux (desconhecido onde SELinux está ausente) |
| Imagens de contêiner atualizadas | média | imagem mais nova no registro (só em máquinas com contêineres) |
| Nenhuma imagem de contêiner com mais de 180 dias | baixa | idade da imagem (só em máquinas com contêineres) |

Fontes: [CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index),
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often).

### Atualizações de fornecedor para outras integrações {#vendor-updates-for-other-integrations}

Uma verificação de atualização por produto, a partir da informação de
atualização que cada coletor já lê:

| Tipo | Verificação | Severidade |
|---|---|---|
| FortiGate | Patch FortiOS instalado | **crítica** |
| FortiGate | Licenças FortiGuard válidas | alta |
| MikroTik | RouterOS atualizado | alta |
| MikroTik | Firmware RouterBOOT corresponde ao RouterOS | baixa |
| Home Assistant | Componentes do Home Assistant atualizados | média |
| Nextcloud | Servidor Nextcloud atualizado | alta |
| Nextcloud | Apps Nextcloud atualizados | média |
| AdGuard Home | AdGuard Home atualizado | média |
| Pi-hole | Pi-hole atualizado | média |
| Plex | Plex Media Server atualizado | média |
| Paperless-ngx | Paperless-ngx atualizado | média |
| Tailscale | Clientes Tailscale atualizados | média |

### Active Directory {#active-directory}

Um dispositivo Active Directory é avaliado a partir dos achados estilo
PingCastle de seu coletor: contas Kerberoastable, contas sem
pré-autenticação Kerberos, delegação irrestrita, idade da senha
`krbtgt`, tamanho de grupo privilegiado, cobertura LAPS, política de
senha, contas inativas. Cada achado é uma verificação que passa quando
envolve zero objetos; a severidade que o coletor reporta é a usada.
Achados ainda não reportados são listados como desconhecidos.

Fontes: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview),
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password).

### GitLab {#gitlab}

Lido da API de administração, com um token de acesso pessoal de
administrador; desconhecido com um comum.

| Verificação | Severidade | Dados |
|---|---|---|
| Autenticação de dois fatores exigida | alta | configuração `require_two_factor_authentication` |
| Cadastro público desativado | média | configuração `signup_enabled` |
| Migrações de banco de dados aplicadas | média | migrações pendentes depois de uma atualização |

Fonte: [GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html).

### Forgejo e Gitea {#forgejo-and-gitea}

| Verificação | Severidade | Dados |
|---|---|---|
| Tarefas agendadas rodam no horário | média | tarefas cuja próxima execução já está atrasada |

Desconhecido sem um token de acesso de administrador do site: o Forgejo
e o Gitea reservam a lista de tarefas agendadas à administração.

Fonte: [Forgejo API](https://forgejo.org/docs/latest/user/api/).

## Limites {#limits}

A pontuação só reflete o que o DumbMonit pode ver. É um convite a olhar,
não uma auditoria, e uma nota A não significa que um dispositivo é
seguro.

Boas práticas que **não** são verificadas ainda porque os coletores não
leem os dados hoje:

| Tipo | Não verificado (precisa de dados novos) |
|---|---|
| Proxmox VE | firewall de datacenter/nó ativado; autenticação de dois fatores em `root@pam` e outras contas de admin |
| Proxmox Backup Server | criptografia do lado do cliente dos backups (precisa ler manifestos de backup) |
| Synology DSM | verificação em 2 passos para administradores; conta `admin` padrão desativada; bloqueio automático; firewall DSM; redirecionamento HTTP-para-HTTPS; exposição QuickConnect; se uma atualização DSM mais nova está disponível |
| OPNsense / pfSense | GUI web ou SSH acessível pela WAN; credenciais padrão; atualizações do sistema pfSense (não API REST) |
| UniFi | SSH do dispositivo com credenciais padrão; configurações do site |
| Monitores TLS | TLS 1.0/1.1 ou suítes de cifra fracas ainda oferecidas — a sondagem só negocia TLS 1.2 e 1.3, então um servidor limitado a versões mais antigas aparece como uma sondagem falhada em vez disso |
| Hosts com agente | status do firewall do host; configuração SSH; status do Windows Defender |
| Docker | contêineres rodando privileged, como root, ou com o socket Docker montado |

Outros limites:

- Uma verificação é só tão recente quanto seus dados: alguns coletores
  releem atualizações ou assinaturas a cada poucas horas.
- Os pesos são uma escolha de produto, a mesma para toda instalação.
- Tipos sem nenhuma verificação não têm cartão Segurança e não aparecem
  na página Segurança.
