# Backup e restauração {#backup-and-restore}

Duas coisas diferentes, para dois acidentes diferentes.

| | Pacote criptografado | Backup local programado |
|---|---|---|
| O que contém | A configuração: dispositivos e suas credenciais, regras, canais, páginas de status, contas, tokens. | Todo o banco de dados, como um arquivo, mais `secret.key`. |
| Para onde vai | Onde você colocar — é criptografado com uma frase secreta que você escolhe. | `/data/backups/`, dentro do mesmo volume. |
| Do que te salva | Um servidor perdido, uma mudança para hardware novo, uma instalação do zero. | A mudança que você se arrepende dez minutos depois, uma atualização que deu errado. |
| O que não contém | Histórico de métricas, estado e histórico de alertas, o log de auditoria, sessões. | Histórico de métricas (`vm/`). |

Nenhum dos dois substitui um arquivo de todo o volume se você quer os
gráficos de volta também; isso está no final desta página.

!!! danger "`/data/secret.key` é o que descriptografa suas credenciais de dispositivo"
    Comunidades SNMP, tokens Proxmox, senhas SMTP e URLs de heartbeat são
    armazenadas criptografadas com uma chave derivada do segredo da
    instância. Na primeira inicialização o DumbMonit a escreve em
    `/data/secret.key` (ou a lê de `DUMBMONIT_SECRET` se você definir
    uma).

    **Uma cópia do banco de dados sem esse arquivo restaura uma instância
    que não consegue falar com nada.** O servidor percebe isso e recusa
    iniciar com uma mensagem explícita em vez de falhar em toda sondagem
    — mas as credenciais se perdem, e todo dispositivo precisa ser
    informado de novo.

    O pacote criptografado é a única exceção: seus segredos são
    recriptografados com sua frase secreta, então ele restaura em uma
    instância nova que tem seu próprio segredo, recém-criado.

## O pacote criptografado {#the-encrypted-bundle}

**Configurações → Backup → Baixar o pacote.** Escolha uma frase secreta
de pelo menos 16 caracteres — três palavras são suficientes — e a
confirme. O arquivo que vem é JSON simples com um corpo criptografado:

```json
{
  "format": "dumbmonit-backup",
  "version": 1,
  "created_at": "2026-09-22 19:04:11",
  "source_version": "0.1.0-alpha.3",
  "summary": { "targets": 12, "channels": 2, "rules": 41 },
  "kdf": { "algorithm": "argon2id", "salt": "…", "m_cost": 32768, "t_cost": 3, "p_cost": 1 },
  "cipher": "aes-256-gcm",
  "payload": "…"
}
```

O cabeçalho permanece legível para que você possa dizer o que é um
arquivo antes de digitar qualquer coisa. Tudo mais — endereços incluídos
— está dentro de `payload`, criptografado com AES-256-GCM sob uma chave
derivada de sua frase secreta pelo Argon2id. Não há **recuperação**: um
pacote cuja frase secreta se perde está perdido.

!!! warning "O pacote contém toda credencial desta instância"
    Qualquer um com o arquivo e a frase secreta pode ler a senha de todo
    dispositivo que você monitora. Mantenha-o onde você mantém senhas.

### O que está nele {#what-is-in-it}

| Seção | Conteúdo |
|---|---|
| `targets` | Dispositivos: nome, tipo, endereço, intervalo, opções, pai, agente relay, e a credencial. |
| `rules` | Regras de alerta, incluindo integradas que você reajustou ou desativou. |
| `rule_overrides` | Limites e exclusões por dispositivo. |
| `channels` | Canais de notificação, suas configurações, sua política por canal e seus segredos. |
| `notify_policy` | A política de notificação global (agrupamento, limite por hora, oscilação, URL pública). |
| `silences` | Janelas de manutenção. |
| `status_pages` | Páginas de status públicas e os serviços que cada uma lista. |
| `incidents` | Incidentes e anúncios de manutenção, com suas atualizações. |
| `users` | Contas: nome de usuário, nome de exibição, papel, link SSO, ativado ou não. |
| `agent_tokens` | Tokens de cadastro de agente, como hashes. Agentes instalados continuam enviando depois de uma restauração. |
| `api_tokens` | Tokens de API, como hashes. Tokens existentes continuam funcionando. |
| `push_monitors` | Tokens de heartbeat, então as URLs que seus jobs cron já chamam permanecem as mesmas. |

Dispositivos são correspondidos por **tipo + endereço**, que também é a
chave única da tabela: importar o mesmo pacote duas vezes nunca cria uma
segunda cópia de nada.

### Senhas de conta {#account-passwords}

Por padrão contas são exportadas **sem** seu hash de senha, segredo TOTP
ou códigos de recuperação. Contas restauradas então existem com o nome e
papel certos, mas ninguém pode entrar com elas até um administrador
definir uma senha — ou até elas entrarem via SSO, que não precisa de nada
do pacote.

Ative **Incluir senhas de conta e segredos 2FA** quando você está movendo
uma instância em vez de copiar sua configuração. Um hash Argon2 em um
arquivo que viaja pode ser atacado offline por tanto tempo quanto o
atacante quiser; o padrão é o seguro.

### O que não está nele {#what-is-not-in-it}

Histórico de métricas (que vive no VictoriaMetrics, não no banco de
dados), estado e histórico de alertas, linhas de base de anomalia, o log
de auditoria, sessões abertas, o registro e última visualização de
agentes, e os caches de sondagem dos painéis Proxmox e Synology. São
observações, não configurações: eles voltam por conta própria.

E **não o segredo da instância**. Esse é o ponto: o pacote carrega seus
segredos recriptografados sob sua frase secreta precisamente para que
possa ser restaurado em algum lugar que nunca viu `secret.key`.

### Restaurando em uma instância nova {#restoring-onto-a-fresh-instance}

1. Inicie o novo contêiner e crie a primeira conta de administrador em
   `/setup`, com o código de configuração impresso em seus logs. Essa
   conta é sua; a restauração não a tocará.
2. **Configurações → Backup → Restaurar um pacote.** Escolha o arquivo,
   digite a frase secreta, e clique em **Verificar este backup**.
3. Leia o relatório. Ele diz, seção por seção, o que seria **criado**, o
   que seria **atualizado** porque difere do que já está aqui, e o que já
   é idêntico — mais uma nota para qualquer coisa que não pôde colocar,
   como uma regra que apontava para um dispositivo que o pacote não
   contém.
4. Clique em **Restaurar de verdade**.

Três regras que a restauração nunca quebra:

- **Nada é jamais excluído.** Uma restauração faz existir o que o pacote
  descreve; qualquer coisa que a instância tenha além disso permanece.
- **Nada é jamais duplicado.** Todo item é correspondido por sua chave
  natural — tipo + endereço para um dispositivo, `uid` para uma regra,
  nome para um canal, slug para uma página de status. Restaurar o mesmo
  pacote duas vezes reporta tudo como já idêntico e não escreve nada.
- **Contas existentes são deixadas intactas.** Uma conta cujo nome de
  usuário já está aqui é ignorada, nunca sobrescrita: um arquivo não deve
  conseguir mudar o papel ou a senha do administrador executando a
  restauração.

Uma simulação não é uma simulação da restauração, ela *é* a restauração,
executada dentro de uma transação que é revertida ao final. O relatório
não pode estar errado sobre o que aplicar faria.

Um pacote escrito por um DumbMonit mais novo é recusado com uma frase que
diz isso — atualize primeiro, depois restaure. Um pacote que foi
modificado, ou cuja frase secreta está errada, também é recusado: o
AES-GCM não consegue distinguir os dois, e nem a mensagem.

### Pela linha de comando {#from-the-command-line}

Ambos os caminhos precisam de uma **sessão** de administrador, não um
token de API: um token vive em um script, e um script não tem nada a ver
exportando toda credencial da instância.

```bash
# Entrar
curl -s -c jar -X POST http://localhost:8080/api/auth/login \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"username":"admin","password":"…"}'

# Exportar
curl -s -b jar -X POST http://localhost:8080/api/backup \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"passphrase":"three words are enough"}' -o backup.json

# Simulação, depois aplicar
jq -n --slurpfile b backup.json \
  '{bundle: $b[0], passphrase: "three words are enough", apply: false}' > restore.json
curl -s -b jar -X POST http://localhost:8080/api/backup/restore \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d @restore.json | jq
```

## Backups locais programados {#scheduled-local-backups}

O DumbMonit escreve uma cópia de seu próprio banco de dados em uma
programação, por padrão **uma vez por dia, mantendo as últimas sete**,
em `/data/backups/`:

```
/data/backups/dumbmonit-20260922-030000.db
/data/backups/dumbmonit-20260922-030000.key
```

O arquivo `.db` é produzido com o `VACUUM INTO` do SQLite: uma cópia
online e consistente escrita enquanto o servidor continua rodando, não
uma cópia de arquivo que poderia capturar o banco de dados no meio de
uma escrita. O `sqlite3` o abre como está, sem journal para reaplicar. O
`.key` ao lado é a cópia de `secret.key` — juntos, o par é suficiente
para reconstruir a instância.

Eles vivem no mesmo volume que o banco de dados, então desfazem um erro,
não um disco perdido. Copie-os para fora da máquina se você quer mais
que isso.

### Configurações {#settings}

| Variável | Padrão | Função |
|---|---|---|
| `DUMBMONIT_BACKUP_ENABLED` | `1` | `0`, `false`, `no` ou `off` para parar de escrever backups programados. |
| `DUMBMONIT_BACKUP_DIR` | `<diretório de dados>/backups` | Onde são escritos. Aponte para um segundo volume para sobreviver à perda do primeiro. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Horas entre dois backups. O primeiro acontece um intervalo depois da inicialização, não na inicialização. |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quantos são mantidos. Os mais antigos são removidos, com seu `.key`. |

Nada mais no diretório é jamais tocado: só arquivos nomeados
`dumbmonit-<timestamp>.db` e seus `.key` são rotacionados.

Quando o segredo da instância vem de `DUMBMONIT_SECRET` não há arquivo
para copiar, e nenhum é escrito — mantenha esse valor em seu gerenciador
de senhas em vez disso. A seção Backup diz em qual dos dois casos você
está.

### Saber que rodou {#knowing-that-it-ran}

Cada execução publica três séries:

| Métrica | Significado |
|---|---|
| `dumbmonit_instance_backup_last_success_seconds` | Timestamp Unix do último backup **bem sucedido**. Uma tentativa falhada nunca o atualiza. |
| `dumbmonit_instance_backup_last_status` | `1` se a última tentativa teve sucesso, `0` se falhou. |
| `dumbmonit_instance_backup_size_bytes` | Tamanho do último arquivo escrito. |

A regra integrada **DumbMonit backup did not run**
(`instance_backup_missing`) dispara quando o último sucesso tem mais de
dois dias — uma noite perdida pode ser um reinício, duas é um problema.
A série só existe uma vez que um primeiro backup foi escrito, então uma
instância com a programação desligada nunca é incomodada com isso.

**Configurações → Backup** mostra a mesma coisa sem MetricsQL: a última
execução e se teve sucesso, os arquivos mantidos e seu tamanho, e
**Fazer backup agora** para escrever um imediatamente.

### Restaurando um {#restoring-one}

Pare a stack, coloque o arquivo de volta sob o nome do banco de dados, e
restaure a chave ao lado dele:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data alpine sh -c '
  cp /data/backups/dumbmonit-20260922-030000.db /data/dumbmonit.db &&
  cp /data/backups/dumbmonit-20260922-030000.key /data/secret.key &&
  rm -f /data/dumbmonit.db-wal /data/dumbmonit.db-shm'
docker compose start
```

Remover os arquivos `-wal` e `-shm` importa: eles pertencem ao banco de
dados que você acabou de substituir, e o SQLite tentaria aplicá-los ao
novo.

Para ler um backup sem restaurá-lo, copie-o para fora e abra-o com
`sqlite3`:

```bash
docker run --rm -v dumbmonit-data:/data -v "$PWD:/out" alpine \
  cp /data/backups/dumbmonit-20260922-030000.db /out/
sqlite3 dumbmonit-20260922-030000.db 'SELECT name, kind, address FROM targets;'
```

Credenciais nesse arquivo permanecem ilegíveis sem o `secret.key`
correspondente.

## Arquivando todo o volume {#archiving-the-whole-volume}

A única forma de manter os gráficos também. Um volume nomeado,
`dumbmonit-data`, mantém tudo: o banco de dados, o segredo, os backups
locais e os dados do VictoriaMetrics.

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Para restaurar, crie o volume, extraia o arquivo nele da mesma forma,
depois `docker compose up -d`.

Se o espaço importa, `--exclude=./vm` mantém o arquivo pequeno: o banco
de dados e o segredo são a configuração, `vm/` é só o histórico.

## O que fazer antes de uma atualização {#what-to-do-before-an-upgrade}

1. **Configurações → Backup → Fazer backup agora** — trinta segundos, e
   inclui o segredo.
2. Baixe um pacote também se a atualização for grande, ou se você está
   mudando para outra máquina.
3. `docker compose pull && docker compose up -d`. Migrações rodam na
   inicialização.

Se der errado, [restaure o backup local](#restoring-one) e inicie a tag
de imagem anterior.
