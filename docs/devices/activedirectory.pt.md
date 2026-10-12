# Active Directory {#active-directory}

Um domínio Windows via LDAP, só de leitura: os controladores de
domínio e quem detém cada função FSMO, os membros dos grupos
privilegiados, a política de senhas, contagens de conta, e os factos
brutos de segurança que uma auditoria procura primeiro — contas
abertas a Kerberoasting ou AS-REP roasting, delegação não restrita,
uma senha krbtgt antiga, computadores sem LAPS, versões do Windows
fora de suporte.

O DumbMonit liga-se com uma conta de serviço dedicada que é um
**utilizador de domínio simples, membro de nenhum grupo**. No Active
Directory todo utilizador autenticado pode ler o que o DumbMonit lê;
nenhum direito administrativo é necessário, e nenhum deve ser dado. O
DumbMonit nunca escreve no diretório, nunca lê um atributo de senha
(do LAPS lê a data de expiração, nunca a senha) e nunca pede os
direitos de replicação que exporiam segredos.

Um dispositivo cobre todo o domínio: aponte-o para um controlador de
domínio e o DumbMonit encontra os outros na partição de configuração.

## O que o LDAP pode e não pode dizer {#what-ldap-can-and-cannot-tell}

O LDAP mostra o diretório como está, não os registos do que aconteceu
nele. Em particular:

- **Replicação.** O controlador de domínio consultado publica o
  estado dos seus parceiros de replicação de entrada (último sucesso,
  falhas consecutivas, último erro) só a uma conta que detém o
  direito *Monitor active directory replication*. Sem ele, o painel
  diz *replication status not readable* em vez de adivinhar. Só a
  vista do controlador de domínio consultado é mostrada: para ver os
  parceiros de outro, adicione-o como um segundo dispositivo.
- **Não visível via LDAP**: registos de eventos (inícios de sessão,
  bloqueios conforme acontecem, alterações de grupo como eventos),
  conteúdo de Group Policy, SYSVOL, zonas DNS, e os próprios hashes de
  senha.
- **As datas de último início de sessão** vêm de
  `lastLogonTimestamp`, que os controladores de domínio só replicam a
  cada 9 a 14 dias: uma conta contada como inativa está inativa com
  uma margem de duas semanas.
- **A pertença a grupos** é lida com aninhamento (a regra
  `LDAP_MATCHING_RULE_IN_CHAIN`), através de `member`/`memberOf`. Um
  grupo primário definido por `primaryGroupID` não é uma ligação
  `member` e não é visto.

## Como lê {#how-it-reads}

Dois ritmos:

- **A cada verificação** (cada minuto por predefinição): ligação e
  bind, o RootDSE (níveis funcionais, relógio, sincronização), o
  objeto de domínio (política de senhas, quota de conta de máquina),
  as cinco funções FSMO, os controladores de domínio da partição de
  configuração, os grupos privilegiados e os seus membros efetivos, a
  funcionalidade Recycle Bin, e uma ligação TCP à porta LDAP de cada
  outro controlador de domínio.
- **Em segundo plano** a cada `inventory_minutes` (15 por
  predefinição): toda conta de utilizador e computador, só com os
  atributos que as descobertas precisam, em páginas de 500.
  Enumerar todo o diretório a cada minuto carregaria os controladores
  de domínio para nada.

A primeira verificação espera alguns segundos pelo inventário; num
domínio grande as descobertas de conta aparecem na verificação
seguinte.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_ad_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `bind_ok` | 1 quando a conta de serviço é aceite, 0 quando o controlador de domínio a recusa | |
| `connect_seconds`, `bind_seconds` | tempo para abrir a ligação (TLS incluído) e para fazer bind | |
| `tls_verified` | 1 quando o certificado é verificado | |
| `domain_info` | valor 1 | `domain`, `forest`, `dc` |
| `domain_functional_level`, `forest_functional_level` | 0 (2000) a 7 (2016), 10 (2025) | |
| `dc_synchronized`, `dc_global_catalog_ready` | o controlador de domínio consultado terminou a sua replicação inicial, o seu catálogo global está pronto | |
| `clock_skew_seconds` | relógio do controlador de domínio menos relógio do anfitrião do DumbMonit | |
| `recycle_bin_enabled` | 1 quando o AD Recycle Bin está ativado | |
| `dc_count` | controladores de domínio do domínio | |
| `dc_info` | valor 1 | `dc`, `site`, `gc`, `rodc`, `os` |
| `dc_reachable` | 1 ou 0 por controlador de domínio; nenhuma série quando o seu nome não resolve a partir do DumbMonit | `dc` |
| `fsmo_role` | valor 1 | `role` (`schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`), `holder` |
| `replication_readable` | 1 quando o estado de replicação de entrada pode ser lido | |
| `replication_consecutive_failures`, `replication_last_success_age_seconds` | por parceiro de entrada e partição | `source`, `naming_context` |
| `privileged_group_members`, `privileged_group_enabled_members` | membros efetivos, aninhamento incluído, grupos deixados de fora | `group` |
| `privileged_group_fingerprint` | um hash dos membros: muda quando alguém se junta ou sai | `group` |
| `users_total`, `users_enabled`, `users_disabled`, `users_locked_out`, `users_password_never_expires`, `users_stale` | contas de utilizador | |
| `computers_total`, `computers_enabled`, `computers_stale`, `groups_total` | computadores e grupos | |
| `admincount_accounts` | utilizadores protegidos por AdminSDHolder (`adminCount=1`) | |
| `krbtgt_password_age_seconds` | idade da senha krbtgt | |
| `laps_schema`, `laps_computers`, `laps_eligible_computers` | LAPS no esquema; computadores não-DC ativados com uma expiração LAPS, de todos eles | |
| `password_min_length`, `password_history_length`, `password_max_age_seconds` (0 = nunca), `password_complexity`, `lockout_threshold`, `machine_account_quota` | política de domínio predefinida | |
| `finding_count` | objetos afetados por uma descoberta, 0 quando examinada e limpa | `finding`, `severity`, `title` |
| `inventory_age_seconds`, `scrape_errors` | idade do último inventário completo; partes do diretório que não puderam ser lidas | |

Os grupos privilegiados são encontrados pelo seu identificador
conhecido, não o seu nome, para que um domínio instalado noutro idioma
funcione da mesma forma: Domain Admins (RID 512), Enterprise Admins
(519) e Schema Admins (518) — os últimos dois só no domínio raiz da
floresta — e os incorporados Administrators, Account Operators, Server
Operators, Print Operators e Backup Operators.

## Descobertas {#findings}

As descobertas são factos brutos — quantos objetos, e quais — não uma
pontuação; a [pontuação de segurança](../using/security-score.md) do
dispositivo pesa-as. Cada uma tem um `id` estável, uma severidade na
escala da pontuação (low, medium, high, critical, mostrada na página
como Info, Advisory e Warning), uma categoria, uma contagem e até
vinte nomes de conta de amostra. Uma descoberta que foi examinada e
não encontrou nada é mantida com uma contagem de 0, para que um
problema resolvido fique verde em vez de desaparecer; as descobertas
sobre contas aparecem uma vez o primeiro inventário em segundo plano
concluído. São servidas como estão por
`GET /api/targets/{id}/ad/findings` e publicadas como
`dumbmonit_ad_finding_count{finding, severity, title}`.

| `id` | Severidade | O quê |
|---|---|---|
| `unconstrained_delegation` | critical | contas ativadas de confiança para delegação não restrita, controladores de domínio excetuados |
| `kerberoastable_privileged` | critical | membros ativados de grupos privilegiados com um nome principal de serviço |
| `asrep_roastable_users` | high | contas ativadas com a pré-autenticação Kerberos desativada (`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | high | utilizadores ativados com um nome principal de serviço |
| `krbtgt_password_age` | high | senha krbtgt com mais de 180 dias |
| `laps_coverage` | high quando o LAPS está ausente, medium quando parcial | computadores ativados (controladores de domínio excetuados) sem uma senha LAPS |
| `reversible_encryption` | high | utilizadores cuja senha é guardada com encriptação reversível |
| `guest_enabled` | high | a conta incorporada Guest (RID 501) está ativada |
| `obsolete_os` | high | computadores ativados em Windows XP, Vista, 7, 8, 2000, Server 2003, 2008 ou 2012 |
| `dc_unreachable` | high | controladores de domínio cuja porta LDAP não responde |
| `replication_failing` | high | parceiros de replicação de entrada com falhas consecutivas |
| `dc_not_synchronized` | high | o controlador de domínio consultado não terminou a sua replicação inicial |
| `fsmo_role_orphaned` | high | uma função FSMO detida por um controlador de domínio eliminado |
| `password_policy` | high abaixo de 8 caracteres ou sem bloqueio, medium caso contrário | lacunas na política de domínio predefinida contra a referência: 14 caracteres, um limiar de bloqueio, complexidade |
| `privileged_group_size` | medium | mais de 5 contas ativadas em Domain Admins, Enterprise Admins e Administrators juntos |
| `stale_accounts` | medium | utilizadores e computadores ativados inativos há mais do que os limiares |
| `protocol_transition_delegation` | medium | delegação restrita com transição de protocolo |
| `des_only` | medium | contas restritas a chaves Kerberos DES |
| `password_not_required` | medium | utilizadores autorizados a uma senha vazia (`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | medium | utilizadores privilegiados cuja senha nunca expira |
| `privileged_stale` | medium | utilizadores privilegiados inativos há mais do que `stale_days_users` |
| `schema_admins_not_empty` | medium | Schema Admins tem membros |
| `functional_level_old` | medium | nível funcional de domínio abaixo de 2012 R2 |
| `clock_skew` | medium | mais de cinco minutos entre o controlador de domínio e o DumbMonit |
| `windows10_end_of_support` | medium | computadores ativados em Windows 10 |
| `admincount_orphans` | low | utilizadores `adminCount=1` que já não estão num grupo privilegiado |
| `password_never_expires` | low | utilizadores ativados cuja senha nunca expira |
| `machine_account_quota` | low | qualquer utilizador pode juntar computadores ao domínio |
| `recycle_bin_disabled` | low | o AD Recycle Bin não está ativado |

As [regras incorporadas](../alerting/rules.md#active-directory) que se
aplicam:

- **Domain controller unreachable**: outro controlador de domínio do
  domínio deixa de responder na sua porta LDAP durante cinco minutos
  (Warning). O consultado é coberto por *Device unreachable*.
- **Active Directory bind failed**: o controlador de domínio recusa a
  conta de serviço — senha expirada, desativada ou bloqueada
  (Advisory).
- **Privileged group membership changed**: alguém juntou-se ou saiu
  de um grupo privilegiado, aninhamento incluído (Advisory, resolve-se
  uma hora depois).
- **Active Directory replication failing**: um parceiro de entrada
  falha durante meia hora (Advisory), quando o estado de replicação é
  legível.
- **krbtgt password older than 180 days** (Info).

## A página do dispositivo {#the-device-page}

O painel Active Directory mostra o domínio (níveis funcionais, o
controlador de domínio consultado, desvio de relógio), os
controladores de domínio com o seu site, funções e se respondem, os
grupos privilegiados com os seus membros, a política de senhas e
contagens de conta, e a tabela de descobertas com palavras de
severidade.

## Criar uma conta de serviço só de leitura para o DumbMonit {#create-a-read-only-service-account-for-dumbmonit}

1. Crie um utilizador dedicado para o DumbMonit, por exemplo svc-dumbmonit, e não o adicione a nenhum grupo. Um utilizador de domínio simples basta: todo utilizador autenticado pode ler o que o DumbMonit lê. No PowerShell, com o módulo Active Directory:

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. Dê-lhe uma senha longa e aleatória. Se as senhas expiram no seu domínio, o DumbMonit levanta Active Directory bind failed no dia em que esta expirar.

3. Opcional: para ver o estado de replicação de entrada, conceda à conta o direito Monitor active directory replication no próprio objeto de domínio. Não revela nenhuma senha nem nenhum segredo.

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. O LDAPS precisa de um certificado em cada controlador de domínio; uma autoridade de certificação empresarial (AD CS) emite um automaticamente. Exporte o certificado dessa autoridade em formato X.509 codificado em Base-64 e cole-o como CA certificate abaixo.

5. No DumbMonit, introduza o nome de um controlador de domínio como o seu certificado o nomeia, por exemplo «dc1.corp.example.com», e inicie sessão como svc-dumbmonit@corp.example.com. O DumbMonit encontra os outros controladores de domínio por si próprio e verifica que cada um responde.

6. O DumbMonit só lê: nunca escreve no diretório, e do LAPS lê a data de expiração, nunca a senha.

!!! warning
    O LDAP simples na porta 389 envia a senha da conta de serviço em texto simples: o DumbMonit recusa-o a menos que marque Allow plain LDAP, e um controlador de domínio que exige assinatura LDAP também o recusa. Use LDAPS, ou StartTLS na porta 389.

Para impedir a conta de abrir uma sessão em qualquer lugar, negue-lhe
*Log on locally* e *Log on through Remote Desktop Services* por
Group Policy: um bind LDAP não é afetado. Não a restrinja com *Log On
To* ou *Deny access to this computer from the network*: os
controladores de domínio recusariam então o bind.

### Mínimo privilégio, em detalhe {#least-privilege-in-detail}

- A conta não precisa de nenhum grupo além de Domain Users, que toda
  conta tem.
- Não use uma conta de serviço gerida (gMSA): não pode fazer um bind
  LDAP simples.
- Não conceda *Replicating Directory Changes* (ou *All*): esse é o
  direito DCSync, que expõe todo hash de senha. O DumbMonit nunca
  precisa dele.
- *Monitor active directory replication* só permite ler metadados de
  replicação (parceiros, último sucesso, códigos de erro). É
  opcional.

## LDAPS e certificados {#ldaps-and-certificates}

- **LDAPS (porta 636)** é a predefinição. Todo controlador de domínio
  de um domínio com uma autoridade de certificação empresarial recebe
  um certificado automaticamente; sem uma, o controlador de domínio
  não escuta na 636. O certificado tem de transportar o nome que o
  DumbMonit usa: introduza o controlador de domínio pelo seu nome DNS,
  não o seu endereço IP, a menos que o certificado liste o endereço.
- **CA certificate**: cole o PEM da autoridade raiz (ou emissora). Só
  essa autoridade é então confiada. Sem ele, o DumbMonit confia nas
  raízes públicas e no armazenamento do sistema do seu contentor, que
  não conhecem uma autoridade empresarial privada.
- **StartTLS (porta 389)** encripta da mesma forma, depois de uma
  saudação em texto simples. Use-o quando a porta 636 está filtrada.
- **Accept an unverifiable certificate** mantém a ligação encriptada
  mas aceita qualquer certificado: qualquer um no caminho poderia
  fazer-se passar pelo controlador de domínio e receber a senha da
  conta de serviço.
- **LDAP channel binding** (`LdapEnforceChannelBinding`) diz respeito
  a binds NTLM e Kerberos via TLS; o bind simples que o DumbMonit usa
  não é afetado.

## Credenciais {#credentials}

| Campo | Valor |
|---|---|
| Nome de utilizador | `svc-dumbmonit@corp.example.com`, `CORP\svc-dumbmonit`, ou o DN completo da conta |
| Senha | a senha da conta, guardada encriptada e nunca mostrada de novo |

Uma senha vazia é recusada: o Active Directory aceitá-la-ia como um
bind não autenticado sem verificar nada.

## Opções {#options}

| Opção | Predefinição | O quê |
|---|---|---|
| Connection security (`security`) | `ldaps` | `ldaps`, `starttls` ou `plain` |
| Port (`port`) | 636, ou 389 | usada se o endereço não der nenhuma |
| CA certificate (`ca_cert`) | | PEM da autoridade que emitiu os certificados dos controladores de domínio |
| Accept an unverifiable certificate (`insecure_tls`) | desligado | |
| Allow plain LDAP (`allow_plaintext`) | desligado | obrigatório para `plain` |
| Check every domain controller (`check_all_dcs`) | ativado | ligação TCP à porta LDAP de cada controlador de domínio |
| Inactive user after (`stale_days_users`) | 90 | dias |
| Inactive computer after (`stale_days_computers`) | 90 | dias |
| Full inventory every (`inventory_minutes`) | 15 | minutos, 1 a 1440 |
| Timeout (`request_timeout_seconds`) | 8 | por operação LDAP, 1 a 60 |

## Resolução de problemas {#troubleshooting}

- **TLS certificate rejected**: cole o certificado da autoridade como
  CA certificate, e introduza o controlador de domínio por um nome
  que o seu certificado lista.
- **Bind refused: wrong user name or password (LDAP result 49)**: o
  código de diagnóstico do controlador de domínio é traduzido —
  senha expirada, conta desativada, bloqueada, ou tem de alterar a
  sua senha no próximo início de sessão.
- **The domain controller requires a protected connection**: a
  assinatura LDAP é imposta; mude para LDAPS ou StartTLS.
- **A domain controller shows as unresolved**: o seu nome não
  resolve a partir do anfitrião do DumbMonit. Aponte o DNS do
  DumbMonit para os servidores DNS do domínio, ou desmarque *Check
  every domain controller*. Os nomes não resolvidos nunca geram um
  alerta.
- **Replication status not readable**: conceda *Monitor active
  directory replication* como no passo 3, ou aceite que a replicação
  não é mostrada.
