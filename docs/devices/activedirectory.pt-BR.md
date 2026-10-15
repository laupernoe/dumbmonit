# Active Directory {#active-directory}

Um domínio Windows via LDAP, somente leitura: os controladores de domínio e
quem detém cada papel FSMO, os membros dos grupos privilegiados, a
política de senha, contagens de contas, e os fatos de segurança brutos que
uma auditoria procura primeiro — contas abertas a Kerberoasting ou AS-REP
roasting, delegação irrestrita, uma senha krbtgt antiga, computadores sem
LAPS, versões do Windows fora de suporte.

O DumbMonit se conecta com uma conta de serviço dedicada que é um
**usuário de domínio simples, membro de nenhum grupo**. No Active
Directory todo usuário autenticado pode ler o que o DumbMonit lê; nenhum
direito administrativo é necessário, e nenhum deveria ser dado. O
DumbMonit nunca escreve no diretório, nunca lê um atributo de senha (do
LAPS lê a data de expiração, nunca a senha) e nunca pede os direitos de
replicação que exporiam segredos.

Um monitor cobre todo o domínio: aponte-o para um controlador de
domínio e o DumbMonit encontra os outros na partição de configuração.

## O que o LDAP pode e não pode dizer {#what-ldap-can-and-cannot-tell}

O LDAP mostra o diretório como está, não os logs do que aconteceu nele. Em
particular:

- **Replicação.** O controlador de domínio consultado publica o estado de
  seus parceiros de replicação de entrada (último sucesso, falhas
  consecutivas, último erro) apenas para uma conta que possui o direito
  *Monitor active directory replication*. Sem ele, o painel diz *status
  de replicação não legível* em vez de adivinhar. Só a visão do
  controlador de domínio consultado é mostrada: para ver os parceiros de
  outro, adicione-o como um segundo monitor.
- **Não visível via LDAP**: logs de eventos (logins, bloqueios conforme
  acontecem, mudanças de grupo como eventos), conteúdo de Group Policy,
  SYSVOL, zonas DNS, e os próprios hashes de senha.
- **Datas de último login** vêm de `lastLogonTimestamp`, que
  controladores de domínio replicam apenas a cada 9 a 14 dias: uma conta
  contada como inativa é inativa dentro de duas semanas.
- **Membros de grupo** são lidos com aninhamento (a regra
  `LDAP_MATCHING_RULE_IN_CHAIN`), via `member`/`memberOf`. Um grupo
  primário definido por `primaryGroupID` não é um link `member` e não é
  visto.

## Como ele lê {#how-it-reads}

Dois ritmos:

- **A cada verificação** (cada minuto por padrão): conexão e bind, o
  RootDSE (níveis funcionais, relógio, sincronização), o objeto de
  domínio (política de senha, cota de conta de máquina), os cinco papéis
  FSMO, os controladores de domínio da partição de configuração, os
  grupos privilegiados e seus membros efetivos, o recurso Recycle Bin, e
  uma conexão TCP com a porta LDAP de cada outro controlador de domínio.
- **Em segundo plano** a cada `inventory_minutes` (15 por padrão): toda
  conta de usuário e computador, apenas com os atributos que os achados
  precisam, em páginas de 500. Enumerar todo o diretório a cada minuto
  sobrecarregaria os controladores de domínio para nada.

A primeira verificação espera alguns segundos pelo inventário; em um
domínio grande os achados de conta aparecem na próxima verificação.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_ad_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `bind_ok` | 1 quando a conta de serviço é aceita, 0 quando o controlador de domínio a recusa | |
| `connect_seconds`, `bind_seconds` | tempo para abrir a conexão (TLS incluído) e para o bind | |
| `tls_verified` | 1 quando o certificado é verificado | |
| `domain_info` | valor 1 | `domain`, `forest`, `dc` |
| `domain_functional_level`, `forest_functional_level` | 0 (2000) a 7 (2016), 10 (2025) | |
| `dc_synchronized`, `dc_global_catalog_ready` | o controlador de domínio consultado terminou sua replicação inicial, seu catálogo global está pronto | |
| `clock_skew_seconds` | relógio do controlador de domínio menos relógio do host DumbMonit | |
| `recycle_bin_enabled` | 1 quando o AD Recycle Bin está ativado | |
| `dc_count` | controladores de domínio do domínio | |
| `dc_info` | valor 1 | `dc`, `site`, `gc`, `rodc`, `os` |
| `dc_reachable` | 1 ou 0 por controlador de domínio; nenhuma série quando seu nome não resolve a partir do DumbMonit | `dc` |
| `fsmo_role` | valor 1 | `role` (`schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`), `holder` |
| `replication_readable` | 1 quando o status de replicação de entrada pode ser lido | |
| `replication_consecutive_failures`, `replication_last_success_age_seconds` | por parceiro de entrada e partição | `source`, `naming_context` |
| `privileged_group_members`, `privileged_group_enabled_members` | membros efetivos, aninhamento incluído, grupos deixados de fora | `group` |
| `privileged_group_fingerprint` | um hash dos membros: muda quando alguém entra ou sai | `group` |
| `users_total`, `users_enabled`, `users_disabled`, `users_locked_out`, `users_password_never_expires`, `users_stale` | contas de usuário | |
| `computers_total`, `computers_enabled`, `computers_stale`, `groups_total` | computadores e grupos | |
| `admincount_accounts` | usuários protegidos pelo AdminSDHolder (`adminCount=1`) | |
| `krbtgt_password_age_seconds` | idade da senha krbtgt | |
| `laps_schema`, `laps_computers`, `laps_eligible_computers` | LAPS no esquema; computadores ativados que não são DC com uma expiração LAPS, de todos eles | |
| `password_min_length`, `password_history_length`, `password_max_age_seconds` (0 = nunca), `password_complexity`, `lockout_threshold`, `machine_account_quota` | política de domínio padrão | |
| `finding_count` | objetos envolvidos em um achado, 0 quando examinado e limpo | `finding`, `severity`, `title` |
| `inventory_age_seconds`, `scrape_errors` | idade do último inventário completo; partes do diretório que não puderam ser lidas | |

Os grupos privilegiados são encontrados por seu identificador conhecido,
não seu nome, então um domínio instalado em outro idioma funciona da mesma
forma: Domain Admins (RID 512), Enterprise Admins (519) e Schema Admins
(518) — os dois últimos apenas no domínio raiz da floresta — e os
embutidos Administrators, Account Operators, Server Operators, Print
Operators e Backup Operators.

## Achados {#findings}

Achados são fatos brutos — quantos objetos, e quais — não um score; a
[pontuação de segurança](../using/security-score.md) do monitor os
pesa. Cada um tem um `id` estável, uma severidade na escala do score
(low, medium, high, critical, mostrada na página como Info, Aviso e
Alerta), uma categoria, uma contagem e até vinte nomes de conta de
amostra. Um achado que foi examinado e nada encontrou é mantido com uma
contagem de 0, então um problema corrigido fica verde em vez de
desaparecer; achados sobre contas aparecem uma vez que o primeiro
inventário em segundo plano terminou. São servidos como estão por
`GET /api/targets/{id}/ad/findings` e publicados como
`dumbmonit_ad_finding_count{finding, severity, title}`.

| `id` | Severidade | O que é |
|---|---|---|
| `unconstrained_delegation` | crítico | contas ativadas confiáveis para delegação irrestrita, exceto controladores de domínio |
| `kerberoastable_privileged` | crítico | membros ativados de grupos privilegiados com um nome principal de serviço |
| `asrep_roastable_users` | alto | contas ativadas com pré-autenticação Kerberos desativada (`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | alto | usuários ativados com um nome principal de serviço |
| `krbtgt_password_age` | alto | senha krbtgt com mais de 180 dias |
| `laps_coverage` | alto quando LAPS está ausente, médio quando parcial | computadores ativados (exceto controladores de domínio) sem uma senha LAPS |
| `reversible_encryption` | alto | usuários cuja senha é armazenada com criptografia reversível |
| `guest_enabled` | alto | a conta embutida Guest (RID 501) está ativada |
| `obsolete_os` | alto | computadores ativados no Windows XP, Vista, 7, 8, 2000, Server 2003, 2008 ou 2012 |
| `dc_unreachable` | alto | controladores de domínio cuja porta LDAP não responde |
| `replication_failing` | alto | parceiros de replicação de entrada com falhas consecutivas |
| `dc_not_synchronized` | alto | o controlador de domínio consultado não terminou sua replicação inicial |
| `fsmo_role_orphaned` | alto | um papel FSMO mantido por um controlador de domínio excluído |
| `password_policy` | alto abaixo de 8 caracteres ou sem bloqueio, médio caso contrário | lacunas na política de domínio padrão contra a linha de base: 14 caracteres, um limite de bloqueio, complexidade |
| `privileged_group_size` | médio | mais de 5 contas ativadas em Domain Admins, Enterprise Admins e Administrators juntos |
| `stale_accounts` | médio | usuários e computadores ativados inativos por mais que os limites |
| `protocol_transition_delegation` | médio | delegação restrita com transição de protocolo |
| `des_only` | médio | contas restritas a chaves Kerberos DES |
| `password_not_required` | médio | usuários permitidos uma senha vazia (`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | médio | usuários privilegiados cuja senha nunca expira |
| `privileged_stale` | médio | usuários privilegiados inativos por mais que `stale_days_users` |
| `schema_admins_not_empty` | médio | Schema Admins tem membros |
| `functional_level_old` | médio | nível funcional do domínio abaixo de 2012 R2 |
| `clock_skew` | médio | mais de cinco minutos entre o controlador de domínio e o DumbMonit |
| `windows10_end_of_support` | médio | computadores ativados no Windows 10 |
| `admincount_orphans` | baixo | usuários `adminCount=1` que não estão mais em um grupo privilegiado |
| `password_never_expires` | baixo | usuários ativados cuja senha nunca expira |
| `machine_account_quota` | baixo | qualquer usuário pode associar computadores ao domínio |
| `recycle_bin_disabled` | baixo | o AD Recycle Bin não está ativado |

As [regras integradas](../alerting/rules.md#active-directory) que se
aplicam:

- **Domain controller unreachable**: outro controlador de domínio do
  domínio para de responder em sua porta LDAP por cinco minutos
  (Warning). O consultado é cobertor por *Monitor unreachable*.
- **Active Directory bind failed**: o controlador de domínio recusa a
  conta de serviço — senha expirada, desativada ou bloqueada (Advisory).
- **Privileged group membership changed**: alguém entrou ou saiu de um
  grupo privilegiado, aninhamento incluído (Advisory, resolve uma hora
  depois).
- **Active Directory replication failing**: um parceiro de entrada falha
  por meia hora (Advisory), quando o status de replicação é legível.
- **krbtgt password older than 180 days** (Info).

## A página do monitor {#the-device-page}

O painel Active Directory mostra o domínio (níveis funcionais, o
controlador de domínio consultado, desvio de relógio), os controladores
de domínio com seu site, papéis e se respondem, os grupos privilegiados
com seus membros, a política de senha e contagens de conta, e a tabela de
achados com palavras de severidade.

## Criar uma conta de serviço somente leitura para o DumbMonit {#create-a-read-only-service-account-for-dumbmonit}

1. Crie um usuário dedicado para o DumbMonit, por exemplo
   svc-dumbmonit, e não o adicione a nenhum grupo. Um usuário de domínio
   simples é suficiente: todo usuário autenticado pode ler o que o
   DumbMonit lê. No PowerShell, com o módulo Active Directory:

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. Dê a ele uma senha longa e aleatória. Se senhas expiram no seu
   domínio, o DumbMonit dispara Active Directory bind failed no dia em
   que esta expirar.

3. Opcional: para ver o status de replicação de entrada, conceda à conta
   o direito Monitor active directory replication no próprio objeto de
   domínio. Isso não revela nenhuma senha ou segredo.

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. O LDAPS precisa de um certificado em cada controlador de domínio; uma
   autoridade de certificação corporativa (AD CS) emite um
   automaticamente. Exporte o certificado dessa autoridade em formato
   X.509 codificado em Base-64 e cole-o como certificado CA abaixo.

5. No DumbMonit, informe o nome de um controlador de domínio como seu
   certificado o nomeia, por exemplo "dc1.corp.example.com", e entre como
   svc-dumbmonit@corp.example.com. O DumbMonit encontra os outros
   controladores de domínio por conta própria e verifica que cada um
   responde.

6. O DumbMonit só lê: ele nunca escreve no diretório, e do LAPS lê a data
   de expiração, nunca a senha.

!!! warning
    LDAP simples na porta 389 envia a senha da conta de serviço em texto
    claro: o DumbMonit a recusa a menos que você marque Permitir LDAP
    simples, e um controlador de domínio que exige assinatura LDAP também
    a recusa. Use LDAPS, ou StartTLS na porta 389.

Para impedir que a conta abra uma sessão em qualquer lugar, negue a ela
*Log on locally* e *Log on through Remote Desktop Services* via Group
Policy: um bind LDAP não é afetado. Não a restrinja com *Log On To* ou
*Deny access to this computer from the network*: os controladores de
domínio então recusariam o bind.

### Mínimo privilégio, em detalhe {#least-privilege-in-detail}

- A conta não precisa de nenhum grupo além de Domain Users, que toda
  conta tem.
- Não use uma conta de serviço gerenciada (gMSA): ela não pode fazer um
  bind LDAP simples.
- Não conceda *Replicating Directory Changes* (ou *All*): esse é o
  direito DCSync, que expõe todo hash de senha. O DumbMonit nunca precisa
  dele.
- *Monitor active directory replication* só permite ler metadados de
  replicação (parceiros, último sucesso, códigos de erro). É opcional.

## LDAPS e certificados {#ldaps-and-certificates}

- **LDAPS (porta 636)** é o padrão. Todo controlador de domínio de um
  domínio com uma autoridade de certificação corporativa recebe um
  certificado automaticamente; sem uma, o controlador de domínio não
  escuta na 636. O certificado precisa carregar o nome que o DumbMonit
  usa: informe o controlador de domínio por seu nome DNS, não seu
  endereço IP, a menos que o certificado liste o endereço.
- **Certificado CA**: cole o PEM da autoridade raiz (ou emissora). Só
  essa autoridade é então confiável. Sem ele, o DumbMonit confia nas
  raízes públicas e no armazenamento do sistema de seu contêiner, que não
  conhecem uma autoridade corporativa privada.
- **StartTLS (porta 389)** criptografa da mesma forma, depois de uma
  saudação em texto claro. Use-o quando a porta 636 está filtrada.
- **Aceitar um certificado não verificável** mantém a conexão
  criptografada mas aceita qualquer certificado: qualquer um no caminho
  poderia se passar pelo controlador de domínio e receber a senha da
  conta de serviço.
- **LDAP channel binding** (`LdapEnforceChannelBinding`) diz respeito a
  binds NTLM e Kerberos via TLS; o bind simples que o DumbMonit usa não é
  afetado.

## Credenciais {#credentials}

| Campo | Valor |
|---|---|
| Nome de usuário | `svc-dumbmonit@corp.example.com`, `CORP\svc-dumbmonit`, ou o DN completo da conta |
| Senha | a senha da conta, armazenada criptografada e nunca mostrada de novo |

Uma senha vazia é recusada: o Active Directory a aceitaria como um bind
não autenticado sem verificar nada.

## Opções {#options}

| Opção | Padrão | O que é |
|---|---|---|
| Segurança da conexão (`security`) | `ldaps` | `ldaps`, `starttls` ou `plain` |
| Porta (`port`) | 636, ou 389 | usada se o endereço não der uma |
| Certificado CA (`ca_cert`) | | PEM da autoridade que emitiu os certificados dos controladores de domínio |
| Aceitar um certificado não verificável (`insecure_tls`) | desligado | |
| Permitir LDAP simples (`allow_plaintext`) | desligado | necessário para `plain` |
| Verificar cada controlador de domínio (`check_all_dcs`) | ligado | conexão TCP com a porta LDAP de cada controlador de domínio |
| Usuário inativo após (`stale_days_users`) | 90 | dias |
| Computador inativo após (`stale_days_computers`) | 90 | dias |
| Inventário completo a cada (`inventory_minutes`) | 15 | minutos, 1 a 1440 |
| Tempo limite (`request_timeout_seconds`) | 8 | por operação LDAP, 1 a 60 |

## Solução de problemas {#troubleshooting}

- **Certificado TLS recusado**: cole o certificado da autoridade como
  certificado CA, e informe o controlador de domínio por um nome que seu
  certificado lista.
- **Bind recusado: nome de usuário ou senha errados (resultado LDAP 49)**:
  o código de diagnóstico do controlador de domínio é traduzido — senha
  expirada, conta desativada, bloqueada, ou precisa mudar sua senha no
  próximo login.
- **O controlador de domínio exige uma conexão protegida**: a assinatura
  LDAP é exigida; mude para LDAPS ou StartTLS.
- **Um controlador de domínio aparece como não resolvido**: seu nome não
  resolve a partir do host DumbMonit. Aponte o DNS do DumbMonit para os
  servidores DNS do domínio, ou desmarque *Verificar cada controlador de
  domínio*. Nomes não resolvidos nunca geram um alerta.
- **Status de replicação não legível**: conceda *Monitor active directory
  replication* como no passo 3, ou aceite que a replicação não é
  mostrada.
