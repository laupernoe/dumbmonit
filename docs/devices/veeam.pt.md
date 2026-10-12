# Veeam Backup & Replication {#veeam-backup-replication}

Um servidor Veeam Backup & Replication através da sua API REST: tarefas
cuja última execução falhou ou terminou com um aviso, sessões falhadas
das últimas 24 horas, repositórios a enchê-se, a licença, e — por
computador ou VM protegido — a sua última cópia de segurança
bem-sucedida.

O DumbMonit inicia sessão com uma conta Windows que detém apenas a
função Veeam Backup Viewer (concessão de senha OAuth2 em
`/api/oauth2/token`, porta 9419) e mantém o token durante a sua vida de
quinze minutos. Cada chamada transporta o cabeçalho obrigatório
`x-api-version`, `1.1-rev0` por predefinição: a revisão que chegou com a
versão 12.0.

Os estados das tarefas (`/api/v1/jobs/states`) são a única leitura
obrigatória. As outras — `/api/v1/serverInfo`, `/api/v1/sessions`
(criadas nas últimas 24 horas), `/api/v1/backupInfrastructure/repositories/states`,
`/api/v1/license` e `/api/v1/restorePoints` (os pontos de restauro de
todo objeto protegido, à escala do servidor, mais recentes primeiro) —
são lidas quando a função o permite. O Veeam reserva algumas rotas para
a função Backup Administrator, a licença entre elas nas versões atuais:
uma recusa aí é mostrada como «não legível», nunca como um erro. Um
servidor demasiado antigo para conhecer `/api/v1/restorePoints` — a
rota apareceu depois da 12.0 — é tratado da mesma forma: nenhum
dispositivo, nenhum erro.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir da referência da API REST do
    Veeam e do centro de ajuda (autenticação, versionamento, os modelos
    de tarefa, sessão, repositório, licença e ponto de restauro), não
    contra um servidor de cópias de segurança em funcionamento. Quais
    rotas a função Viewer pode ler varia com a versão: diga-nos o que o
    seu servidor recusa.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_veeam_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `version_info` | `version` | Valor 1: a compilação, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Por tarefa. |
| `job_failed`, `job_warning` | `job`, `type` | 1 quando a última execução de uma tarefa ativada falhou, ou terminou com um aviso. Uma tarefa desativada mantém o seu último resultado, que já não diz nada: não conta como nenhum dos dois. |
| `job_result_info` | `job`, `type`, `result` | Valor 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Desde a última execução; ausente para uma tarefa que nunca correu. |
| `job_progress_percent` | `job`, `type` | Enquanto a tarefa corre. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Contagens; no máximo 200 tarefas são descritas uma a uma. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sessões criadas nas últimas 24 horas, por resultado. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Por repositório. Um repositório de armazenamento de objetos não tem capacidade: só o seu espaço usado é publicado. |
| `license_valid` | | 1 quando o estado da licença é `Valid`. |
| `license_info` | `status`, `type`, `edition` | Valor 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Até a licença e o contrato de suporte expirarem; ausente para uma licença perpétua. |
| `license_instances_licensed`, `license_instances_used` | | Uso de licença por instância. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1 quando a conta pode lê-la, 0 quando o Veeam recusou. |

As [regras incorporadas](../alerting/rules.md#veeam-backup-replication)
que se aplicam:

- **Veeam job failed**: a última execução de uma tarefa ativada falhou
  (Warning, lembrado diariamente).
- **Veeam job ended with a warning** (Advisory, lembrado diariamente).
- **Veeam repository nearly full**: mais de 90% durante trinta minutos
  (Advisory).
- **Veeam license expiring**: menos de 30 dias restantes (Advisory,
  lembrado diariamente). Só quando a conta pode ler a licença.
- **Client device stale**: um computador ou VM protegido passou
  `device_stale_days` sem uma cópia de segurança bem-sucedida (Advisory,
  lembrado diariamente); ver [Dispositivos](#devices) abaixo.

## Dispositivos {#devices}

Todo objeto com pelo menos um ponto de restauro — uma VM, um computador
protegido por agente físico ou virtual, uma partilha de ficheiros — é
publicado sob a família genérica partilhada por toda integração do
DumbMonit que acompanha dispositivos protegidos —
`dumbmonit_client_device_*`, prefixada sem `veeam`, documentada na
íntegra em [Dispositivos cliente](../alerting/rules.md#client-devices):

| Métrica | O quê | Etiquetas |
|---|---|---|
| `last_backup_timestamp_seconds` | O ponto de restauro mais recente do objeto | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo uma vez que um objeto passou `device_stale_days` (3 por predefinição) sem um novo ponto de restauro; alimenta a regra [Dispositivo obsoleto](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `backup`) |

**Limites a conhecer**: uma tarefa protege os seus objetos como um
grupo (`jobs/states` só dá uma contagem, `objectsCount`); a data por
objeto vem de `/api/v1/restorePoints` em vez disso, lida para todo o
servidor numa chamada. O Veeam só cria um ponto de restauro depois de
uma cópia de segurança ter pelo menos parcialmente tido sucesso, por
isso o mais recente por objeto já é «a última cópia de segurança
bem-sucedida» — nada é filtrado pelo resultado aqui. O Veeam não expõe
uma «última ligação» separada da própria cópia de segurança, por isso
`last_seen_timestamp_seconds` e o sinal `connection` nunca são
publicados; um dispositivo mostra `last_connection` como «—» na tabela
da página do dispositivo. O `type` do dispositivo é o próprio nome de
plataforma do Veeam (`VSphere`, `WindowsPhysical`, `HyperV`…), mostrado
como está.

## A página do dispositivo {#the-device-page}

O painel Backups diz o que está mal numa frase, mostra as tarefas
falhadas, as sessões falhadas das últimas 24 horas, as tarefas em
execução e a licença, depois lista as tarefas ativadas — as falhadas
primeiro — com o seu tipo e última execução, e os repositórios com a
sua ocupação e espaço livre. A tabela de dispositivos protegidos (nome,
plataforma, última cópia de segurança, estado) segue-se, da família
genérica acima.

## Dar ao DumbMonit uma conta Veeam Backup Viewer {#give-dumbmonit-a-veeam-backup-viewer-account}

1. No servidor de cópias de segurança, crie uma conta Windows local para o DumbMonit com uma senha longa e aleatória e nenhum outro direito na máquina (uma conta de domínio também funciona).

    ```
    net user dumbmonit * /add
    ```

2. Na consola Veeam, abra o menu principal > Users and Roles > Security, clique em Add, introduza a conta (por exemplo VBR01\dumbmonit) e dê-lhe só a função Veeam Backup Viewer.

3. No DumbMonit, introduza o endereço do servidor de cópias de segurança, por exemplo «vbr.lan», e a conta como HOST\dumbmonit ou DOMAIN\dumbmonit com a sua senha. A API REST escuta na porta 9419 com um certificado autoassinado: marque Accept an unverifiable certificate a menos que tenha instalado o seu próprio.

!!! warning
    O Veeam reserva algumas rotas da API REST para a função Backup Administrator, a licença entre elas nas versões atuais. Com a função Viewer o DumbMonit lê as tarefas, sessões, repositórios e pontos de restauro, e mostra a licença como não legível: nunca pede mais.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador e senha | A conta com a função Veeam Backup Viewer, como `HOST\dumbmonit` ou `DOMAIN\dumbmonit`. |

Endereço: o nome de anfitrião ou IP do servidor de cópias de segurança,
`anfitrião:porta`, ou um URL. Opções:

- **Protocol** (`https`) e **Port** (9419): os do serviço da API REST.
- **API version** (`1.1-rev0`): enviada como `x-api-version`. Altere-a
  só se o servidor a recusar; um servidor v13 documenta `1.3-rev1`.
- **Accept an unverifiable certificate**: o próprio certificado da API
  REST é autoassinado.
- **Timeout per request** (15 s).
- **Device staleness** (`device_stale_days`, 3 dias por predefinição):
  ver [Dispositivos](#devices) acima.

## Resolução de problemas {#troubleshooting}

«Veeam refused the login»: o nome de utilizador precisa do seu anfitrião
ou domínio (`VBR01\dumbmonit`), ou a conta não tem nenhuma função Veeam.

«Veeam refused to list the jobs (403)»: a conta tem uma função que não
pode ler tarefas; dê-lhe Veeam Backup Viewer.

«Veeam refused the API version»: defina a opção de versão da API para
uma que o servidor liste em `https://<server>:9419/swagger`.

A licença mostra «Not readable»: esperado com a função Viewer nas
versões atuais; o DumbMonit não pede mais.

Nenhum dispositivo, ou o painel Devices está vazio: `restore_points` lê
«Not readable», provavelmente porque o servidor é mais antigo do que a
rota (qualquer coisa antes das revisões posteriores da 12.0) em vez de
um problema de função — os pontos de restauro são inventário comum, não
uma página só de administrador.
