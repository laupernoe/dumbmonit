# Pontuação de segurança

A pontuação de segurança é uma nota **opcional** de **A** a **F** (e uma
pontuação de 0 a 100) mostrada ao lado dos monitores em que algumas
verificações dizem algo útil: hosts Linux e Windows com o agente, Proxmox VE e
Proxmox Backup Server, Synology, OPNsense, sites e Active Directory. Os outros
monitores não mostram nada.

!!! warning "Indicativa, não é uma auditoria"
    A pontuação é **aproximada e não exaustiva**. Ela só lê o que o DumbMonit
    já coleta de um monitor e compara com um punhado de boas práticas do
    fabricante. Uma nota A não significa que um monitor seja seguro, e uma
    verificação ausente não significa que a configuração esteja correta. Use-a
    como um convite para olhar com atenção e, quando for importante, use uma
    ferramenta de avaliação de verdade (PingCastle, Mozilla HTTP Observatory,
    Lynis, o assistente de segurança do próprio fabricante…).

## Ativar

A pontuação vem **desativada por padrão**. Um administrador a ativa em
**Configurações → Conta e segurança → Pontuação de segurança**. Depois de
ativada:

- cada monitor pontuado recebe um pequeno **selo em forma de escudo** com
  sua letra na lista de monitores, e um selo **Security** com a pontuação na
  sua página;
- clicar no selo abre o **relatório** do monitor: o que falha, com a
  evidência e a correção, o que passa, os passos para melhorar a pontuação
  nesse tipo de monitor e um link para a seção correspondente abaixo.

Desativá-la oculta os selos, interrompe o cálculo periódico e as séries
`dumbmonit_security_score` deixam de ser gravadas.

!!! note "Atualização a partir da 0.1.0-alpha.8 ou anterior"
    A pontuação antes ficava sempre ativa, com uma página própria
    **Security**. Uma instalação que já monitorava um monitor pontuável
    mantém a pontuação ativada após a atualização; uma instalação nova começa
    com ela desativada. O antigo endereço `/security` agora abre a
    configuração.

## Como a pontuação é calculada

O DumbMonit nunca pede nada extra ao monitor para calcular a pontuação. As
verificações leem os últimos valores que os coletores já registraram (olhando
6 horas para trás), então uma pontuação não custa nada ao monitor.

Cada verificação tem uma categoria (exposição, correções, autenticação,
criptografia, backup, configuração), uma **severidade** que é o seu peso
(critical 10, high 6, medium 3, low 1), um resultado (**passa**, **falha** ou
**desconhecido** quando o dado não é coletado), a evidência, a correção e um
link para a página do fabricante em que se baseia.

**Pontuação** = 100 × (peso das verificações que passam) ÷ (peso das que
passam + falham). **As verificações desconhecidas são mostradas, mas nunca
contadas**: uma pontuação não deve cair porque uma conta de monitoramento não
tem um privilégio, nem subir porque nada se sabe. Um monitor cujas
verificações são todas desconhecidas *não é pontuado*.

| Nota | Pontuação | Palavra |
|---|---|---|
| A | 90–100 | Forte |
| B | 75–89 | Boa |
| C | 60–74 | Razoável |
| D | 40–59 | Fraca |
| F | 0–39 | Ruim |

**Uma verificação crítica que falha limita a nota a C**, qualquer que seja a
pontuação.

## Histórico e alertas

Enquanto a pontuação está ativada, o servidor a recalcula a cada 15 minutos e a
armazena como `dumbmonit_security_score{target="<id>"}` (e o número de
verificações que falham como `dumbmonit_security_checks_failed`). Duas regras
integradas a usam: **Security score dropped** (10 pontos ou mais abaixo do
melhor valor das últimas 24 horas) e **Security score low** (abaixo de 40).
Veja [Regras de alerta](../alerting/rules.md#security-score).

## Verificações e como melhorá-las

### Hosts Linux { #linux-hosts }

Máquinas com o agente. As verificações de SSH e de firewall precisam de um
agente da mesma versão do servidor; com um agente mais antigo ficam
desconhecidas.

| Verificação | Severidade | Dado |
|---|---|---|
| Security updates installed | high | atualizações de segurança pendentes (onde o gerenciador de pacotes as distingue) |
| No pending updates | low | atualizações pendentes |
| No reboot pending | medium | reinicialização necessária |
| SSH root login with a password refused | high | `PermitRootLogin` de `sshd -T` (`prohibit-password` passa) |
| SSH accepts keys only | medium | `PasswordAuthentication`, e `KbdInteractiveAuthentication` com `UsePAM` |
| Host firewall filters inbound traffic | high | uma cadeia input de nftables ou iptables que descarta ou rejeita |
| SELinux enforcing | medium | modo do SELinux (desconhecido onde o SELinux não existe) |
| Container images up to date | medium | imagem mais recente no registro (só com contêineres) |
| No container image older than 180 days | low | idade da imagem (só com contêineres) |

Para melhorá-la:

1. Instale as atualizações de segurança pendentes e depois ative as
   atualizações de segurança automáticas: `unattended-upgrades` no Debian e no
   Ubuntu, `dnf-automatic` no Fedora e no RHEL.
2. Reinicie após atualizações do kernel ou de bibliotecas.
3. Verifique se sua chave SSH funciona e depois defina `PermitRootLogin no`,
   `PasswordAuthentication no` e `KbdInteractiveAuthentication no` em
   `/etc/ssh/sshd_config` e recarregue o sshd.
4. Ative um firewall de host que descarte o tráfego de entrada não solicitado:
   `ufw enable` no Ubuntu, firewalld no Fedora e no RHEL, um conjunto de regras
   nftables nos demais.
5. Mantenha o SELinux em enforcing onde a distribuição o inclui; baixe
   regularmente imagens de contêineres mais recentes.

Fontes: [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Hosts Windows { #windows-hosts }

Máquinas com o agente Windows, da mesma versão do servidor (lido do registro;
um agente mais antigo não reporta nenhuma destas).

| Verificação | Severidade | Dado |
|---|---|---|
| Windows Firewall on for every profile | high | perfis de domínio, privado e público (política de grupo primeiro) |
| No reboot pending | medium | Windows Update ou manutenção de componentes aguardando uma reinicialização |
| Remote Desktop requires Network Level Authentication | high | passa quando a Área de Trabalho Remota está desativada |
| SMBv1 disabled | high | configuração do servidor SMBv1 |

Para melhorá-la:

1. Instale as atualizações do Windows (Configurações → Windows Update) e
   reinicie quando solicitado.
2. Mantenha o Firewall do Windows Defender ativado nos perfis de domínio,
   privado e público.
3. Se a Área de Trabalho Remota estiver ativada, exija a autenticação em nível
   de rede (NLA) e acesse-a por uma VPN, nunca diretamente da internet.
4. Remova o SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Fontes: [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE { #proxmox-ve }

| Verificação | Severidade | Dado |
|---|---|---|
| Security updates installed | high | atualizações de segurança pendentes por nó (precisa de `Sys.Modify` no token, senão desconhecida) |
| Proxmox VE packages up to date | medium | pacotes Proxmox atualizáveis por nó |
| Running the installed kernel | medium | kernel instalado mais recente que o em execução |
| No test repository in use | medium | repositórios padrão habilitados |
| Package repositories correctly configured | medium | erros e avisos que o Proxmox VE levanta sobre os repositórios |
| Enterprise repository subscription | low | status da assinatura por nó |
| Web interface certificate valid for 14+ days | medium | expiração do certificado do nó |
| Every guest is in a backup job | high | convidados não selecionados por nenhum job de backup |
| Backup jobs succeed | high | último resultado de cada job de backup |

Para melhorá-la:

1. Use um repositório ao qual você tem direito: enterprise com assinatura, caso
   contrário `pve-no-subscription`; nunca `pvetest` em um nó que importe.
2. Instale as atualizações pelo painel **Updates** do nó e depois reinicie com
   o novo kernel.
3. Coloque cada VM e contêiner em um job de backup, de preferência para um
   Proxmox Backup Server, e corrija qualquer job que falhe.
4. Substitua o certificado web autoassinado por um ACME.

Fontes: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server { #proxmox-backup-server }

| Verificação | Severidade | Dado |
|---|---|---|
| System updates installed | high | atualizações pendentes (precisa de `Sys.Modify` em `/system`) |
| Running the installed Backup Server version | medium | daemons que ainda executam a versão anterior à atualização |
| Verify job scheduled | high | número de jobs de verificação |
| Prune job scheduled | medium | número de jobs de prune |
| Backups copied elsewhere (sync or tape) | medium | jobs de sincronização e de backup em fita (regra 3-2-1) |
| Garbage collection clean, no bad chunks | high | último resultado do GC e chunks defeituosos por datastore |
| Certificate valid for 14+ days | medium | expiração do certificado |

Para melhorá-la:

1. Instale as atualizações e reinicie quando o kernel ou o `proxmox-backup`
   mudaram.
2. Agende um job de verificação e um de prune em cada datastore.
3. Mantenha uma cópia em outro lugar: um job de sincronização para um segundo
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
| Auto Block enabled | medium | configuração do Auto Block (só conta de administrador) |
| DSM firewall enabled | medium | configuração do firewall (só conta de administrador) |
| Hyper Backup task configured | medium | número de tarefas do Hyper Backup |
| Hyper Backup tasks succeed | high | último resultado de cada tarefa |

As quatro configurações são lidas uma vez por hora e só quando o DumbMonit
entra com uma conta de administrador; com uma conta comum ficam desconhecidas e
não contam. A verificação em duas etapas e o redirecionamento para HTTPS não
são lidos.

Para melhorá-la:

1. Execute a última versão do DSM 7 para o seu modelo e deixe o DSM instalar
   automaticamente as atualizações importantes (Painel de Controle →
   Atualização e Restauração).
2. Desative a conta integrada `admin`; entre com um administrador pessoal
   protegido por verificação em duas etapas.
3. Ative o Auto Block e o firewall (Painel de Controle → Segurança) e não
   encaminhe portas do DSM para a internet: use uma VPN.
4. Faça backup com o Hyper Backup para outro monitor ou um destino na
   nuvem e corrija qualquer tarefa que falhe: RAID e snapshots não são
   backups.

Fontes: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense { #opnsense }

| Verificação | Severidade | Dado |
|---|---|---|
| Firmware updates installed | high | atualizações de pacotes pendentes (desconhecida até o OPNsense verificar atualizações) |
| On the current major release series | medium | nova série principal disponível |
| No reboot pending after an update | medium | reinicialização necessária |
| Packet filter enabled | **critical** | estado do `pf` |

Para melhorá-la:

1. Verifique atualizações em System → Firmware → Status, instale-as e reinicie
   quando solicitado.
2. Passe para a nova versão principal quando ela sair.
3. Mantenha o filtro de pacotes ativado.
4. Permita a interface web e o SSH apenas pela LAN ou por uma VPN, nunca pela
   WAN.

Fontes: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Sites { #websites }

Para os monitores **HTTP(S)** e **TLS** que negociam TLS, e para os
monitores **Website health & SEO**. Um monitor HTTP simples não tem
pontuação.

| Verificação | Severidade | Dado |
|---|---|---|
| Certificate valid for 14+ days | high | dias até a expiração (monitores HTTP(S) e TLS) |
| Certificate chain trusted | high | verificação da cadeia (monitores HTTP(S) e TLS) |
| TLS 1.3 negotiated | low | versão do protocolo negociada (monitores HTTP(S) e TLS) |
| Served over HTTPS | high | endereço final após redirecionamentos (Website health) |
| Strict-Transport-Security header | medium | cabeçalho com um `max-age` acima de 0 |
| X-Content-Type-Options: nosniff | low | valor do cabeçalho |
| Protected against clickjacking | low | `X-Frame-Options` ou `frame-ancestors` da CSP |
| Content-Security-Policy header | low | uma política aplicada (report-only não conta) |

Os cabeçalhos só são lidos em respostas HTTPS.

Para melhorá-la:

1. Sirva o site apenas por HTTPS e redirecione o HTTP simples para HTTPS.
2. Automatize a renovação do certificado (ACME, p. ex. Let's Encrypt) e sirva a
   cadeia completa.
3. Adicione cabeçalhos de segurança no servidor web ou no proxy reverso:
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff` e uma
   `Content-Security-Policy` com `frame-ancestors`.
4. Confronte com o [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory),
   que testa muito mais do que esta pontuação.

Fontes: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory { #active-directory }

Pontuado a partir dos achados no estilo PingCastle do coletor do Active
Directory: contas kerberoastable, contas sem pré-autenticação Kerberos,
delegação irrestrita, idade da senha do `krbtgt`, tamanho dos grupos
privilegiados, cobertura do LAPS, política de senhas, contas obsoletas. Cada
achado é uma verificação que passa quando diz respeito a zero objetos.

Para melhorá-la:

1. Corrija primeiro os achados críticos e altos: são os caminhos habituais até
   os Domain Admins.
2. Se a senha do `krbtgt` for antiga, redefina-a duas vezes, com algumas horas
   de intervalo, e remova a delegação irrestrita dos computadores que não são
   controladores de domínio.
3. Passe os serviços executados com contas de usuário com SPN para contas de
   serviço gerenciadas de grupo (gMSA).
4. Execute o [PingCastle](https://www.pingcastle.com/) para uma avaliação
   completa.

Fontes: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Limites

- A pontuação reflete apenas o que o DumbMonit consegue ver; muitas boas
  práticas não são verificadas porque nenhum coletor lê o dado: firewall e
  autenticação de dois fatores do Proxmox, interface web do OPNsense exposta na
  WAN, contêineres Docker executados com privilégios ou com o socket do Docker
  montado, TLS 1.0/1.1 ainda oferecidos (a sonda só negocia TLS 1.2 e 1.3), e
  muitas outras.
- Uma verificação é tão atual quanto os seus dados: alguns coletores só relêem
  as atualizações a cada poucas horas.
- Os pesos são uma escolha de produto, igual para todas as instalações.
- Só os tipos acima são pontuados. Os outros (switches, impressoras, UniFi,
  pfSense…) não mostram selo de propósito: duas ou três verificações dariam uma
  nota sem significado.
