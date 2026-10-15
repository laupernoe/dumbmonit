# Pontuação de segurança

A pontuação de segurança é uma classificação **opcional** de **A** a **F** (e
uma pontuação de 0 a 100) mostrada junto dos dispositivos em que algumas
verificações dizem algo útil: hosts Linux e Windows com o agente, Proxmox VE e
Proxmox Backup Server, Synology, OPNsense, sites web e Active Directory. Os
outros dispositivos não mostram nada.

!!! warning "Indicativa, não é uma auditoria"
    A pontuação é **aproximada e não exaustiva**. Lê apenas o que o DumbMonit
    já recolhe de um dispositivo e compara-o com um punhado de boas práticas do
    fabricante. Uma classificação A não significa que um dispositivo seja
    seguro, e uma verificação em falta não significa que a definição esteja
    correta. Use-a como um convite a olhar com atenção e, quando for
    importante, use uma verdadeira ferramenta de avaliação (PingCastle, Mozilla
    HTTP Observatory, Lynis, o assistente de segurança do próprio fabricante…).

## Ativá-la

A pontuação está **desativada por predefinição**. Um administrador ativa-a em
**Definições → Conta e segurança → Pontuação de segurança**. Depois de ativada:

- cada dispositivo pontuado recebe um pequeno **distintivo em forma de escudo**
  com a sua letra na lista de dispositivos, e um distintivo **Security** com a
  pontuação na sua página;
- clicar no distintivo abre o **relatório** do dispositivo: o que falha, com a
  evidência e a correção, o que passa, os passos para melhorar a pontuação
  neste tipo de dispositivo e uma ligação para a secção correspondente abaixo.

Desativá-la oculta os distintivos, pára o cálculo periódico e as séries
`dumbmonit_security_score` deixam de ser escritas.

!!! note "Atualização a partir da 0.1.0-alpha.8 ou anterior"
    A pontuação estava antes sempre ativa, com a sua própria página
    **Security**. Uma instalação que já monitorizava um dispositivo pontuável
    mantém a pontuação ativada após a atualização; uma instalação nova começa
    com ela desativada. O antigo endereço `/security` abre agora a definição.

## Como a pontuação é calculada

O DumbMonit nunca pede nada extra ao dispositivo para calcular a pontuação. As
verificações leem os últimos valores que os coletores já registaram (olhando 6
horas para trás), pelo que uma pontuação não custa nada ao dispositivo.

Cada verificação tem uma categoria (exposição, correções, autenticação,
cifragem, cópia de segurança, configuração), uma **severidade** que é o seu
peso (critical 10, high 6, medium 3, low 1), um resultado (**passa**,
**falha** ou **desconhecido** quando o dado não é recolhido), a evidência, a
correção e uma ligação para a página do fabricante em que se baseia.

**Pontuação** = 100 × (peso das verificações que passam) ÷ (peso das que
passam + falham). **As verificações desconhecidas são mostradas mas nunca
contadas**: uma pontuação não deve baixar porque a uma conta de monitorização
falta um privilégio, nem subir porque nada se sabe. Um dispositivo cujas
verificações são todas desconhecidas *não é pontuado*.

| Classificação | Pontuação | Palavra |
|---|---|---|
| A | 90–100 | Forte |
| B | 75–89 | Boa |
| C | 60–74 | Razoável |
| D | 40–59 | Fraca |
| F | 0–39 | Má |

**Uma verificação crítica que falha limita a classificação a C**, seja qual
for a pontuação.

## Histórico e alertas

Enquanto a pontuação está ativada, o servidor recalcula-a a cada 15 minutos e
guarda-a como `dumbmonit_security_score{target="<id>"}` (e o número de
verificações que falham como `dumbmonit_security_checks_failed`). Duas regras
integradas usam-na: **Security score dropped** (10 pontos ou mais abaixo do
melhor valor das últimas 24 horas) e **Security score low** (abaixo de 40).
Veja [Regras de alerta](../alerting/rules.md#security-score).

## Verificações e como melhorá-las

### Hosts Linux { #linux-hosts }

Máquinas com o agente. As verificações de SSH e da firewall precisam de um
agente da mesma versão do servidor; com um agente mais antigo são
desconhecidas.

| Verificação | Severidade | Dado |
|---|---|---|
| Security updates installed | high | atualizações de segurança pendentes (onde o gestor de pacotes as distingue) |
| No pending updates | low | atualizações pendentes |
| No reboot pending | medium | reinício necessário |
| SSH root login with a password refused | high | `PermitRootLogin` de `sshd -T` (`prohibit-password` passa) |
| SSH accepts keys only | medium | `PasswordAuthentication`, e `KbdInteractiveAuthentication` com `UsePAM` |
| Host firewall filters inbound traffic | high | uma cadeia input de nftables ou iptables que descarta ou rejeita |
| SELinux enforcing | medium | modo do SELinux (desconhecido onde o SELinux não existe) |
| Container images up to date | medium | imagem mais recente no registo (só com contentores) |
| No container image older than 180 days | low | idade da imagem (só com contentores) |

Para a melhorar:

1. Instale as atualizações de segurança pendentes e depois ative as
   atualizações de segurança automáticas: `unattended-upgrades` no Debian e no
   Ubuntu, `dnf-automatic` no Fedora e no RHEL.
2. Reinicie após atualizações do kernel ou de bibliotecas.
3. Verifique se a sua chave SSH funciona e depois defina `PermitRootLogin no`,
   `PasswordAuthentication no` e `KbdInteractiveAuthentication no` em
   `/etc/ssh/sshd_config` e recarregue o sshd.
4. Ative uma firewall de host que descarte o tráfego de entrada não solicitado:
   `ufw enable` no Ubuntu, firewalld no Fedora e no RHEL, um conjunto de regras
   nftables nos restantes.
5. Mantenha o SELinux em enforcing onde a distribuição o inclui; obtenha
   regularmente imagens de contentores mais recentes.

Fontes: [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Hosts Windows { #windows-hosts }

Máquinas com o agente Windows, da mesma versão do servidor (lido do registo; um
agente mais antigo não reporta nenhuma destas).

| Verificação | Severidade | Dado |
|---|---|---|
| Windows Firewall on for every profile | high | perfis de domínio, privado e público (primeiro a política de grupo) |
| No reboot pending | medium | Windows Update ou manutenção de componentes à espera de um reinício |
| Remote Desktop requires Network Level Authentication | high | passa quando o Ambiente de Trabalho Remoto está desativado |
| SMBv1 disabled | high | definição do servidor SMBv1 |

Para a melhorar:

1. Instale as atualizações do Windows (Definições → Windows Update) e reinicie
   quando for pedido.
2. Mantenha a Firewall do Windows Defender ativada nos perfis de domínio,
   privado e público.
3. Se o Ambiente de Trabalho Remoto estiver ativado, exija a autenticação ao
   nível da rede (NLA) e aceda-lhe por uma VPN, nunca diretamente a partir da
   internet.
4. Remova o SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Fontes: [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE { #proxmox-ve }

| Verificação | Severidade | Dado |
|---|---|---|
| Security updates installed | high | atualizações de segurança pendentes por nó (precisa de `Sys.Modify` no token, senão desconhecida) |
| Proxmox VE packages up to date | medium | pacotes Proxmox atualizáveis por nó |
| Running the installed kernel | medium | kernel instalado mais recente do que o em execução |
| No test repository in use | medium | repositórios standard ativados |
| Package repositories correctly configured | medium | erros e avisos que o Proxmox VE levanta sobre os repositórios |
| Enterprise repository subscription | low | estado da subscrição por nó |
| Web interface certificate valid for 14+ days | medium | expiração do certificado do nó |
| Every guest is in a backup job | high | convidados não selecionados por nenhuma tarefa de cópia |
| Backup jobs succeed | high | último resultado de cada tarefa de cópia |

Para a melhorar:

1. Use um repositório a que tenha direito: enterprise com subscrição, caso
   contrário `pve-no-subscription`; nunca `pvetest` num nó que importe.
2. Instale as atualizações a partir do painel **Updates** do nó e depois
   reinicie com o novo kernel.
3. Ponha cada VM e contentor numa tarefa de cópia, idealmente para um Proxmox
   Backup Server, e corrija qualquer tarefa que falhe.
4. Substitua o certificado web autoassinado por um ACME.

Fontes: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server { #proxmox-backup-server }

| Verificação | Severidade | Dado |
|---|---|---|
| System updates installed | high | atualizações pendentes (precisa de `Sys.Modify` em `/system`) |
| Running the installed Backup Server version | medium | daemons que ainda executam a versão anterior à atualização |
| Verify job scheduled | high | número de tarefas de verificação |
| Prune job scheduled | medium | número de tarefas de prune |
| Backups copied elsewhere (sync or tape) | medium | tarefas de sincronização e de cópia em fita (regra 3-2-1) |
| Garbage collection clean, no bad chunks | high | último resultado do GC e chunks defeituosos por datastore |
| Certificate valid for 14+ days | medium | expiração do certificado |

Para a melhorar:

1. Instale as atualizações e reinicie quando o kernel ou o `proxmox-backup`
   mudaram.
2. Agende uma tarefa de verificação e uma de prune em cada datastore.
3. Mantenha uma cópia noutro local: uma tarefa de sincronização para um segundo
   Backup Server, ou fita.
4. Use um certificado ACME para a interface web.

Fontes: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html).

### Synology { #synology }

| Verificação | Severidade | Dado |
|---|---|---|
| DSM 7 or later | high | versão do DSM |
| DSM up to date | high | há uma atualização do DSM disponível (só conta de administrador) |
| Built-in admin account disabled | medium | estado da conta `admin` (só conta de administrador) |
| Auto Block enabled | medium | definição do Auto Block (só conta de administrador) |
| DSM firewall enabled | medium | definição da firewall (só conta de administrador) |
| Hyper Backup task configured | medium | número de tarefas do Hyper Backup |
| Hyper Backup tasks succeed | high | último resultado de cada tarefa |

As quatro definições são lidas uma vez por hora e só quando o DumbMonit inicia
sessão com uma conta de administrador; com uma conta normal ficam
desconhecidas e não contam. A verificação em dois passos e o redirecionamento
para HTTPS não são lidos.

Para a melhorar:

1. Execute a última versão do DSM 7 para o seu modelo e deixe o DSM instalar
   automaticamente as atualizações importantes (Painel de controlo →
   Atualização e restauro).
2. Desative a conta integrada `admin`; inicie sessão com um administrador
   pessoal protegido por verificação em dois passos.
3. Ative o Auto Block e a firewall (Painel de controlo → Segurança) e não
   encaminhe portas do DSM para a internet: use uma VPN.
4. Faça cópias com o Hyper Backup para outro dispositivo ou um destino na
   nuvem e corrija qualquer tarefa que falhe: RAID e instantâneos não são
   cópias de segurança.

Fontes: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense { #opnsense }

| Verificação | Severidade | Dado |
|---|---|---|
| Firmware updates installed | high | atualizações de pacotes pendentes (desconhecida até o OPNsense ter procurado atualizações) |
| On the current major release series | medium | nova série principal disponível |
| No reboot pending after an update | medium | reinício necessário |
| Packet filter enabled | **critical** | estado do `pf` |

Para a melhorar:

1. Procure atualizações em System → Firmware → Status, instale-as e reinicie
   quando for pedido.
2. Passe para a nova versão principal quando sair.
3. Mantenha o filtro de pacotes ativado.
4. Permita a interface web e o SSH apenas a partir da LAN ou de uma VPN, nunca
   a partir da WAN.

Fontes: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Sites web { #websites }

Para os monitores **HTTP(S)** e **TLS** que negociam TLS, e para os
dispositivos **Website health & SEO**. Um monitor HTTP simples não tem
pontuação.

| Verificação | Severidade | Dado |
|---|---|---|
| Certificate valid for 14+ days | high | dias até à expiração (monitores HTTP(S) e TLS) |
| Certificate chain trusted | high | verificação da cadeia (monitores HTTP(S) e TLS) |
| TLS 1.3 negotiated | low | versão do protocolo negociada (monitores HTTP(S) e TLS) |
| Served over HTTPS | high | endereço final após redirecionamentos (Website health) |
| Strict-Transport-Security header | medium | cabeçalho com um `max-age` superior a 0 |
| X-Content-Type-Options: nosniff | low | valor do cabeçalho |
| Protected against clickjacking | low | `X-Frame-Options` ou `frame-ancestors` da CSP |
| Content-Security-Policy header | low | uma política aplicada (report-only não conta) |

Os cabeçalhos só são lidos em respostas HTTPS.

Para a melhorar:

1. Sirva o site apenas por HTTPS e redirecione o HTTP simples para HTTPS.
2. Automatize a renovação do certificado (ACME, p. ex. Let's Encrypt) e sirva a
   cadeia completa.
3. Adicione cabeçalhos de segurança no servidor web ou no proxy inverso:
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff` e uma
   `Content-Security-Policy` com `frame-ancestors`.
4. Confronte com o [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory),
   que testa muito mais do que esta pontuação.

Fontes: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory { #active-directory }

Pontuado a partir dos achados ao estilo PingCastle do coletor do Active
Directory: contas kerberoastable, contas sem pré-autenticação Kerberos,
delegação sem restrições, idade da palavra-passe do `krbtgt`, tamanho dos
grupos privilegiados, cobertura do LAPS, política de palavras-passe, contas
obsoletas. Cada achado é uma verificação que passa quando diz respeito a zero
objetos.

Para a melhorar:

1. Corrija primeiro os achados críticos e altos: são os caminhos habituais para
   os Domain Admins.
2. Se a palavra-passe do `krbtgt` for antiga, reponha-a duas vezes, com umas
   horas de intervalo, e remova a delegação sem restrições dos computadores que
   não sejam controladores de domínio.
3. Passe os serviços que correm com contas de utilizador com SPN para contas de
   serviço geridas de grupo (gMSA).
4. Execute o [PingCastle](https://www.pingcastle.com/) para uma avaliação
   completa.

Fontes: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Limites

- A pontuação reflete apenas o que o DumbMonit consegue ver; muitas boas
  práticas não são verificadas porque nenhum coletor lê o dado: firewall e
  autenticação de dois fatores do Proxmox, interface web do OPNsense exposta na
  WAN, contentores Docker a correr com privilégios ou com o socket do Docker
  montado, TLS 1.0/1.1 ainda oferecidos (a sonda só negoceia TLS 1.2 e 1.3), e
  muitas mais.
- Uma verificação é tão atual quanto os seus dados: alguns coletores só relêem
  as atualizações de poucas em poucas horas.
- Os pesos são uma escolha de produto, igual para todas as instalações.
- Só os tipos acima são pontuados. Os outros (switches, impressoras, UniFi,
  pfSense…) não mostram distintivo de propósito: duas ou três verificações
  dariam uma classificação sem significado.
