# Veeam Backup & Replication

Um servidor Veeam Backup & Replication através de sua API REST: jobs cuja
última execução falhou ou terminou com um aviso, sessões com falha das
últimas 24 horas, repositórios enchendo, a licença, e — por computador ou
VM protegido — seu último backup bem sucedido.

O DumbMonit entra com uma conta Windows que possui apenas o papel Veeam
Backup Viewer (concessão de senha OAuth2 em `/api/oauth2/token`, porta
9419) e mantém o token por sua vida útil de quinze minutos. Toda chamada
carrega o cabeçalho obrigatório `x-api-version`, `1.1-rev0` por padrão: a
revisão que veio com a versão 12.0.

Os estados de job (`/api/v1/jobs/states`) são a única leitura obrigatória.
Os outros — `/api/v1/serverInfo`, `/api/v1/sessions` (criados nas últimas
24 horas), `/api/v1/backupInfrastructure/repositories/states`,
`/api/v1/license` e `/api/v1/restorePoints` (pontos de restauração de todo
objeto protegido, em todo o servidor, mais recentes primeiro) — são lidos
quando o papel permite. O Veeam reserva algumas rotas para o papel Backup
Administrator, a licença entre elas nas versões atuais: uma recusa ali é
mostrada como "não legível", nunca como um erro. Um servidor muito antigo
para conhecer `/api/v1/restorePoints` — a rota apareceu depois da 12.0— é
tratado da mesma forma: nenhum dispositivo, nenhum erro.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir da referência da API REST do
    Veeam e do centro de ajuda (autenticação, versionamento, os modelos de
    job, sessão, repositório, licença e ponto de restauração), não contra
    um servidor de backup em execução. Quais rotas o papel Viewer pode ler
    varia com a versão: avise-nos o que seu servidor recusa.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_veeam_`.

| Métrica | Rótulos | Significado |
|---|---|---|
| `version_info` | `version` | Valor 1: o build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Por job. |
| `job_failed`, `job_warning` | `job`, `type` | 1 quando a última execução de um job ativado falhou, ou terminou com um aviso. Um job desativado mantém seu último resultado, que não diz mais nada: não conta como nenhum dos dois. |
| `job_result_info` | `job`, `type`, `result` | Valor 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Desde a última execução; ausente para um job que nunca rodou. |
| `job_progress_percent` | `job`, `type` | Enquanto o job roda. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Contagens; no máximo 200 jobs são descritos um a um. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sessões criadas nas últimas 24 horas, por resultado. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Por repositório. Um repositório de armazenamento de objetos não tem capacidade: apenas seu espaço usado é publicado. |
| `license_valid` | | 1 quando o status da licença é `Valid`. |
| `license_info` | `status`, `type`, `edition` | Valor 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Até a licença e o contrato de suporte expirarem; ausente para uma licença perpétua. |
| `license_instances_licensed`, `license_instances_used` | | Uso de licença de instância. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1 quando a conta pode lê-la, 0 quando o Veeam recusou. |

As [regras integradas](../alerting/rules.md#veeam-backup-replication) que
se aplicam:

- **Veeam job failed**: a última execução de um job ativado falhou
  (Warning, lembrada diariamente).
- **Veeam job ended with a warning** (Advisory, lembrada diariamente).
- **Veeam repository nearly full**: mais de 90% por trinta minutos
  (Advisory).
- **Veeam license expiring**: menos de 30 dias restantes (Advisory,
  lembrada diariamente). Apenas quando a conta pode ler a licença.
- **Client device stale**: um computador ou VM protegido passou
  `device_stale_days` sem um backup bem sucedido (Advisory, lembrada
  diariamente); veja [Dispositivos](#devices) abaixo.

## Dispositivos

Todo objeto com pelo menos um ponto de restauração — uma VM, um computador
físico ou virtual protegido por agente, um compartilhamento de arquivo — é
publicado sob a família genérica compartilhada por toda integração do
DumbMonit que rastreia dispositivos protegidos —
`dumbmonit_client_device_*`, com prefixo sem `veeam`, documentada
completamente em [Dispositivos clientes](../alerting/rules.md#client-devices):

| Métrica | O que é | Rótulos |
|---|---|---|
| `last_backup_timestamp_seconds` | O ponto de restauração mais recente do objeto | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo uma vez que um objeto passou `device_stale_days` (3 por padrão) sem um novo ponto de restauração; aciona a regra [Dispositivo cliente desatualizado](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `backup`) |

**Limitações que vale a pena conhecer**: um job protege seus objetos como
um grupo (`jobs/states` dá apenas uma contagem, `objectsCount`); a data por
objeto vem de `/api/v1/restorePoints` em vez disso, lida para todo o
servidor em uma chamada. O Veeam cria um ponto de restauração apenas uma
vez que um backup tenha ao menos parcialmente sucedido, então o mais
recente por objeto já é "o último backup bem sucedido" — nada é filtrado
pelo resultado aqui. O Veeam não expõe uma "última conexão" separada do
próprio backup, então `last_seen_timestamp_seconds` e o sinal `connection`
nunca são publicados; um dispositivo mostra `last_connection` como "—" na
tabela da página do dispositivo. O `type` do dispositivo é o próprio nome
de plataforma do Veeam (`VSphere`, `WindowsPhysical`, `HyperV`…), mostrado
como está.

## A página do dispositivo

O painel Backups diz o que está errado em uma frase, mostra os jobs com
falha, as sessões com falha das últimas 24 horas, os jobs em execução e a
licença, depois lista os jobs ativados — falhas primeiro — com seu tipo e
última execução, e os repositórios com sua ocupação e espaço livre. A
tabela de dispositivos protegidos (nome, plataforma, último backup,
status) segue, da família genérica acima.

## Dar ao DumbMonit uma conta Veeam Backup Viewer

1. No servidor de backup, crie uma conta Windows local para o DumbMonit
   com uma senha longa e aleatória e nenhum outro direito na máquina (uma
   conta de domínio também funciona).

    ```
    net user dumbmonit * /add
    ```

2. No console Veeam, abra o menu principal > Users and Roles > Security,
   clique em Add, informe a conta (por exemplo VBR01\dumbmonit) e dê a ela
   apenas o papel Veeam Backup Viewer.

3. No DumbMonit, informe o endereço do servidor de backup, por exemplo
   "vbr.lan", e a conta como HOST\dumbmonit ou DOMAIN\dumbmonit com sua
   senha. A API REST escuta na porta 9419 com um certificado autoassinado:
   marque Aceitar um certificado não verificável a menos que você tenha
   instalado o seu próprio.

!!! warning
    O Veeam reserva algumas rotas da API REST para o papel Backup
    Administrator, a licença entre elas nas versões atuais. Com o papel
    Viewer o DumbMonit lê os jobs, sessões, repositórios e pontos de
    restauração, e mostra a licença como não legível: ele nunca pede mais.

## Credenciais

| Credencial | Campos |
|---|---|
| Nome de usuário e senha | A conta com o papel Veeam Backup Viewer, como `HOST\dumbmonit` ou `DOMAIN\dumbmonit`. |

Endereço: o nome de host ou IP do servidor de backup, `host:porta`, ou uma
URL. Opções:

- **Protocolo** (`https`) e **Porta** (9419): os do serviço de API REST.
- **Versão da API** (`1.1-rev0`): enviada como `x-api-version`. Mude
  apenas se o servidor a recusar; um servidor v13 documenta `1.3-rev1`.
- **Aceitar um certificado não verificável**: o próprio certificado da
  API REST é autoassinado.
- **Tempo limite por requisição** (15 s).
- **Desatualização de dispositivo** (`device_stale_days`, 3 dias por
  padrão): veja [Dispositivos](#devices) acima.

## Solução de problemas

"Veeam refused the login": o nome de usuário precisa de seu host ou
domínio (`VBR01\dumbmonit`), ou a conta não tem papel Veeam.

"Veeam refused to list the jobs (403)": a conta tem um papel que não pode
ler jobs; dê a ela Veeam Backup Viewer.

"Veeam refused the API version": defina a opção de versão da API para uma
que o servidor lista em `https://<server>:9419/swagger`.

A licença mostra "Not readable": esperado com o papel Viewer nas versões
atuais; o DumbMonit não pede mais.

Nenhum dispositivo, ou o painel Devices está vazio: `restore_points` lê
"Not readable", provavelmente porque o servidor é mais antigo que a rota
(qualquer coisa antes das revisões posteriores da 12.0) em vez de um
problema de papel — pontos de restauração são inventário comum, não uma
página restrita a administrador.
