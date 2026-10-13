# Veeam Backup & Replication {#veeam-backup-replication}

Um servidor Veeam Backup & Replication através da sua API REST: tarefas cuja
última execução falhou ou terminou com aviso, sessões falhadas das últimas 24
horas, repositórios a encher, a licença, e — por computador ou VM protegido — a
sua última cópia de segurança bem-sucedida.

O DumbMonit inicia sessão com uma conta Windows que tem apenas o papel Veeam
Backup Viewer (OAuth2 password grant em `/api/oauth2/token`, porta 9419) e
mantém o token durante o seu tempo de vida de quinze minutos. Cada chamada leva
o cabeçalho obrigatório `x-api-version`, `1.1-rev0` por predefinição: a
revisão que veio com a versão 12.0.

Os estados das tarefas (`/api/v1/jobs/states`) são a única leitura obrigatória.
As outras — `/api/v1/serverInfo`, `/api/v1/sessions` (criadas nas últimas 24
horas), `/api/v1/backupInfrastructure/repositories/states`, `/api/v1/license` e
`/api/v1/restorePoints` (os pontos de restauro de todos os objetos protegidos,
em todo o servidor, os mais recentes primeiro) — são lidas quando o papel o
permite. O Veeam reserva algumas rotas para o papel Backup Administrator, entre
elas a licença nas versões atuais: uma recusa aí é mostrada como «não legível»,
nunca como erro. Um servidor demasiado antigo para conhecer
`/api/v1/restorePoints` — a rota apareceu depois da 12.0 — é tratado da mesma
forma: sem dispositivos, sem erro.

!!! warning "Validado apenas contra a documentação do fabricante — ainda não testado num sistema real"

    Esta integração foi construída a partir da referência da API REST e do
    centro de ajuda do Veeam (autenticação, versionamento, os modelos de
    tarefa, sessão, repositório, licença e ponto de restauro), não contra um
    servidor de cópias de segurança em funcionamento. As rotas que o papel
    Viewer pode ler variam com a versão: diga-nos o que o seu servidor recusa.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_veeam_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `version_info` | `version` | Valor 1: a build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Por tarefa. |
| `job_failed`, `job_warning` | `job`, `type` | 1 quando a última execução de uma tarefa ativada falhou, ou terminou com aviso. Uma tarefa desativada mantém o seu último resultado, que já não diz nada: não conta como nenhum dos dois. |
| `job_result_info` | `job`, `type`, `result` | Valor 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Desde a última execução; ausente para uma tarefa que nunca correu. |
| `job_progress_percent` | `job`, `type` | Enquanto a tarefa corre. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Contagens; no máximo 200 tarefas são descritas uma a uma. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sessões criadas nas últimas 24 horas, por resultado. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Por repositório. Um repositório de armazenamento de objetos não tem capacidade: só o espaço usado é publicado. |
| `license_valid` | | 1 quando o estado da licença é `Valid`. |
| `license_info` | `status`, `type`, `edition` | Valor 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Até a licença e o contrato de suporte expirarem; ausente para uma licença perpétua. |
| `license_instances_licensed`, `license_instances_used` | | Uso da licença de instâncias. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1 quando a conta a pode ler, 0 quando o Veeam recusou. |

As [regras incorporadas](../alerting/rules.md#veeam-backup--replication) que se aplicam:

- **Veeam job failed**: a última execução de uma tarefa ativada falhou
  (Warning, lembrada diariamente).
- **Veeam job ended with a warning** (Advisory, lembrada diariamente).
- **Veeam repository nearly full**: mais de 90 % durante trinta minutos
  (Advisory).
- **Veeam license expiring**: menos de 30 dias restantes (Advisory,
  lembrada diariamente). Apenas quando a conta pode ler a licença.
- **Client device stale**: um computador ou VM protegido passou
  `device_stale_days` sem uma cópia de segurança bem-sucedida (Advisory,
  lembrado diariamente); veja [Dispositivos](#devices) abaixo.

## Dispositivos {#devices}

Todo o objeto com pelo menos um ponto de restauro — uma VM, um computador
físico ou virtual protegido por agente, uma partilha de ficheiros — é publicado
sob a família genérica partilhada por todas as integrações do DumbMonit que
acompanham dispositivos protegidos — `dumbmonit_client_device_*`, com prefixo
sem `veeam`, documentada por inteiro em
[Dispositivos de clientes](../alerting/rules.md#client-devices):

| Métrica | O quê | Etiquetas |
|---|---|---|
| `last_backup_timestamp_seconds` | O ponto de restauro mais recente do objeto | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo quando um objeto passou `device_stale_days` (3 por predefinição) sem um novo ponto de restauro; alimenta a regra [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `backup`) |

**Limites a conhecer**: uma tarefa protege os seus objetos em grupo
(`jobs/states` dá apenas uma contagem, `objectsCount`); a data por objeto vem
antes de `/api/v1/restorePoints`, lido para todo o servidor numa só chamada. O
Veeam só cria um ponto de restauro quando uma cópia de segurança teve êxito pelo
menos parcial, pelo que o mais recente por objeto já é «a última cópia de
segurança bem-sucedida» — nada é filtrado pelo resultado aqui. O Veeam não
expõe uma «última ligação» separada da própria cópia de segurança, pelo que
`last_seen_timestamp_seconds` e o sinal `connection` nunca são publicados; um
dispositivo mostra `last_connection` como «—» na tabela da página do
dispositivo. O `type` do dispositivo é o nome de plataforma do próprio Veeam
(`VSphere`, `WindowsPhysical`, `HyperV`…), mostrado tal como está.

## A página do dispositivo {#the-device-page}

O painel Backups diz numa frase o que está errado, mostra as tarefas falhadas,
as sessões falhadas das últimas 24 horas, as tarefas em execução e a licença,
depois lista as tarefas ativadas — as falhas primeiro — com o seu tipo e última
execução, e os repositórios com a sua ocupação e espaço livre. A tabela de
dispositivos protegidos (nome, plataforma, última cópia de segurança, estado)
segue-se, a partir da família genérica acima.

## Dar ao DumbMonit uma conta Veeam Backup Viewer {#give-dumbmonit-a-veeam-backup-viewer-account}

1. No servidor de cópias de segurança, crie uma conta Windows local para o DumbMonit com uma palavra-passe longa e aleatória e nenhum outro direito na máquina (uma conta de domínio também serve).

    ```
    net user dumbmonit * /add
    ```

2. Na consola Veeam, abra o menu principal > Users and Roles > Security, clique em Add, introduza a conta (por exemplo VBR01\dumbmonit) e dê-lhe apenas o papel Veeam Backup Viewer.

3. No DumbMonit, introduza o endereço do servidor de cópias de segurança, por exemplo «vbr.lan», e a conta como HOST\dumbmonit ou DOMAIN\dumbmonit com a sua palavra-passe. A API REST escuta na porta 9419 com um certificado autoassinado: marque Aceitar um certificado não verificável a menos que tenha instalado o seu.

!!! warning
    O Veeam reserva algumas rotas da API REST para o papel Backup Administrator, entre elas a licença nas versões atuais. Com o papel Viewer o DumbMonit lê as tarefas, sessões, repositórios e pontos de restauro, e mostra a licença como não legível: nunca pede mais.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador e palavra-passe | A conta com o papel Veeam Backup Viewer, como `HOST\dumbmonit` ou `DOMAIN\dumbmonit`. |

Endereço: o nome de anfitrião ou IP do servidor de cópias de segurança,
`host:port`, ou um URL. Opções:

- **Protocolo** (`https`) e **Porta** (9419): os do serviço da API REST.
- **Versão da API** (`1.1-rev0`): enviada como `x-api-version`. Altere-a apenas
  se o servidor a recusar; um servidor v13 documenta `1.3-rev1`.
- **Aceitar um certificado não verificável**: o certificado da própria API REST
  é autoassinado.
- **Tempo limite por pedido** (15 s).
- **Inatividade de dispositivos** (`device_stale_days`, 3 dias por
  predefinição): veja [Dispositivos](#devices) acima.

## Resolução de problemas {#troubleshooting}

«Veeam refused the login»: o nome de utilizador precisa do seu anfitrião ou
domínio (`VBR01\dumbmonit`), ou a conta não tem papel Veeam.

«Veeam refused to list the jobs (403)»: a conta tem um papel que não pode ler
tarefas; dê-lhe Veeam Backup Viewer.

«Veeam refused the API version»: defina a opção Versão da API para uma que o
servidor liste em `https://<server>:9419/swagger`.

A licença mostra «Not readable»: esperado com o papel Viewer nas versões
atuais; o DumbMonit não pede mais.

Sem dispositivos, ou o painel Dispositivos está vazio: `restore_points` indica
«Not readable», provavelmente porque o servidor é mais antigo do que a rota
(qualquer coisa anterior às revisões posteriores da 12.0) e não por um problema
de papel — os pontos de restauro são inventário normal, não uma página só para
administradores.
